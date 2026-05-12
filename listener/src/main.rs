use dbus::arg::{RefArg, Variant};
use dbus::blocking::{Connection, Proxy};
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use dbus::message::MatchRule;
use dbus::Message;
use glob::glob;
use inotify::{Inotify, WatchMask};
use libc::{poll, pollfd, POLLIN};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const SENSOR_SRV: &str = "net.hadess.SensorProxy";
const SENSOR_PATH: &str = "/net/hadess/SensorProxy";
const STATE_DIR: &str = "/var/lib/iio-niri-toggle";
const STATE_FILE: &str = "/var/lib/iio-niri-toggle/state.json";
const IPC_SOCK: &str = "/var/run/iio-niri-toggle.sock";
const MONITOR: &str = "eDP-1";

// ── state.json ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Config {
    auto_rotate: bool,
    locked_transform: String,
}

fn read_config() -> Config {
    let mut f = match std::fs::File::open(STATE_FILE) {
        Ok(f) => f,
        Err(_) => return Config { auto_rotate: true, locked_transform: String::new() },
    };
    let mut s = String::new();
    if f.read_to_string(&mut s).is_err() {
        return Config { auto_rotate: true, locked_transform: String::new() };
    }
    match serde_json::from_str::<Value>(&s) {
        Ok(v) => Config {
            auto_rotate: v.get("auto_rotate").and_then(|x| x.as_bool()).unwrap_or(true),
            locked_transform: v.get("locked_transform")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        },
        Err(_) => Config { auto_rotate: true, locked_transform: String::new() },
    }
}

fn write_state(auto: bool, monitor: &str) {
    let _ = std::fs::create_dir_all(STATE_DIR);
    let d = serde_json::json!({
        "auto_rotate": auto,
        "locked_transform": null,
        "monitor": monitor,
    });
    if let Ok(s) = serde_json::to_string(&d) {
        let _ = std::fs::write(STATE_FILE, s.as_bytes());
        let _ = std::fs::set_permissions(STATE_FILE, std::fs::Permissions::from_mode(0o644));
    }
}

fn init_state() {
    if !Path::new(STATE_FILE).exists() {
        let _ = std::fs::create_dir_all(STATE_DIR);
        let d = serde_json::json!({
            "auto_rotate": true,
            "locked_transform": null,
            "monitor": MONITOR,
        });
        if let Ok(s) = serde_json::to_string(&d) {
            let _ = std::fs::write(STATE_FILE, s.as_bytes());
        }
    }
}

// ── niri ────────────────────────────────────────────────────────────────────

fn find_niri_socket() -> Option<String> {
    let mut socks: Vec<_> = glob("/run/user/*/niri*.sock")
        .ok()?
        .filter_map(|e| e.ok())
        .collect();
    socks.sort();
    socks.into_iter().next().map(|p| p.to_string_lossy().to_string())
}

fn set_transform(tr: &str) -> bool {
    let sock = match find_niri_socket() {
        Some(s) => s,
        None => return false,
    };
    let output = Command::new("niri")
        .args(["msg", "output", MONITOR, "transform", tr])
        .env("NIRI_SOCKET", &sock)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            eprintln!("listener: applied {} -> {}", MONITOR, tr);
            true
        }
        Ok(o) => {
            eprintln!("listener: apply failed: {}", String::from_utf8_lossy(&o.stderr));
            false
        }
        Err(e) => {
            eprintln!("listener: apply error: {}", e);
            false
        }
    }
}

fn orientation_to_transform(orient: &str) -> &str {
    match orient {
        "normal" => "normal",
        "bottom-up" => "180",
        "left-up" => "90",
        "right-up" => "270",
        _ => "",
    }
}

fn apply(cfg: &Config, orient: &str) -> bool {
    let tr = if cfg.auto_rotate { orientation_to_transform(orient) } else { &cfg.locked_transform };
    if tr.is_empty() { return false; }
    set_transform(tr)
}

// ── D-Bus ───────────────────────────────────────────────────────────────────

fn setup_dbus() -> Result<(Connection, String), Box<dyn std::error::Error>> {
    let conn = Connection::new_system()?;
    let proxy = Proxy::new(SENSOR_SRV, SENSOR_PATH, Duration::from_secs(5), &conn);
    conn.add_match_no_cb(
        "type='signal',interface='org.freedesktop.DBus.Properties',\
         path='/net/hadess/SensorProxy',sender='net.hadess.SensorProxy'",
    )?;
    proxy.method_call::<(), _, _, _>(SENSOR_SRV, "ClaimAccelerometer", ())?;

    conn.process(Duration::from_millis(0))?;
    let orient: String = proxy.get(SENSOR_SRV, "AccelerometerOrientation")?;
    Ok((conn, orient))
}

