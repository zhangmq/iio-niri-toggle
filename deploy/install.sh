#!/bin/bash
# install.sh — Install iio-niri-toggle system-wide.
# Works both from the release tarball (binary next to this script)
# and from a repo checkout (binary under ../target/release/).
set -euo pipefail

FILES_DIR="$(cd "$(dirname "$0")" && pwd)"

if [ -f "$FILES_DIR/iio-niri-toggle" ]; then
    RELEASE_BIN="$FILES_DIR/iio-niri-toggle"
elif [ -f "$FILES_DIR/../target/release/iio-niri-toggle" ]; then
    RELEASE_BIN="$FILES_DIR/../target/release/iio-niri-toggle"
else
    echo "→ Binary not found, building first (run outside sudo if rustup is not root)"
    (cd "$FILES_DIR/.." && cargo build --release)
    RELEASE_BIN="$FILES_DIR/../target/release/iio-niri-toggle"
fi

echo "=== iio-niri-toggle installer ==="

echo "→ Installing /usr/local/bin/iio-niri-toggle"
install -m 755 "$RELEASE_BIN" /usr/local/bin/iio-niri-toggle

echo "→ Installing /etc/systemd/system/iio-niri-toggle.service"
install -m 644 "$FILES_DIR/iio-niri-toggle.service" /etc/systemd/system/iio-niri-toggle.service

echo "→ Restarting iio-niri-toggle.service"
systemctl daemon-reload
systemctl enable iio-niri-toggle.service
systemctl restart iio-niri-toggle.service
systemctl status iio-niri-toggle.service --no-pager

if [ -d "$FILES_DIR/plugin" ]; then
    echo
    echo "→ DMS plugin found at $FILES_DIR/plugin (optional)."
    echo "  To enable the panel/control-center widget, link it for your user:"
    echo "  ln -s \"$FILES_DIR/plugin\" ~/.config/DankMaterialShell/plugins/iio-niri-toggle"
fi

echo "=== Done ==="
