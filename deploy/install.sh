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

# DMS plugin (optional): only when DMS is actually installed on this system.
# Detection: DMS module dir (installed shell) or the user's DMS config dir.
if [ -d "$FILES_DIR/plugin" ]; then
    TARGET_USER="${SUDO_USER:-${USER:-root}}"
    USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"

    if [ -d /usr/share/quickshell/dms ] || [ -d "$USER_HOME/.config/DankMaterialShell" ]; then
        MODE="user"
        if [ "$TARGET_USER" = "root" ]; then
            MODE="system"   # no user context — system-level only
        elif [ -t 0 ]; then
            echo
            read -r -p "DMS 插件安装位置? [u] 用户级（默认） / [s] 系统级: " REPLY || true
            case "$REPLY" in
                s|S) MODE="system" ;;
            esac
        fi

        if [ "$MODE" = "system" ]; then
            PLUGIN_DEST="/etc/xdg/quickshell/dms-plugins/iio-niri-toggle"
        else
            PLUGIN_DEST="$USER_HOME/.config/DankMaterialShell/plugins/iio-niri-toggle"
        fi

        echo "→ Installing DMS plugin to $PLUGIN_DEST"
        rm -rf "$PLUGIN_DEST"
        mkdir -p "$(dirname "$PLUGIN_DEST")"
        cp -r "$FILES_DIR/plugin/." "$PLUGIN_DEST/"
        if [ "$MODE" = "system" ]; then
            chmod -R a+rX "$PLUGIN_DEST"
        else
            chown -R "$TARGET_USER:" "$PLUGIN_DEST"
            chmod -R u+rwX,go+rX "$PLUGIN_DEST"
        fi
        echo "  (DMS 会自动重载插件；若未出现请重启 DMS shell)"
    else
        echo "→ DMS 未安装，跳过 DMS 插件安装（仅安装守护进程）"
    fi
fi

echo "=== Done ==="
