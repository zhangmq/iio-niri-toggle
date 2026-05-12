# iio-niri-toggle — Project Status

## Architecture

```
┌─ iio-niri-toggle (daemon, bash, systemd) ─────────────────┐
│                                                            │
│  Starts persistent iio-niri-listener (Rust) once at boot   │
│  Never restarts it. Listener survives session transitions  │
│                                                            │
│  Per session:                                              │
│    _wait_for_sock (event-driven, no poll)                  │
│    → export NIRI_SOCKET                                    │
│    → _daemon_apply_session (locked-mode preset)            │
│    → monitor loop (inotifywait on state.json + sock del)   │
│                                                            │
│  IPC: Unix socket /var/run/iio-niri-toggle.sock            │
│  QML plugin / CLI sends lock/unlock/status via this socket │
└────────────────────────────────────────────────────────────┘

┌─ iio-niri-listener (Rust, persistent) ─────────────────────┐
│                                                            │
│  1. D-Bus: connect to system bus                           │
│  2. ClaimAccelerometer (once, at boot, when stable)        │
│  3. Subscribe to PropertiesChanged signals                 │
│  4. conn.process(1000ms) = event-driven, 1s health check   │
│  5. Each cycle: read config + apply (always, no tracking)  │
│                                                            │
│  Auto-rotate: sensor → transform map → niri msg            │
│  Locked mode: state.json locked_transform → niri msg       │
│  No socket tracking: always reads & applies each cycle     │
└────────────────────────────────────────────────────────────┘
```

## Files

| Path | Description |
|------|-------------|
| files/iio-niri-toggle | Main daemon (bash) |
| files/iio-niri-listener | Rust listener binary |
| files/iio-niri-toggle.service | systemd service |
| files/install.sh | Install script |
| listener/src/main.rs | Rust listener source |
| listener/Cargo.toml | Rust project config |
| plugin.json | QML plugin config |
| IIONiriToggle.qml | QML panel |

## Status

### Working
- Auto-rotate after boot
- Lock rotation mode
- Session transition detection (socket deletion event)
- All inotifywait timeouts removed (no -t 30 / -t 5)
- Daemon has zero sleep/poll (all waits are inotifywait blocking)
- IPC lock/unlock commands

### TODO
- [x] Locked mode pre-set (set transform before starting iio-niri)
- [x] Session switch acceleration (socket deletion event)
- [x] iio-niri crash detection + auto-restart
- [x] Persistent Rust listener (no per-session restart)
- [x] All inotifywait timeouts removed
- [x] Event-driven waiting (no sleep/poll)
- [x] Fix auto-rotate session switch bug
- [x] Fix locked mode session switch bug
- [x] Clean up zombie IPC processes
- [x] Commit feat/persistent-listener branch

### Key Paths

| Variable | Path |
|----------|------|
| state.json | /var/lib/iio-niri-toggle/state.json |
| IPC socket | /var/run/iio-niri-toggle.sock |
| Rust listener | /usr/local/bin/iio-niri-listener |
| daemon | /usr/local/bin/iio-niri-toggle |
| service | /etc/systemd/system/iio-niri-toggle.service |

### state.json Format

```json
{
  "auto_rotate": true,
  "locked_transform": null,
  "monitor": "eDP-1"
}
```

- auto_rotate: true = auto-rotate mode, false = locked mode
- locked_transform: "normal" | "90" | "180" | "270" | null
