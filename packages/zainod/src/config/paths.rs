use std::{net::SocketAddr, path::PathBuf};

use crate::error::IndexerError;

/// Returns the default path for Zaino's ephemeral authentication cookie.
pub fn default_ephemeral_cookie_path() -> PathBuf {
    zaino_common::xdg::resolve_path_with_xdg_runtime_defaults("zaino/.cookie")
}

/// Loads the default file path for zebra's local db.
pub fn default_zebra_db_path() -> PathBuf {
    zaino_common::xdg::resolve_path_with_xdg_cache_defaults("zebra")
}

/// Resolves a hostname to a SocketAddr.
pub(super) fn fetch_socket_addr_from_hostname(address: &str) -> Result<SocketAddr, IndexerError> {
    zaino_common::net::resolve_socket_addr(address)
        .map_err(|error| IndexerError::ConfigError(format!("Invalid address '{address}': {error}")))
}
