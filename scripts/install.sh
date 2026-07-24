#!/bin/sh
# GravixLayer CLI installer (POSIX sh — macOS / Linux)
# Usage: curl -fsSL https://cli.gravixlayer.ai/install | sh
#
# Detects the platform, downloads the matching GitHub Release archive,
# verifies SHA-256 (fail-closed), installs the binary, and ensures the
# install directory is on PATH (writes a profile block when needed).
#
# Environment overrides:
#   GRAVIXLAYER_VERSION     — tag (e.g. v0.1.0 or v0.2.0-alpha.1)
#   GRAVIXLAYER_INSTALL_DIR — install dir (default: $HOME/.local/bin)
#   GRAVIXLAYER_NO_VERIFY   — skip checksum verification (NOT recommended)
#   GRAVIXLAYER_REPO        — GitHub repo (default: gravixlayer/gravixlayer-cli)
#
# Stable vs alpha:
#   Unset GRAVIXLAYER_VERSION → GitHub /releases/latest (stable only).
#   Pin an alpha: GRAVIXLAYER_VERSION=v0.2.0-alpha.1

set -eu

REPO="${GRAVIXLAYER_REPO:-gravixlayer/gravixlayer-cli}"
BINARY_NAME="gravixlayer"
SYMLINK_NAME="grx"
# Prefer user-local bin (no sudo).
BIN_DIR="${GRAVIXLAYER_INSTALL_DIR:-$HOME/.local/bin}"
RELEASES_BASE="https://github.com/${REPO}/releases"

os=""
arch=""
platform=""
version=""
tarball_name=""
tarball_url=""
checksum_url=""
tarball=""
checksum_file=""
binary=""
expected_checksum=""
actual=""
tmpdir=""
path_action="already"
path_profile=""
api_url=""
body=""
profile=""
begin_marker=""
end_marker=""
path_line=""
url=""
dest=""
file=""
expected=""

info() {
  printf '\033[0;34m[info]\033[0m  %s\n' "$*"
}

success() {
  printf '\033[0;32m[ok]\033[0m    %s\n' "$*"
}

warn() {
  printf '\033[0;33m[warn]\033[0m  %s\n' "$*" >&2
}

error() {
  printf '\033[0;31m[error]\033[0m %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || error "required command not found: $1"
}

cleanup() {
  if [ -n "${tmpdir:-}" ] && [ -d "$tmpdir" ]; then
    rm -rf -- "$tmpdir"
  fi
}

detect_platform() {
  arch="$(uname -m)"

  case "$(uname -s)" in
    Linux)
      os="linux"
      case "$arch" in
        x86_64) platform="x86_64-unknown-linux-musl" ;;
        aarch64|arm64) platform="aarch64-unknown-linux-musl" ;;
        *) error "unsupported Linux architecture: $arch" ;;
      esac
      ;;
    Darwin)
      os="darwin"
      case "$arch" in
        x86_64) platform="x86_64-apple-darwin" ;;
        arm64) platform="aarch64-apple-darwin" ;;
        *) error "unsupported macOS architecture: $arch" ;;
      esac
      ;;
    MINGW*|MSYS*|CYGWIN*)
      error "Windows detected. Please use the PowerShell installer instead:
  irm 'https://cli.gravixlayer.ai/install.ps1' | iex"
      ;;
    *)
      error "unsupported operating system: $(uname -s)"
      ;;
  esac
}

json_tag_name() {
  body="$1"
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
  version="${GRAVIXLAYER_VERSION:-}"
  if [ -n "$version" ]; then
    case "$version" in
      v*) ;;
      *) version="v${version}" ;;
    esac
    return
  fi

  need_cmd curl
  api_url="https://api.github.com/repos/${REPO}/releases/latest"
  body="$(curl -fsSL -H 'Accept: application/vnd.github+json' -H 'User-Agent: gravixlayer-installer' "$api_url")" \
    || error "failed to fetch latest release from GitHub"
  version="$(json_tag_name "$body")"
  [ -n "$version" ] || error "failed to determine the latest release version"
}

download() {
  url="$1"
  dest="$2"
  need_cmd curl
  curl -fsSL --progress-bar -o "$dest" "$url" || error "download failed: $url"
}

