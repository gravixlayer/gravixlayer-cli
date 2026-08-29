# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Pre-releases use `X.Y.Z-alpha.N` (for example `0.2.0-alpha.1`) and are published
as GitHub prereleases so `install` without `GRAVIXLAYER_VERSION` continues to
resolve the latest **stable** release.

## [Unreleased]

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
