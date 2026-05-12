use dbus::blocking::{Connection, Proxy};
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use glob::glob;
use serde_json::Value;
use std::io::Read;
use std::process::Command;
use std::time::Duration;

const SENSOR_SERVICE: &str = "net.hadess.SensorProxy";
const SENSOR_PATH: &str = "/net/hadess/SensorProxy";
const STATE_FILE: &str = "/var/lib/iio-niri-toggle/state.json";
const MONITOR: &str = "eDP-1";

fn find_niri_socket() -> Option<String> {
    let mut socks: Vec<_> = glob("/run/user/*/niri*.sock")
        .ok()?
        .filter_map(|e| e.ok())
        .collect();
    socks.sort();
    socks.into_iter().next().map(|p| p.to_string_lossy().to_string())
}

fn read_config() -> (bool, String) {
    let mut f = match std::fs::File::open(STATE_FILE) {
        Ok(f) => f,
        Err(_) => return (true, String::new()),
    };
    let mut s = String::new();
    if f.read_to_string(&mut s).is_err() {
        return (true, String::new());
    }
    match serde_json::from_str::<Value>(&s) {
        Ok(v) => {
            let unlocked = v.get("auto_rotate").and_then(|x| x.as_bool()).unwrap_or(true);
            let locked = v.get("locked_transform")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            (unlocked, locked)
        }
        Err(_) => (true, String::new()),
    }
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("listener: connecting to system bus");
    let conn = Connection::new_system()?;

    conn.add_match_no_cb(
        "type='signal',interface='org.freedesktop.DBus.Properties',sender='net.hadess.SensorProxy'",
    )?;
    eprintln!("listener: subscribed to signals");

    let proxy = Proxy::new(SENSOR_SERVICE, SENSOR_PATH, Duration::from_secs(5), &conn);
    proxy.method_call::<(), _, _, _>(SENSOR_SERVICE, "ClaimAccelerometer", ())?;
    eprintln!("listener: claim OK");

    conn.process(Duration::from_millis(0))?;
    let initial: String = proxy.get(SENSOR_SERVICE, "AccelerometerOrientation")?;
    eprintln!("listener: initial: {}", initial);
    let (unlocked, locked_tr) = read_config();
    let tr = if unlocked { orientation_to_transform(&initial) } else { &locked_tr };
    if !tr.is_empty() && set_transform(tr) {
        eprintln!("listener: initialized");
    }

    eprintln!("listener: monitoring orientation changes...");
    loop {
        conn.process(Duration::from_millis(1000))?;
        let orient: String = proxy.get(SENSOR_SERVICE, "AccelerometerOrientation")?;
        let (unlocked, locked_tr) = read_config();
        let tr = if unlocked { orientation_to_transform(&orient) } else { &locked_tr };
        if !tr.is_empty() {
            set_transform(tr);
        }
    }
}