/// Re-query orientation from iio-sensor-proxy (blocking, up to 5s).
fn requery_orientation(conn: &Connection) -> Result<String, Box<dyn std::error::Error>> {
    conn.process(Duration::from_millis(200))?;
    let proxy = Proxy::new(SENSOR_SRV, SENSOR_PATH, Duration::from_secs(5), conn);
    let orient: String = proxy.get(SENSOR_SRV, "AccelerometerOrientation")?;
    Ok(orient)
}

// Parse AccelerometerOrientation from PropertiesChanged signal body.
fn parse_orient_signal(msg: &Message) -> Option<String> {
    let (interface, changed, _invalidated): (
        String,
        HashMap<String, Variant<Box<dyn RefArg>>>,
        Vec<String>,
    ) = msg.read3().ok()?;
    if interface != SENSOR_SRV { return None; }
    let variant = changed.get("AccelerometerOrientation")?;
    variant.0.as_str().map(|s| s.to_string())
}

// ── inotify ─────────────────────────────────────────────────────────────────

fn setup_inotify() -> Result<Inotify, Box<dyn std::error::Error>> {
    let inotify = Inotify::init()?;
    {
        let mut watches = inotify.watches();
        watches.add(Path::new(STATE_DIR), WatchMask::CLOSE_WRITE)?;
        watches.add(Path::new("/run/user"), WatchMask::CREATE)?;
    }
    Ok(inotify)
}

fn read_inotify_events(inotify: &mut Inotify, buffer: &mut [u8]) -> Vec<String> {
    let mut names = Vec::new();
    let events = match inotify.read_events_blocking(buffer) {
        Ok(e) => e,
        Err(_) => return names,
    };
    for event in events {
        if let Some(name) = event.name {
            names.push(name.to_string_lossy().to_string());
        }
    }
    names
}

// ── IPC server ──────────────────────────────────────────────────────────────

fn setup_ipc() -> Result<UnixListener, Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file(IPC_SOCK);
    let listener = UnixListener::bind(IPC_SOCK)?;
    std::fs::set_permissions(IPC_SOCK, std::fs::Permissions::from_mode(0o666))?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

fn handle_ipc_client(mut stream: UnixStream) {
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let req: Value = match serde_json::from_slice(&buf[..n]) {
        Ok(v) => v,
        Err(_) => return,
    };
    let cmd = req.get("command").and_then(|x| x.as_str()).unwrap_or("");

    let resp = match cmd {
        "lock" | "unlock" => {
            let auto = cmd == "unlock";
            write_state(auto, MONITOR);
            serde_json::json!({"ok": true, "auto_rotate": auto})
        }
        "status" => {
            let cfg = read_config();
            serde_json::json!({
                "ok": true,
                "state": {
                    "auto_rotate": cfg.auto_rotate,
                    "locked_transform": if cfg.locked_transform.is_empty() {
                        Value::Null
                    } else {
                        Value::String(cfg.locked_transform)
                    },
                    "monitor": MONITOR,
                }
            })
        }
        _ => serde_json::json!({"ok": false, "error": format!("unknown command: {}", cmd)}),
    };
    let _ = stream.write_all(format!("{}\n", serde_json::to_string(&resp).unwrap_or_default()).as_bytes());
}

// ── daemon ──────────────────────────────────────────────────────────────────

