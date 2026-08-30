use std::{net::SocketAddr, path::PathBuf};

use serde::{Deserialize, Serialize};
use zaino_common::{Network, ServiceConfig, StorageConfig, ValidatorConfig};
use zaino_serve::server::config::{GrpcServerConfig, JsonRpcServerConfig};
use zaino_state::DonationAddress;

use super::paths::default_zebra_db_path;

/// Default port for the Prometheus metrics endpoint.
pub const DEFAULT_METRICS_PORT: u16 = 9998;

/// On-disk selector for the validator connection (`backend = "direct" | "rpc"` in the
/// config file), mapped to [`zaino_state::ValidatorConnectionType`] at spawn.
///
/// The legacy values `"state"` / `"fetch"` remain accepted as aliases for backward
/// compatibility with existing `zainod.toml` files.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    /// Direct Zebra `ReadStateService` access (formerly `state`).
    ///
    /// More efficient but requires running on the same machine as Zebra.
    #[serde(alias = "state")]
    Direct,
    /// JSON-RPC access (formerly `fetch`).
    ///
    /// Compatible with Zebra or another Zaino instance.
    #[default]
    #[serde(alias = "fetch")]
    Rpc,
}

/// Operator-facing mempool bounds, as they appear in `[mempool]`.
///
/// A TOML mirror of [`zaino_mempool::MempoolConfig`], which cannot be
/// deserialized directly (its runtime-adjustable bound is a shared atomic).
/// Every field is optional: an absent one keeps the built-in default, so an
/// existing config file without a `[mempool]` section is unaffected.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MempoolSettings {
    /// Maximum total mempool cost Zaino will hold, in bytes (default 128 MiB).
    ///
    /// A denial-of-service backstop, deliberately above the validator's own
    /// ZIP-401 limit so healthy operation never reaches it. Over-bound
    /// transactions are refused and the mempool is reported as incomplete.
    pub max_cost_bytes: Option<u64>,
    /// How often to poll the validator's mempool, in milliseconds (default 500).
    pub poll_interval_ms: Option<u64>,
    /// Minimum gap between verbose mempool listings, in milliseconds (default:
    /// the poll interval).
    ///
    /// The validator answers the listing by walking its whole mempool. Raising
    /// this above the poll interval trades *addition-visibility* latency for
    /// validator load: between listings, new transactions are deferred (not
    /// dropped), while removals and the tip re-tag still apply — so tip-coherent
    /// reads are unaffected.
    pub metadata_min_interval_ms: Option<u64>,
    /// Maximum exclude suffixes a client may send to a filtered mempool read
    /// (default 1024).
    pub max_exclude_count: Option<usize>,
}

impl MempoolSettings {
    /// Applies these settings over the built-in defaults.
    ///
    /// Infallible: the only value that could be rejected here — a zero poll
    /// interval — is refused by [`ZainodConfig::check_config`] before this runs, so
    /// the operator sees a named configuration error rather than a panic deep in
    /// the runtime. `NonZeroU64::new` still cannot be unwrapped blindly, so a
    /// zero that somehow reached this point falls back to the default rather
    /// than taking the process down.
    pub(super) fn to_mempool_config(&self) -> zaino_mempool::MempoolConfig {
        let mut config = zaino_mempool::MempoolConfig::default();
        if let Some(max_cost_bytes) = self.max_cost_bytes {
            config.set_max_cost_bytes(max_cost_bytes);
        }
        if let Some(poll_interval_ms) = self.poll_interval_ms.and_then(std::num::NonZeroU64::new) {
            config.set_poll_interval_ms(poll_interval_ms);
            config.set_metadata_min_interval(config.poll_interval());
        }
        if let Some(metadata_min_interval_ms) = self.metadata_min_interval_ms {
            config.set_metadata_min_interval(std::time::Duration::from_millis(
                metadata_min_interval_ms,
            ));
        }
        if let Some(max_exclude_count) = self.max_exclude_count {
            config.set_max_exclude_count(max_exclude_count);
        }
        config
    }
}

