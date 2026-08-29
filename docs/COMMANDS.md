# gravixlayer-cli — Command Reference

## Global Flags

These flags apply to every command:

| Flag | Short | Default | Description |
|---|---|---|---|
| `--api-key <KEY>` | | env / keyring | API key (overrides stored key) |
| `--profile <NAME>` | | `default` | Config profile to use |
| `--base-url <URL>` | | `https://api.gravixlayer.ai` | API base URL |
| `--json` | | false | Output raw JSON instead of formatted tables |
| `--quiet` | `-q` | false | Suppress all output except errors |
| `--verbose` | `-v` | false | Enable debug logging (`GRAVIXLAYER_LOG=debug`) |
| `--timeout <SECS>` | | `60` | HTTP request timeout in seconds |
| `--help` | `-h` | | Print help |
| `--version` | `-V` | | Print version |

Primary binary: `gravixlayer` — aliased as `grx` (install script creates the symlink).

---

## Command Hierarchy

```
grx / gravixlayer [GLOBAL FLAGS]
├── auth        login | logout | status | token
├── config      show | set <key> <val> | unset <key> | profiles | use-profile <name>
├── runtime
│   ├── create  [--template] [--cloud] [--region] [--timeout] [--env K=V]... [--wait] [--name]
│   ├── list    [--limit N]
│   ├── get     <id>
│   ├── kill    <id>
│   ├── pause   <id>
│   ├── resume  <id>
│   ├── metrics <id>
│   ├── timeout <id> <seconds>
│   ├── connect <id>
│   ├── exec    <id> <cmd> [args...]
│   ├── run     <id> (--code <expr> | --file <path>) [--lang] [--timeout]
│   ├── shell   <id>
│   ├── context show|set|clear
│   ├── code-context <id>
│   ├── service web-url|list|revoke
│   ├── ssh     enable|disable|status
│   ├── files   upload|download|ls|cat|write|write-many|info|rm|mkdir|chmod
│   └── git     clone|status|branch|checkout|pull|push|fetch|add|commit|branch-create|branch-delete
├── template    list|get|snapshot|build|status|delete
├── agent
│   ├── init | create
│   ├── dev     [dir]
│   ├── up      [dir]
│   ├── build   [dir] [--wait]
│   ├── status  <build-id>
│   ├── deploy  [dir] [--wait | --no-wait]
│   ├── get     <id>
│   ├── invoke  <id> (--input JSON | --input-file path)
│   ├── stream  <id> (--input JSON | --input-file path)
│   ├── package | dockerfile | serve
│   └── destroy <id> [--force]
├── provider    create|list|get|update|delete
│                 add-secret|list-secrets|update-secret|delete-secret
│                 attach|detach|list-attached
├── network-policy
│                create|list|get|update|delete
│                add-rule|list-rules|update-rule|delete-rule
│                attach|detach|list-attached
├── billing     summary [--month YYYY-MM] [--project-id] | history | quotas
├── validate    [dir]
├── package     [dir] [--output <path>]
├── completions bash|zsh|fish|powershell|elvish
├── doctor
└── update      [--check | --version <VERSION>]
```

---

## auth

Manage API key authentication.

### `grx auth login`

Store an API key and validate it against the API.

```
grx auth login [--api-key <KEY>]
```

If `--api-key` is omitted, an interactive password prompt is shown. The key is
validated with an authenticated control-plane call (`GET /v1/agents/runtime?limit=1`)
and stored in the OS keychain via `keyring`.

### `grx auth logout`

Remove the stored API key from the OS keychain.

### `grx auth status`

Check whether a stored API key is present and valid. Prints the user email and account ID.

---

## config

Manage persistent user configuration stored in `~/.gravixlayer/config.toml`.

### `grx config set <key> <value>`

```bash
grx config set base_url https://api.gravixlayer.ai
grx config set default_template base-small
grx config set default_region us-east-1
grx config set default_cloud aws
```

### `grx config show`

Print the resolved configuration for the active profile.

### `grx config unset <key>`

Remove a stored value so the built-in default applies again.

### `grx config profiles`

List configured profiles and show which one is active.

### `grx config use-profile <name>`

Switch the active profile. A single command can override it with `--profile`,
or the `GRAVIXLAYER_PROFILE` environment variable.

---

## runtime

Manage sandboxes — isolated, hardware-virtualized environments for running code.

### `grx runtime create`

Create a new runtime. All flags are optional — zero-flag invocation works immediately.