fn cmd_daemon() -> Result<(), Box<dyn std::error::Error>> {
    init_state();
    eprintln!("listener: daemon started");

    // D-Bus (blocking once at startup)
    let (conn, initial_orient) = setup_dbus()?;
    eprintln!("listener: initial orientation: {}", initial_orient);

    // inotify
    let mut inotify = setup_inotify()?;
    let mut inotify_buf = [0u8; 4096];

    // IPC
    let ipc_listener = setup_ipc()?;
    eprintln!("listener: IPC socket at {}", IPC_SOCK);

    let inotify_fd = inotify.as_raw_fd();
    let ipc_fd = ipc_listener.as_raw_fd();

    // Orientation: only re-queried when PropertiesChanged signal arrives.
    let orient_pending = Arc::new(AtomicBool::new(false));
    let mut cached_orient = initial_orient;

    // Register D-Bus PropertiesChanged handler
    {
        let pending = Arc::clone(&orient_pending);
        let rule = MatchRule::new_signal("org.freedesktop.DBus.Properties", "PropertiesChanged")
            .with_sender(SENSOR_SRV)
            .with_path(SENSOR_PATH);
        conn.add_match::<(), _>(rule, move |_: (), _conn: &Connection, msg: &Message| {
            if let Some(_orient) = parse_orient_signal(msg) {
                pending.store(true, Ordering::SeqCst);
            }
            true
        })?;
    }

    // Initial apply
    let cfg = read_config();
    apply(&cfg, &cached_orient);

    // Event loop: poll inotify + IPC fds, D-Bus via short-timeout fallback
    let mut sock_cache: Option<String> = None;
    let mut health = Instant::now();
    loop {
        let mut fds = [
            pollfd { fd: inotify_fd, events: POLLIN as i16, revents: 0 },
            pollfd { fd: ipc_fd, events: POLLIN as i16, revents: 0 },
        ];

        // 200ms poll: event-driven for inotify/IPC, but bounds D-Bus latency
        let ret = unsafe { poll(fds.as_mut_ptr(), 2, 200) };
        if ret < 0 {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        let mut need_apply = false;

        // ── inotify ───────────────────────────────────────────────
        if ret > 0 && (fds[0].revents & POLLIN as i16) != 0 {
            let events = read_inotify_events(&mut inotify, &mut inotify_buf);
            for name in &events {
                if name == "state.json" || name.starts_with("niri") {
                    need_apply = true;
                }
            }
        }

        // ── IPC ───────────────────────────────────────────────────
        if ret > 0 && (fds[1].revents & POLLIN as i16) != 0 {
            while let Ok((stream, _)) = ipc_listener.accept() {
                handle_ipc_client(stream);
                // IPC writes state.json → inotify will trigger need_apply
            }
        }

        // ── D-Bus: process pending messages ───────────────────────
        let _ = conn.process(Duration::ZERO);
        let signal_hit = orient_pending.swap(false, Ordering::SeqCst);
        if signal_hit {
            match requery_orientation(&conn) {
                Ok(orient) if orient != cached_orient => {
                    eprintln!("listener: orientation: {} -> {}", cached_orient, orient);
                    cached_orient = orient;
                    need_apply = true;
                }
                Err(e) => eprintln!("listener: orientation query: {}", e),
                _ => {}
            }
        }

        // ── Socket check (every iteration, cheap glob) ────────────
        let current_sock = find_niri_socket();
        if current_sock != sock_cache {
            sock_cache = current_sock;
            need_apply = true;
        }

        // ── Periodic health: re-query orientation every 30s ───────
        // (safety net if PropertiesChanged signals are missed)
        if health.elapsed() >= Duration::from_secs(30) {
            health = Instant::now();
            match requery_orientation(&conn) {
                Ok(orient) if orient != cached_orient => {
                    eprintln!("listener: health check: {} -> {}", cached_orient, orient);
                    cached_orient = orient;
                    need_apply = true;
                }
                _ => {}
            }
        }

        // ── Apply ─────────────────────────────────────────────────
        if need_apply {
            let cfg = read_config();
            apply(&cfg, &cached_orient);
        }
    }
}

// ── client ──────────────────────────────────────────────────────────────────

fn cmd_send(command: &str) {
    let cmd = match command {
        "lock" | "unlock" | "status" => command.to_string(),
        "toggle" => {
            let mut stream = match UnixStream::connect(IPC_SOCK) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("iio-niri-toggle: cannot connect to daemon");
                    std::process::exit(1);
                }
            };
            let _ = stream.write_all(b"{\"command\":\"status\"}\n");
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let resp: Value = serde_json::from_slice(&buf[..n]).unwrap_or_default();
            let auto = resp.get("state")
                .and_then(|s| s.get("auto_rotate"))
                .and_then(|a| a.as_bool())
                .unwrap_or(true);
            if auto { "lock".to_string() } else { "unlock".to_string() }
        }
        _ => {
            eprintln!("Usage: iio-niri-listener send <lock|unlock|status|toggle>");
            std::process::exit(1);
        }
    };

    let mut stream = match UnixStream::connect(IPC_SOCK) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("iio-niri-toggle: cannot connect to daemon");
            std::process::exit(1);
        }
    };
    let payload = format!("{{\"command\":\"{}\"}}\n", cmd);
    let _ = stream.write_all(payload.as_bytes());

    if cmd == "status" {
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        if n > 0 {
            if let Ok(v) = serde_json::from_slice::<Value>(&buf[..n]) {
                if let Some(state) = v.get("state") {
                    let auto = state.get("auto_rotate").and_then(|x| x.as_bool()).unwrap_or(true);
                    let locked = state.get("locked_transform").and_then(|x| x.as_str()).unwrap_or("none");
                    println!("auto_rotate: {}", auto);
                    println!("locked_transform: {}", locked);
                }
            }
        }
    }
}

// ── entry ───────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("daemon") => {
            if let Err(e) = cmd_daemon() {
                eprintln!("listener: daemon error: {}", e);
                std::process::exit(1);
            }
        }
        Some("send") => {
            let cmd = args.get(2).map(|s| s.as_str()).unwrap_or("");
            cmd_send(cmd);
        }
        Some("lock") => cmd_send("lock"),
        Some("unlock") => cmd_send("unlock"),
        Some("status") => cmd_send("status"),
        _ => {
            eprintln!("Usage: iio-niri-listener {{daemon|send <cmd>|lock|unlock|status}}");
            std::process::exit(1);
        }
    }
}
