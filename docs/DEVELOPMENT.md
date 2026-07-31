# gravixlayer-cli — Development Guide

## Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust stable toolchain | stable (see `rust-toolchain.toml`) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `cross` (cross-compilation) | latest | `cargo install cross --git https://github.com/cross-rs/cross --locked` |
| Docker | any | Required by `cross` for Linux musl targets |

---

## Local Build

```bash
git clone https://github.com/gravixlayer/gravixlayer-cli
cd gravixlayer-cli

# Debug build (fast compile, no optimizations)
cargo build

# Release build (optimized, ~10MB binary)
cargo build --release

# Run directly via cargo
cargo run -- --help
cargo run -- runtime create --wait
cargo run -- agent create

# Symlink for local development
ln -sf "$PWD/target/debug/gravixlayer" "$PWD/target/debug/grx"
export PATH="$PWD/target/debug:$PATH"
grx --version
```

---

## Running Tests

```bash
# Unit tests (in src/) + CLI smoke tests (tests/cli_smoke.rs)
cargo test --locked

# Live E2E against a real project (not run in CI)
export GRAVIXLAYER_API_KEY=your_key
./scripts/test_cli.sh

# Single test by name
cargo test test_retry_backoff
cargo test --test cli_smoke

# Debug logging during tests
RUST_LOG=debug cargo test -- --nocapture
```

### Test layout

```
src/**/tests.rs modules   # Unit tests co-located with code (~59+)
tests/cli_smoke.rs        # assert_cmd smoke tests (help/version/completions)
scripts/test_cli.sh       # Optional live API E2E (requires GRAVIXLAYER_API_KEY)
```

Unit coverage focuses on terminal framing, retry/backoff, scaffold/archive,
config, and agent project discovery. HTTP client mocking via `httpmock` is
available as a dev-dependency for future API-layer tests.

---

## Cross-Compilation

All release targets. Linux musl builds use `cross` (Docker-based); macOS and Windows
builds run on native GitHub Actions runners.

| Target | Command |
|---|---|
| `x86_64-unknown-linux-musl` | `cross build --release --target x86_64-unknown-linux-musl` |
| `aarch64-unknown-linux-musl` | `cross build --release --target aarch64-unknown-linux-musl` |
| `x86_64-apple-darwin` | `cargo build --release --target x86_64-apple-darwin` |
| `aarch64-apple-darwin` | `cargo build --release --target aarch64-apple-darwin` |
| `x86_64-pc-windows-msvc` | `cargo build --release --target x86_64-pc-windows-msvc` |

Linux musl targets produce **fully static binaries** with no shared library dependencies.
The binary can be dropped onto any Linux system without installing glibc.

### Why rustls, not OpenSSL?

`reqwest` is configured with:

```toml
[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream", "multipart"] }
```

This eliminates the OpenSSL dependency entirely, which enables fully static `musl` linking
without requiring a cross-compiled OpenSSL. Binary size stays small and build times are shorter.

---

## Project Structure

```
src/
├── main.rs              # #[tokio::main], build AppContext, dispatch to cmd::*
├── cli.rs               # All clap subcommands and arg structs (derive API)
├── ctx.rs               # AppContext { api, config, project, output }
├── config/
│   ├── mod.rs           # UserConfig: loads ~/.gravixlayer/config.toml
│   └── project.rs       # GravixlayerProject: loads gravixlayer/gravixlayer.json
├── api/
│   ├── mod.rs           # ApiClient::new() — reqwest client with auth + retry
│   ├── error.rs         # ApiError enum (thiserror): 5 typed variants
│   ├── retry.rs         # retry_with_backoff: (1<<n)*1000 + rand(0..1000) ms
│   ├── runtime.rs       # All runtime API calls
│   ├── template.rs      # All template API calls
│   ├── agent.rs         # All agent API calls
│   └── billing.rs       # All billing API calls
├── output/
│   ├── mod.rs           # OutputMode enum (Human | Json | Quiet), spinner factory
│   └── table.rs         # comfy-table builders for runtime, template, agent
├── cmd/
│   ├── auth.rs
│   ├── config.rs
│   ├── runtime/
│   │   ├── mod.rs
│   │   ├── lifecycle.rs  # create/list/get/kill/pause/resume/metrics
│   │   ├── exec.rs       # exec, run
│   │   ├── shell.rs      # WebSocket PTY
│   │   ├── files.rs
│   │   ├── git.rs
│   │   ├── ssh.rs
│   │   └── context.rs
│   ├── template.rs
│   ├── agent/
│   │   ├── mod.rs
│   │   ├── create.rs     # dialoguer wizard + scaffold file generation
│   │   ├── dev.rs        # ephemeral runtime + notify watcher
│   │   ├── build.rs      # tar.gz assembly + multipart upload
│   │   ├── deploy.rs     # build → deploy pipeline
│   │   └── ops.rs        # list/get/invoke/stream/logs/destroy
│   ├── billing.rs
│   ├── completions.rs
│   └── update.rs
├── terminal/
│   ├── protocol.rs       # Binary frame encode/decode (0x01–0x04)
│   └── pty.rs            # crossterm raw mode + SIGWINCH resize
└── scaffold/
    ├── wizard.rs         # 8-step dialoguer wizard
    └── templates/        # Embedded via include_str!()
        ├── langgraph/    # agent.py, pyproject.toml
        ├── openai_agents/
        ├── google_adk/
        ├── claude_agent/
        ├── langchain/
        ├── custom/
        └── common/       # .gitignore, gravixlayer.json.tmpl
```

