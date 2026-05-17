use crate::output::OutputFormat;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "sbctl", about = "Sandbox platform operator CLI")]
pub struct Cli {
    #[arg(
        long,
        env = "SBCTL_CONTROLLER_URL",
        default_value = "http://localhost:9090"
    )]
    pub controller_url: String,

    #[arg(long, short, value_enum, default_value_t = OutputFormat::Table)]
    pub output: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage drones
    Drones {
        #[command(subcommand)]
        action: DronesAction,
    },
    /// Manage sandboxes
    Sandboxes {
        #[command(subcommand)]
        action: SandboxesAction,
    },
    /// Manage snapshots
    Snapshots {
        #[command(subcommand)]
        action: SnapshotsAction,
    },
    /// Manage tenants
    Tenants {
        #[command(subcommand)]
        action: TenantsAction,
    },
    /// Manage secrets
    Secrets {
        #[command(subcommand)]
        action: SecretsAction,
    },
    /// Query audit log
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
    /// Show controller status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum DronesAction {
    /// List drones in a cluster
    List {
        #[arg(long)]
        cluster: String,
    },
    /// Drain a drone (stop scheduling new sandboxes)
    Drain {
        #[arg(long)]
        cluster: String,
        #[arg(long)]
        drone: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SandboxesAction {
    /// List sandboxes
    List {
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Get sandbox details
    Get { id: String },
    /// Terminate a sandbox
    Terminate {
        id: String,
        #[arg(long)]
        hard: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum SnapshotsAction {
    /// List snapshots
    List {
        #[arg(long)]
        tenant: Option<String>,
    },
    /// Get snapshot details
    Get { id: String },
    /// Delete a snapshot
    Delete { id: String },
}

#[derive(Subcommand, Debug)]
pub enum TenantsAction {
    /// List tenants
    List,
}

#[derive(Subcommand, Debug)]
pub enum SecretsAction {
    /// List secrets for a tenant
    List {
        #[arg(long)]
        tenant: String,
    },
    /// Create a secret
    Create {
        #[arg(long)]
        value: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// Delete a secret
    Delete { id: String },
}

#[derive(Subcommand, Debug)]
pub enum AuditAction {
    /// Query audit events
    Query {
        #[arg(long)]
        tenant: Option<String>,
        #[arg(long)]
        sandbox_id: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u32,
    },
}

pub fn parse() -> Cli {
    Cli::parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn try_parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(args)
    }

    #[test]
    fn parse_status() {
        let cli = try_parse(&["sbctl", "status"]).expect("parse");
        assert!(matches!(cli.command, Command::Status));
        assert_eq!(cli.controller_url, "http://localhost:9090");
    }

    #[test]
    fn parse_drones_list() {
        let cli = try_parse(&["sbctl", "drones", "list", "--cluster", "prod"]).expect("parse");
        match cli.command {
            Command::Drones {
                action: DronesAction::List { cluster },
            } => assert_eq!(cluster, "prod"),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_drones_drain() {
        let cli = try_parse(&[
            "sbctl",
            "drones",
            "drain",
            "--cluster",
            "prod",
            "--drone",
            "d-1",
        ])
        .expect("parse");
        match cli.command {
            Command::Drones {
                action: DronesAction::Drain { cluster, drone },
            } => {
                assert_eq!(cluster, "prod");
                assert_eq!(drone, "d-1");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_sandboxes_list_no_filter() {
        let cli = try_parse(&["sbctl", "sandboxes", "list"]).expect("parse");
        match cli.command {
            Command::Sandboxes {
                action: SandboxesAction::List { tenant, status },
            } => {
                assert!(tenant.is_none());
                assert!(status.is_none());
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_sandboxes_list_with_filters() {
        let cli = try_parse(&[
            "sbctl",
            "sandboxes",
            "list",
            "--tenant",
            "acme",
            "--status",
            "running",
        ])
        .expect("parse");
        match cli.command {
            Command::Sandboxes {
                action: SandboxesAction::List { tenant, status },
            } => {
                assert_eq!(tenant.as_deref(), Some("acme"));
                assert_eq!(status.as_deref(), Some("running"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_sandboxes_get() {
        let cli = try_parse(&["sbctl", "sandboxes", "get", "sb-abc123"]).expect("parse");
        match cli.command {
            Command::Sandboxes {
                action: SandboxesAction::Get { id },
            } => assert_eq!(id, "sb-abc123"),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_sandboxes_terminate_soft() {
        let cli = try_parse(&["sbctl", "sandboxes", "terminate", "sb-123"]).expect("parse");
        match cli.command {
            Command::Sandboxes {
                action: SandboxesAction::Terminate { id, hard },
            } => {
                assert_eq!(id, "sb-123");
                assert!(!hard);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_sandboxes_terminate_hard() {
        let cli =
            try_parse(&["sbctl", "sandboxes", "terminate", "--hard", "sb-123"]).expect("parse");
        match cli.command {
            Command::Sandboxes {
                action: SandboxesAction::Terminate { id, hard },
            } => {
                assert_eq!(id, "sb-123");
                assert!(hard);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_snapshots_list() {
        let cli = try_parse(&["sbctl", "snapshots", "list"]).expect("parse");
        assert!(matches!(
            cli.command,
            Command::Snapshots {
                action: SnapshotsAction::List { .. }
            }
        ));
    }

    #[test]
    fn parse_snapshots_delete() {
        let cli = try_parse(&["sbctl", "snapshots", "delete", "snap-abc"]).expect("parse");
        match cli.command {
            Command::Snapshots {
                action: SnapshotsAction::Delete { id },
            } => assert_eq!(id, "snap-abc"),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_secrets_list() {
        let cli = try_parse(&["sbctl", "secrets", "list", "--tenant", "acme"]).expect("parse");
        match cli.command {
            Command::Secrets {
                action: SecretsAction::List { tenant },
            } => assert_eq!(tenant, "acme"),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_audit_query_defaults() {
        let cli = try_parse(&["sbctl", "audit", "query"]).expect("parse");
        match cli.command {
            Command::Audit {
                action:
                    AuditAction::Query {
                        tenant,
                        sandbox_id,
                        limit,
                    },
            } => {
                assert!(tenant.is_none());
                assert!(sandbox_id.is_none());
                assert_eq!(limit, 50);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parse_output_json() {
        let cli = try_parse(&["sbctl", "-o", "json", "status"]).expect("parse");
        assert!(matches!(cli.output, OutputFormat::Json));
    }

    #[test]
    fn parse_output_table() {
        let cli = try_parse(&["sbctl", "--output", "table", "status"]).expect("parse");
        assert!(matches!(cli.output, OutputFormat::Table));
    }

    #[test]
    fn parse_custom_controller_url() {
        let cli = try_parse(&[
            "sbctl",
            "--controller-url",
            "http://ctrl.prod:9090",
            "status",
        ])
        .expect("parse");
        assert_eq!(cli.controller_url, "http://ctrl.prod:9090");
    }

    #[test]
    fn missing_subcommand_is_error() {
        let result = try_parse(&["sbctl"]);
        assert!(result.is_err());
    }

    #[test]
    fn drones_list_requires_cluster() {
        let result = try_parse(&["sbctl", "drones", "list"]);
        assert!(result.is_err());
    }
}
