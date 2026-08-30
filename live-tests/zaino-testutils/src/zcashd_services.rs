use zaino_common::{network::ActivationHeights, Network};
use zaino_state::{NodeBackedIndexerService, NodeBackedIndexerServiceSubscriber, ZcashService};
use zcash_local_net::validator::zcashd::Zcashd;

use crate::{service_bundles::spawn_fetch_service, Rpc, TestManager, ValidatorKind};

/// A zcashd-backed manager and two standalone RPC services.
pub struct ZcashdDualFetchServices {
    /// The launched zcashd and Zaino manager.
    pub test_manager: TestManager<Zcashd, Rpc>,
    /// Owned zcashd-direct RPC service.
    pub zcashd_fetch_service: NodeBackedIndexerService,
    /// Subscriber whose answers come from zcashd directly.
    pub zcashd_subscriber: NodeBackedIndexerServiceSubscriber,
    /// Owned Zaino-pointed RPC service.
    pub zaino_fetch_service: NodeBackedIndexerService,
    /// Subscriber whose answers come through Zaino JSON-RPC.
    pub zaino_subscriber: NodeBackedIndexerServiceSubscriber,
}

#[allow(deprecated)]
impl ZcashdDualFetchServices {
    /// Mine blocks and wait for both subscribers to observe the new tip.
    pub async fn generate_blocks_and_wait_for_tips(&self, n: u32) {
        self.test_manager
            .generate_blocks_and_wait_for_tips(n, &self.zaino_subscriber, &self.zcashd_subscriber)
            .await;
    }
}

/// Launch zcashd and the two standalone fetch services.
#[allow(deprecated)]
pub async fn launch_zcashd_dual_fetch_services() -> ZcashdDualFetchServices {
    launch_zcashd_dual_fetch_services_at(ActivationHeights::default()).await
}

/// Launch the zcashd service bundle with explicit activation heights.
#[allow(deprecated)]
pub async fn launch_zcashd_dual_fetch_services_at(
    activation_heights: ActivationHeights,
) -> ZcashdDualFetchServices {
    let test_manager = TestManager::<Zcashd, Rpc>::launch(
        &ValidatorKind::Zcashd,
        None,
        Some(activation_heights),
        None,
        true,
        true,
        false,
    )
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let zcashd_fetch_service = spawn_fetch_service(
        test_manager.full_node_rpc_listen_address.to_string(),
        None,
        test_manager
            .local_net
            .data_dir()
            .path()
            .join("zcashd-fetch-service-zaino"),
        Network::Regtest,
    )
    .await;
    let zcashd_subscriber = zcashd_fetch_service.get_subscriber().inner();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let zaino_fetch_service = spawn_fetch_service(
        test_manager
            .zaino_json_rpc_listen_address
            .expect("zaino jsonrpc address must be active for these tests")
            .to_string(),
        test_manager.json_server_cookie_dir.clone(),
        test_manager
            .local_net
            .data_dir()
            .path()
            .join("zaino-fetch-service-zaino"),
        Network::Regtest,
    )
    .await;
    let zaino_subscriber = zaino_fetch_service.get_subscriber().inner();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    ZcashdDualFetchServices {
        test_manager,
        zcashd_fetch_service,
        zcashd_subscriber,
        zaino_fetch_service,
        zaino_subscriber,
    }
}
