# iio-niri-toggle

Auto-rotate screen for [niri](https://github.com/niri-wm/niri) Wayland compositor using [iio-sensor-proxy](https://gitlab.freedesktop.org/hadess/iio-sensor-proxy/).

## Background

Designed for x86 2-in-1 tablets running niri + [DankMaterialShell](https://danklinux.com/), where the built-in screen auto-rotation is missing.

Initially based on [iio-niri](https://github.com/Zhaith-Izaliel/iio-niri), but later rewritten as a standalone single-binary daemon for practical reasons: session switch handling, greetd compatibility, persistent state across sessions, and simplified deployment.

Currently targets the built-in display only. Future support for auto-detecting the internal display is possible but not planned.

## Dependencies

- [niri](https://github.com/niri-wm/niri) Wayland compositor
- [iio-sensor-proxy](https://gitlab.freedesktop.org/hadess/iio-sensor-proxy/) — hardware sensor daemon
- [DankMaterialShell](https://danklinux.com/) + Quickshell — panel widget (optional)
- systemd — service management

## Build Dependencies

- Rust ≥ 1.91 (edition 2021)
- libdbus development headers (`pkg-config`)

## Installation

```bash
cd iio-niri-toggle && cargo build --release
sudo bash files/install.sh
```

The QML plugin (`plugin.json` + `IIONiriToggle.qml`) provides a panel button to toggle auto-rotate on/off. Place the project directory under your Quickshell plugin path.

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

### State Machine

Two modes:
- **Auto-rotate** — transform driven by real-time sensor orientation (D-Bus signals)
- **Locked** — transform fixed to persisted value; sensor changes ignored

State is persisted to `/var/lib/iio-niri-toggle/state.json`. The apply block is read-only (never writes state.json).

## Collaboration

This is a personal tool. There is no plan for collaborative development. You are welcome to fork and adapt it to your needs.

## License

MIT