```
grx runtime create [OPTIONS]

Options:
  --template <NAME>     VM image template      [default: base-small]
  --cloud <CLOUD>       Cloud provider         [default: aws]
  --region <REGION>     Region                 [default: us-east-1]
  --timeout <SECS>      Max idle timeout       [default: none]
  --env <K=V>           Set environment var    (repeatable)
  --wait                Poll until status=running before returning
  --name <NAME>         Optional display name

Examples:
  grx runtime create
  grx runtime create --template base-large --region westus2 --wait
  grx runtime create --env PYTHONPATH=/app --env DEBUG=1 --wait
```

Known templates: `base-small`, `base-medium`, `base-large`.

### `grx runtime list`

List all runtimes for the authenticated account.

```
grx runtime list [--limit <N>]

Columns: ID | Name | Status | Template | Region | Created
```

### `grx runtime get <id>`

Print detailed info for a single runtime (status, endpoints, metrics, config).

### `grx runtime kill <id>`

Terminate a runtime immediately. This action is irreversible.

### `grx runtime pause <id>`

Freeze the runtime. Billing pauses and the runtime's state is preserved as a
snapshot, with memory and running processes frozen intact.

### `grx runtime resume <id>`

Restore a paused runtime from its snapshot. VM is live again in under 200ms.

### `grx runtime metrics <id>`

Print current resource usage:

```
Columns: CPU% | Memory Used | Memory Total | Disk Used | Disk Total
```

### `grx runtime exec <id> <cmd> [args...]`

Execute a one-shot command inside the runtime and stream its output to stdout.

```bash
grx runtime exec abc123 python --version
grx runtime exec abc123 pip install numpy
grx runtime exec abc123 bash -c "ls -la /app"
```

### `grx runtime run <id>`

Evaluate code or a script file inside the runtime. Each `run` is one-shot:
variables from one call are not available in the next. Create a context with
`grx runtime context` and pass it when you need interpreter state to persist.

```
grx runtime run <id> --code <EXPR>
grx runtime run <id> --file <PATH>

Options:
  --code <EXPR>     Inline code expression to evaluate
  --file <PATH>     Local script file to upload and execute
  --lang <LANG>     Language hint                    [default: python]
  --timeout <SECS>  Execution timeout in seconds

Examples:
  grx runtime run abc123 --code "x = 42; print(f'x = {x}')"
  grx runtime run abc123 --file ./analysis.py
```

### `grx runtime shell <id>`

Open an interactive PTY shell session over WebSocket.

```
grx runtime shell <id>

Connects to: GET /v1/runtime/<id>/terminal (WebSocket upgrade)
Auth:         Authorization: Bearer <api_key> header (not ?token= query param)
Protocol:     Binary frames — 0x01 input/output, 0x02 resize/ready, 0x03 close/exit, 0x04 error
Exit:         Ctrl+C or type 'exit'
```

Terminal size is synced on connect and on SIGWINCH (window resize).

### `grx runtime context`

Manage persistent execution contexts within a runtime. Pass the returned
`context_id` to keep interpreter state across `run` calls.

```
grx runtime context create <runtime-id> [--name <NAME>]
grx runtime context get    <runtime-id> <context-id>
grx runtime context delete <runtime-id> <context-id>
```

### `grx runtime ssh`

Manage SSH access to the runtime.

```
grx runtime ssh enable  <id>
grx runtime ssh disable <id>
grx runtime ssh status  <id>
```

### `grx runtime files`

File system operations inside the runtime.

```
grx runtime files ls       <id> [--path <PATH>]
grx runtime files cat      <id> <path>
grx runtime files write    <id> <path> <content>
grx runtime files upload   <id> <local-path> <remote-path>
grx runtime files download <id> <remote-path> <local-path>
grx runtime files rm       <id> <path>
grx runtime files mkdir    <id> <path>
grx runtime files chmod    <id> <path> <mode>
```

### `grx runtime git`

Git operations executed inside the runtime. Every subcommand that works on an
existing checkout takes `--path`, the repository directory inside the runtime.

```
grx runtime git clone          <id> <url> [--target-dir <DIR>] [--branch <B>] [--depth <N>]
grx runtime git status         <id> --path <PATH>
grx runtime git branch         <id> --path <PATH> [--all | --remote]
grx runtime git checkout       <id> <branch> --path <PATH>
grx runtime git pull           <id> --path <PATH> [--remote <R>] [--branch <B>]
grx runtime git push           <id> --path <PATH> [--remote <R>] [--refspec <SPEC>]
grx runtime git fetch          <id> --path <PATH> [--remote <R>]
grx runtime git add            <id> --path <PATH> [--files <GLOB>]
grx runtime git commit         <id> --path <PATH> --message <MSG>
grx runtime git branch-create  <id> <name> --path <PATH> [--start-point <REF>]
grx runtime git branch-delete  <id> <name> --path <PATH> [--force]
```

