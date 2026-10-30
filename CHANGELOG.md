# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Pre-releases use `X.Y.Z-alpha.N` (for example `0.2.0-alpha.1`) and are published
as GitHub prereleases so `install` without `GRAVIXLAYER_VERSION` continues to
resolve the latest **stable** release.


## [0.1.17] - 2026-10-30

### Fixed

- **`runtime template events` 401.** The build-event SSE handshake went
  out without the bearer token (a bare request builder instead of the
  authorized streaming client), so the server rejected it with
  `authorization header required` and the durable event log — including
  the actual build-failure reason — was unreachable. A mock test now
  pins the authorized handshake.


## [0.1.16] - 2026-10-30

### Added

- **Desktop accessibility commands.** `runtime desktop a11y-snapshot`
  (`--scope`, `--app`, `--pid`, `--max-depth`, `--limit`, `--all`, `--role`,
  `--name`, `--diff`, `--rev`), `a11y-find` (`--role`, `--name`, `--ref`,
  `--max`), and `a11y-action` (`<op> --ref` with `--action`, `--action-index`,
  `--text`, `--value`, `--timeout-ms`) expose the ref-addressed accessibility
  surface both SDKs already carry.
- **Desktop recording commands.** `runtime desktop recordings`,
  `recording <id>`, `record-start` (`--include`, `--label`, `--window-id`),
  `record-stop`, `record-mark`, `record-delete`, and `record-download`
  (`--artifact video|trajectory <local>`) cover the full recording lifecycle;
  `view-control <shared|human|agent>` sets the live-view takeover mode.
- **Browser context and storage commands.** `runtime browser contexts`,
  `context-create` (`--proxy`, `--storage-state`), `context-delete <id>`,
  `storage-export` (`--context`, `--out`), and `storage-import` (`--state` or
  `--state-file`) manage isolated browser contexts and their cookie/storage
  jars; `browser events` streams one SSE event page (`--since` resumes,
  `--wait-ms` bounds the page); `browser targets --context <id>` opens a new
  tab inside an isolated context.

## [0.1.15] - 2026-10-27

### Changed

- **Storage buckets are one public resource.** `storage bucket create`
  provisions a managed bucket and `storage bucket connect` connects an existing
  S3/S3-compatible bucket; `list`, `get`, `verify`, and `delete` operate on
  both through `source`. Mounts take only `"bucket"`; `storage link` is
  removed.
- **Mount validation rejects credential fields on bucket mounts.**
  `credential_provider`/`credential_secret` are git-mount fields; a bucket
  mount carrying them now fails locally instead of being silently ignored —
  matching the other stray-field refusals and both SDKs.

### Fixed

- **`runtime desktop components`/`start`/`stop`/`restart`/`logs` read the
  live wire schema.** The response structs still parsed the retired
  `state`/`sub_state`/`description` and `timestamp`/`next_cursor`/
  `truncated` fields, so component state rendered blank and log paging
  never surfaced a cursor. Structs now match the wire — components carry
  `active`/`sub`/`result`/`restarts`/`started_usec`, log entries carry
  `identifier`/`unit`/`priority`/`usec`, and paging uses `cursor`.
- **`browser open --arg` and `pty create --arg` accept hyphen values.**
  Engine/shell flags (`--arg "--start-maximized"`, `--arg "-l"`) were
  rejected as unknown CLI flags; both pass-through args now use
  `allow_hyphen_values` like every clap pass-through does.

## [0.1.14] - 2026-10-25

### Fixed

- **`--output json` stdout stays machine-parseable.** `Spinner::finish_ok`
  wrote `✔ …` status lines to stdout, so commands like
  `template build --output json` interleaved progress text into the JSON
  payload and broke `json.load`/`jq` pipelines. Status now goes to stderr
  — stdout carries only the requested output.

## [0.1.13] - 2026-10-25

### Changed
- **`runtime desktop` is now the single computer-use command group** — it
  absorbs every `runtime display` subcommand (enable/disable/status/info/
  position/keys/screenshot/mouse/keyboard/clipboard/wait-stable/view/
  stop-view/act/resize/windows/window-op/apps/launch/open) plus the session
  ops renamed to `components`, `start`/`stop`/`restart NAME`, `logs`.
  `runtime display` is removed; `runtime browser` is unchanged.
