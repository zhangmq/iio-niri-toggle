#!/bin/bash
# uninstall.sh — Remove the iio-niri-toggle deployment: systemd service, binary,
# IPC socket, state, and the DMS plugin (system- and user-level).
# Usage: bash uninstall.sh [-y]        (-y skips the confirmation prompt)
set -euo pipefail

TARGET_USER="${SUDO_USER:-${USER:-}}"
USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"

if [ "${1:-}" != "-y" ]; then
    read -r -p "确定卸载 iio-niri-toggle（含 DMS 插件）？[y/N] " REPLY || true
    case "$REPLY" in
        y|Y) ;;
        *) echo "取消"; exit 1 ;;
    esac
fi

echo "=== Removing iio-niri-toggle ==="
sudo systemctl disable --now iio-niri-toggle.service 2>/dev/null || true
sudo rm -f /etc/systemd/system/iio-niri-toggle.service
sudo rm -f /usr/local/bin/iio-niri-toggle
sudo rm -f /var/run/iio-niri-toggle.sock
sudo rm -rf /var/lib/iio-niri-toggle
sudo rm -rf /etc/xdg/quickshell/dms-plugins/iio-niri-toggle
sudo systemctl daemon-reload
rm -rf "$USER_HOME/.config/DankMaterialShell/plugins/iio-niri-toggle"

echo "=== Done. iio-niri-toggle removed ==="
