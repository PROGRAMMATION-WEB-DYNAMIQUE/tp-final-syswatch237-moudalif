use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::Local;
use syswatch::{collect_snapshot, format_response, SystemSnapshot};

const DEFAULT_ADDR: &str = "0.0.0.0:7878";
const DEFAULT_LOG: &str = "syswatch.log";

fn log_event(log_file: &str, message: &str) {
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{ts}] {message}\n");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn refresh_loop(shared_snapshot: Arc<Mutex<SystemSnapshot>>) {
    loop {
        match collect_snapshot() {
            Ok(new_snapshot) => {
                if let Ok(mut guard) = shared_snapshot.lock() {
                    *guard = new_snapshot;
                }
            }
            Err(err) => eprintln!("snapshot refresh error: {err}"),
        }
        thread::sleep(Duration::from_secs(5));
    }
}

fn handle_client(
    mut stream: TcpStream,
    peer: String,
    shared_snapshot: Arc<Mutex<SystemSnapshot>>,
    log_file: String,
) {
    log_event(&log_file, &format!("CONNECT {peer}"));
    let banner = "SysWatch Agent ready. Type 'help' for commands.\n";
    if stream.write_all(banner.as_bytes()).is_err() {
        log_event(&log_file, &format!("WRITE_ERROR {peer} banner"));
        return;
    }

    let cloned = match stream.try_clone() {
        Ok(s) => s,
        Err(err) => {
            log_event(&log_file, &format!("CLONE_ERROR {peer} {err}"));
            return;
        }
    };
    let mut reader = BufReader::new(cloned);

    loop {
        if stream.write_all(b"> ").is_err() {
            log_event(&log_file, &format!("WRITE_ERROR {peer} prompt"));
            break;
        }

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                log_event(&log_file, &format!("DISCONNECT {peer}"));
                break;
            }
            Ok(_) => {
                let command = line.trim();
                if command.is_empty() {
                    continue;
                }

                log_event(&log_file, &format!("CMD {peer} {command}"));

                let response = {
                    let snapshot_guard = match shared_snapshot.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            let _ = stream.write_all(b"Internal lock error.\n");
                            break;
                        }
                    };
                    format_response(&snapshot_guard, command)
                };

                if stream.write_all(response.as_bytes()).is_err() || stream.write_all(b"\n").is_err() {
                    log_event(&log_file, &format!("WRITE_ERROR {peer} response"));
                    break;
                }

                if command.eq_ignore_ascii_case("quit") {
                    log_event(&log_file, &format!("QUIT {peer}"));
                    break;
                }
            }
            Err(err) => {
                log_event(&log_file, &format!("READ_ERROR {peer} {err}"));
                break;
            }
        }
    }
}

fn main() {
    let bind_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_ADDR.to_string());
    let log_file = std::env::args()
        .nth(2)
        .unwrap_or_else(|| DEFAULT_LOG.to_string());

    let initial_snapshot = collect_snapshot().unwrap_or_else(|_| SystemSnapshot {
        cpu: syswatch::CpuInfo { usage_percent: 0.0 },
        memory: syswatch::MemInfo {
            used_mb: 0,
            total_mb: 0,
            usage_percent: 0.0,
        },
        top_processes: Vec::new(),
    });
    let shared_snapshot = Arc::new(Mutex::new(initial_snapshot));

    {
        let refresher_snapshot = Arc::clone(&shared_snapshot);
        thread::spawn(move || refresh_loop(refresher_snapshot));
    }

    let listener = TcpListener::bind(&bind_addr).unwrap_or_else(|err| {
        panic!("cannot bind on {bind_addr}: {err}");
    });
    println!("SysWatch agent listening on {bind_addr}");
    println!("Log file: {log_file}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|_| "<unknown>".to_string());
                let snapshot = Arc::clone(&shared_snapshot);
                let log_path = log_file.clone();
                thread::spawn(move || handle_client(stream, peer, snapshot, log_path));
            }
            Err(err) => eprintln!("incoming connection error: {err}"),
        }
    }
}
