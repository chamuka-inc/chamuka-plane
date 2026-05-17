use self::{
    acme::AcmeDatabase,
    backend::BackendDatabase,
    backend_actions::BackendActionDatabase,
    backend_key::KeysDatabase,
    cluster::ClusterDatabase,
    connect::ConnectError,
    controller::ControllerDatabase,
    credential::CredentialDatabase,
    drone::DroneDatabase,
    extension_action::ExtensionActionDatabase,
    node::NodeDatabase,
    sandbox::SandboxDatabase,
    secret::SecretDatabase,
    snapshot::SnapshotDatabase,
    subscribe::{EventSubscriptionManager, Notification, NotificationPayload, Subscription},
    tenant::TenantDatabase,
    usage_record::UsageRecordDatabase,
};
use plane_common::{
    types::{ClusterName, ConnectRequest, ConnectResponse, RevokeRequest},
    PlaneClient,
};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::{Arc, OnceLock};
use tokio::sync::broadcast::Receiver;

pub mod acme;
pub mod backend;
pub mod backend_actions;
pub mod backend_key;
pub mod cluster;
pub mod connect;
pub mod controller;
pub mod credential;
pub mod drone;
pub mod extension_action;
pub mod node;
pub mod sandbox;
pub mod secret;
pub mod snapshot;
pub mod subscribe;
pub mod tenant;
pub mod usage_record;
pub mod util;

pub async fn connect_and_migrate(db: &str) -> sqlx::Result<PlaneDatabase> {
    let db_pool = PgPoolOptions::new().max_connections(20).connect(db).await?;
    sqlx::migrate!("schema/migrations").run(&db_pool).await?;
    Ok(PlaneDatabase::new(db_pool))
}

pub async fn connect(db: &str) -> sqlx::Result<PlaneDatabase> {
    let db_pool = PgPoolOptions::new().max_connections(20).connect(db).await?;
    Ok(PlaneDatabase::new(db_pool))
}

#[derive(Clone)]
pub struct PlaneDatabase {
    pub pool: PgPool,
    subscription_manager: Arc<OnceLock<EventSubscriptionManager>>,
}

impl PlaneDatabase {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            subscription_manager: Arc::default(),
        }
    }

    pub fn acme(&self) -> acme::AcmeDatabase<'_> {
        AcmeDatabase::new(&self.pool)
    }

    pub fn drone(&self) -> DroneDatabase<'_> {
        DroneDatabase::new(&self.pool)
    }

    pub fn cluster(&self) -> ClusterDatabase<'_> {
        ClusterDatabase::new(&self.pool)
    }

    pub fn node(&self) -> NodeDatabase<'_> {
        NodeDatabase::new(&self.pool)
    }

    pub fn backend(&self) -> BackendDatabase<'_> {
        BackendDatabase::new(self)
    }

    pub fn backend_actions(&self) -> BackendActionDatabase {
        BackendActionDatabase::new(&self.pool)
    }

    pub fn keys(&self) -> backend_key::KeysDatabase<'_> {
        KeysDatabase::new(&self.pool)
    }

    pub fn controller(&self) -> controller::ControllerDatabase<'_> {
        ControllerDatabase::new(&self.pool)
    }

    pub fn sandbox(&self) -> SandboxDatabase<'_> {
        SandboxDatabase::new(&self.pool)
    }

    pub fn snapshot(&self) -> SnapshotDatabase<'_> {
        SnapshotDatabase::new(&self.pool)
    }

    pub fn credential(&self) -> CredentialDatabase<'_> {
        CredentialDatabase::new(&self.pool)
    }

    pub fn secret(&self) -> SecretDatabase<'_> {
        SecretDatabase::new(&self.pool)
    }

    pub fn extension_action(&self) -> ExtensionActionDatabase<'_> {
        ExtensionActionDatabase::new(&self.pool)
    }

    pub fn tenant(&self) -> TenantDatabase<'_> {
        TenantDatabase::new(&self.pool)
    }

    pub fn usage_record(&self) -> UsageRecordDatabase<'_> {
        UsageRecordDatabase::new(&self.pool)
    }

    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query_scalar!("select 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn connect(
        &self,
        default_cluster: Option<&ClusterName>,
        request: &ConnectRequest,
        client: &PlaneClient,
        best_of: i64,
    ) -> Result<ConnectResponse, ConnectError> {
        connect::connect(&self.pool, default_cluster, request, client, best_of).await
    }
    pub async fn revoke(&self, request: &RevokeRequest) -> Result<(), ConnectError> {
        connect::revoke(&self.pool, request).await
    }

    pub async fn clean_up_tokens(&self) -> Result<(), sqlx::Error> {
        connect::clean_up_tokens(&self.pool).await
    }

    /// this limits the number of events returned, so it may be necessary
    /// to call this function multiple times to get all events since a given id
    pub async fn get_events_since(
        &self,
        since: i32,
    ) -> Result<Vec<Notification<Value>>, sqlx::Error> {
        EventSubscriptionManager::get_events_since(&self.pool, since).await
    }

    fn subscription_manager(&self) -> &EventSubscriptionManager {
        self.subscription_manager
            .get_or_init(|| EventSubscriptionManager::new(&self.pool))
    }

    pub fn subscribe<T: NotificationPayload>(&self) -> Subscription<T> {
        self.subscription_manager().subscribe(None)
    }

    pub fn subscribe_with_key<T: NotificationPayload>(&self, key: &str) -> Subscription<T> {
        self.subscription_manager().subscribe(Some(key))
    }

    pub fn subscribe_all_events(&self) -> Receiver<Notification<Value>> {
        self.subscription_manager().subscribe_all_events()
    }
}
