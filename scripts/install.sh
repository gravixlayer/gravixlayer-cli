#!/usr/bin/env bash
# GravixLayer CLI installer
# Usage: curl -fsSL https://cli.gravixlayer.ai/install | bash
#
# Detects the platform, downloads the correct pre-built binary from GitHub
# Releases, verifies the SHA-256 checksum (fail-closed), installs to
# /usr/local/bin (or ~/.local/bin), and creates a `grx` symlink.
#
# Environment overrides:
#   GRAVIXLAYER_VERSION     — install a specific version tag (e.g. v0.1.0 or v0.2.0-alpha.1)
#   GRAVIXLAYER_INSTALL_DIR — override the installation directory (default: /usr/local/bin)
#   GRAVIXLAYER_NO_VERIFY   — set to any non-empty value to skip checksum verification (NOT recommended)
#   GRAVIXLAYER_REPO        — override GitHub repo (default: gravixlayer/gravixlayer-cli)
#
# Stable vs alpha:
#   Unset GRAVIXLAYER_VERSION → GitHub /releases/latest (stable only; alphas are prereleases).
#   Pin an alpha explicitly: GRAVIXLAYER_VERSION=v0.2.0-alpha.1

set -euo pipefail

REPO="${GRAVIXLAYER_REPO:-gravixlayer/gravixlayer-cli}"
BINARY_NAME="gravixlayer"
SYMLINK_NAME="grx"
INSTALL_DIR="${GRAVIXLAYER_INSTALL_DIR:-/usr/local/bin}"
RELEASES_BASE="https://github.com/${REPO}/releases"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

info()    { printf '\033[0;34m[info]\033[0m  %s\n' "$*"; }
success() { printf '\033[0;32m[ok]\033[0m    %s\n' "$*"; }
warn()    { printf '\033[0;33m[warn]\033[0m  %s\n' "$*" >&2; }
error()   { printf '\033[0;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || error "required command not found: $1"
}

# ---------------------------------------------------------------------------
# Detect OS and architecture
# ---------------------------------------------------------------------------

detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-musl" ;;
                aarch64|arm64) echo "aarch64-unknown-linux-musl" ;;
                *) error "unsupported Linux architecture: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  echo "x86_64-apple-darwin" ;;
                arm64)   echo "aarch64-apple-darwin" ;;
                *) error "unsupported macOS architecture: $arch" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            error "Windows detected. Please use the PowerShell installer instead:
  irm 'https://cli.gravixlayer.ai/install.ps1' | iex"
            ;;
        *)
            error "unsupported operating system: $os"
            ;;
    esac
}

# ---------------------------------------------------------------------------
# Version resolution (stable = /releases/latest; pin alphas via env)
# ---------------------------------------------------------------------------

json_tag_name() {
    # Prefer jq / python for minified GitHub JSON; fall back to sed.
    local body="$1"
    if command -v jq >/dev/null 2>&1; then
        printf '%s' "$body" | jq -r '.tag_name // empty'
        return
    fi
    if command -v python3 >/dev/null 2>&1; then
        printf '%s' "$body" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("tag_name") or "")'
        return
    fi
    printf '%s' "$body" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
}

resolve_version() {
    local version="${GRAVIXLAYER_VERSION:-}"
    if [ -n "$version" ]; then
        case "$version" in
            v*) echo "$version" ;;
            *)  echo "v${version}" ;;
        esac
        return
    fi

    need_cmd curl
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    local body
    body="$(curl -fsSL -H 'Accept: application/vnd.github+json' -H 'User-Agent: gravixlayer-installer' "$api_url")" \
        || error "failed to fetch latest release from GitHub"
    version="$(json_tag_name "$body")"
    [ -n "$version" ] || error "failed to determine the latest release version"
    echo "$version"
}

# ---------------------------------------------------------------------------
# Download
# ---------------------------------------------------------------------------

download() {
    local url="$1" dest="$2"
    need_cmd curl
    curl -fsSL --progress-bar -o "$dest" "$url" \
        || error "download failed: $url"
}

# ---------------------------------------------------------------------------
# Checksum verification (fail-closed)
# ---------------------------------------------------------------------------

