# iio-niri-toggle — Project Status

## Architecture

```
┌─ iio-niri-toggle (single Rust binary) ──────────────────────┐
│                                                              │
│  poll(inotify_fd, ipc_fd, 200ms)                             │
│    ├─ inotify (state.json) → need_apply = true               │
│    ├─ IPC (lock/unlock/status) → write_state + need_apply    │
│    ├─ D-Bus (PropertiesChanged) → update cached_orient       │
│    ├─ socket change (glob) → need_apply = true               │
│    ├─ 30s health check (auto-rotate only) → update orient    │
│    └─ apply block (pure, no side effects)                     │
│                                                              │
│  D-Bus connection: ClaimAccelerometer, subscribe to signals   │
│  State: /var/lib/iio-niri-toggle/state.json                  │
│  IPC: Unix socket /var/run/iio-niri-toggle.sock              │
│  CLI: iio-niri-toggle {daemon|send|lock|unlock|status}       │
└──────────────────────────────────────────────────────────────┘
```

## Functional Specification

### Two Modes

#### Auto-rotate mode

| Dimension | Rule |
|-----------|------|
| Flag | `auto_rotate = true`, `locked_transform = null` |
| Transform source | Live sensor orientation via D-Bus `AccelerometerOrientation` |
| Sensor dependency | Strong — every transform is driven by sensor signal |
| state.json dependency | None — transform fully derived from sensor |

| Event | Action | Write state.json |
|-------|--------|:---:|
| D-Bus `PropertiesChanged` | Update cached orient → apply | No |
| New niri socket (session switch) | Apply cached orient | No |
| IPC `unlock` command | Write `auto_rotate=true, locked_transform=null` → apply cached orient | Yes (one-shot) |
| 30s health check (signal loss safety net) | `requery_orientation` → apply if changed | No |

#### Locked mode

| Dimension | Rule |
|-----------|------|
| Flag | `auto_rotate = false`, `locked_transform` = non-empty |
| Transform source | `locked_transform` from state.json (persisted fixed value) |
| Sensor dependency | **None** — sensor signals are ignored for apply |
| state.json dependency | Required — `locked_transform` must survive session switches |

| Event | Action | Write state.json |
|-------|--------|:---:|
| IPC `lock` command | Capture current niri transform → write `locked_transform` → apply | Yes (one-shot) |
| New niri socket (session switch) | Read `locked_transform` → apply | No |
| Sensor change | **Ignored** | No |
| Health check | **Skipped** (sensor irrelevant in locked mode) | No |

### state.json Write Rules

**Only written on these triggers, never elsewhere:**

| Trigger | Written content |
|---------|----------------|
| First boot (no state.json) | `{"auto_rotate": true, "locked_transform": null, "monitor": "eDP-1"}` |
| IPC `lock` | `{"auto_rotate": false, "locked_transform": "<current transform>", "monitor": "eDP-1"}` |
| IPC `unlock` | `{"auto_rotate": true, "locked_transform": null, "monitor": "eDP-1"}` |

**Forbidden:** apply block must NOT write state.json.

### Key Constraints

1. **Apply block is pure**: no write_state, no D-Bus calls, no events.
2. **Locked mode ignores sensor**: D-Bus signals still update `cached_orient` but do NOT set `need_apply`.
3. **Socket retry**: apply fails + socket exists → keep `need_apply` for retry; apply fails + no socket → stop.
4. **IPC is synchronous**: lock/unlock writes state.json in the IPC handler; apply follows on next poll iteration.

### Known Issues

- `inotify` on `/run/user/` only watches direct children. Socket creation in existing user directories (same-UID greetd) relies on 200ms poll timeout as fallback. Latency ≤200ms.

### Key Paths

| Variable | Path |
|----------|------|
| state.json | `/var/lib/iio-niri-toggle/state.json` |
| IPC socket | `/var/run/iio-niri-toggle.sock` |
| Binary | `/usr/local/bin/iio-niri-toggle` |
| Service | `/etc/systemd/system/iio-niri-toggle.service` |

### state.json Format

```json
{
  "auto_rotate": true,
  "locked_transform": null,
  "monitor": "eDP-1"
}
```

- `auto_rotate`: true = auto-rotate mode, false = locked mode
- `locked_transform`: `"normal"` | `"90"` | `"180"` | `"270"` | `null`
