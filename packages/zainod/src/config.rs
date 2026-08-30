//! Zaino config.

mod conversion;
mod loading;
mod paths;
mod serialization;
mod types;
mod validation;

pub use loading::{load_config, load_config_with_env};
pub use paths::{default_ephemeral_cookie_path, default_zebra_db_path};
pub use serialization::{generate_default_config, GENERATED_CONFIG_HEADER};
pub use types::{
    BackendType, MempoolSettings, PrivacyGrpcSettings, ZainodConfig, DEFAULT_METRICS_PORT,
};
pub(crate) fn is_private_listen_addr(addr: &std::net::SocketAddr) -> bool {
    validation::is_private_addr(addr)
}

#[cfg(test)]
mod tests;
