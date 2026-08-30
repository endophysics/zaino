use zaino_state::{
    CommonBackendConfig, DirectConnectionConfig, NodeBackedIndexerServiceConfig,
    ValidatorConnectionType,
};

use crate::error::IndexerError;

use super::{paths::fetch_socket_addr_from_hostname, BackendType, ZainodConfig};

impl TryFrom<ZainodConfig> for NodeBackedIndexerServiceConfig {
    type Error = IndexerError;

    fn try_from(config: ZainodConfig) -> Result<Self, Self::Error> {
        let connection = match config.backend {
            BackendType::Rpc => ValidatorConnectionType::Rpc,
            BackendType::Direct => {
                let grpc_listen_address = config
                    .validator_settings
                    .validator_grpc_listen_address
                    .as_ref()
                    .ok_or_else(|| {
                        IndexerError::ConfigError(
                            "Missing validator_grpc_listen_address in configuration".to_string(),
                        )
                    })?;
                let validator_grpc_address = fetch_socket_addr_from_hostname(grpc_listen_address)
                    .map_err(|error| {
                    let message = match error {
                        IndexerError::ConfigError(message) => message,
                        other => other.to_string(),
                    };
                    IndexerError::ConfigError(format!(
                        "Invalid validator_grpc_listen_address '{grpc_listen_address}': {message}"
                    ))
                })?;
                let validator_state_config = zebra_state::Config {
                    cache_dir: config.zebra_db_path.clone(),
                    ephemeral: false,
                    delete_old_database: true,
                    debug_stop_at_height: None,
                    debug_validity_check_interval: None,
                    should_backup_non_finalized_state: true,
                    debug_skip_non_finalized_state_backup_task: false,
                };
                let validator_cookie_auth =
                    config.validator_settings.validator_cookie_path.is_some();

                ValidatorConnectionType::Direct(DirectConnectionConfig {
                    validator_state_config,
                    validator_grpc_address,
                    validator_cookie_auth,
                })
            }
        };

        Ok(NodeBackedIndexerServiceConfig {
            common: build_common(config),
            connection,
        })
    }
}

fn build_common(config: ZainodConfig) -> CommonBackendConfig {
    CommonBackendConfig {
        validator_rpc_address: config.validator_settings.validator_jsonrpc_listen_address,
        validator_cookie_path: config.validator_settings.validator_cookie_path,
        validator_rpc_user: config
            .validator_settings
            .validator_user
            .unwrap_or_else(|| "xxxxxx".to_string()),
        validator_rpc_password: config
            .validator_settings
            .validator_password
            .unwrap_or_else(|| "xxxxxx".to_string()),
        service: config.service,
        storage: config.storage,
        ephemeral_finalised_state: config.ephemeral_finalised_state,
        network: config.network,
        donation_address: config.donation_address,
        mempool: config.mempool.to_mempool_config(),
        indexer_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
