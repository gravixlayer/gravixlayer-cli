# Gravix Layer CLI

Official command-line interface for the [Gravix Layer](https://gravixlayer.ai) AI agent platform.

Manage runtimes (sandboxes), templates, agents, identity providers, network policies, and billing from your terminal.

## Install

### macOS / Linux

```sh
curl -fsSL https://cli.gravixlayer.ai/install | sh
```

### Windows (PowerShell)

```powershell
irm 'https://cli.gravixlayer.ai/install.ps1' | iex
```

### From source

```bash
git clone https://github.com/gravixlayer/gravixlayer-cli
cd gravixlayer-cli
cargo install --path . --locked
```

Requires Rust stable (see `rust-toolchain.toml`).

## Quick start

```bash
gravixlayer auth login          # stores API key in the OS keyring
gravixlayer runtime create -t base-small --wait
gravixlayer runtime exec <id> -- uname -a
gravixlayer --help
```

Set `GRAVIXLAYER_API_KEY` to override the keyring for CI.

## Versioning

This project follows [Semantic Versioning](https://semver.org/).

- **0.1.0** — first public release
- Pre-releases use the industry-standard form `X.Y.Z-alpha.N` (for example `0.2.0-alpha.1`) before a stable cut
- Git tags match Cargo versions with a `v` prefix (`v0.1.0`, `v0.2.0-alpha.1`)
- Untagged install resolves GitHub **latest stable** (prereleases are skipped)

## Documentation

| Doc | Description |
|-----|-------------|
| [docs/INSTALL.md](docs/INSTALL.md) | Installers, upgrades, uninstall |
| [docs/COMMANDS.md](docs/COMMANDS.md) | Command reference |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Internal layout |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Local build, test, and release |
| [CHANGELOG.md](CHANGELOG.md) | Release notes |
| [SECURITY.md](SECURITY.md) | Vulnerability reporting |
| [Platform docs](https://docs.gravixlayer.ai) | Product / API documentation |

## Binary name

The cargo binary is `gravixlayer`. Install scripts also create a `grx` symlink for convenience.

## License

Apache License 2.0 — see [LICENSE](LICENSE).
