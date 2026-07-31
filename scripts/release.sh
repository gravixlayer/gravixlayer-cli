#!/bin/sh
# Bump version, commit, tag, and push a release.
#
# Usage:
#   1. Edit CHANGELOG.md — add ## [X.Y.Z] - YYYY-MM-DD
#   2. ./scripts/release.sh X.Y.Z
#
# Everyday commits do NOT use this script — only when shipping binaries.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VER="${1:-}"
if [ -z "$VER" ]; then
  echo "usage: $0 <X.Y.Z[-pre]>" >&2
  exit 1
fi

TAG="v${VER}"

# Allow release-related files to already be edited; refuse any other dirt.
DIRTY="$(git status --porcelain | awk '
  $2 == "CHANGELOG.md" { next }
  $2 == "Cargo.toml" { next }
  $2 == "Cargo.lock" { next }
  $2 == "scripts/release.sh" { next }
  $2 == "scripts/set-version.sh" { next }
  $2 == ".github/workflows/release.yml" { next }
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

# Drop a bad same-version tag from a previous failed attempt.
if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Removing local tag $TAG..."
  git tag -d "$TAG" >/dev/null
fi
if git ls-remote --exit-code --tags origin "refs/tags/${TAG}" >/dev/null 2>&1; then
  echo "Removing remote tag $TAG..."
  git push origin ":refs/tags/${TAG}"
fi

./scripts/set-version.sh "$VER"

git add Cargo.toml Cargo.lock CHANGELOG.md \
  scripts/set-version.sh scripts/release.sh \
  .github/workflows/release.yml
git add -u

if ! git diff --cached --name-only | grep -qx 'Cargo.lock'; then
  echo "ERROR: Cargo.lock is not staged — lockfile was not synced. Aborting." >&2
  exit 1
fi
# Cargo.toml may already be at the target version from a previous attempt;
# Cargo.lock sync is the critical gate for --locked CI builds.

git commit -m "chore: release ${VER}"
git push origin HEAD

# Final gate — same checks Release CI runs before building.
ACTUAL="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' Cargo.toml | head -1)"
LOCK_VER="$(awk '
  /^name = "gravixlayer"$/ { want=1; next }
  want && /^version = "/ {
    gsub(/^version = "/, "")
    gsub(/"$/, "")
    print
    exit
  }
' Cargo.lock)"
if [ "$ACTUAL" != "$VER" ] || [ "$LOCK_VER" != "$VER" ]; then
  echo "ERROR: refusing to tag — Cargo.toml=$ACTUAL Cargo.lock=$LOCK_VER expected=$VER" >&2
  exit 1
fi
cargo metadata --format-version 1 --locked >/dev/null

git tag -a "$TAG" -m "Release ${VER}"
git push origin "$TAG"

echo
echo "Released ${TAG}. Watch GitHub Actions → Release."
echo "Install: curl -fsSL https://cli.gravixlayer.ai/install | sh"
