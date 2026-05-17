use axum::Json;
use common::ext::agentd::{ProcessInfo, ProcessesResponse};
use tokio::process::Command;

pub async fn get() -> Json<ProcessesResponse> {
    let processes = list_processes().await;
    Json(ProcessesResponse { processes })
}

async fn list_processes() -> Vec<ProcessInfo> {
    #[cfg(target_os = "macos")]
    {
        list_processes_ps().await
    }
    #[cfg(target_os = "linux")]
    {
        list_processes_proc().await
    }
}

#[cfg(target_os = "linux")]
async fn list_processes_proc() -> Vec<ProcessInfo> {
    let output = Command::new("ps")
        .args(["-eo", "pid,ppid,stat,rss,comm", "--no-headers"])
        .output()
        .await
        .ok();

    let Some(output) = output else {
        return Vec::new();
    };

    parse_ps_output(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "macos")]
async fn list_processes_ps() -> Vec<ProcessInfo> {
    let output = Command::new("ps")
        .args(["-eo", "pid,ppid,stat,rss,comm"])
        .output()
        .await
        .ok();

    let Some(output) = output else {
        return Vec::new();
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    // Skip the header line
    parse_ps_output(&lines[1..].join("\n"))
}

fn parse_ps_output(output: &str) -> Vec<ProcessInfo> {
    let mut processes = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(5, char::is_whitespace).collect();
        if parts.len() < 5 {
            continue;
        }

        let pid = match parts[0].trim().parse::<u32>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let ppid = parts[1].trim().parse::<u32>().ok();
        let state = Some(parts[2].trim().to_string());
        let mem_bytes = parts[3].trim().parse::<u64>().ok().map(|kb| kb * 1024);
        let command = parts[4].trim().to_string();

        processes.push(ProcessInfo {
            pid,
            ppid,
            command,
            cpu_percent: None,
            mem_bytes,
            state,
        });
    }

    processes
}
