#!/usr/bin/env bash
# Installs the latest gai release binary from GitHub Releases.
#
# Usage on the VPS:
#   curl -fsSL https://raw.githubusercontent.com/casablanque-code/gai/main/install.sh | sudo bash
#
# Or pin a version:
#   curl -fsSL .../install.sh | sudo bash -s -- v0.1.0

set -euo pipefail

REPO="casablanque-code/gai"
TARGET="x86_64-unknown-linux-musl"
INSTALL_DIR="/usr/local/bin"
VERSION="${1:-latest}"

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/${REPO}/releases/latest/download/gai-${TARGET}.tar.gz"
else
  URL="https://github.com/${REPO}/releases/download/${VERSION}/gai-${TARGET}.tar.gz"
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "downloading ${URL}"
curl -fsSL "$URL" -o "${TMP_DIR}/gai.tar.gz"
tar xzf "${TMP_DIR}/gai.tar.gz" -C "$TMP_DIR"

install -m 0755 "${TMP_DIR}/gai" "${INSTALL_DIR}/gai"
echo "installed to ${INSTALL_DIR}/gai"
"${INSTALL_DIR}/gai" --version || true
