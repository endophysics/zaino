use std::path::PathBuf;

use zaino_common::{DatabaseConfig, Network, ServiceConfig, StorageConfig};
use zaino_state::{
    NodeBackedIndexerService, NodeBackedIndexerServiceConfig, NodeBackedIndexerServiceSubscriber,
    ZcashService,
};
use zcash_local_net::{validator::Validator, MinerPool};
use zebra_chain::parameters::NetworkKind;

use crate::{default_mining_pool, Direct, Rpc, TestManager, ValidatorExt, ValidatorKind};

/// A manager plus standalone RPC- and direct-connection services.
pub struct StateAndFetchServices<V: Validator> {
    /// The launched validator and Zaino manager.
    pub test_manager: TestManager<V, Direct>,
    /// Owned RPC service that keeps `fetch_subscriber` alive.
    pub fetch_service: NodeBackedIndexerService,
    /// Subscriber to the RPC service.
    pub fetch_subscriber: NodeBackedIndexerServiceSubscriber,
    /// Owned direct service that keeps `state_subscriber` alive.
    pub state_service: NodeBackedIndexerService,
    /// Subscriber to the direct service.
    pub state_subscriber: NodeBackedIndexerServiceSubscriber,
}

impl<V: ValidatorExt> StateAndFetchServices<V> {
    /// Mine blocks and wait for both subscribers to observe the new tip.
    pub async fn generate_blocks_and_wait_for_tips(&self, n: u32) {
        self.test_manager
            .generate_blocks_and_wait_for_tips(n, &self.fetch_subscriber, &self.state_subscriber)
            .await;
    }
}

/// Launch a direct manager and standalone RPC and direct services.
pub async fn launch_state_and_fetch_services<V: ValidatorExt>(
    validator: &ValidatorKind,
    chain_cache: Option<PathBuf>,
    enable_zaino: bool,
    network: Option<NetworkKind>,
) -> StateAndFetchServices<V> {
    launch_state_and_fetch_services_mining_to(
        default_mining_pool(validator),
        validator,
        chain_cache,
        enable_zaino,
        network,
    )
    .await
}

/// Launch the service bundle mining to a caller-selected pool.
pub async fn launch_state_and_fetch_services_mining_to<V: ValidatorExt>(
    mine_to_pool: MinerPool,
    validator: &ValidatorKind,
    chain_cache: Option<PathBuf>,
    enable_zaino: bool,
    network: Option<NetworkKind>,
) -> StateAndFetchServices<V> {
    let test_manager = TestManager::<V, Direct>::launch_mining_to(
        mine_to_pool,
        validator,
        network,
        None,
        chain_cache.clone(),
        enable_zaino,
        false,
        false,
    )
    .await
    .unwrap();

    let network_type = match network {
        Some(NetworkKind::Mainnet) => {
            println!("Waiting for validator to spawn..");
            tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
            Network::Mainnet
        }
        Some(NetworkKind::Testnet) => {
            println!("Waiting for validator to spawn..");
            tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
            Network::PubTestnet
        }
        _ => Network::Regtest,
    };
    test_manager.local_net.print_stdout();

    let fetch_service = spawn_fetch_service(
        test_manager.full_node_rpc_listen_address.to_string(),
        None,
        test_manager
            .local_net
            .data_dir()
            .path()
            .join("fetch-service-zaino"),
        network_type,
    )
    .await;
    let fetch_subscriber = fetch_service.get_subscriber().inner();
    let state_chain_cache_dir = chain_cache.unwrap_or_else(|| test_manager.data_dir.clone());
    let state_service =
        NodeBackedIndexerService::spawn(NodeBackedIndexerServiceConfig::new_direct(
            zebra_state::Config {
                cache_dir: state_chain_cache_dir,
                ephemeral: false,
                delete_old_database: true,
                debug_stop_at_height: None,
                debug_validity_check_interval: None,
                should_backup_non_finalized_state: false,
                debug_skip_non_finalized_state_backup_task: false,
            },
            test_manager.full_node_rpc_listen_address.to_string(),
            test_manager.full_node_grpc_listen_address,
            false,
            None,
            None,
            None,
            ServiceConfig::default(),
            StorageConfig {
                database: DatabaseConfig {
                    path: test_manager
                        .local_net
                        .data_dir()
                        .path()
                        .join("state-srvice-zaino"),
                    ..Default::default()
                },
                ..Default::default()
            },
            false,
            network_type,
            None,
        ))
        .await
        .unwrap();
    let state_subscriber = state_service.get_subscriber().inner();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    StateAndFetchServices {
        test_manager,
        fetch_service,
        fetch_subscriber,
        state_service,
        state_subscriber,
    }
}

pub(super) async fn spawn_fetch_service(
    rpc_url: String,
    cookie_dir: Option<PathBuf>,
    db_path: PathBuf,
    network: Network,
) -> NodeBackedIndexerService {
    NodeBackedIndexerService::spawn(NodeBackedIndexerServiceConfig::new_rpc(
        rpc_url,
        cookie_dir,
        None,
        None,
        ServiceConfig::default(),
        StorageConfig {
            database: DatabaseConfig {
                path: db_path,
                ..Default::default()
            },
            ..Default::default()
        },
        false,
        network,
        None,
    ))
    .await
    .unwrap()
}

/// Launch an RPC manager and return its service subscriber.
pub async fn launch_with_fetch_subscriber<V: ValidatorExt>(
    validator: &ValidatorKind,
    chain_cache: Option<PathBuf>,
) -> (TestManager<V, Rpc>, NodeBackedIndexerServiceSubscriber) {
    launch_with_fetch_subscriber_mining_to::<V>(
        default_mining_pool(validator),
        validator,
        chain_cache,
    )
    .await
}

/// Launch an RPC manager mining to a caller-selected pool and return its subscriber.
pub async fn launch_with_fetch_subscriber_mining_to<V: ValidatorExt>(
    mine_to_pool: MinerPool,
    validator: &ValidatorKind,
    chain_cache: Option<PathBuf>,
) -> (TestManager<V, Rpc>, NodeBackedIndexerServiceSubscriber) {
    let mut test_manager = TestManager::<V, Rpc>::launch_mining_to(
        mine_to_pool,
        validator,
        None,
        None,
        chain_cache,
        true,
        false,
        false,
    )
    .await
    .unwrap();
    let fetch_service_subscriber = test_manager.service_subscriber.take().unwrap();
    (test_manager, fetch_service_subscriber)
}

/// Return `info` with its error timestamp cleared for cross-source comparison.
pub fn get_info_with_zeroed_timestamp(
    mut info: zaino_primitives::types::rpc::NodeInfo,
) -> zaino_primitives::types::rpc::NodeInfo {
    info.errors_timestamp = None;
    info
}
