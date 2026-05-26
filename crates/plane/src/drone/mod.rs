use self::{
    executor::Executor,
    extension_handler::ExtensionHandler,
    heartbeat::HeartbeatLoop,
    key_manager::KeyManager,
    runtime::{
        docker::DockerRuntimeConfig,
        unix_socket::{UnixSocketRuntime, UnixSocketRuntimeConfig},
        Runtime,
    },
    state_store::StateStore,
};
use crate::signals::wait_for_shutdown_signal;
use anyhow::Result;
use chrono::Utc;
use plane_common::{
    names::DroneName,
    protocol::{
        BackendAction, BackendActionMessage, MessageFromDrone, MessageToDrone, RenewKeyResponse,
    },
    typed_socket::{client::TypedSocketConnector, TypedSocketSender},
    types::{BackendState, ClusterName, DronePoolName},
    util::random_reconnect_interval,
    PlaneClient,
};
use runtime::docker::DockerRuntime;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{set_permissions, File, Permissions},
    net::IpAddr,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;
use url::Url;

mod backend_manager;
pub mod command;
pub mod egress_bootstrap;
pub mod egress_publisher;
mod executor;
pub mod extension_handler;
mod heartbeat;
mod key_manager;
pub mod runtime;
mod state_store;

pub async fn drone_loop(
    name: DroneName,
    mut connection: TypedSocketConnector<MessageFromDrone>,
    executor: Executor,
    extension_handler: Option<Arc<ExtensionHandler>>,
) {
    let executor = Arc::new(executor);
    let key_manager = Arc::new(Mutex::new(KeyManager::new(executor.clone())));

    loop {
        if let Some(ref handler) = extension_handler {
            handler.clear_processed();
        }

        let mut socket = connection.connect_with_retry(&name).await;
        let _heartbeat_guard = HeartbeatLoop::start(socket.sender(MessageFromDrone::Heartbeat));

        {
            let socket = socket.sender(MessageFromDrone::BackendMetrics);
            executor
                .runtime
                .metrics_callback(Box::new(move |metrics_message| {
                    if let Err(err) = socket.send(metrics_message) {
                        tracing::error!(?err, "Error sending metrics message.");
                    }
                }));
        };

        key_manager
            .lock()
            .expect("Key manager lock poisoned")
            .set_sender(socket.sender(MessageFromDrone::RenewKey));

        {
            // Forward state changes to the socket.
            // This will start by sending any existing unacked events.
            let sender = socket.sender(MessageFromDrone::BackendEvent);
            let key_manager = key_manager.clone();
            if let Err(err) = executor.register_listener(move |message| {
                if matches!(message.state, BackendState::Terminated { .. }) {
                    key_manager
                        .lock()
                        .expect("Key manager lock poisoned.")
                        .unregister_key(&message.backend_id);
                }

                if let Err(e) = sender.send(message) {
                    tracing::error!(%e, "Error sending message.");
                }
            }) {
                tracing::error!(?err, "Error registering listener.");
                continue;
            }
        }

        let mut log_interval = tokio::time::interval(std::time::Duration::from_secs(60));
        log_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut message_counts: HashMap<&'static str, u64> = HashMap::new();

        let reconnect_at = tokio::time::Instant::now() + random_reconnect_interval();

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(reconnect_at) => {
                    tracing::info!("Periodic reconnect triggered.");
                    break;
                }
                _ = log_interval.tick() => {
                    let (outgoing, incoming) = socket.channel_depths();
                    tracing::info!(
                        outgoing_pending = outgoing,
                        incoming_pending = incoming,
                        action = message_counts.get("action").copied().unwrap_or(0),
                        ack_event = message_counts.get("ack_event").copied().unwrap_or(0),
                        renew_key_response = message_counts.get("renew_key_response").copied().unwrap_or(0),
                        "Drone channel stats (last 60s)"
                    );
                    message_counts.clear();
                }
                message_result = socket.recv() => {
                    let Some(message) = message_result else {
                        tracing::warn!("Connection closed.");
                        break;
                    };

                    match &message {
                        MessageToDrone::Action(_) => {
                            *message_counts.entry("action").or_insert(0) += 1;
                        }
                        MessageToDrone::AckEvent { .. } => {
                            *message_counts.entry("ack_event").or_insert(0) += 1;
                        }
                        MessageToDrone::RenewKeyResponse(_) => {
                            *message_counts.entry("renew_key_response").or_insert(0) += 1;
                        }
                        MessageToDrone::PushNetworkPolicy(_)
                        | MessageToDrone::PushCredential(_)
                        | MessageToDrone::RevokeCredential { .. }
                        | MessageToDrone::Snapshot(_) => {
                            *message_counts.entry("extension_action").or_insert(0) += 1;
                        }
                    }

                    let key_manager = key_manager.clone();
                    let ext_handler = extension_handler.clone();
                    tokio::spawn(handle_message(
                        message,
                        key_manager,
                        socket.sender(|x| x),
                        executor.clone(),
                        ext_handler,
                    ));
                }
            }
        }
    }

    pub async fn handle_message(
        message: MessageToDrone,
        key_manager: Arc<Mutex<KeyManager>>,
        sender: TypedSocketSender<MessageFromDrone>,
        executor: Arc<Executor>,
        extension_handler: Option<Arc<ExtensionHandler>>,
    ) {
        match message {
            MessageToDrone::Action(BackendActionMessage {
                action_id,
                backend_id,
                action,
                ..
            }) => {
                tracing::info!(?backend_id, ?action, "Received action.");

                if let BackendAction::Spawn { key, .. } = &action {
                    if key.deadlines.soft_terminate_at.0 < Utc::now() {
                        tracing::warn!(
                            ?backend_id,
                            "Received spawn request with deadline in the past. Ignoring."
                        );
                    }

                    let result = key_manager
                        .lock()
                        .expect("Key manager lock poisoned.")
                        .register_key(backend_id.clone(), key.clone());

                    if !result {
                        tracing::warn!(
                            ?backend_id,
                            "Key already registered for backend. Ignoring spawn request."
                        );
                        return;
                    }
                }

                if let Err(err) = executor.apply_action(&backend_id, &action).await {
                    tracing::error!(?err, "Error applying action.");
                    return;
                }

                if let Err(err) = sender.send(MessageFromDrone::AckAction { action_id }) {
                    tracing::error!(?err, "Error acking action.");
                }
            }
            MessageToDrone::AckEvent { event_id } => {
                if let Err(err) = executor.ack_event(event_id) {
                    tracing::error!(?err, "Error acking event.");
                }
            }
            MessageToDrone::RenewKeyResponse(renew_key_response) => {
                let RenewKeyResponse { backend, deadlines } = renew_key_response;
                tracing::debug!(?backend, ?deadlines, "Received key renewal response.");

                if let Some(deadlines) = deadlines {
                    key_manager
                        .lock()
                        .expect("Key manager lock poisoned.")
                        .update_deadlines(&backend, deadlines);
                } else {
                    tracing::warn!("Key renewal failed.");
                }
            }
            msg @ (MessageToDrone::PushNetworkPolicy(_)
            | MessageToDrone::PushCredential(_)
            | MessageToDrone::RevokeCredential { .. }
            | MessageToDrone::Snapshot(_)) => {
                if let Some(ref handler) = extension_handler {
                    handler.dispatch(msg, &sender).await;
                } else {
                    tracing::warn!("Received extension action but no handler configured");
                }
            }
        }
    }
}

