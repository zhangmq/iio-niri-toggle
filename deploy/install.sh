#!/bin/bash
# install.sh — Install iio-niri-toggle system-wide
set -euo pipefail

FILES_DIR="$(cd "$(dirname "$0")" && pwd)"
RELEASE_BIN="$FILES_DIR/../target/release/iio-niri-toggle"

echo "=== iio-niri-toggle installer ==="

if [ ! -f "$RELEASE_BIN" ]; then
    echo "→ Binary not found, building first (run outside sudo if rustup is not root)"
    (cd "$FILES_DIR/.." && cargo build --release)
fi

echo "→ Installing /usr/local/bin/iio-niri-toggle"
install -m 755 "$RELEASE_BIN" /usr/local/bin/iio-niri-toggle

echo "→ Installing /etc/systemd/system/iio-niri-toggle.service"
install -m 644 "$FILES_DIR/iio-niri-toggle.service" /etc/systemd/system/iio-niri-toggle.service

echo "→ Restarting iio-niri-toggle.service"
systemctl daemon-reload
systemctl enable iio-niri-toggle.service
systemctl restart iio-niri-toggle.service
systemctl status iio-niri-toggle.service --no-pager

echo "=== Done ==="
