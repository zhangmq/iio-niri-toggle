# iio-niri-toggle

Auto-rotate screen for [niri](https://github.com/niri-wm/niri) Wayland compositor using [iio-sensor-proxy](https://gitlab.freedesktop.org/hadess/iio-sensor-proxy/).

## Background

Designed for x86 2-in-1 tablets running niri + [DankMaterialShell](https://danklinux.com/), where the built-in screen auto-rotation is missing.

Initially based on [iio-niri](https://github.com/Zhaith-Izaliel/iio-niri), but later rewritten as a standalone single-binary daemon for practical reasons: session switch handling, greetd compatibility, persistent state across sessions, and simplified deployment.

Auto-detects the internal display (eDP/DSI/LVDS) via sysfs at startup. Exits with an error if no built-in display is found.

## Dependencies

- [niri](https://github.com/niri-wm/niri) Wayland compositor
- [iio-sensor-proxy](https://gitlab.freedesktop.org/hadess/iio-sensor-proxy/) — hardware sensor daemon
- [DankMaterialShell](https://danklinux.com/) + Quickshell — panel widget (optional)
- systemd — service management

## Build Dependencies

- Rust ≥ 1.91 (edition 2021)
- libdbus development headers (`pkg-config`)

## Installation

### Daemon

```bash
cargo build --release
sudo bash deploy/install.sh
```

### DMS Widget (Optional)

`install.sh` installs the widget too and asks for the location:
- **User-level** (default) — `~/.config/DankMaterialShell/plugins/iio-niri-toggle`
- **System-level** — `/etc/xdg/quickshell/dms-plugins/iio-niri-toggle`

Manual install:

```bash
# user-level
cp -r plugin ~/.config/DankMaterialShell/plugins/iio-niri-toggle
# system-level
sudo cp -r plugin /etc/xdg/quickshell/dms-plugins/iio-niri-toggle
```

Then add `iio-niri-toggle` to your DMS panel bar configuration.

The widget appears in two places:

- **DankBar** — click to toggle auto-rotate on/off
- **Control Center** — open the control center, click the edit button to enter edit mode, find "屏幕旋转" in the widget list, and add it to the grid. Click the tile to toggle.

Widget labels follow the system language (English / 简体中文).

## Usage

| Command | Description |
|---------|-------------|
| `iio-niri-toggle daemon` | Start the daemon (managed by systemd) |
| `iio-niri-toggle lock` | Lock current screen orientation |
| `iio-niri-toggle unlock` | Resume auto-rotation |
| `iio-niri-toggle status` | Show current state |
| `iio-niri-toggle toggle` | Toggle between locked and auto-rotate |

## Architecture

Single Rust binary with a poll-based event loop (200ms timeout), integrating in a single thread:

- **D-Bus** — connects to iio-sensor-proxy, subscribes to `AccelerometerOrientation` changes
- **inotify** — watches state file changes and niri socket lifecycle in `/run/user/`
- **IPC** — Unix domain socket for lock/unlock/status commands
- **niri CLI** — applies transforms via `niri msg output <monitor> transform <tr>`
- **Health check** — re-queries orientation every 30s as fallback for missed signals
- **systemd hardening** — the daemon runs as root (it must reach the logged-in user's niri socket under `/run/user/<uid>`); the unit ships with sandbox options (`ProtectSystem`, `PrivateNetwork`, capability stripping) to keep privileges minimal

### State Machine

Two modes:
- **Auto-rotate** — transform driven by real-time sensor orientation (D-Bus signals)
- **Locked** — transform fixed to persisted value; sensor changes ignored

State is persisted to `/var/lib/iio-niri-toggle/state.json`. The apply block is read-only (never writes state.json).

## Known Limitations

- **DMS control center widget lazy-loading**: If only the control center widget is enabled (no DankBar pill), the QML plugin instance is not active until the control center is opened at least once. During this time, `iio-niri-toggle lock/unlock` CLI commands will work but the toast notification will not show. This is a DMS/Quickshell behavior — plugins without a bar pill are not instantiated until their control center slot is rendered.

## Releases

Tag a version to build and publish automatically:

```bash
git tag v1.0.1 && git push origin v1.0.1
```

GitHub Actions (archlinux container) packages `iio-niri-toggle-<version>-x86_64-unknown-linux-gnu.tar.gz` (binary, systemd unit, DMS plugin, install script and docs) plus a `SHA256SUMS` checksum and attaches them to a GitHub Release.

### Install from a Release

```bash
# Download and verify
curl -LO https://github.com/zhangmq/iio-niri-toggle/releases/download/v1.0.1/iio-niri-toggle-1.0.1-x86_64-unknown-linux-gnu.tar.gz
curl -LO https://github.com/zhangmq/iio-niri-toggle/releases/download/v1.0.1/SHA256SUMS
sha256sum -c SHA256SUMS

# Extract and install
tar xzf iio-niri-toggle-1.0.1-x86_64-unknown-linux-gnu.tar.gz
cd iio-niri-toggle-1.0.1-x86_64-unknown-linux-gnu
sudo bash install.sh
```

Runtime dependencies: glibc and libdbus-1 (present by default on Arch-based systems).

## Collaboration

This is a personal tool. There is no plan for collaborative development. You are welcome to fork and adapt it to your needs.

## License

MIT
