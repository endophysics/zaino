use std::path::PathBuf;

use tempfile::TempDir;
use zaino_common::Network;

use super::{
    super::load_config,
    support::{create_test_config_file, EnvGuard},
};

#[test]
fn test_cookie_dir_logic() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let first = r#"
backend = "fetch"
network = "PubTestnet"
zebra_db_path = "/zebra/db"
[storage.database]
path = "/zaino/db"
[json_server_settings]
json_rpc_listen_address = "127.0.0.1:8237"
cookie_dir = ""
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let first_path = create_test_config_file(&temp_dir, first, "s1.toml");
    let first_config = load_config(&first_path).expect("Config S1 failed");
    assert!(first_config
        .json_server_settings
        .as_ref()
        .unwrap()
        .cookie_dir
        .is_some());

    let second = r#"
backend = "fetch"
network = "PubTestnet"
zebra_db_path = "/zebra/db"
[storage.database]
path = "/zaino/db"
[json_server_settings]
json_rpc_listen_address = "127.0.0.1:8237"
cookie_dir = "/my/cookie/path"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let second_path = create_test_config_file(&temp_dir, second, "s2.toml");
    let second_config = load_config(&second_path).expect("Config S2 failed");
    assert_eq!(
        second_config
            .json_server_settings
            .as_ref()
            .unwrap()
            .cookie_dir,
        Some(PathBuf::from("/my/cookie/path"))
    );

    let third = r#"
backend = "fetch"
network = "PubTestnet"
zebra_db_path = "/zebra/db"
[storage.database]
path = "/zaino/db"
[json_server_settings]
json_rpc_listen_address = "127.0.0.1:8237"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let third_path = create_test_config_file(&temp_dir, third, "s3.toml");
    let third_config = load_config(&third_path).expect("Config S3 failed");
    assert!(third_config
        .json_server_settings
        .unwrap()
        .cookie_dir
        .is_none());
}

#[test]
fn test_env_override_toml_and_defaults() {
    let guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
network = "PubTestnet"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
[privacy_grpc_settings]
listen_address = "127.0.0.1:9140"
allow_transaction_specific_reads = false
allow_transparent_address_reads = false
metrics_window_seconds = 61
"#;
    guard.set_var("ZAINO_NETWORK", "Mainnet");
    guard.set_var(
        "ZAINO_JSON_SERVER_SETTINGS__JSON_RPC_LISTEN_ADDRESS",
        "127.0.0.1:0",
    );
    guard.set_var("ZAINO_JSON_SERVER_SETTINGS__COOKIE_DIR", "/env/cookie/path");
    guard.set_var("ZAINO_STORAGE__CACHE__CAPACITY", "12345");
    guard.set_var("ZAINO_EPHEMERAL_FINALISED_STATE", "true");
    guard.set_var(
        "ZAINO_PRIVACY_GRPC_SETTINGS__LISTEN_ADDRESS",
        "127.0.0.1:9137",
    );
    guard.set_var(
        "ZAINO_PRIVACY_GRPC_SETTINGS__ALLOW_TRANSACTION_SPECIFIC_READS",
        "true",
    );
    guard.set_var(
        "ZAINO_PRIVACY_GRPC_SETTINGS__ALLOW_TRANSPARENT_ADDRESS_READS",
        "true",
    );
    guard.set_var("ZAINO_PRIVACY_GRPC_SETTINGS__METRICS_WINDOW_SECONDS", "120");
    let path = create_test_config_file(&temp_dir, content, "test_config.toml");
    let config = load_config(&path).expect("load_config should succeed");
    assert_eq!(config.network, Network::Mainnet);
    assert_eq!(config.storage.cache.capacity, 12345);
    assert!(config.ephemeral_finalised_state);
    assert_eq!(
        config.json_server_settings.as_ref().unwrap().cookie_dir,
        Some(PathBuf::from("/env/cookie/path"))
    );
    let privacy = config
        .privacy_grpc_settings
        .expect("environment variables must configure privacy gRPC settings");
    assert_eq!(
        privacy.server.listen_address,
        "127.0.0.1:9137".parse().unwrap()
    );
    assert!(privacy.allow_transaction_specific_reads);
    assert!(privacy.allow_transparent_address_reads);
    assert_eq!(privacy.metrics_window_seconds, 120);
}

#[test]
fn test_toml_overrides_defaults() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
network = "Regtest"
[json_server_settings]
json_rpc_listen_address = ""
cookie_dir = ""
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "test_config.toml");
    assert!(load_config(&path).is_err());
}

#[test]
fn test_invalid_env_var_type() {
    let guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    guard.set_var("ZAINO_STORAGE__CACHE__CAPACITY", "not_a_number");
    let path = create_test_config_file(&temp_dir, content, "test_config.toml");
    assert!(load_config(&path).is_err());
}
