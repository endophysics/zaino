//! Zaino : Zingo-Indexer implementation.

use tokio::time::Instant;
use tracing::info;

use zaino_rpc::probe_node;
use zaino_state::{
    IndexerService, LightWalletService, NodeBackedIndexerService, NodeBackedIndexerServiceConfig,
    ZcashIndexer, ZcashService,
};
use zaino_status::StatusType;

use crate::{config::ZainodConfig, error::IndexerError};

use lifecycle::{EndpointListeners, EndpointPlan, EndpointServers};

mod lifecycle;

#[cfg(feature = "test_dependencies")]
/// Feature-gated controls for assembled daemon lifecycle tests.
pub mod test_support;

#[cfg(all(test, feature = "test_dependencies"))]
mod tests;

/// Zaino, the Zingo-Indexer.
pub struct Indexer<Service: ZcashService + LightWalletService> {
    servers: EndpointServers,
    /// Chain fetch service state process handler..
    service: Option<IndexerService<Service>>,
}

/// Starts Indexer service.
///
/// Currently only takes an IndexerConfig.
pub async fn start_indexer(
    config: ZainodConfig,
) -> Result<tokio::task::JoinHandle<Result<(), IndexerError>>, IndexerError> {
    startup_message();
    info!("Starting Zaino");
    spawn_indexer(config).await
}

/// Spawns a new Indexer server.
pub async fn spawn_indexer(
    config: ZainodConfig,
) -> Result<tokio::task::JoinHandle<Result<(), IndexerError>>, IndexerError> {
    config.check_config()?;
    info!(
        address = %config.validator_settings.validator_jsonrpc_listen_address,
        "Checking connection with node"
    );
    if let Some(donation_address) = &config.donation_address {
        info!(%donation_address, "instance donation address");
    }
    let zebrad_uri = probe_node(
        &config.validator_settings.validator_jsonrpc_listen_address,
        config.validator_settings.validator_cookie_path.as_deref(),
        config.validator_settings.validator_user.clone(),
        config.validator_settings.validator_password.clone(),
    )
    .await?;

    info!(uri = %zebrad_uri, "Connected to node via JsonRPSee");

    // Both the JSON-RPC (`Rpc`) and direct-`ReadStateService` (`Direct`) connections are
    // now served by the single `NodeBackedIndexerService`; the connection is selected
    // inside the config conversion from `config.backend`.
    let service_config = NodeBackedIndexerServiceConfig::try_from(config.clone())?;
    Indexer::<NodeBackedIndexerService>::launch_inner(service_config, config)
        .await
        .map(|res| res.0)
}

