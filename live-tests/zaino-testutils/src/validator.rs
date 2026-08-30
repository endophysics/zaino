use std::{future::Future, net::Ipv4Addr};

use zaino_common::{network::ActivationHeights, validator::ValidatorConfig};
use zainodlib::config::BackendType;
#[cfg(feature = "zcashd_support")]
use zcash_local_net::validator::zcashd::{Zcashd, ZcashdConfig};
use zcash_local_net::validator::{zebrad::Zebrad, Validator};
use zcash_local_net::{
    error::LaunchError, logs::LogsToStdoutAndStderr, process::Process,
    validator::zebrad::ZebradConfig, MinerPool,
};

use crate::activation::ZEBRAD_DEFAULT_ACTIVATION_HEIGHTS;

/// Test-side selector for the validator connection used by the indexer service.
pub trait ValidatorConnectionMarker: Send + Sync + 'static {
    /// The on-disk backend selector this connection maps to.
    const BACKEND: BackendType;
}

/// JSON-RPC validator connection.
#[derive(Debug, Clone, Copy)]
pub struct Rpc;

impl ValidatorConnectionMarker for Rpc {
    const BACKEND: BackendType = BackendType::Rpc;
}

/// Direct Zebra `ReadStateService` validator connection.
#[derive(Debug, Clone, Copy)]
pub struct Direct;

impl ValidatorConnectionMarker for Direct {
    const BACKEND: BackendType = BackendType::Direct;
}

/// Represents the type of validator to launch.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ValidatorKind {
    /// Zcashd.
    #[cfg(feature = "zcashd_support")]
    Zcashd,
    /// Zebrad.
    Zebrad,
}

impl ValidatorKind {
    /// Default regtest activation heights for this validator kind.
    pub fn default_activation_heights(self) -> ActivationHeights {
        match self {
            #[cfg(feature = "zcashd_support")]
            Self::Zcashd => ActivationHeights::default(),
            Self::Zebrad => ZEBRAD_DEFAULT_ACTIVATION_HEIGHTS,
        }
    }
}

/// Config for validators.
pub enum ValidatorTestConfig {
    /// Zcashd Config.
    #[cfg(feature = "zcashd_support")]
    ZcashdConfig(ZcashdConfig),
    /// Zebrad Config.
    ZebradConfig(ZebradConfig),
}

/// Validator functionality needed by the test infrastructure.
pub trait ValidatorExt: Validator + LogsToStdoutAndStderr {
    /// Launch the validator and return its Zaino connection config.
    fn launch_validator_and_return_config(
        config: Self::Config,
    ) -> impl Future<Output = Result<(Self, ValidatorConfig), LaunchError>> + Send + Sync;
}

impl ValidatorExt for Zebrad {
    async fn launch_validator_and_return_config(
        config: ZebradConfig,
    ) -> Result<(Self, ValidatorConfig), LaunchError> {
        let zebrad = Zebrad::launch(config).await?;
        let validator_config = ValidatorConfig {
            validator_jsonrpc_listen_address: format!(
                "{}:{}",
                Ipv4Addr::LOCALHOST,
                zebrad.rpc_listen_port()
            ),
            validator_grpc_listen_address: Some(format!(
                "{}:{}",
                Ipv4Addr::LOCALHOST,
                zebrad.indexer_listen_port()
            )),
            validator_cookie_path: None,
            validator_user: Some("xxxxxx".to_string()),
            validator_password: Some("xxxxxx".to_string()),
        };
        Ok((zebrad, validator_config))
    }
}

#[cfg(feature = "zcashd_support")]
impl ValidatorExt for Zcashd {
    async fn launch_validator_and_return_config(
        config: Self::Config,
    ) -> Result<(Self, ValidatorConfig), LaunchError> {
        let zcashd = Zcashd::launch(config).await?;
        let validator_config = ValidatorConfig {
            validator_jsonrpc_listen_address: format!("{}:{}", Ipv4Addr::LOCALHOST, zcashd.port()),
            validator_grpc_listen_address: None,
            validator_cookie_path: None,
            validator_user: Some("xxxxxx".to_string()),
            validator_password: Some("xxxxxx".to_string()),
        };
        Ok((zcashd, validator_config))
    }
}

/// The pool used to fund wallets from shielded coinbase outputs.
pub const SHIELDED_FUNDING_POOL: MinerPool = MinerPool::Orchard;

/// The default mining pool for a validator kind.
pub fn default_mining_pool(validator: &ValidatorKind) -> MinerPool {
    if validator == &ValidatorKind::Zebrad {
        MinerPool::Transparent
    } else {
        MinerPool::Orchard
    }
}
