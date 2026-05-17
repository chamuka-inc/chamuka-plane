use axum::Json;
use common::ext::agentd::{ListeningPort, PortProtocol, PortsResponse};
use std::process::Command;

pub async fn get() -> Json<PortsResponse> {
    let listening = list_listening_ports();
    Json(PortsResponse { listening })
}

fn list_listening_ports() -> Vec<ListeningPort> {
    #[cfg(target_os = "macos")]
    {
        list_ports_lsof()
    }
    #[cfg(target_os = "linux")]
    {
        list_ports_ss()
    }
}

#[cfg(target_os = "linux")]
fn list_ports_ss() -> Vec<ListeningPort> {
    let output = Command::new("ss").args(["-tlnp", "-H"]).output().ok();

    let Some(output) = output else {
        return Vec::new();
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let local_addr = parts[3];
        if let Some(port_str) = local_addr.rsplit(':').next() {
            if let Ok(port) = port_str.parse::<u16>() {
                let process_name = parts
                    .get(5)
                    .and_then(|s| s.strip_prefix("users:((\""))
                    .and_then(|s| s.split('"').next())
                    .map(String::from);

                ports.push(ListeningPort {
                    port,
                    protocol: PortProtocol::Tcp,
                    process_pid: None,
                    process_name,
                });
            }
        }
    }

    ports
}

#[cfg(target_os = "macos")]
fn list_ports_lsof() -> Vec<ListeningPort> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-nP", "-F", "pcn"])
        .output()
        .ok();

    let Some(output) = output else {
        return Vec::new();
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();
    let mut current_pid: Option<u32> = None;
    let mut current_name: Option<String> = None;

    for line in stdout.lines() {
        if let Some(pid_str) = line.strip_prefix('p') {
            current_pid = pid_str.parse().ok();
        } else if let Some(name) = line.strip_prefix('c') {
            current_name = Some(name.to_string());
        } else if let Some(addr) = line.strip_prefix('n') {
            if let Some(port_str) = addr.rsplit(':').next() {
                if let Ok(port) = port_str.parse::<u16>() {
                    ports.push(ListeningPort {
                        port,
                        protocol: PortProtocol::Tcp,
                        process_pid: current_pid,
                        process_name: current_name.clone(),
                    });
                }
            }
        }
    }

    ports
}
