#!/bin/sh
# Bump Cargo.toml, commit, tag, and push a release.
# Usage:
#   1. Edit CHANGELOG.md for the new version
#   2. ./scripts/release.sh 0.1.1
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VER="${1:-}"
if [ -z "$VER" ]; then
  echo "usage: $0 <X.Y.Z[-pre]>" >&2
  exit 1
fi

TAG="v${VER}"

# Allow CHANGELOG + these scripts to already be edited; refuse any other dirt.
DIRTY="$(git status --porcelain | awk '
  $2 == "CHANGELOG.md" { next }
  $2 == "scripts/release.sh" { next }
  $2 == "scripts/set-version.sh" { next }
  { print }
')"
if [ -n "$DIRTY" ]; then
  echo "ERROR: unrelated dirty files — commit or stash them first:" >&2
  echo "$DIRTY" >&2
  exit 1
fi

if ! grep -q "\[${VER}\]" CHANGELOG.md 2>/dev/null; then
  echo "ERROR: CHANGELOG.md has no ## [${VER}] heading." >&2
  echo "       Add it, then re-run: $0 $VER" >&2
  exit 1
fi

# Drop a bad same-version tag if a previous attempt left one on this machine.
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Removing local tag $TAG from a previous attempt..."
  git tag -d "$TAG" >/dev/null
fi
if git ls-remote --exit-code --tags origin "refs/tags/${TAG}" >/dev/null 2>&1; then
  echo "Removing remote tag $TAG from a previous attempt..."
  git push origin ":refs/tags/${TAG}"
fi

./scripts/set-version.sh "$VER"

git add Cargo.toml Cargo.lock CHANGELOG.md scripts/set-version.sh scripts/release.sh
git add -u

if ! git diff --cached --name-only | grep -q '^Cargo.toml$'; then
  echo "ERROR: Cargo.toml is not staged — version bump did not take. Aborting." >&2
  exit 1
fi

git commit -m "chore: release ${VER}"
git push origin HEAD

ACTUAL="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' Cargo.toml | head -1)"
if [ "$ACTUAL" != "$VER" ]; then
  echo "ERROR: refusing to tag — Cargo.toml is '$ACTUAL', expected '$VER'" >&2
  exit 1
fi

git tag -a "$TAG" -m "Release ${VER}"
git push origin "$TAG"

echo
echo "Released ${TAG}. Watch GitHub Actions → Release."
echo "Install: curl -fsSL https://cli.gravixlayer.ai/install | sh"
