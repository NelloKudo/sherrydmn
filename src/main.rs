use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use zbus::blocking::Connection;

const POLL_INTERVAL: Duration = Duration::from_millis(12000);

// process comm names that show up when proton is running
const WINE_COMMS: &[&str] = &["wine", "wineserver"];

const PPD_DEST: &str = "org.freedesktop.UPower.PowerProfiles";
const PPD_PATH: &str = "/org/freedesktop/UPower/PowerProfiles";
const PPD_IFACE: &str = "org.freedesktop.UPower.PowerProfiles";

fn hold_performance(conn: &Connection) -> Option<u32> {
    let reply = conn
        .call_method(
            Some(PPD_DEST),
            PPD_PATH,
            Some(PPD_IFACE),
            "HoldProfile",
            &("performance", "game running", "sherrydmn"),
        )
        .map_err(|e| eprintln!("sherrydmn: HoldProfile failed: {e}"))
        .ok()?;

    let hold: u32 = reply
        .body()
        .deserialize()
        .map_err(|e| eprintln!("sherrydmn: failed to parse hold id: {e}"))
        .ok()?;

    eprintln!("sherrydmn: performance profile held (hold={hold})");
    Some(hold)
}

fn release_performance(conn: &Connection, hold: u32) {
    match conn.call_method(Some(PPD_DEST), PPD_PATH, Some(PPD_IFACE), "ReleaseProfile", &(hold,)) {
        Ok(_) => eprintln!("sherrydmn: performance profile released"),
        Err(e) => eprintln!("sherrydmn: ReleaseProfile failed: {e}"),
    }
}

fn find_proton_process() -> Option<String> {
    let entries = match fs::read_dir("/proc") {
        Ok(e) => e,
        Err(_) => return None,
    };

    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();

        // skip non-pid directories
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let pid: u32 = match name.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let pid_dir = entry.path();

        // check /proc/{pid}/comm against known proton process names
        if let Ok(comm) = fs::read_to_string(pid_dir.join("comm")) {
            let comm = comm.trim().to_string();
            for needle in WINE_COMMS {
                if comm.contains(needle) {
                    return Some(format!("pid {pid} comm={comm}"));
                }
            }
        }

        // check /proc/{pid}/cmdline for "proton" in the path
        if let Ok(cmdline) = fs::read(pid_dir.join("cmdline")) {
            if cmdline
                .windows(b"proton".len())
                .any(|w| w.eq_ignore_ascii_case(b"proton"))
            {
                let readable = cmdline
                    .iter()
                    .map(|&b| if b == 0 { b' ' } else { b })
                    .collect::<Vec<u8>>();
                let readable = String::from_utf8_lossy(&readable);
                return Some(format!("pid {pid} cmdline={readable}"));
            }
        }
    }

    None
}

fn main() {
    let conn = Connection::system().expect("failed to connect to system dbus");

    let alive = Arc::new(AtomicBool::new(true));
    let flag = alive.clone();

    ctrlc::set_handler(move || {
        flag.store(false, Ordering::SeqCst);
    })
    .expect("failed to set signal handler");

    let mut perf_hold: Option<u32> = None;

    while alive.load(Ordering::SeqCst) {
        let found = find_proton_process();

        if let Some(info) = found {
            if perf_hold.is_none() {
                eprintln!("sherrydmn: proton detected ({info})");
                perf_hold = hold_performance(&conn);
            }
        } else if let Some(hold) = perf_hold.take() {
            release_performance(&conn, hold);
        }

        thread::sleep(POLL_INTERVAL);
    }

    // release on exit
    if let Some(hold) = perf_hold {
        release_performance(&conn, hold);
    }
}