verify_checksum() {
    local file="$1" expected="$2"

    if [ -n "${GRAVIXLAYER_NO_VERIFY:-}" ]; then
        warn "checksum verification skipped (GRAVIXLAYER_NO_VERIFY is set)"
        return
    fi

    [ -n "$expected" ] || error "empty checksum — refusing to install"

    local actual
    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$file" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "$file" | awk '{print $1}')"
    else
        error "no sha256 utility found (sha256sum or shasum required)"
    fi

    # Accept either bare hex or `SHA256 (file) = hex` / `hex  filename` formats.
    expected="$(printf '%s' "$expected" | tr -d '\r' | awk '{print $1}')"
    actual="$(printf '%s' "$actual" | tr 'A-F' 'a-f')"
    expected="$(printf '%s' "$expected" | tr 'A-F' 'a-f')"

    if [ "$actual" != "$expected" ]; then
        error "checksum mismatch!
  expected: $expected
  actual:   $actual"
    fi
    success "checksum verified"
}

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

main() {
    need_cmd uname
    need_cmd tar
    need_cmd mktemp

    local platform version tarball_name tarball_url checksum_url
    local tmpdir

    platform="$(detect_platform)"
    version="$(resolve_version)"

    info "Installing gravixlayer ${version} for ${platform}"

    tarball_name="${BINARY_NAME}-${version}-${platform}.tar.gz"
    tarball_url="${RELEASES_BASE}/download/${version}/${tarball_name}"
    checksum_url="${RELEASES_BASE}/download/${version}/${tarball_name}.sha256"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    local tarball="${tmpdir}/${tarball_name}"
    local checksum_file="${tmpdir}/${tarball_name}.sha256"

    info "Downloading ${tarball_url}"
    download "$tarball_url" "$tarball"

    info "Downloading checksum ${checksum_url}"
    if ! curl -fsSL -o "$checksum_file" "$checksum_url"; then
        if [ -n "${GRAVIXLAYER_NO_VERIFY:-}" ]; then
            warn "checksum file not available — continuing because GRAVIXLAYER_NO_VERIFY is set"
        else
            error "checksum file not available at ${checksum_url}
Refusing to install without verification. Set GRAVIXLAYER_NO_VERIFY=1 to override (not recommended)."
        fi
    else
        local expected_checksum
        expected_checksum="$(tr -d '\r' < "$checksum_file" | awk '{print $1}')"
        verify_checksum "$tarball" "$expected_checksum"
    fi

    info "Extracting archive"
    tar -xzf "$tarball" -C "$tmpdir"

    local binary="${tmpdir}/${BINARY_NAME}"
    [ -f "$binary" ] || error "binary not found in archive: ${BINARY_NAME}"
    chmod +x "$binary"

    if [ -w "$INSTALL_DIR" ]; then
        mv "$binary" "${INSTALL_DIR}/${BINARY_NAME}"
        ln -sf "${INSTALL_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${SYMLINK_NAME}"
    elif command -v sudo >/dev/null 2>&1; then
        info "Requesting sudo to install to ${INSTALL_DIR}"
        sudo mv "$binary" "${INSTALL_DIR}/${BINARY_NAME}"
        sudo ln -sf "${INSTALL_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${SYMLINK_NAME}"
    else
        local user_bin="${HOME}/.local/bin"
        mkdir -p "$user_bin"
        mv "$binary" "${user_bin}/${BINARY_NAME}"
        ln -sf "${user_bin}/${BINARY_NAME}" "${user_bin}/${SYMLINK_NAME}"
        INSTALL_DIR="$user_bin"
        warn "installed to ${user_bin} (sudo not available)"
        warn "Add ${user_bin} to your PATH if it is not already:"
        warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi

    success "gravixlayer ${version} installed to ${INSTALL_DIR}/${BINARY_NAME}"
    success "grx symlink created at ${INSTALL_DIR}/${SYMLINK_NAME}"

    if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
        info "Installed version: $("${BINARY_NAME}" --version 2>/dev/null || true)"
    fi

    cat <<EOF

Get started:
  gravixlayer auth login        # save your API key
  gravixlayer doctor            # verify local install
  gravixlayer runtime create    # spin up a cloud runtime
  gravixlayer --help

Documentation: https://docs.gravixlayer.ai
EOF
}

main "$@"
