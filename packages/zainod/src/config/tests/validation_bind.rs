use crate::error::IndexerError;

use super::{super::ZainodConfig, support::json_config_with};

#[test]
fn json_rpc_loopback_bind_is_accepted() {
    json_config_with("127.0.0.1:8237")
        .check_config()
        .expect("loopback JSON-RPC bind must be accepted");
}

#[test]
fn json_rpc_private_ipv4_bind_is_accepted() {
    json_config_with("192.168.1.10:8237")
        .check_config()
        .expect("RFC1918 JSON-RPC bind must be accepted");
}

#[test]
fn json_rpc_ipv6_ula_bind_is_accepted() {
    json_config_with("[fc00::1]:8237")
        .check_config()
        .expect("IPv6 ULA JSON-RPC bind must be accepted");
}

#[test]
fn no_json_server_settings_is_accepted() {
    let config = ZainodConfig {
        json_server_settings: None,
        ..ZainodConfig::default()
    };
    config
        .check_config()
        .expect("config without a JSON-RPC server must be accepted");
}

#[cfg(not(feature = "allow_unencrypted_public_json_rpc_bind"))]
#[test]
fn json_rpc_public_bind_is_rejected() {
    match json_config_with("8.8.8.8:8237").check_config() {
        Err(IndexerError::ConfigError(message)) => assert!(
            message.contains("allow_unencrypted_public_json_rpc_bind"),
            "error should name the override feature, got: {message}"
        ),
        other => panic!("expected ConfigError for public JSON-RPC bind, got {other:?}"),
    }
}

#[cfg(not(feature = "allow_unencrypted_public_json_rpc_bind"))]
#[test]
fn json_rpc_unspecified_bind_is_rejected() {
    match json_config_with("0.0.0.0:8237").check_config() {
        Err(IndexerError::ConfigError(_)) => {}
        other => panic!("expected ConfigError for unspecified JSON-RPC bind, got {other:?}"),
    }
}

#[cfg(feature = "allow_unencrypted_public_json_rpc_bind")]
#[test]
fn json_rpc_public_bind_allowed_with_feature() {
    json_config_with("8.8.8.8:8237")
        .check_config()
        .expect("public JSON-RPC bind must be accepted under the override feature");
}
