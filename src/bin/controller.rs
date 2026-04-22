use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

const DEFAULT_AGENTS: &[&str] = &["127.0.0.1:7878", "127.0.0.1:7879"];

fn send_command(addr: &str, command: &str) -> io::Result<String> {
    let mut stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;

    let mut reader = BufReader::new(stream.try_clone()?);

    let mut banner = String::new();
    let _ = reader.read_line(&mut banner);

    stream.write_all(format!("{command}\n").as_bytes())?;

    let mut response = String::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.trim() == ">" || line.trim() == "> " {
                    break;
                }
                response.push_str(&line);
                if command.eq_ignore_ascii_case("quit") {
                    break;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
            Err(err) if err.kind() == io::ErrorKind::TimedOut => break,
            Err(err) => return Err(err),
        }
    }

    let _ = stream.write_all(b"quit\n");
    Ok(response.trim().to_string())
}

fn print_help() {
    println!("Controller commands:");
    println!("  list");
    println!("  send <id> <cpu|mem|ps|all|help>");
    println!("  broadcast <cpu|mem|ps|all|help>");
    println!("  quit");
}

fn main() {
    let agents = std::env::args()
        .skip(1)
        .collect::<Vec<_>>();
    let agents = if agents.is_empty() {
        DEFAULT_AGENTS.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    } else {
        agents
    };

    println!("SysWatch Controller");
    println!("Loaded {} agent(s).", agents.len());
    print_help();

    let stdin = io::stdin();
    loop {
        print!("controller> ");
        let _ = io::stdout().flush();

        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            println!("input error");
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("quit") {
            println!("bye.");
            break;
        }

        if trimmed.eq_ignore_ascii_case("list") {
            for (idx, agent) in agents.iter().enumerate() {
                println!("{idx}: {agent}");
            }
            continue;
        }

        if trimmed.eq_ignore_ascii_case("help") {
            print_help();
            continue;
        }

        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        match parts.as_slice() {
            ["send", id, cmd] => {
                let idx = match id.parse::<usize>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("invalid id: {id}");
                        continue;
                    }
                };
                let Some(addr) = agents.get(idx) else {
                    println!("unknown id: {idx}");
                    continue;
                };
                match send_command(addr, cmd) {
                    Ok(resp) => println!("[{addr}]\n{}\n", if resp.is_empty() { "<empty>" } else { &resp }),
                    Err(err) => println!("[{addr}] error: {err}"),
                }
            }
            ["broadcast", cmd] => {
                for addr in &agents {
                    match send_command(addr, cmd) {
                        Ok(resp) => println!("[{addr}]\n{}\n", if resp.is_empty() { "<empty>" } else { &resp }),
                        Err(err) => println!("[{addr}] error: {err}"),
                    }
                }
            }
            _ => {
                println!("unknown command: {trimmed}");
                print_help();
            }
        }
    }
}
