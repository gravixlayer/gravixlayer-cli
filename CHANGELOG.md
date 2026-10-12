# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Pre-releases use `X.Y.Z-alpha.N` (for example `0.2.0-alpha.1`) and are published
as GitHub prereleases so `install` without `GRAVIXLAYER_VERSION` continues to
resolve the latest **stable** release.

## [Unreleased]
## [0.1.8] - 2026-10-12
### Added
- `runtime fork` starts `count` children from one capture — `--count` (1-100),
  `--timeout`, `--env`, `--metadata`, and `--persist-snapshot`/`--name` for the
  persisted capture. Per-child failures print to stderr without failing the
  batch; the command fails when no child starts.
- `snapshot fork` starts `count` runtimes from a named snapshot — no capture
  runs and the source runtime does not need to be alive.
- `runtime commands` manages background commands: `list`, `get <pid>`,
  `kill <pid> [--signal KILL|TERM|INT|HUP]` (server default KILL), and
  `attach <pid>` which streams scrollback then live output — the same frames
  `exec --stream` renders.
- `runtime run` accepts `--code`/`-c` for inline code (alternative to a script
  path), `--language` to override extension inference, and `--context-id` to
  execute inside a persistent code context.
- `runtime pty create` accepts `--arg` for shell arguments (repeatable).
- `runtime git clone`/`pull`/`push`/`fetch` accept `--timeout-seconds` — a
  per-operation server-side deadline (default 300s, maximum 900s) so remote
  operations fail with a timeout error instead of holding the request.

### Fixed
- Retry now matches the SDK policy: 429 retries on any method, while
  502/503/504 and connection errors retry only on replayable methods
  (GET/HEAD/PUT/DELETE/OPTIONS). A POST that may have committed server-side
  can no longer be replayed by the retry loop.
- `runtime code-context create`/`get` no longer fail to deserialize the API
  response. The server emits the context id twice (`id` and `context_id`,
  same value), and the serde `alias` rejected the response as a duplicate
  field. Both keys are now read; `context_id` wins, matching the SDKs.

### Added
- `runtime exec --background` starts a command and prints its record, including the pid, without waiting for it to exit. `--timeout 0` sets no deadline. `--background` cannot be combined with `--stream`.

## [0.1.7] - 2026-09-23
### Fixed
- `GRAVIXLAYER_BASE_URL` / `config set base_url` now accept `http://` for a
  loopback API (`localhost`, `127.0.0.0/8`, `[::1]`), so a local `gravix-api`
  dev server is reachable. Non-loopback `http://` is still rejected — the
  client keeps `https_only` enforcement off-box.

### Changed
- `rust-toolchain.toml` now includes the `rust-analyzer` component so the
  editor uses the toolchain-matched analyzer instead of the extension binary.

## [0.1.6] - 2026-08-29
### Changed
- Occupancy quota is HTTP 403 and is not retried. 429 is only the create-rate
  window and still retries with backoff.

## [0.1.5] - 2026-08-20

- Fixed Template pipeline bug

## [0.1.4] - 2026-08-20

### Added

- `updated default cloud and region to aws and us-east-1`

## [0.1.3] - 2026-08-14

### Added

- `snapshot create|list|get|delete|activate|deactivate`
- `runtime create --snapshot` (mutually exclusive with `--template`)

## [0.1.2] - 2026-08-13

### Added

- `runtime pty` for programmatic PTY sessions (create, list, attach, send, resize, signal, kill)
- `runtime exec --stream` for incremental stdout/stderr over SSE
- Runtime files: `mv`, `cp`, `chown`, `watch`, `find`, and `replace`

## [0.1.1] - 2026-07-31

### Fixed

- POSIX `install.sh` (`curl|sh`), default install to `~/.local/bin`, and PATH via shell profiles
- Native self-update path (no vulnerable `self_update` / `quick-xml` dependency)
- Runtime git: required `--path`, clone default under `/workspace/<repo>`, non-zero exit for CI, per-operation `--auth-token` / `GRAVIXLAYER_GIT_TOKEN`

### Added

- `scripts/release.sh` and `scripts/set-version.sh` for one-command releases
- Tag-driven GitHub Actions release pipeline (`release.yml`)
- Fail-closed SHA-256 verification in `install.sh` / `install.ps1`
- `gravixlayer doctor` for local install/auth diagnostics
- Template `snapshot` command and `kind` / `project_id` list filters
- Billing `summary --month` / `--project_id` and expanded history filters
- API response field parity with the Python SDK and public API

## [0.1.0] - TBD

First public release.
