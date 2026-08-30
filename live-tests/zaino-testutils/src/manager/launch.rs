use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use tracing::{debug, instrument};
use zaino_common::{network::ActivationHeights, Network};
#[cfg(feature = "zcashd_support")]
use zainodlib::config::BackendType;
use zcash_local_net::{validator::ValidatorConfig as _, MinerPool};
use zebra_chain::parameters::NetworkKind;

use super::{
    indexer::{disabled_indexer, launch_indexer, IndexerLaunch},
    PrivacyEndpointConfig, TestManager, TestManagerLaunch,
};
use crate::{
    activation::to_local_net_activation_heights,
    validator::{default_mining_pool, ValidatorConnectionMarker, ValidatorExt, ValidatorKind},
};

impl<C, Conn> TestManager<C, Conn>
where
    C: ValidatorExt,
    Conn: ValidatorConnectionMarker,
{
    /// Launches the validator and optional Zaino services.
    pub async fn launch(
        validator: &ValidatorKind,
        network: Option<NetworkKind>,
        activation_heights: Option<ActivationHeights>,
        chain_cache: Option<PathBuf>,
        enable_zaino: bool,
        enable_zaino_jsonrpc_server: bool,
        enable_clients: bool,
    ) -> Result<Self, std::io::Error> {
        Self::launch_mining_to(
            default_mining_pool(validator),
            validator,
            network,
            activation_heights,
            chain_cache,
            enable_zaino,
            enable_zaino_jsonrpc_server,
            enable_clients,
        )
        .await
    }

    /// Launches the validator mining to the selected pool.
    #[instrument(
        name = "TestManager::launch",
        skip(activation_heights, chain_cache),
        fields(validator = ?validator, network = ?network, mine_to_pool = ?mine_to_pool, enable_zaino, enable_clients)
    )]
    #[expect(
        clippy::too_many_arguments,
        reason = "compatibility wrapper for existing live-test launch call sites"
    )]
    pub async fn launch_mining_to(
        mine_to_pool: MinerPool,
        validator: &ValidatorKind,
        network: Option<NetworkKind>,
        activation_heights: Option<ActivationHeights>,
        chain_cache: Option<PathBuf>,
        enable_zaino: bool,
        enable_zaino_jsonrpc_server: bool,
        enable_clients: bool,
    ) -> Result<Self, std::io::Error> {
        Self::launch_mining_to_with_privacy(TestManagerLaunch {
            mine_to_pool,
            validator,
            network,
            activation_heights,
            chain_cache,
            enable_zaino,
            enable_zaino_jsonrpc_server,
            enable_clients,
            privacy_endpoint: None,
        })
        .await
    }

    /// Launches a real validator and one Zaino instance serving legacy and privacy gRPC.
    pub async fn launch_with_privacy(
        validator: &ValidatorKind,
        privacy: PrivacyEndpointConfig,
    ) -> Result<Self, std::io::Error> {
        Self::launch_mining_to_with_privacy(TestManagerLaunch {
            mine_to_pool: default_mining_pool(validator),
            validator,
            network: None,
            activation_heights: None,
            chain_cache: None,
            enable_zaino: true,
            enable_zaino_jsonrpc_server: false,
            enable_clients: false,
            privacy_endpoint: Some(privacy),
        })
        .await
    }

    async fn launch_mining_to_with_privacy(
        launch: TestManagerLaunch<'_>,
    ) -> Result<Self, std::io::Error> {
        #[cfg(feature = "zcashd_support")]
        if launch.validator == &ValidatorKind::Zcashd && Conn::BACKEND == BackendType::Direct {
            return Err(std::io::Error::other(
                "Cannot use the direct (ReadStateService) connection with zcashd.",
            ));
        }
        zaino_common::logging::try_init();
        if launch.enable_clients && !launch.enable_zaino {
            return Err(std::io::Error::other(
                "Cannot enable clients when zaino is not enabled.",
            ));
        }

        let activation_heights = launch
            .activation_heights
            .unwrap_or_else(|| launch.validator.default_activation_heights());
        let network_kind = launch.network.unwrap_or(NetworkKind::Regtest);
        let zaino_network = match network_kind {
            NetworkKind::Mainnet => Network::Mainnet,
            NetworkKind::Testnet => Network::PubTestnet,
            NetworkKind::Regtest => Network::Regtest,
        };
        let mut validator_config = C::Config::default();
        validator_config.set_test_parameters(
            launch.mine_to_pool,
            to_local_net_activation_heights(&activation_heights),
            launch.chain_cache.clone(),
        );
        debug!("[TEST] Launching validator");
        let (local_net, validator_settings) =
            C::launch_validator_and_return_config(validator_config)
                .await
                .expect("to launch a default validator");
        let rpc_listen_port = local_net.get_port();
        debug!(rpc_port = rpc_listen_port, "[TEST] Validator launched");
        let data_dir = local_net.data_dir().path().to_path_buf();
        let zebra_db_path = launch.chain_cache.unwrap_or_else(|| data_dir.clone());
        let launched_zaino = if launch.enable_zaino {
            launch_indexer(IndexerLaunch {
                backend: Conn::BACKEND,
                validator_settings: validator_settings.clone(),
                zaino_db_path: data_dir.join("zaino"),
                zebra_db_path,
                network: zaino_network,
                enable_jsonrpc: launch.enable_zaino_jsonrpc_server,
                privacy_endpoint: launch.privacy_endpoint,
            })
            .await?
        } else {
            disabled_indexer()
        };
        if launch.enable_clients {
            return Err(std::io::Error::other(
                "enable_clients is unsupported in zaino-testutils: build lightclients in the e2e workspace from TestManager's gRPC address instead.",
            ));
        }

        let manager = Self {
            local_net,
            data_dir,
            network: network_kind,
            full_node_rpc_listen_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                rpc_listen_port,
            ),
            full_node_grpc_listen_address: validator_settings
                .validator_grpc_listen_address
                .as_ref()
                .and_then(|address| address.parse().ok())
                .unwrap_or(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)),
            zaino_handle: launched_zaino.handle,
            zaino_json_rpc_listen_address: launched_zaino.json_address,
            zaino_grpc_listen_address: launched_zaino.grpc_address,
            zaino_privacy_grpc_listen_address: launched_zaino.privacy_grpc_address,
            service_subscriber: launched_zaino.subscriber,
            json_server_cookie_dir: launched_zaino.json_cookie_dir,
            _connection: PhantomData,
        };
        manager.activate_nu5_nu6(launch.enable_zaino).await;
        Ok(manager)
    }
}