verify_checksum() {
  file="$1"
  expected="$2"

  if [ -n "${GRAVIXLAYER_NO_VERIFY:-}" ]; then
    warn "checksum verification skipped (GRAVIXLAYER_NO_VERIFY is set)"
    return
  fi

  [ -n "$expected" ] || error "empty checksum — refusing to install"

  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$file" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  elif command -v openssl >/dev/null 2>&1; then
    actual="$(openssl dgst -sha256 "$file" | sed 's/^.*= //')"
  else
    error "no sha256 utility found (sha256sum, shasum, or openssl required)"
  fi

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

pick_profile() {
  # Shell-specific startup files (login vs interactive differ on macOS/Linux).
  case "$os:${SHELL:-}" in
    darwin:*/zsh)
      printf '%s\n' "$HOME/.zprofile"
      ;;
    darwin:*/bash)
      printf '%s\n' "$HOME/.bash_profile"
      ;;
    linux:*/zsh)
      printf '%s\n' "$HOME/.zshrc"
      ;;
    linux:*/bash)
      printf '%s\n' "$HOME/.bashrc"
      ;;
    *)
      printf '%s\n' "$HOME/.profile"
      ;;
  esac
}

ensure_path() {
  path_action="already"
  path_profile=""

  case ":$PATH:" in
    *":$BIN_DIR:"*)
      return
      ;;
  esac

  # Current shell (so this session can run gravixlayer immediately).
  # Always append to existing PATH — never replace it.
  PATH="$BIN_DIR:$PATH"
  export PATH

  profile="$(pick_profile)"
  path_profile="$profile"
  begin_marker="# >>> gravixlayer installer >>>"
  end_marker="# <<< gravixlayer installer <<<"
  # Expand BIN_DIR now; keep \$PATH for future shells.
  path_line="export PATH=\"${BIN_DIR}:\$PATH\""

  if [ -f "$profile" ] && grep -F "$begin_marker" "$profile" >/dev/null 2>&1; then
    if grep -F "$path_line" "$profile" >/dev/null 2>&1; then
      path_action="configured"
      return
    fi
  fi

  {
    printf '\n%s\n' "$begin_marker"
    printf '%s\n' "$path_line"
    printf '%s\n' "$end_marker"
  } >>"$profile"
  path_action="added"
}

install_binary() {
  mkdir -p "$BIN_DIR"
  mv "$binary" "${BIN_DIR}/${BINARY_NAME}"
  chmod +x "${BIN_DIR}/${BINARY_NAME}"
  ln -sf "${BIN_DIR}/${BINARY_NAME}" "${BIN_DIR}/${SYMLINK_NAME}"
}

main() {
  need_cmd uname
  need_cmd tar
  need_cmd mktemp

  trap cleanup EXIT INT HUP TERM

  detect_platform
  resolve_version

  info "Installing gravixlayer ${version} for ${platform}"
  info "Install directory: ${BIN_DIR}"

  tarball_name="${BINARY_NAME}-${version}-${platform}.tar.gz"
  tarball_url="${RELEASES_BASE}/download/${version}/${tarball_name}"
  checksum_url="${RELEASES_BASE}/download/${version}/${tarball_name}.sha256"

  tmpdir="$(mktemp -d)"
  tarball="${tmpdir}/${tarball_name}"
  checksum_file="${tmpdir}/${tarball_name}.sha256"

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
    expected_checksum="$(tr -d '\r' < "$checksum_file" | awk '{print $1}')"
    verify_checksum "$tarball" "$expected_checksum"
  fi

  info "Extracting archive"
  tar -xzf "$tarball" -C "$tmpdir"

  binary="${tmpdir}/${BINARY_NAME}"
  [ -f "$binary" ] || error "binary not found in archive: ${BINARY_NAME}"
  chmod +x "$binary"

  install_binary
  ensure_path

  success "gravixlayer ${version} installed to ${BIN_DIR}/${BINARY_NAME}"
  success "grx symlink created at ${BIN_DIR}/${SYMLINK_NAME}"

  case "$path_action" in
    added)
      info "Added ${BIN_DIR} to PATH in ${path_profile}"
      info "Open a new terminal, or run:  export PATH=\"${BIN_DIR}:\$PATH\""
      ;;
    configured)
      info "PATH already configured in ${path_profile}"
      ;;
    already)
      ;;
  esac

  if command -v "${BINARY_NAME}" >/dev/null 2>&1; then
    info "Installed version: $("${BINARY_NAME}" --version 2>/dev/null || true)"
  else
    warn "gravixlayer is installed but not yet visible on PATH in this shell."
    warn "Run:  export PATH=\"${BIN_DIR}:\$PATH\""
    warn "Or:   ${BIN_DIR}/${BINARY_NAME} --version"
  fi

  cat <<EOF

Get started:
  gravixlayer auth login        # save your API key
  gravixlayer doctor            # verify local install
  gravixlayer runtime create    # spin up a cloud runtime
  gravixlayer --help

Update later:
  gravixlayer update

Uninstall:
  rm -f "${BIN_DIR}/${BINARY_NAME}" "${BIN_DIR}/${SYMLINK_NAME}"
  # optional: rm -rf ~/.gravixlayer

Documentation: https://docs.gravixlayer.ai
EOF
}

main "$@"
