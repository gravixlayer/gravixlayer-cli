#!/bin/sh
# Set [package].version in Cargo.toml, sync Cargo.lock, and verify both.
# Usage: ./scripts/set-version.sh 0.1.1
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VER="${1:-}"
if [ -z "$VER" ]; then
  echo "usage: $0 <X.Y.Z[-pre]>" >&2
  exit 1
fi

PKG_NAME="$(sed -n 's/^name[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' Cargo.toml | head -1)"
if [ -z "$PKG_NAME" ]; then
  echo "ERROR: could not read package name from Cargo.toml" >&2
  exit 1
fi

python3 - "$VER" <<'PY'
import re, sys

ver = sys.argv[1]
path = "Cargo.toml"
text = open(path).read()


def repl_package(m):
    body, n = re.subn(
        r'(?m)^version\s*=\s*"[^"]*"',
        'version     = "%s"' % ver,
        m.group(0),
        count=1,
    )
    if n != 1:
        sys.exit('ERROR: could not find version = "..." under [package]')
    return body


new, n = re.subn(r"(?ms)^\[package\].*?(?=^\[|\Z)", repl_package, text, count=1)
if n != 1:
    sys.exit("ERROR: could not find [package] section in Cargo.toml")
open(path, "w").write(new)
PY

# Sync ONLY this package's version in Cargo.lock.
# `cargo metadata --no-deps` does NOT rewrite Cargo.lock — that caused the
# v0.1.1 Release failure (toml 0.1.1 / lock 0.1.0 under --locked).
cargo update -p "$PKG_NAME"

lock_pkg_version() {
  awk -v name="$1" '
    $0 == "name = \"" name "\"" { want=1; next }
    want && /^version = "/ {
      sub(/^version = "/, "")
      sub(/"$/, "")
      print
      exit
    }
  ' Cargo.lock
}

ACTUAL="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' Cargo.toml | head -1)"
LOCK_VER="$(lock_pkg_version "$PKG_NAME")"

if [ "$ACTUAL" != "$VER" ]; then
  echo "ERROR: expected Cargo.toml version $VER, got '$ACTUAL'" >&2
  exit 1
fi
if [ "$LOCK_VER" != "$VER" ]; then
  echo "ERROR: Cargo.lock still has $PKG_NAME $LOCK_VER (expected $VER)" >&2
  exit 1
fi

# Same gate CI uses for --locked builds.
cargo metadata --format-version 1 --locked >/dev/null

echo "version $VER  (Cargo.toml + Cargo.lock + --locked ok)"
