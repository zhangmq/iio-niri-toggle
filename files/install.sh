#!/bin/bash
# install.sh — Install iio-niri-toggle system-wide
set -euo pipefail

FILES_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== iio-niri-toggle installer ==="

echo "→ Building Rust binary"
(cd "$FILES_DIR/../listener" && cargo build --release)

echo "→ Installing /usr/local/bin/iio-niri-listener"
install -m 755 "$FILES_DIR/../listener/target/release/iio-niri-listener" /usr/local/bin/iio-niri-listener

echo "→ Installing compat wrapper /usr/local/bin/iio-niri-toggle"
install -m 755 "$FILES_DIR/iio-niri-toggle" /usr/local/bin/iio-niri-toggle

echo "→ Installing /etc/systemd/system/iio-niri-toggle.service"
install -m 644 "$FILES_DIR/iio-niri-toggle.service" /etc/systemd/system/iio-niri-toggle.service

echo "→ Enabling iio-niri-toggle.service"
systemctl daemon-reload
systemctl enable iio-niri-toggle.service
systemctl restart iio-niri-toggle.service || true

echo "=== Done ==="