- `browser command`/`targets`/`status` now parse the typed target record
  (target_id/url/title/type/active/loading) instead of the navigate shape
  or untyped values, so JSON output carries the real fields.

### Added
- **`runtime browser` reaches flag parity with the SDKs** — every declared
  flag now lands on the wire: `navigate --target/--wait-until/--timeout-ms`,
  `screenshot --target/--element/--wait-stable-ms/--timeout-ms/--quality`,
  `snapshot --target/--full-page/--rev/--filter-role/--filter-name`,
  `evaluate --target/--await-promise/--world/--timeout-ms`,
  `action --tab/--secret-ref/--button/--click-count/--delay-ms/
  --settle-timeout-ms/--target-timeout-ms`,
  `targets --url/--no-activate` (open a new tab),
  `pdf --landscape/--print-background/--page-ranges/--paper-*-in/--margin-*`,
  `downloads --wait-name/--timeout-ms`,
  `view --target/--fps/--quality/--role/--rotate-token`,
  `dialog --target/--accept/--prompt-text/--auto-dismiss`, and
  `open --engine/--headless/--profile-dir/--arg/--url`.
- `download <id> --out` now falls back to the download's recorded filename
  before the id.
- `browser action --target-json` / `browser screenshot --element-json` —
  full element-target objects verbatim (`pierce`, `inner` frame targets,
  `exact`, `attrs`) for shadow-DOM / iframe elements the string spec can't
  express; each conflicts with its string `--target`/`--element` form.
- `template events <build_id>` — streams the durable build event log
  (SSE): stored events replay first, then live `step`/`stdout`/`stderr`
  output until `stream_end`.

### Security
- `zip` bumped to 9 (RUSTSEC-2026-0369 — the extractor path is unused;
  `cargo-deny` now passes clean alongside `h2`, `rustls`,
  `rustls-webpki`, `tokio`, and `webpki-roots` patch bumps).

## [0.1.12] - 2026-10-24
### Added
- `runtime display enable --mode auto|spawn|attach` — attach to the
  image's session display or spawn a virtual one.
- `runtime display info`, `position`, `keys`, `stop-view`, and
  `clipboard get|set`.
- `runtime display screenshot --region x,y,w,h --scale-pct N
  --max-width/--max-height PX --quality N --cursor --format webp`.
- `runtime display view --role watch` prints the view-only token.
- `runtime display mouse` verbs `down`, `up`, `drag` (`--to-x/--to-y`,
  `--steps`, `--duration-ms`) plus `--click-count`, `--modifiers` and
  `--coordinate-space`; `keyboard` accepts `--mode paste` and
  `--modifiers`.
- `runtime desktop status|start|stop|restart|logs` — session component
  lifecycle and journal paging on desktop-template runtimes.
- `runtime display act --actions '[…]'|file|- --settle-ms N --timeout-ms N
  --observe '{…}'` — one agent step per call; prints the observation's
  `settled`/`rev` and screenshot summary.
- `runtime display resize W H`, `runtime display windows [--class C]`,
  `runtime display window-op W OP [--x/--y/--w/--h]`,
  `runtime display apps`, `runtime display launch APP [--uri U]
  [--wait-window]`, and `runtime display open TARGET [--wait-window]`.
- `runtime display screenshot --if-rev N` skips the transfer when the
  frame is unchanged (prints the revision instead of writing a file);
  `runtime display view --window ID` streams one window;
  `runtime display clipboard set --png FILE` / `get --png-out FILE` move
  clipboard images.
- `runtime browser view --role watch` mints a view-only link — the
  returned `view_token` (`&vt=` on the URL) sees frames but can never
  inject input. `--rotate-token` re-mints the URL's auth token.

### Changed
- `runtime create --agent`/`--agent-id` exits with an explicit deprecation
  error instead of sending the removed field — the API rejects `agent_id`
  outright; per-runtime MCP policy attaches via the `mcp` spec.

### Fixed
- `runtime browser` percent-encodes `browser_id`, `target_id`, and
  `download_id` in route paths — an id containing `/`, `?`, or `#`
  previously corrupted the route.