---

## Key Invariants

These must be maintained across all contributions:

- **Commands never contain HTTP logic** — all API calls live exclusively in `src/api/`
- **`AppContext` is always `&AppContext`** — never cloned, never owned by a command
- **API key is always `secrecy::Secret<String>`** — never printed, never in logs, zeroized on drop
- **`--json` bypasses all formatting** — outputs raw `serde_json::Value`, no color, no tables
- **All async I/O uses `tokio`** — no `std::thread::spawn` for network or file operations
- **`anyhow::Result` in commands; `ApiError` in `api/`** — never mix the two error types

---

## Adding a New Command

### 1. Add the arg struct to `src/cli.rs`

```rust
#[derive(Subcommand)]
pub enum RuntimeCommand {
    // existing variants ...
    MyNew(MyNewArgs),
}

#[derive(Args)]
pub struct MyNewArgs {
    /// Runtime ID
    pub runtime_id: String,

    /// Optional flag with a default
    #[arg(long, default_value = "default")]
    pub mode: String,
}
```

### 2. Add the API call to `src/api/runtime.rs`

```rust
pub async fn my_new_operation(
    &self,
    runtime_id: &str,
    mode: &str,
) -> Result<MyNewResponse, ApiError> {
    let resp = self
        .client
        .post(format!("{}/v1/runtime/{}/my-new", self.base_url, runtime_id))
        .json(&serde_json::json!({ "mode": mode }))
        .send()
        .await?;
    self.handle_response::<MyNewResponse>(resp).await
}
```

### 3. Add the command handler in `src/cmd/runtime/`

```rust
// src/cmd/runtime/my_new.rs
use crate::ctx::AppContext;
use crate::cli::MyNewArgs;

pub async fn run(ctx: &AppContext, args: &MyNewArgs) -> anyhow::Result<()> {
    let spinner = ctx.output.spinner("Running operation...");
    let result = ctx.api
        .runtime
        .my_new_operation(&args.runtime_id, &args.mode)
        .await?;
    spinner.finish_and_clear();
    ctx.output.print_or_json(&result)?;
    Ok(())
}
```

### 4. Wire it in `src/cmd/runtime/mod.rs`

```rust
RuntimeCommand::MyNew(args) => my_new::run(ctx, args).await,
```

### 5. Add a mock test in `tests/mock/api_client.rs`

```rust
#[tokio::test]
async fn test_my_new_operation() {
    let server = MockServer::start_async().await;
    server.mock(|when, then| {
        when.method(POST)
            .path("/v1/runtime/rt_test/my-new")
            .json_body(json!({ "mode": "default" }));
        then.status(200).json_body(json!({ "result": "ok" }));
    });
    let client = ApiClient::new(server.base_url(), "test-key", 30).unwrap();
    let resp = client.runtime.my_new_operation("rt_test", "default").await.unwrap();
    assert_eq!(resp.result, "ok");
}
```

---

## Adding a New Framework Template

### 1. Create the template directory

```
src/scaffold/templates/<framework_name>/
├── agent.py
├── main.py
└── pyproject.toml
```

Use `{{variable_name}}` for template substitution. Available variables:

| Variable | Example |
|---|---|
| `{{agent_name}}` | `MyAgent` (PascalCase) |
| `{{agent_name_kebab}}` | `my-agent` (kebab-case) |
| `{{model_name}}` | `claude-sonnet-4-5-20250514` |
| `{{model_provider}}` | `anthropic` |
| `{{api_key_env}}` | `ANTHROPIC_API_KEY` |
| `{{python_version}}` | `3.12` |
| `{{http_port}}` | `8080` |

### 2. Embed in `src/scaffold/wizard.rs`

