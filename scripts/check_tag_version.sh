#!/usr/bin/env bash
# Fails if the pushed tag (vX.Y.Z) doesn't match the workspace version in
# Cargo.toml. Shared by release.yml and publish.yml so there's exactly one
# place this comparison lives — and one place to unit-test it (see
# check_tag_version_test.sh).
#
# Usage: check_tag_version.sh <tag-ref-name> <path-to-workspace-Cargo.toml>
set -euo pipefail

tag_ref="${1:?usage: check_tag_version.sh <tag-ref-name> <cargo-toml-path>}"
cargo_toml="${2:?usage: check_tag_version.sh <tag-ref-name> <cargo-toml-path>}"

tag_version="${tag_ref#v}"
cargo_version="$(grep -m1 '^version' "$cargo_toml" | sed -E 's/version = "(.*)"/\1/')"

if [ "$tag_version" != "$cargo_version" ]; then
  echo "::error::tag v${tag_version} does not match Cargo.toml version ${cargo_version}"
  exit 1
fi

echo "OK: tag v${tag_version} matches Cargo.toml version ${cargo_version}"
