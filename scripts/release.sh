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

OLD_VERSION="$(grep -m1 -E '^version = "[0-9]+\.[0-9]+\.[0-9]+"' Cargo.toml | sed -E 's/version = "(.*)"/\1/')"
if [ -z "$OLD_VERSION" ]; then
  echo "error: could not read the current workspace version from Cargo.toml" >&2
  exit 1
fi
if [ "$OLD_VERSION" = "$NEW_VERSION" ]; then
  echo "error: ${NEW_VERSION} is already the current version" >&2
  exit 1
fi

echo "==> bumping workspace version ${OLD_VERSION} -> ${NEW_VERSION}"
sed -i.bak -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/s//version = \"${NEW_VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

# Internal path-dependency pins (gai-probe -> gai-core, gai -> gai-core/gai-probe)
# aren't workspace-inherited and don't get touched by the substitution above —
# this is what let v0.2.0's crates.io publish nearly break (stale "0.1.3" pins
# didn't satisfy the newly-published 0.2.0). Bump every literal occurrence of
# the old version string across the member manifests too.
for member_toml in gai-core/Cargo.toml gai-probe/Cargo.toml gai/Cargo.toml; do
  sed -i.bak -E "s/version = \"${OLD_VERSION}\"/version = \"${NEW_VERSION}\"/g" "$member_toml"
  rm -f "${member_toml}.bak"
done

# Hard-verify the bump actually landed before doing anything irreversible.
# Silent no-op failures here are exactly how v0.2.1 got tagged with
# Cargo.toml still reading 0.2.0 — sed doesn't error on zero matches, so
# this used to sail straight through to `git tag` and `git push`.
if ! grep -q "^version = \"${NEW_VERSION}\"" Cargo.toml; then
  echo "error: Cargo.toml still doesn't show version = \"${NEW_VERSION}\" after the bump — aborting before tag/push" >&2
  exit 1
fi

echo "==> regenerating Cargo.lock"
cargo build --workspace --quiet

echo "==> committing"
git add Cargo.toml Cargo.lock CHANGELOG.md gai-core/Cargo.toml gai-probe/Cargo.toml gai/Cargo.toml
if git diff --cached --quiet; then
  echo "error: nothing staged to commit even though the version bump verified above — refusing to tag an unchanged commit" >&2
  exit 1
fi
git commit -m "chore(release): v${NEW_VERSION}"

if git rev-parse "v${NEW_VERSION}" >/dev/null 2>&1; then
  echo "error: tag v${NEW_VERSION} already exists locally" >&2
  exit 1
fi

echo "==> tagging"
git tag "v${NEW_VERSION}"

echo "==> pushing"
git push origin main
git push origin "v${NEW_VERSION}"

echo "==> done. Watch the release workflow: https://github.com/casablanque-code/gai/actions"
