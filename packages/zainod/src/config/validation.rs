use std::net::{IpAddr, SocketAddr};

use tracing::info;
#[cfg(any(
    feature = "no_tls_use_unencrypted_traffic",
    feature = "allow_unencrypted_public_json_rpc_bind"
))]
use tracing::warn;
use zaino_common::{try_resolve_address, AddressResolution};
use zaino_serve::server::config::GrpcServerConfig;

use crate::error::IndexerError;

use super::{paths::fetch_socket_addr_from_hostname, ZainodConfig};

impl ZainodConfig {
    /// Performs checks on config data.
    pub(crate) fn check_config(&self) -> Result<(), IndexerError> {
        validate_grpc_tls_paths(&self.grpc_settings)?;
        if let Some(privacy_settings) = &self.privacy_grpc_settings {
            validate_grpc_tls_paths(&privacy_settings.server)?;
            if privacy_settings.metrics_window_seconds == 0 {
                return Err(IndexerError::ConfigError(
                    "privacy_grpc_settings.metrics_window_seconds must be greater than zero."
                        .to_string(),
                ));
            }
            if privacy_settings.metrics_window_seconds > u64::from(u32::MAX) {
                return Err(IndexerError::ConfigError(
                    "privacy_grpc_settings.metrics_window_seconds must fit in uint32.".to_string(),
                ));
            }
        }

        if let Some(ref cookie_path) = self.validator_settings.validator_cookie_path {
            if !std::path::Path::new(cookie_path).exists() {
                return Err(IndexerError::ConfigError(format!(
                    "Validator cookie authentication is enabled, but cookie path '{cookie_path:?}' does not exist."
                )));
            }
        }

        match try_resolve_address(&self.validator_settings.validator_jsonrpc_listen_address) {
            AddressResolution::Resolved(validator_addr) => {
                if !super::is_private_listen_addr(&validator_addr) {
                    return Err(IndexerError::ConfigError(
                        "Zaino may only connect to Zebra with private IP addresses.".to_string(),
                    ));
                }
            }
            AddressResolution::UnresolvedHostname { ref address, .. } => {
                info!(%address, "validator address cannot be resolved at config time");
            }
            AddressResolution::InvalidFormat { address, reason } => {
                return Err(IndexerError::ConfigError(format!(
                    "Invalid validator address '{address}': {reason}"
                )));
            }
        }

        #[cfg(not(feature = "no_tls_use_unencrypted_traffic"))]
        {
            validate_grpc_public_bind(&self.grpc_settings)?;
            if let Some(privacy_settings) = &self.privacy_grpc_settings {
                validate_grpc_public_bind(&privacy_settings.server)?;
            }
        }

        #[cfg(feature = "no_tls_use_unencrypted_traffic")]
        warn!("Zaino built using no_tls_use_unencrypted_traffic feature, proceed with caution.");

        #[cfg(not(feature = "allow_unencrypted_public_json_rpc_bind"))]
        if let Some(ref json_settings) = self.json_server_settings {
            if !super::is_private_listen_addr(&json_settings.json_rpc_listen_address) {
                return Err(IndexerError::ConfigError(
                    "JSON-RPC server may only bind to private or loopback addresses. \
                     Build with the `allow_unencrypted_public_json_rpc_bind` feature to \
                     override (trusted private networks only)."
                        .to_string(),
                ));
            }
        }

        #[cfg(feature = "allow_unencrypted_public_json_rpc_bind")]
        warn!(
            "Zaino built with allow_unencrypted_public_json_rpc_bind: the JSON-RPC \
             server may bind to public addresses without encryption. Proceed with caution."
        );

        if let Some(ref json_settings) = self.json_server_settings {
            if json_settings.json_rpc_listen_address == self.grpc_settings.listen_address {
                return Err(IndexerError::ConfigError(
                    "gRPC server and JsonRPC server must listen on different addresses."
                        .to_string(),
                ));
            }
            if let Some(privacy_settings) = &self.privacy_grpc_settings {
                if json_settings.json_rpc_listen_address == privacy_settings.server.listen_address {
                    return Err(IndexerError::ConfigError(
                        "Privacy gRPC server and JsonRPC server must listen on different addresses."
                            .to_string(),
                    ));
                }
            }
        }

        if let Some(privacy_settings) = &self.privacy_grpc_settings {
            if self.grpc_settings.listen_address == privacy_settings.server.listen_address {
                return Err(IndexerError::ConfigError(
                    "Legacy and privacy gRPC servers must listen on different addresses."
                        .to_string(),
                ));
            }
        }

        if let Some(max_cost_bytes) = self.mempool.max_cost_bytes {
            let floor = zaino_mempool::config::MEMPOOL_TRANSACTION_COST_THRESHOLD;
            if max_cost_bytes < floor {
                return Err(IndexerError::ConfigError(format!(
                    "mempool.max_cost_bytes ({max_cost_bytes}) is below the ZIP-401 \
                     per-transaction floor ({floor}); it cannot admit a single \
                     transaction. Raise it to at least {floor}."
                )));
            }
        }

        if self.mempool.poll_interval_ms == Some(0) {
            return Err(IndexerError::ConfigError(
                "mempool.poll_interval_ms must be greater than zero; a zero poll \
                 period is rejected by the runtime timer and would abort at startup."
                    .to_string(),
            ));
        }

        Ok(())
    }
}

fn validate_grpc_tls_paths(server_settings: &GrpcServerConfig) -> Result<(), IndexerError> {
    if let Some(tls) = &server_settings.tls {
        if !tls.cert_path.exists() {
            return Err(IndexerError::ConfigError(format!(
                "TLS is enabled, but certificate path {:?} does not exist.",
                tls.cert_path
            )));
        }
        if !tls.key_path.exists() {
            return Err(IndexerError::ConfigError(format!(
                "TLS is enabled, but key path {:?} does not exist.",
                tls.key_path
            )));
        }
    }
    Ok(())
}

#[cfg(not(feature = "no_tls_use_unencrypted_traffic"))]
fn validate_grpc_public_bind(server_settings: &GrpcServerConfig) -> Result<(), IndexerError> {
    let grpc_addr = fetch_socket_addr_from_hostname(&server_settings.listen_address.to_string())?;
    if !super::is_private_listen_addr(&grpc_addr) && server_settings.tls.is_none() {
        return Err(IndexerError::ConfigError(
            "TLS required when connecting to external addresses.".to_string(),
        ));
    }
    Ok(())
}

/// Validates that the configured `address` is either:
/// - An RFC1918 (private) IPv4 address, or
/// - An IPv6 Unique Local Address (ULA)
pub(super) fn is_private_addr(addr: &SocketAddr) -> bool {
    let ip = addr.ip();
    match ip {
        IpAddr::V4(ipv4) => ipv4.is_private() || ipv4.is_loopback(),
        IpAddr::V6(ipv6) => ipv6.is_unique_local() || ip.is_loopback(),
    }
}
