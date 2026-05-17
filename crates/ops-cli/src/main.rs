#![allow(clippy::print_stdout, clippy::print_stderr, clippy::expect_used)]

mod cli;
mod client;
mod display;
mod error;
mod output;

use cli::{AuditAction, Command, DronesAction, SandboxesAction, SecretsAction, SnapshotsAction};
use client::ControllerClient;
use display::DroneRow;
use error::CliError;
use output::{render_list, render_single};

fn main() {
    let cli = cli::parse();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");
    rt.block_on(async {
        if let Err(e) = run(cli).await {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    });
}

async fn run(cli: cli::Cli) -> Result<(), CliError> {
    let client = ControllerClient::new(cli.controller_url);
    let fmt = cli.output;

    match cli.command {
        Command::Status => {
            let status = client.status().await?;
            println!("{}", render_single(&status, fmt));
        }

        Command::Drones { action } => match action {
            DronesAction::List { cluster } => {
                let state = client.cluster_state(&cluster).await?;
                let rows: Vec<DroneRow> = state.drones.iter().map(DroneRow::from_state).collect();
                println!("{}", render_list(&rows, fmt));
            }
            DronesAction::Drain { cluster, drone } => {
                client.drain_drone(&cluster, &drone).await?;
                println!("Drone {drone} in cluster {cluster} set to draining.");
            }
        },

        Command::Sandboxes { action } => match action {
            SandboxesAction::List { .. } => {
                println!("(sandboxes list requires extension API — not yet implemented)");
            }
            SandboxesAction::Get { .. } => {
                println!("(sandbox get requires extension API — not yet implemented)");
            }
            SandboxesAction::Terminate { id, hard } => {
                if hard {
                    client.hard_terminate(&id).await?;
                    println!("Backend {id} hard-terminated.");
                } else {
                    client.soft_terminate(&id).await?;
                    println!("Backend {id} soft-terminated.");
                }
            }
        },

        Command::Snapshots { action } => match action {
            SnapshotsAction::List { .. } => {
                println!("(snapshots list requires extension API — not yet implemented)");
            }
            SnapshotsAction::Get { id } => {
                let snap = client.get_snapshot(&id).await?;
                println!("{}", render_single(&snap, fmt));
            }
            SnapshotsAction::Delete { id } => {
                client.delete_snapshot(&id).await?;
                println!("Snapshot {id} deleted.");
            }
        },

        Command::Tenants { .. } => {
            println!("(tenants requires extension API — not yet implemented)");
        }

        Command::Secrets { action } => match action {
            SecretsAction::List { .. } => {
                println!("(secrets list requires extension API — not yet implemented)");
            }
            SecretsAction::Create { .. } => {
                println!("(secrets create requires extension API — not yet implemented)");
            }
            SecretsAction::Delete { .. } => {
                println!("(secrets delete requires extension API — not yet implemented)");
            }
        },

        Command::Audit { action } => match action {
            AuditAction::Query { .. } => {
                println!("(audit query requires extension API — not yet implemented)");
            }
        },
    }

    Ok(())
}