`clone` defaults `--target-dir` to `/workspace/<repository name>`, the same
directory `git clone` would create locally.

**Private repositories.** `clone`, `pull`, `fetch`, and `push` accept
`--auth-token`, which also reads `GRAVIXLAYER_GIT_TOKEN` from the environment so
a workflow need not put the token on the command line. The token authenticates
one operation and is never written into the checkout, so each command that
contacts the remote needs its own. `push` also accepts `--username` and
`--password` for remotes that need a real account; `--auth-token` wins when both
are given.

**Exit codes.** These commands exit with git's own status, so a failed operation
stops a shell chain or a CI step the way it would running git locally:

```bash
grx runtime git clone "$RT" "$URL" --target-dir /workspace/app \
  && grx runtime exec "$RT" -- make test
```

---

## template

Manage VM image templates.

### `grx template list`

```
grx template list [--limit <N>]

Columns: ID | Name | Status | Python Version | Size | Created
```

### `grx template get <id>`

Print detailed template info including installed packages and build config.

### `grx template build`

Build a new template from a Dockerfile or public Docker image.

```
grx template build --name <NAME> [OPTIONS]

Options:
  --dockerfile <FILE>     Path to Dockerfile (contents sent to the API)
  --docker-image <IMAGE>  Public image (e.g. ubuntu:24.04)
  --vcpu-count <N>        vCPUs for the template VM   [default: 2]
  --memory-mb <MB>        Memory in MB                [default: 1024]
  --disk-mb <MB>          Disk size in MB             [default: 4096]
  --wait                  Poll until the build completes

Examples:
  grx template build \
    --dockerfile ./base.Dockerfile \
    --name my-base \
    --vcpu-count 2 \
    --memory-mb 2048 \
    --disk-mb 6144 \
    --wait

  grx template build --docker-image ubuntu:24.04 --name my-ubuntu --wait
```

### `grx template status <build-id>`

Poll the status of a running template build.

### `grx template delete <id>`

Delete a custom template. Built-in templates cannot be deleted.

---

## agent

Build, deploy, and manage AI agents.

### `grx agent create`

Interactive 8-step wizard that scaffolds a new agent project directory.

```
grx agent create

Steps:
  1. Agent name        (alphanumeric + hyphens, e.g. my-agent)
  2. Description       (optional)
  3. Framework         (LangGraph | OpenAI Agents | Google ADK | Claude Agent SDK | LangChain | Custom)
  4. Model provider    (depends on framework: Anthropic | OpenAI | Google | Custom)
  5. Model name        (pre-filled from matrix, or enter custom)
  6. Protocols         (multi-select: http | a2a | mcp)
  7. Resources         (small: 1vCPU/1GB | medium: 2vCPU/2GB | large: 4vCPU/4GB)
  8. Confirm and create

Output structure:
  my-agent/
  ├── gravixlayer/
  │   ├── gravixlayer.json   # project config — source of truth for all grx commands
  │   ├── .env.local         # API keys for model provider (gitignored)
  │   └── state.json         # deployed agent state, auto-managed (gitignored)
  ├── app/
  │   └── MyAgent/
  │       ├── agent.py       # framework-specific agent definition
  │       └── pyproject.toml # Python dependencies
  └── .gitignore
```

All template files are embedded in the binary — scaffold works offline, no network needed.

### Model Provider Matrix

| Framework | Provider | Env Var | Default Model |
|---|---|---|---|
| LangGraph | Anthropic | `ANTHROPIC_API_KEY` | `claude-sonnet-4-5-20250514` |
| LangGraph | OpenAI | `OPENAI_API_KEY` | `gpt-4.1` |
| OpenAI Agents | OpenAI | `OPENAI_API_KEY` | `gpt-4.1` |
| Google ADK | Google | `GOOGLE_API_KEY` | `gemini-2.5-flash` |
| Claude Agent SDK | Anthropic | `ANTHROPIC_API_KEY` | `claude-opus-4-7` |
| LangChain | Anthropic | `ANTHROPIC_API_KEY` | `claude-sonnet-4-5-20250514` |
| LangChain | OpenAI | `OPENAI_API_KEY` | `gpt-4.1` |
| Custom | Any | user-defined | n/a |

### `gravixlayer.json` Schema

The `gravixlayer/gravixlayer.json` file is the source of truth for all `grx agent` commands.
All values have defaults — no manual editing required after `grx agent create`.