```rust
const MY_FRAMEWORK_AGENT: &str = include_str!("templates/my_framework/agent.py");
const MY_FRAMEWORK_MAIN:  &str = include_str!("templates/my_framework/main.py");
const MY_FRAMEWORK_PYPROJECT: &str = include_str!("templates/my_framework/pyproject.toml");
```

### 3. Add the `Framework` variant

In `src/cli.rs` (or wherever the `Framework` enum lives), add the new variant.
Add the corresponding match arm in the wizard and the model provider matrix.

---

## Release Process

Tag-driven releases. Version lives in **one place**: `Cargo.toml` `[package].version`.
Git tags are `v` + that version. The release workflow fails if they disagree.

| Kind | Version example | Tag | GitHub Release | `curl …/install \| sh` |
|------|-----------------|-----|----------------|------------------------|
| Stable | `0.1.0` | `v0.1.0` | latest | installs this |
| Patch | `0.1.1` | `v0.1.1` | latest | installs this |
| Minor | `0.2.0` | `v0.2.0` | latest | installs this |
| Major | `1.0.0` | `v1.0.0` | latest | installs this |
| Alpha | `0.2.0-alpha.1` | `v0.2.0-alpha.1` | **prerelease** | ignored (stays on latest stable) |
| Beta | `0.2.0-beta.1` | `v0.2.0-beta.1` | **prerelease** | ignored |

Pin any build: `GRAVIXLAYER_VERSION=v0.2.0-alpha.1 curl -fsSL https://cli.gravixlayer.ai/install | sh`

| Asset | Name |
|-------|------|
| Unix | `gravixlayer-<tag>-<rust-triple>.tar.gz` (+ `.sha256`) |
| Windows | `gravixlayer-<tag>-<rust-triple>.zip` (+ `.sha256`) |
| Installers | `install.sh`, `install.ps1`, `SHA256SUMS` |

### When to bump (and when not to)

| Change type | Commit prefix | Bump version? | Ship a tag? |
|-------------|---------------|---------------|-------------|
| Docs, CI, comments, ops | `chore:` / `docs:` | No | No |
| Bug fix for users | `fix:` (work commit) | Later → **patch** | When you cut release |
| New feature, compatible | `feat:` (work commit) | Later → **minor** or alpha | When you cut release |
| Breaking CLI/API | `feat!:` / `fix!:` | Later → **major** | When you cut release |
| Risky / unfinished feature | `feat:` | Later → **alpha** | When you cut release |
| Cutting a release | `chore: release X.Y.Z` | Yes (in that commit) | Yes (tag after) |

Work commits use `fix:` / `feat:` / `chore:`. The version-bump commit is always `chore: release …` — not `fix:`.

Merging a `fix:` to `main` without tagging leaves it unreleased: installed CLIs
keep the last tagged build. Anything users have to run to benefit from — an
installer, updater, or dependency fix — needs a patch tag before `gravixlayer
update` will pick it up.

### Cutting a release

Releases are cut by maintainers and are entirely tag-driven, so there is nothing
to run by hand beyond the version bump:

1. Set `[package].version` in `Cargo.toml` to the new version and refresh
   `Cargo.lock` (`cargo metadata --format-version 1 --no-deps >/dev/null`).
2. Move the `Unreleased` notes in `CHANGELOG.md` under the new version heading.
3. Commit as `chore: release X.Y.Z` and push to `main`.
4. Tag `vX.Y.Z` and push the tag.

Pushing the tag is what publishes: the release workflow builds every target,
generates checksums, and attaches the binaries and installers to the GitHub
Release. A tag whose version disagrees with `Cargo.toml` fails the workflow
rather than publishing a mislabelled build.

Verify a release the way a user would — `gravixlayer update`, or the install
command from the [README](../README.md).

### Version is the single source of truth

The version is defined once in `Cargo.toml` under `[package] version`.
It is embedded at compile time via `env!("CARGO_PKG_VERSION")`.
Git tags use a `v` prefix matching that version.

---

## Debug Logging

Enable structured debug output with the `GRAVIXLAYER_LOG` environment variable:

```bash
GRAVIXLAYER_LOG=debug grx runtime create
GRAVIXLAYER_LOG=trace grx agent build
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

The `tracing-subscriber` is configured with `EnvFilter` — per-module filtering works:

```bash
GRAVIXLAYER_LOG=gravixlayer_cli::api=debug grx runtime list
```

---

## Linting and Formatting

```bash
# Format
cargo fmt

# Lint (same flags as CI)
cargo clippy --all-targets --locked -- -D warnings

# Both (recommended before committing)
cargo fmt && cargo clippy --all-targets --locked -- -D warnings
```

CI enforces fmt, clippy (`-D warnings`), and `cargo test --locked` on Ubuntu, macOS,
and Windows, plus release-target builds for Linux musl, Darwin, and Windows.
