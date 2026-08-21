# gravixlayer-cli — Architecture

## Overview

`gravixlayer` (aliased as `grx`) is a single statically-linked Rust binary that provides
complete control over the Gravixlayer platform from the terminal. Speed is the primary
design goal: zero mandatory flags for common operations, sub-100ms command startup,
and smart defaults derived from the project config file.

## Design Principles

1. **Minimal input** — `grx runtime create` works with zero flags. All defaults mirror the
   Python SDK: `template=base-small`, `cloud=aws`, `region=us-east-1`.
2. **AppContext pattern** — A single `AppContext` struct (api client + config + project +
   output mode) is built once in `main.rs` and passed as `&AppContext` to every command.
   No globals, no thread-locals.
3. **API layer as the abstraction** — All HTTP logic lives in `src/api/`. Commands are thin
   presentation wrappers: call `api::*`, format output.
4. **Config cascade** — Resolution order (highest wins):
   `CLI flag → GRAVIXLAYER_* env var → ~/.gravixlayer/config.toml → gravixlayer/gravixlayer.json`
5. **Project-aware** — Inside a directory containing `gravixlayer/gravixlayer.json`,
   commands like `grx agent dev` and `grx agent deploy` need no additional arguments.

## Repository Layout

```
gravixlayer-cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry: parse CLI, build AppContext, dispatch
│   ├── cli.rs               # clap derive tree — all subcommands and flags
│   ├── ctx.rs               # AppContext struct
│   ├── config/
│   │   ├── mod.rs           # UserConfig — loads ~/.gravixlayer/config.toml
│   │   └── project.rs       # GravixlayerProject — loads gravixlayer/gravixlayer.json
│   ├── api/
│   │   ├── mod.rs           # ApiClient::new(base_url, api_key, timeout)
│   │   ├── error.rs         # ApiError (thiserror): 5 typed variants
│   │   ├── retry.rs         # Exponential backoff with Retry-After support
│   │   ├── runtime.rs       # All runtime HTTP calls
│   │   ├── template.rs      # Template HTTP calls
│   │   ├── agent.rs         # Agent HTTP calls (build, deploy, invoke, logs)
│   │   └── billing.rs       # Billing HTTP calls
│   ├── output/
│   │   ├── mod.rs           # OutputMode enum, print helpers, spinner factory
│   │   └── table.rs         # comfy-table builders for each resource type
│   ├── cmd/
│   │   ├── auth.rs          # login / logout / status / whoami
│   │   ├── config.rs        # set / get / list
│   │   ├── runtime/
│   │   │   ├── mod.rs
│   │   │   ├── lifecycle.rs  # create/list/get/kill/pause/resume/metrics
│   │   │   ├── exec.rs       # exec, run (code evaluation)
│   │   │   ├── shell.rs      # WebSocket PTY — calls terminal/*
│   │   │   ├── files.rs      # ls/cat/write/upload/download/rm/mkdir/chmod
│   │   │   ├── git.rs        # clone/status/branch/checkout/pull/push/...
│   │   │   ├── ssh.rs        # enable/disable/status
│   │   │   └── context.rs    # context create/get/delete
│   │   ├── template.rs       # list/get/build/build-status/delete
│   │   ├── agent/
│   │   │   ├── mod.rs
│   │   │   ├── create.rs     # dialoguer wizard → scaffold files
│   │   │   ├── dev.rs        # ephemeral runtime + notify file watcher loop
│   │   │   ├── build.rs      # tar.gz + multipart upload + poll build status
│   │   │   ├── deploy.rs     # build → deploy pipeline
│   │   │   └── ops.rs        # list/get/invoke/stream/logs/destroy
│   │   ├── billing.rs        # summary/history/quota
│   │   ├── completions.rs    # shell completion generation
│   │   └── update.rs         # self-upgrade via GitHub Releases (native reqwest)
│   ├── terminal/
│   │   ├── protocol.rs       # Binary frame encode/decode (0x01–0x04)
│   │   └── pty.rs            # crossterm raw mode, SIGWINCH resize
│   └── scaffold/
│       ├── wizard.rs         # 8-step agent create wizard
│       └── templates/        # Embedded via include_str!()
│           ├── langgraph/
│           ├── openai_agents/
│           ├── google_adk/
│           ├── claude_agent/
│           ├── langchain/
│           ├── custom/
│           └── common/       # .gitignore, gravixlayer.json (Handlebars template)
├── scripts/
│   ├── install.sh
│   └── install.ps1
└── docs/
    ├── ARCHITECTURE.md  ← this file
    ├── COMMANDS.md
    ├── INSTALL.md
    └── DEVELOPMENT.md
```

## AppContext Data Flow

```
main.rs
  │
  ├── parse_args()               → Cli struct (clap)
  ├── resolve_api_key()          → CLI flag → GRAVIXLAYER_API_KEY env → keyring
  ├── load_user_config()         → ~/.gravixlayer/config.toml
  ├── find_project_config()      → walk up dirs for gravixlayer/gravixlayer.json
  ├── build ApiClient            → reqwest client + base_url + auth header + retry
  ├── determine OutputMode       → Human (tty) | Json (--json) | Quiet (--quiet)
  └── AppContext { api, config, project, output }
        │
        └── dispatch to cmd::*::run(&ctx, args)
```