- `runtime browser snapshot` no longer fails with "missing field
  `snapshot_id`" — the API reply never carried one.
- `runtime browser close`, `runtime display disable`, and every
  `runtime display mouse` / `keyboard` verb accept the API's `{"ok": true}`
  reply instead of exiting 1 after the action succeeded.
- `runtime browser targets` sends `POST` (the route is POST-only); it
  previously sent `GET` and failed.
- `runtime browser action scroll --value dx,dy` sends integer deltas; floats
  were read as 0 by the server, so the scroll did nothing. Non-integer
  values are now rejected.
- `runtime browser evaluate` prints the expression's `value` (it printed
  `null` for every expression) and exits 1 with the message when the
  expression throws.
- `runtime display window-op` accepts `fullscreen`/`unfullscreen` — the
  server supports both ops but the CLI's op list rejected them.
- `runtime display mouse scroll --x/--y` are now sent when given — the
  server positions the pointer before the wheel (they were accepted but
  never transmitted). Omitted axes keep the current pointer position.
- `runtime display enable --width/--height` are optional again — unset, the
  platform default geometry applies instead of a forced client-side size.
- `runtime display view` and `runtime browser view` accept
  `--rotate-token` to mint a fresh auth token for the watch URL
  (invalidates the previous one).

## [0.1.11] - 2026-10-16
### Tests
- `scripts/test_storage_e2e.sh` — opt-in live suite driving the CLI end to
  end: managed buckets, static-credential external buckets, git mounts,
  pause/resume, kill-detach, read-only enforcement, and guest credential
  hygiene. Requires `GRAVIXLAYER_API_KEY` + `GRAVIXLAYER_BASE_URL`; reads
  `~/.config/gravix-test/aws.env` for test credentials.

## [0.1.10] - 2026-10-15
### Added
- `storage` — managed and external buckets: `storage bucket
  create|connect|list|get|verify|delete`. External bucket auth is static
  (secret-provider key references — secret material never crosses the CLI) or
  `oidc` (role assumption); `s3_compatible` buckets take an `--endpoint`
  (https) and `--path-style`.
- `runtime mounts` — `attach --mount '<json>'` (bucket/git at a guest
  path), `list`, `detach <attachment_id>` on a running runtime.
- `runtime create --mount '<json>'` — attach bucket or git mounts
  at boot (repeatable, ≤8). Mount specs are validated locally — bad paths,
  missing bucket targets, non-https endpoints or git URLs fail before
  the request is sent.
- `runtime display` — computer-use pixel plane: `enable`/`disable`/`status`,
  `screenshot` (to file, `--wait-stable-ms`), `mouse` (click/move/scroll),
  `keyboard` (type/press/hotkey), `wait-stable`, `view` (live-watch URL).
- `runtime browser` — computer-use semantic plane: `open`/`close`/`list`/
  `status`/`targets`, `navigate`, `command` (back/forward/reload),
  `screenshot` (viewport/full-page/`--element`), `snapshot` (`--diff`),
  `evaluate`, `action` (click/dblclick/hover/fill/type/press/hotkey/scroll —
  `--target e3`, `css:#x`, `text:…`, `xpath:…`, `role:…`, `x,y`; `fill`
  accepts `--secret-ref`), `dialog`, `downloads`/`download`, `pdf`, `view`.
  All commands honor the shared `--output json` mode.

## [0.1.9] - 2026-10-13
### Added
- `runtime files rm -r`/`--recursive` deletes directories and their contents —
  the flag the SDKs already expose on `file.delete`. Without it a non-empty
  directory is refused, matching the server default.

### Fixed
- HTTP budgets now match the SDKs per operation: `exec`/`run` wait out the
  guest deadline plus 30 s (previously a fixed 300 s cut calls mid-command
  for `--timeout` over 300), `runtime fork`/`snapshot fork`/`snapshot create`
  get the SDKs' 600 s coordination budget, and SSE streams (`exec --stream`,
  `run --stream`, `commands attach`, `files watch`, `pty attach`) move to an
  unbounded transport — the handshake keeps the 300 s deadline and each read
  gets a 45 s grace that only trips on a genuinely stalled connection. A
  stream longer than 300 s no longer dies mid-command.

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
