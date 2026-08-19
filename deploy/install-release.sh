#!/bin/bash
# install-release.sh — Download the latest iio-niri-toggle release from GitHub,
# verify its checksum, extract, and install (binary + systemd unit + DMS plugin;
# install.sh asks where to put the plugin).
# Usage: bash install-release.sh [VERSION]     (default: latest release)
set -euo pipefail

REPO="zhangmq/iio-niri-toggle"
VERSION="${1:-}"

if [ -z "$VERSION" ]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/.*"v//; s/"$//')"
    [ -n "$VERSION" ] || { echo "error: cannot determine the latest release" >&2; exit 1; }
fi

TARBALL="iio-niri-toggle-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
BASE="https://github.com/$REPO/releases/download/v$VERSION"
echo "=== iio-niri-toggle installer (v$VERSION) ==="

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

echo "→ Downloading $TARBALL"
curl -fsSL -O "$BASE/$TARBALL"
curl -fsSL -O "$BASE/SHA256SUMS"

echo "→ Verifying checksum"
sha256sum -c SHA256SUMS

echo "→ Extracting"
tar xzf "$TARBALL"
cd "iio-niri-toggle-${VERSION}-x86_64-unknown-linux-gnu"

echo "→ Installing (sudo)"
sudo bash install.sh
