#!/usr/bin/env bash
# Bumps the workspace version, regenerates Cargo.lock, commits, tags, and
# pushes — the one command that replaces the multi-step manual dance
# (edit Cargo.toml, forget Cargo.lock, tag, push, repeat) that caused
# v0.1.1 and v0.1.2 to ship with a stale --version.
#
# Usage: ./scripts/release.sh 0.1.4

set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <new-version>  (e.g. 0.1.4)" >&2
  exit 1
fi

NEW_VERSION="$1"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! git diff-index --quiet HEAD --; then
  echo "error: working tree is dirty, commit or stash first" >&2
  exit 1
fi

if ! grep -q "\[${NEW_VERSION}\]" CHANGELOG.md 2>/dev/null; then
  echo "warning: CHANGELOG.md has no [${NEW_VERSION}] entry yet — add one before releasing." >&2
  read -rp "Continue anyway? [y/N] " reply
  [[ "$reply" =~ ^[Yy]$ ]] || exit 1
fi

echo "==> bumping workspace version to ${NEW_VERSION}"
sed -i.bak -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/s//version = \"${NEW_VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

echo "==> regenerating Cargo.lock"
cargo build --workspace --quiet

echo "==> committing"
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore(release): v${NEW_VERSION}"

echo "==> tagging"
git tag "v${NEW_VERSION}"

echo "==> pushing"
git push origin main
git push origin "v${NEW_VERSION}"

echo "==> done. Watch the release workflow: https://github.com/casablanque-code/gai/actions"