/// Privacy gRPC endpoint settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrivacyGrpcSettings {
    /// gRPC server listen address and optional TLS configuration.
    #[serde(flatten)]
    pub server: GrpcServerConfig,
    /// Permit transaction-specific reads from the privacy endpoint.
    #[serde(default)]
    pub allow_transaction_specific_reads: bool,
    /// Permit transparent-address reads from the privacy endpoint.
    #[serde(default)]
    pub allow_transparent_address_reads: bool,
    /// Fixed privacy metrics aggregation window in seconds.
    #[serde(default = "default_privacy_metrics_window_seconds")]
    pub metrics_window_seconds: u64,
}

const fn default_privacy_metrics_window_seconds() -> u64 {
    60
}

/// Zaino daemon configuration.
///
/// Field order matters for TOML serialization: simple values must come before tables.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ZainodConfig {
    /// Backend type for fetching blockchain data.
    pub backend: BackendType,
    /// Path to Zebra's state database.
    ///
    /// Required when using the `state` backend.
    pub zebra_db_path: PathBuf,
    /// Run the finalised-state database in ephemeral/stateless mode.
    ///
    /// When enabled, Zaino does not use a persistent on-disk finalised-state database. Finalised
    /// state reads are served from the configured validator/source instead.
    pub ephemeral_finalised_state: bool,
    /// Network to connect to (Mainnet, PubTestnet — The Public Testnet — or Regtest;
    /// `"Testnet"` is accepted as a legacy spelling of PubTestnet).
    pub network: Network,
    /// Prometheus metrics endpoint listen address.
    ///
    /// Set to enable the `/metrics` scrape endpoint. Disabled when `None`.
    /// Requires the `prometheus` feature; ignored without it.
    pub metrics_endpoint: Option<SocketAddr>,
    /// JSON-RPC server settings. Set to enable Zaino's JSON-RPC interface.
    pub json_server_settings: Option<JsonRpcServerConfig>,
    /// gRPC server settings (listen address, TLS configuration).
    pub grpc_settings: GrpcServerConfig,
    /// Optional privacy-scoped gRPC endpoint settings.
    #[serde(default)]
    pub privacy_grpc_settings: Option<PrivacyGrpcSettings>,
    /// Validator connection settings.
    pub validator_settings: ValidatorConfig,
    /// Service-level settings (timeout, channel size).
    pub service: ServiceConfig,
    /// Storage settings (cache and database).
    pub storage: StorageConfig,
    /// Mempool bounds (memory cap, poll cadence, exclude-list caps).
    #[serde(default)]
    pub mempool: MempoolSettings,
    /// Zcash donation UA address
    pub donation_address: Option<DonationAddress>,
}

impl Default for ZainodConfig {
    fn default() -> Self {
        Self {
            backend: BackendType::default(),
            metrics_endpoint: None,
            json_server_settings: None,
            grpc_settings: GrpcServerConfig {
                listen_address: "127.0.0.1:8137"
                    .parse()
                    .expect("hard-coded loopback gRPC socket literal must be valid"),
                tls: None,
            },
            privacy_grpc_settings: None,
            validator_settings: ValidatorConfig {
                validator_grpc_listen_address: Some("127.0.0.1:18230".to_string()),
                validator_jsonrpc_listen_address: "127.0.0.1:18232".to_string(),
                validator_cookie_path: None,
                validator_user: Some("xxxxxx".to_string()),
                validator_password: Some("xxxxxx".to_string()),
            },
            service: ServiceConfig::default(),
            storage: StorageConfig::default(),
            mempool: MempoolSettings::default(),
            ephemeral_finalised_state: false,
            zebra_db_path: default_zebra_db_path(),
            network: Network::PubTestnet,
            donation_address: None,
        }
    }
}
