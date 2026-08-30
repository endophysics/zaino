use std::{net::Ipv4Addr, path::PathBuf};

use zaino_common::{
    validator::ValidatorConfig, CacheConfig, DatabaseConfig, Network, ServiceConfig, StorageConfig,
};
use zaino_serve::server::config::{GrpcServerConfig, JsonRpcServerConfig};
use zaino_state::{NodeBackedIndexerServiceConfig, NodeBackedIndexerServiceSubscriber};
use zainodlib::config::{BackendType, PrivacyGrpcSettings};

use super::{LaunchedZaino, PrivacyEndpointConfig, RunningIndexer};

pub(super) struct IndexerLaunch {
    pub(super) backend: BackendType,
    pub(super) validator_settings: ValidatorConfig,
    pub(super) zaino_db_path: PathBuf,
    pub(super) zebra_db_path: PathBuf,
    pub(super) network: Network,
    pub(super) enable_jsonrpc: bool,
    pub(super) privacy_endpoint: Option<PrivacyEndpointConfig>,
}

pub(super) async fn launch_indexer(launch: IndexerLaunch) -> Result<LaunchedZaino, std::io::Error> {
    let grpc_listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .expect("failed to bind Zaino gRPC listener on 127.0.0.1:0");
    let grpc_address = grpc_listener
        .local_addr()
        .expect("local_addr on a bound listener is infallible");
    let privacy_listener = match launch.privacy_endpoint {
        Some(_) => Some(std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?),
        None => None,
    };
    let privacy_grpc_address = privacy_listener
        .as_ref()
        .map(std::net::TcpListener::local_addr)
        .transpose()?;
    let json_listener = if launch.enable_jsonrpc {
        Some(
            std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
                .expect("failed to bind Zaino JSON-RPC listener on 127.0.0.1:0"),
        )
    } else {
        None
    };
    let json_address = match &json_listener {
        Some(listener) => listener
            .local_addr()
            .expect("local_addr on a bound listener is infallible"),
        None => (Ipv4Addr::LOCALHOST, 0).into(),
    };

    tracing::debug!(
        %grpc_address,
        %json_address,
        "[TEST] Launching Zaino indexer"
    );
    let privacy_grpc_settings =
        launch
            .privacy_endpoint
            .zip(privacy_grpc_address)
            .map(|(privacy, listen_address)| PrivacyGrpcSettings {
                server: GrpcServerConfig {
                    listen_address,
                    tls: None,
                },
                allow_transaction_specific_reads: privacy.allow_transaction_specific_reads,
                allow_transparent_address_reads: privacy.allow_transparent_address_reads,
                metrics_window_seconds: privacy.metrics_window_seconds,
            });
    let indexer_config = zainodlib::config::ZainodConfig {
        backend: launch.backend,
        json_server_settings: if launch.enable_jsonrpc {
            Some(JsonRpcServerConfig {
                json_rpc_listen_address: json_address,
                cookie_dir: None,
            })
        } else {
            None
        },
        grpc_settings: GrpcServerConfig {
            listen_address: grpc_address,
            tls: None,
        },
        privacy_grpc_settings,
        validator_settings: launch.validator_settings,
        service: ServiceConfig::default(),
        storage: StorageConfig {
            cache: CacheConfig::default(),
            database: DatabaseConfig {
                path: launch.zaino_db_path,
                ..Default::default()
            },
        },
        mempool: Default::default(),
        ephemeral_finalised_state: false,
        zebra_db_path: launch.zebra_db_path,
        network: launch.network,
        donation_address: None,
        metrics_endpoint: None,
    };

    let service_config = NodeBackedIndexerServiceConfig::try_from(indexer_config.clone())
        .expect("Failed to convert ZainodConfig to service config");
    let (handle, subscriber): (_, NodeBackedIndexerServiceSubscriber) =
        RunningIndexer::launch_inner_with_listeners(
            service_config,
            indexer_config,
            grpc_listener,
            json_listener,
            privacy_listener,
        )
        .await
        .unwrap();

    Ok(LaunchedZaino {
        handle: Some(handle),
        subscriber: Some(subscriber),
        grpc_address: Some(grpc_address),
        privacy_grpc_address,
        json_address: Some(json_address),
        json_cookie_dir: None,
    })
}

pub(super) const fn disabled_indexer() -> LaunchedZaino {
    LaunchedZaino {
        handle: None,
        subscriber: None,
        grpc_address: None,
        privacy_grpc_address: None,
        json_address: None,
        json_cookie_dir: None,
    }
}