```json
{
  "name": "my-agent",
  "description": "",
  "framework": "langgraph",
  "entry_point": "/usr/local/bin/gravixlayer --framework langgraph --root /app --host 0.0.0.0 --port 8080 --protocols http",
  "start_command": [
    "gravixlayer",
    "agent",
    "serve",
    ".",
    "--framework",
    "langgraph",
    "--host",
    "0.0.0.0",
    "--port",
    "8080",
    "--protocols",
    "http"
  ],
  "python_version": "3.12",
  "http_port": 8080,
  "protocols": ["http"],
  "is_public": false,
  "resources": {
    "vcpu": 2,
    "memory_mb": 2048,
    "disk_mb": 20480
  },
  "environment": [
    { "name": "ANTHROPIC_API_KEY", "from_env_local": true }
  ],
  "tags": []
}
```

### `grx agent dev [dir]`

Launch the agent in ephemeral development mode with file-watch hot-reload.

```
grx agent dev [DIR]

  DIR defaults to current directory if gravixlayer/gravixlayer.json is present.

Behavior:
  1. Read gravixlayer/gravixlayer.json
  2. Create an ephemeral cloud runtime using project defaults
  3. Upload source files to the runtime
  4. Watch app/ for file changes and hot-reload on save
  5. Print the runtime HTTP endpoint URL
  6. Ctrl+C — terminate runtime and exit

No public URL is assigned. The runtime is killed automatically on exit.
```

### `grx agent build [dir]`

Build a deployable agent image from source.

```
grx agent build [DIR] [--wait]

  DIR     project directory   [default: current directory]
  --wait  poll until build completes [default: true]

Build process:
  1. Walk source tree, skip excluded paths
  2. Assemble tar.gz in a temp directory
  3. POST multipart to /v1/agents/template/build-agent
  4. Poll /v1/agents/build-status/<id> with progress spinner

Excluded from archive:
  __pycache__  .git  .venv  venv  env  .env  node_modules  dist  build
  *.egg-info   .DS_Store  .mypy_cache  .pytest_cache  .ruff_cache  .tox
```

### `grx agent status <build-id>`

Show the build status for an agent build.

### `grx agent deploy [dir]`

Build and deploy the agent to a permanent public endpoint.

```
grx agent deploy [DIR] [--wait | --no-wait]

  Runs: build → deploy → poll until running → print endpoint URL

  --wait      wait for deployment to complete  [default]
  --no-wait   return immediately after submitting deploy request

Output on success:
  Agent deployed: https://my-agent-abc123.gravixlayer.ai
  Protocols: HTTP, A2A, MCP
```

### `grx agent get <id>`

Print detailed agent info including endpoint, protocols, build info, and resource usage.

### `grx agent invoke <id>`

Send a single synchronous request to a deployed agent and print the response.

```
grx agent invoke <id> --input '{"message": "Hello"}'
grx agent invoke <id> --input-file request.json
```

### `grx agent stream <id>`

Stream an agent response via SSE (Server-Sent Events), printing chunks as they arrive.

```
grx agent stream <id> --input '{"message": "Explain quantum computing"}'
grx agent stream <id> --input-file request.json
```

### `grx agent destroy <id>`

Permanently delete a deployed agent, its endpoint, and all associated resources.

```
grx agent destroy <id> [--force]

  --force   Skip the confirmation prompt
```

---

## billing

View usage and billing information.

### `grx billing summary`

Print current billing period usage: credits consumed, total spend, remaining quota.

### `grx billing history`

```
grx billing history [--limit <N>]

Columns: Date | Description | Credits | Amount
```

### `grx billing quotas`

Show quota and limit details for the account.

---

## Utility Commands

### `grx validate [dir]`

Validate `gravixlayer/gravixlayer.json` schema without creating any resources.
Prints a structured list of errors and warnings.

### `grx package [dir]`

Dry-run: assemble the tar.gz archive that would be uploaded by `grx agent build`,
and save it locally for inspection.

```
grx package [DIR] [--output <PATH>]

  --output   Path to write the archive     [default: ./agent-package.tar.gz]
```

### `grx completions <shell>`

Generate shell completion script and print it to stdout.

```bash
grx completions bash        >> ~/.bashrc
grx completions zsh         >> ~/.zshrc
grx completions fish        > ~/.config/fish/completions/grx.fish
grx completions powershell  >> $PROFILE
```

### `grx update`

Upgrade `grx` to the latest release from GitHub Releases.

```
grx update [--version <VERSION>]

  --version   Pin to a specific version, e.g. 0.2.0
              Omit for latest stable release
```
