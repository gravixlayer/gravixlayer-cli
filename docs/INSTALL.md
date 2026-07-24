# gravixlayer-cli — Installation

## Quick install

### macOS / Linux

```sh
curl -fsSL https://cli.gravixlayer.ai/install | sh
```

Installs `gravixlayer` and a `grx` symlink into `~/.local/bin` by default
(no sudo). Override with `GRAVIXLAYER_INSTALL_DIR`.

Pin a version:

```sh
GRAVIXLAYER_VERSION=v0.1.0 curl -fsSL https://cli.gravixlayer.ai/install | sh
```

### Windows (PowerShell)

```powershell
irm 'https://cli.gravixlayer.ai/install.ps1' | iex
```

Installs to `%LOCALAPPDATA%\gravixlayer\bin` and adds it to the user `PATH`.

---

## How the installer works

`scripts/install.sh` (and `scripts/install.ps1`) detect OS/arch, resolve the
latest GitHub Release (or `GRAVIXLAYER_VERSION`), download the matching archive
and `.sha256` checksum, verify integrity, and install the binary.

### Release asset names

Archives follow Rust target triples (common for native CLI releases):

| Platform | Archive |
|---|---|
| Linux x86_64 (musl) | `gravixlayer-<tag>-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 (musl) | `gravixlayer-<tag>-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple Silicon | `gravixlayer-<tag>-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `gravixlayer-<tag>-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `gravixlayer-<tag>-x86_64-pc-windows-msvc.zip` |

Each `.tar.gz` contains a single `gravixlayer` binary at the archive root.
Checksum files are named `<archive>.sha256`.

Hosted at: `https://github.com/gravixlayer/gravixlayer-cli/releases`

---

## Manual install

```bash
TAG=v0.1.0
TARGET=aarch64-apple-darwin   # see table above
curl -fL -o gravixlayer.tar.gz \
  "https://github.com/gravixlayer/gravixlayer-cli/releases/download/${TAG}/gravixlayer-${TAG}-${TARGET}.tar.gz"
tar -xzf gravixlayer.tar.gz
chmod +x gravixlayer
sudo mv gravixlayer /usr/local/bin/
sudo ln -sf /usr/local/bin/gravixlayer /usr/local/bin/grx
```

### From source

```bash
cargo install --git https://github.com/gravixlayer/gravixlayer-cli --locked
```

---

## Verify

```bash
gravixlayer --version
# gravixlayer 0.1.0

gravixlayer auth status
```

`grx` is an optional symlink to the same binary.

---

## First-time setup

```bash
gravixlayer auth login
```

Prompts for your API key (from https://app.gravixlayer.ai), **verifies it against
the API**, then stores it in the OS keyring (macOS Keychain / libsecret / Windows
Credential Manager).

Override for CI:

```bash
export GRAVIXLAYER_API_KEY="..."
```

---

## Shell completions

```bash
# bash
gravixlayer completions bash >> ~/.bashrc

# zsh
gravixlayer completions zsh >> ~/.zshrc

# fish
gravixlayer completions fish > ~/.config/fish/completions/gravixlayer.fish
```

---

## Configuration

User config: `~/.gravixlayer/config.toml`

| Variable | Description |
|---|---|
| `GRAVIXLAYER_API_KEY` | API key (overrides keyring) |
| `GRAVIXLAYER_BASE_URL` | API base URL (default `https://api.gravixlayer.ai`) |
| `GRAVIXLAYER_PROFILE` | Config profile name |
| `GRAVIXLAYER_OUTPUT` | `table` \| `json` \| `quiet` |
| `RUST_LOG` | Tracing filter (e.g. `debug`) |

---

## Upgrading

```bash
gravixlayer update              # install latest GitHub Release
gravixlayer update --check      # report only
gravixlayer update --version 0.1.0
```

Or re-run the install script.

---

## Uninstall

### macOS / Linux

```sh
rm -f ~/.local/bin/gravixlayer ~/.local/bin/grx
# If you previously installed to /usr/local/bin:
#   sudo rm -f /usr/local/bin/gravixlayer /usr/local/bin/grx
rm -rf ~/.gravixlayer
```

### Windows (PowerShell)

```powershell
Remove-Item "$env:LOCALAPPDATA\gravixlayer" -Recurse -Force
```