## API Client

`src/api/mod.rs` — `ApiClient` wraps a `reqwest::Client` configured with:

- `rustls-tls` (no OpenSSL dependency — enables fully static musl linking)
- Default timeout: 60s (overridable via `--timeout` or `GRAVIXLAYER_TIMEOUT`)
- `Authorization: Bearer <api_key>` header on every request
- API key stored as `secrecy::Secret<String>` in memory; `zeroize::Zeroize` on drop
- Base URL: `https://api.gravixlayer.ai` (overridable via `GRAVIXLAYER_BASE_URL`)

### Error Types (`src/api/error.rs`)

Mirrors the Python SDK's 5 error types:

| Variant | HTTP Status | Retried? |
|---|---|---|
| `ApiError::Auth` | 401 | No |
| `ApiError::RateLimit { retry_after }` | 429 | Yes (after Retry-After delay) |
| `ApiError::BadRequest { body }` | 400–499 (excl. 401/429) | No |
| `ApiError::Server { status, body }` | 5xx | Yes |
| `ApiError::Connection { source }` | Network error | Yes |

### Retry Logic (`src/api/retry.rs`)

Mirrors the Python SDK's backoff exactly:

```
delay_ms = (1 << attempt) * 1000 + rand(0..1000)
```

- Max 3 retries (4 total attempts)
- Respects `Retry-After` header on 429 responses
- Retryable statuses: 502, 503, 504, and `ApiError::Connection`

## WebSocket Terminal Protocol

`grx runtime shell <id>` upgrades to a WebSocket at `GET /v1/runtime/<id>/terminal`.

Authentication uses `Authorization: Bearer <api_key>` as an HTTP header during the
upgrade handshake — not `?token=` query param, which is reserved for browser clients.

### Binary Frame Format

GravixLayer terminal clients use this binary frame format:

**Client → Server:**

| First byte | Payload | Meaning |
|---|---|---|
| `0x01` | UTF-8 bytes | Keyboard / paste input |
| `0x02` | cols(u16be) + rows(u16be) — 4 bytes, big-endian | Terminal resize |
| `0x03` | empty | Close / disconnect |

**Server → Client:**

| First byte | Payload | Meaning |
|---|---|---|
| `0x01` | UTF-8 bytes | PTY output |
| `0x02` | pid(u32be) + session_id_len(u16be) + session_id_bytes | Session ready |
| `0x03` | exit_code(i32be) + status_len(u16be) + status_bytes | Process exited |
| `0x04` | fatal(u8) + UTF-8 message | Error |

> All multi-byte integers are **big-endian** (network byte order) matching the Go and TypeScript implementations.

Implementation: `src/terminal/protocol.rs`. Raw mode via `crossterm`. SIGWINCH resize
events via `tokio::signal::unix`. WebSocket transport via `tokio-tungstenite`.

## Agent Build Pipeline

`grx agent build` (and the build phase of `grx agent deploy`):

1. Load `GravixlayerProject` from `gravixlayer/gravixlayer.json`
2. Walk source tree with `walkdir`, excluding:
   `__pycache__`, `.git`, `.venv`, `venv`, `env`, `.env`, `node_modules`, `dist`,
   `build`, `*.egg-info`, `.DS_Store`, `.mypy_cache`, `.pytest_cache`, `.ruff_cache`, `.tox`
3. Assemble `tar.gz` in a `tempfile::TempDir` using `tar` + `flate2`
4. `POST /v1/agents/template/build-agent` — multipart body:
   - `archive`: tar.gz file bytes
   - `metadata`: JSON form field (name, framework, entry_point, python_version, resources)
5. Poll `GET /v1/agents/build-status/<build_id>` with `indicatif` spinner until
   `status=complete` or `status=failed`

## Agent Dev Mode

`grx agent dev [dir]` — ephemeral cloud VM with hot-reload:

1. Load `GravixlayerProject`
2. `api::runtime::create()` → wait for `status=running`
3. Upload source files to runtime (standard exclusion list applies)
4. `notify::RecommendedWatcher` on `app/` directory
5. On file change event: re-upload changed file → call `/v1/runtime/<id>/notify-reload`
6. Print runtime endpoint URL
7. Ctrl+C → `api::runtime::kill(<id>)` → exit

No public URL is assigned. The runtime is destroyed when the session ends.

## Scaffold Templates

`grx agent create` generates a project directory from embedded templates.
All template files are compiled into the binary via `include_str!()` — no network
required, works completely offline.

Template variables (Handlebars-style `{{var}}`):

| Variable | Example |
|---|---|
| `{{agent_name}}` | `MyAgent` (PascalCase) |
| `{{agent_name_kebab}}` | `my-agent` (kebab-case) |
| `{{model_name}}` | `claude-sonnet-4-5-20250514` |
| `{{model_provider}}` | `anthropic` |
| `{{api_key_env}}` | `ANTHROPIC_API_KEY` |
| `{{python_version}}` | `3.12` |
| `{{http_port}}` | `8080` |

