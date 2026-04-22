use std::fmt;

use sysinfo::System;

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub usage_percent: f32,
}

#[derive(Debug, Clone)]
pub struct MemInfo {
    pub used_mb: u64,
    pub total_mb: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: String,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub cpu: CpuInfo,
    pub memory: MemInfo,
    pub top_processes: Vec<ProcessInfo>,
}

impl fmt::Display for CpuInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPU Usage: {:.2}%", self.usage_percent)
    }
}

impl fmt::Display for MemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Memory: {} MB / {} MB ({:.2}%)",
            self.used_mb, self.total_mb, self.usage_percent
        )
    }
}

impl fmt::Display for ProcessInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[PID {}] {:<25} CPU {:>6.2}% MEM {:>6} MB",
            self.pid, self.name, self.cpu_percent, self.memory_mb
        )
    }
}

impl fmt::Display for SystemSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.cpu)?;
        writeln!(f, "{}", self.memory)?;
        writeln!(f, "Top Processes:")?;
        for process in &self.top_processes {
            writeln!(f, "  {}", process)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum SnapshotError {
    EmptyProcessName,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotError::EmptyProcessName => write!(f, "process name cannot be empty"),
        }
    }
}

pub fn collect_snapshot() -> Result<SystemSnapshot, SnapshotError> {
    let mut system = System::new_all();
    system.refresh_all();

    let cpu = CpuInfo {
        usage_percent: system.global_cpu_info().cpu_usage(),
    };

    let total_memory_kb = system.total_memory();
    let used_memory_kb = system.used_memory();
    let total_mb = total_memory_kb / 1024;
    let used_mb = used_memory_kb / 1024;
    let usage_percent = if total_memory_kb == 0 {
        0.0
    } else {
        (used_memory_kb as f32 / total_memory_kb as f32) * 100.0
    };

    let memory = MemInfo {
        used_mb,
        total_mb,
        usage_percent,
    };

    let mut processes = system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let name = process.name().trim().to_string();
            if name.is_empty() {
                return Err(SnapshotError::EmptyProcessName);
            }
            Ok(ProcessInfo {
                pid: pid.to_string(),
                name,
                cpu_percent: process.cpu_usage(),
                memory_mb: process.memory() / 1024,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    processes.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
    processes.truncate(5);

    Ok(SystemSnapshot {
        cpu,
        memory,
        top_processes: processes,
    })
}

fn usage_bar(percent: f32) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    let bars = ((clamped / 10.0).round() as usize).min(10);
    format!("[{}{}] {:>6.2}%", "#".repeat(bars), "-".repeat(10 - bars), clamped)
}

pub fn format_response(snapshot: &SystemSnapshot, command: &str) -> String {
    match command.trim().to_ascii_lowercase().as_str() {
        "cpu" => format!("{}\n{}", snapshot.cpu, usage_bar(snapshot.cpu.usage_percent)),
        "mem" => format!(
            "{}\n{}",
            snapshot.memory,
            usage_bar(snapshot.memory.usage_percent)
        ),
        "ps" => {
            if snapshot.top_processes.is_empty() {
                "No process data available.".to_string()
            } else {
                let lines = snapshot
                    .top_processes
                    .iter()
                    .enumerate()
                    .map(|(idx, process)| format!("{}. {}", idx + 1, process))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Top processes by CPU:\n{lines}")
            }
        }
        "all" => snapshot.to_string(),
        "help" => [
            "Available commands:",
            "  cpu  - show global CPU usage",
            "  mem  - show RAM usage",
            "  ps   - show top 5 processes by CPU",
            "  all  - show complete snapshot",
            "  help - show this help",
            "  quit - close connection",
        ]
        .join("\n"),
        "quit" => "Goodbye.".to_string(),
        other => format!("Unknown command '{other}'. Type 'help'."),
    }
}