impl<Service: ZcashService + LightWalletService + Send + Sync + 'static> Indexer<Service>
where
    IndexerError: From<<Service::Subscriber as ZcashIndexer>::Error>,
{
    /// Spawns a new Indexer server.
    // TODO: revise whether returning the subscriber here is the best way to access the service after the indexer is spawned.
    pub async fn launch_inner(
        service_config: Service::Config,
        indexer_config: ZainodConfig,
    ) -> Result<
        (
            tokio::task::JoinHandle<Result<(), IndexerError>>,
            Service::Subscriber,
        ),
        IndexerError,
    > {
        Self::launch_inner_impl(
            service_config,
            indexer_config,
            EndpointListeners::production(),
        )
        .await
    }

    /// Launches the indexer on pre-bound listeners (test-only).
    ///
    /// The harness binds `127.0.0.1:0` for the gRPC server (and the JSON-RPC
    /// server when enabled), reads the OS-assigned ports, and hands the open
    /// sockets here — eliminating the pick-a-port / bind-later race that
    /// otherwise flakes under parallel test execution. The optional JSON-RPC
    /// listener corresponds to its optional configuration section. The privacy
    /// listener and `privacy_grpc_settings` must either both be present or both
    /// be absent; mismatches are rejected before the indexer service starts.
    #[cfg(feature = "test_dependencies")]
    pub async fn launch_inner_with_listeners(
        service_config: Service::Config,
        indexer_config: ZainodConfig,
        grpc_listener: std::net::TcpListener,
        json_listener: Option<std::net::TcpListener>,
        privacy_grpc_listener: Option<std::net::TcpListener>,
    ) -> Result<
        (
            tokio::task::JoinHandle<Result<(), IndexerError>>,
            Service::Subscriber,
        ),
        IndexerError,
    > {
        match (
            indexer_config.privacy_grpc_settings.is_some(),
            privacy_grpc_listener.is_some(),
        ) {
            (true, false) => {
                return Err(IndexerError::ConfigError(
                    "privacy_grpc_settings requires privacy_grpc_listener in launch_inner_with_listeners."
                        .to_string(),
                ));
            }
            (false, true) => {
                return Err(IndexerError::ConfigError(
                    "privacy_grpc_listener requires privacy_grpc_settings in launch_inner_with_listeners."
                        .to_string(),
                ));
            }
            (true, true) | (false, false) => {}
        }
        Self::launch_inner_impl(
            service_config,
            indexer_config,
            EndpointListeners {
                grpc_legacy: Some(grpc_listener),
                grpc_privacy: privacy_grpc_listener,
                json_rpc: json_listener,
            },
        )
        .await
    }

    async fn launch_inner_impl(
        service_config: Service::Config,
        indexer_config: ZainodConfig,
        listeners: EndpointListeners,
    ) -> Result<
        (
            tokio::task::JoinHandle<Result<(), IndexerError>>,
            Service::Subscriber,
        ),
        IndexerError,
    > {
        indexer_config.check_config()?;
        let endpoint_plan = EndpointPlan::from_config(indexer_config)?;
        let service = IndexerService::<Service>::spawn(service_config).await?;
        let subscriber = service.inner_ref().get_subscriber();
        let service_subscriber = subscriber.inner_clone();
        let servers = match EndpointServers::spawn(subscriber, endpoint_plan, listeners).await {
            Ok(servers) => servers,
            Err(error) => {
                let mut service = service.inner();
                service.close();
                return Err(error);
            }
        };

        let mut indexer = Self {
            servers,
            service: Some(service),
        };

        let mut server_interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        let mut last_log_time = Instant::now();
        let log_interval = tokio::time::Duration::from_secs(10);

        let serve_task = tokio::task::spawn(async move {
            loop {
                // Log the servers status.
                if last_log_time.elapsed() >= log_interval {
                    indexer.log_status();
                    last_log_time = Instant::now();
                }

                // Check for restart signals.
                if indexer.check_for_critical_errors() {
                    indexer.close().await;
                    return Err(IndexerError::Restart);
                }

                // Check for shutdown signals.
                if indexer.check_for_shutdown() {
                    indexer.close().await;
                    return Ok(());
                }

                server_interval.tick().await;
            }
        });

        Ok((serve_task, service_subscriber))
    }

    /// Checks indexers status and servers internal statuses for either offline of critical error signals.
    fn check_for_critical_errors(&self) -> bool {
        let status = self.status_int();
        if status == 5 || status >= 7 {
            self.log_status();
            tracing::error!(
                combined_status = status,
                "check_for_critical_errors triggered"
            );
            return true;
        }
        false
    }

    /// Checks indexers status and servers internal status for closure signal.
    fn check_for_shutdown(&self) -> bool {
        if self.status_int() == 4 {
            return true;
        }
        false
    }

    /// Sets the servers to close gracefully.
    async fn close(&mut self) {
        self.servers.close().await;

        if let Some(service) = self.service.take() {
            let mut service = service.inner();
            service.close();
        }
    }

    /// Returns the indexers current status usize, calculates from internal statuses.
    fn status_int(&self) -> usize {
        let service_status = match &self.service {
            Some(service) => service.inner_ref().status(),
            None => return 7,
        };

        usize::from(StatusType::combine(service_status, self.servers.status()))
    }

    /// Returns the current StatusType of the indexer.
    pub fn status(&self) -> StatusType {
        StatusType::from(self.status_int())
    }

    /// Logs the indexers status.
    pub fn log_status(&self) {
        let service_status = match &self.service {
            Some(service) => service.inner_ref().status(),
            None => StatusType::Offline,
        };

        // `chain_state: Ready` on its own is ambiguous: while initial sync or a migration runs, an
        // ephemeral passthrough serves finalised-state reads and reports `Ready` exactly like the
        // real on-disk index. Reporting the mode next to the status is what lets an operator — or a
        // containerised test polling this line — tell the two apart.
        let finalised_state_mode = self
            .service
            .as_ref()
            .map(|service| service.inner_ref().finalised_state_mode().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        self.servers
            .log_status(service_status, &finalised_state_mode);
    }
}

/// Prints Zaino's startup message.
fn startup_message() {
    let welcome_message = r#"
       ░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒████▓░▒▒▒
       ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒████▓▒▒▒▒
       ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░▒▒▒▒▒▒
       ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▓▓▓▓▒▒▒▒▒▒▒▒▒▒▒▒▓▓▒▒▒▒▒▒
       ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒██▓▒▒▒▒▒
       ▒▒▒▒▒▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒██▓▒▒▒▒▒
       ▒▒▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓███▓██▓▒▒▒▒▒
       ▒▒▒▒▒▒▒▓▓▓▓▒███▓░▒▓▓████████████████▓▓▒▒▒▒▒▒▒
       ▒▒▒▒▒▒▓▓▓▓▒▓████▓▓███████████████████▓▒▓▓▒▒▒▒
       ▒▒▒▒▒▓▓▓▓▓▒▒▓▓▓▓████████████████████▓▒▓▓▓▒▒▒▒
       ▒▒▒▒▒▓▓▓▓▓█████████████████████████▓▒▓▓▓▓▓▒▒▒
       ▒▒▒▒▓▓▓▒▓█████████████████████████▓▓▓▓▓▓▓▓▒▒▒
       ▒▒▒▒▒▓▓▓████████████████████████▓▓▓▓▓▓▓▓▓▒▒▒▒
       ▒▒▒▒▒▓▒███████████████████████▒▓▓▓▓▓▓▓▓▓▓▒▒▒▒
       ▒▒▒▒▒▒▓███████████████████▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒
       ▒▒▒▒▒▒▓███████████████▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒
       ▒▒▒▒▒▒▓██████████▓▓▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▒▒
       ▒▒▒▒███▓▒▓▓▓▓▓▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▒▒▒▒▒
       ▒▒▒▓████▒▒▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
       ▒▒▒▒░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
       ▒▒▒▒░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
             Thank you for using ZingoLabs Zaino!

       - Donate to us at https://free2z.cash/zingolabs.

****** Please note Zaino is currently in development and should not be used to run mainnet nodes. ******
    "#;
    println!("{welcome_message}");
}