pub struct Drone {
    drone_loop: JoinHandle<()>,
    pub id: DroneName,
}

impl Drone {
    pub async fn run(config: DroneConfig) -> Result<Self> {
        let client = PlaneClient::new(config.controller_url);

        let runtime: Box<dyn Runtime> = match config.executor_config {
            ExecutorConfig::Docker(docker_config) => {
                let mut rt = DockerRuntime::new(docker_config).await?;
                // If egress is configured (both URL and addr present), fetch the CA
                // and resolve the proxy IP once now so each spawn is a pure env-var
                // injection — no in-container DNS or HTTP needed at bootstrap time.
                if let (Some(admin_url), Some(addr)) = (
                    config.egress_proxy_url.as_ref(),
                    config.egress_proxy_addr.as_ref(),
                ) {
                    let ctx = egress_bootstrap::bootstrap(admin_url, addr).await?;
                    tracing::info!(
                        proxy_addr = %ctx.addr,
                        proxy_ip = %ctx.ip,
                        ca_bytes = ctx.ca_pem.len(),
                        "Egress bootstrap complete",
                    );
                    rt.egress_context = Some(ctx);
                }
                Box::new(rt)
            }
            ExecutorConfig::UnixSocket(unix_socket_config) => {
                Box::new(UnixSocketRuntime::new(unix_socket_config).await?)
            }
        };

        let connector = client.drone_connection(&config.cluster, &config.pool);

        let sqlite_connection = if let Some(db_path) = config.db_path.as_ref() {
            if !db_path.exists() {
                File::create(db_path)?;
                let permissions = Permissions::from_mode(0o600);
                set_permissions(db_path, permissions)?;
            }

            Connection::open(db_path)?
        } else {
            Connection::open_in_memory()?
        };

        let state_store = StateStore::new(sqlite_connection)?;

        let runtime = Arc::new(runtime);
        let executor = Executor::new(runtime, state_store, config.ip).await;

        let extension_handler = config.egress_proxy_url.map(|proxy_url| {
            tracing::info!(url = %proxy_url, "Egress proxy configured; enabling extension handler");
            let publisher = Arc::new(egress_publisher::HttpEgressPublisher::new(proxy_url));
            let snapshot_rt = Arc::new(egress_publisher::NoopSnapshotRuntime);
            Arc::new(ExtensionHandler::new(publisher, snapshot_rt))
        });

        let id = config.name.clone();
        let drone_loop = tokio::spawn(drone_loop(
            id.clone(),
            connector,
            executor,
            extension_handler,
        ));

        Ok(Self { drone_loop, id })
    }

    pub async fn terminate(self) {
        self.drone_loop.abort();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorConfig {
    Docker(DockerRuntimeConfig),
    UnixSocket(UnixSocketRuntimeConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneConfig {
    pub name: DroneName,

    pub executor_config: ExecutorConfig,

    pub controller_url: Url,
    pub cluster: ClusterName,
    pub pool: DronePoolName,
    pub ip: IpAddr,
    pub db_path: Option<PathBuf>,

    pub egress_proxy_url: Option<Url>,
    pub egress_proxy_addr: Option<String>,
}

pub async fn run_drone(config: DroneConfig) -> Result<()> {
    tracing::info!(name=%config.name, "Starting drone");
    let drone = Drone::run(config).await?;

    tracing::info!("Drone started.");
    wait_for_shutdown_signal().await;
    tracing::info!("Shutting down.");

    drone.terminate().await;

    Ok(())
}
