#!/usr/bin/env bash
# Unit test for check_tag_version.sh. Runs on every push/PR via ci.yml —
# this is the piece that was never actually verified to fail on a
# mismatched tag/version (only the success path had been exercised).
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
guard="$script_dir/check_tag_version.sh"

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

cat > "$fixture/Cargo.toml" <<'EOF'
[workspace.package]
version = "1.2.3"
EOF

pass=0

echo "case 1: matching tag/version must succeed"
if "$guard" "v1.2.3" "$fixture/Cargo.toml"; then
  echo "  ok"
else
  echo "  FAIL: expected success for matching version"
  pass=1
fi

echo "case 2: mismatched tag/version must fail"
if "$guard" "v9.9.9" "$fixture/Cargo.toml" 2>/dev/null; then
  echo "  FAIL: expected failure for mismatched version, but guard succeeded"
  pass=1
else
  echo "  ok (guard correctly rejected the mismatch)"
fi

echo "case 3: a partial/short version tag must be rejected"
if "$guard" "v1.2" "$fixture/Cargo.toml" 2>/dev/null; then
  echo "  FAIL: expected failure for an incomplete version tag"
  pass=1
else
  echo "  ok"
fi

exit "$pass"
