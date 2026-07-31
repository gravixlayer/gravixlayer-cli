#!/bin/sh
# Set [package].version in Cargo.toml and verify it stuck.
# Usage: ./scripts/set-version.sh 0.1.1
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VER="${1:-}"
if [ -z "$VER" ]; then
  echo "usage: $0 <X.Y.Z[-pre]>" >&2
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

cargo metadata --format-version 1 --no-deps >/dev/null

ACTUAL="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' Cargo.toml | head -1)"
if [ "$ACTUAL" != "$VER" ]; then
  echo "ERROR: expected Cargo.toml version $VER, got '$ACTUAL'" >&2
  exit 1
fi

echo "Cargo.toml version: $ACTUAL  (ok)"