### Framework Scaffolding Matrix

| Framework | Key Imports | Dependencies |
|---|---|---|
| LangGraph | `from langgraph.graph import StateGraph, MessagesState, START, END`; production served by `/usr/local/bin/gravixlayer`, local dev by `gravixlayer agent serve` | `langgraph>=1.0, langgraph-checkpoint, langchain>=1.0` |
| OpenAI Agents | `from agents import Agent, Runner, function_tool` | `openai-agents>=0.14, fastapi, uvicorn` |
| Google ADK | `from google.adk.agents import Agent`; production served by `/usr/local/bin/gravixlayer`, local dev by `gravixlayer agent serve` | `google-adk>=1.0` |
| Claude Agent SDK | `from claude_agent_sdk import query, ClaudeAgentOptions` | `claude-agent-sdk>=0.1, fastapi, uvicorn, anyio` |
| LangChain | `from langchain.agents import create_agent`; production served by `/usr/local/bin/gravixlayer`, local dev by `gravixlayer agent serve` | `langchain>=1.0, langchain-anthropic, langchain-openai` |
| Custom | FastAPI stub with `/invoke` POST + `/health` GET | `fastapi, uvicorn` |

For LangGraph, LangChain, and Google ADK, the lifecycle CLI owns local development and control-plane operations. Production templates run the Python SDK runtime through the image-local `/usr/local/bin/gravixlayer` command; the runtime loads user framework code and exposes HTTP/A2A without installing the lifecycle CLI inside the agent image.

## Crate Selection Rationale

| Crate | Version | Role | Why |
|---|---|---|---|
| `clap` + `clap_complete` | 4.x | Arg parsing, shell completions | Industry standard; compile-time checked args |
| `tokio` | 1.x | Async runtime | De-facto standard; `full` feature set |
| `reqwest` | 0.12 | HTTP client | `rustls-tls` eliminates OpenSSL; enables static musl binary |
| `a2a-rs` | 0.2 | A2A server protocol | Official Rust A2A types, agent-card routes, JSON-RPC routes, REST routes, and task handler abstractions |
| `serde` + `serde_json` | 1.x | Serialization | Universal Rust standard |
| `indicatif` | 0.17 | Spinners, progress bars | Best-in-class; multi-progress support |
| `console` | 0.15 | ANSI color, isatty detection | Works correctly on all platforms |
| `dialoguer` | 0.11 | Wizard prompts | Handles non-TTY, Ctrl+C, and edge cases correctly |
| `crossterm` | 0.28 | Raw terminal mode | Cross-platform; pairs with tokio-tungstenite |
| `comfy-table` | 7.x | Table output | Clean API; Unicode-aware column widths |
| `toml` | 0.8 | Config file | Serde-compatible; standard Rust config format |
| `dirs` | 5.x | `~/.gravixlayer/` path | Correct XDG/platform paths on all OSes |
| `keyring` | 3.x | OS credential store | Keychain (macOS), Credential Manager (Win), libsecret (Linux) |
| `anyhow` | 1.x | Error context in commands | Simple propagation at application layer |
| `thiserror` | 2.x | Typed errors in `api/` | Library-quality errors with structured variants |
| `tracing` + `tracing-subscriber` | 0.1/0.3 | Structured logging | `GRAVIXLAYER_LOG=debug` for debugging |
| `tokio-tungstenite` | 0.24 | WebSocket PTY | Async WS on tokio; no blocking |
| `tar` + `flate2` | 0.4/1.x | Agent build archive | Pure Rust; no system tar dependency |
| `walkdir` | 2.x | Directory traversal | Handles symlinks, permissions correctly |
| `notify` | 7.x | File watcher | Cross-platform; debounced events |
| `secrecy` + `zeroize` | 0.8/1.x | Secret memory hygiene | API key never appears in heap dumps or logs |
| `tempfile` | 3.x | Temp staging dir for tar | Auto-cleaned on drop |
| GitHub Actions `release.yml` | release | Tag → build → checksum → GitHub Release |
| `assert_cmd` + `httpmock` | dev | Testing | CLI integration + HTTP mock tests |

## Release Targets

| Target Triple | Build Method | Binary Name |
|---|---|---|
| `x86_64-unknown-linux-musl` | `cross` (Docker) | `gravixlayer-linux-x86_64` |
| `aarch64-unknown-linux-musl` | `cross` (Docker) | `gravixlayer-linux-aarch64` |
| `x86_64-apple-darwin` | macOS GitHub runner | `gravixlayer-darwin-x86_64` |
| `aarch64-apple-darwin` | macOS GitHub runner | `gravixlayer-darwin-aarch64` |
| `x86_64-pc-windows-msvc` | Windows GitHub runner | `gravixlayer-windows-x86_64.exe` |

Linux musl targets produce fully static binaries — no shared library dependencies,
deployable to any Linux system without glibc.

## v1 Scope Exclusions

The following are deferred to v2:

- Admin / team management endpoints
- Wallet, payments, invoice management
- ratatui TUI dashboard
