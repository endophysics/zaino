use tempfile::TempDir;

use crate::error::IndexerError;

use super::{
    super::load_config,
    support::{create_test_config_file, EnvGuard},
};

#[test]
fn test_cookie_auth_not_forced_for_non_loopback_ip() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
backend = "fetch"
network = "PubTestnet"
[validator_settings]
validator_jsonrpc_listen_address = "192.168.1.10:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "no_cookie_auth.toml");
    let result = load_config(&path);
    assert!(
        result.is_ok(),
        "Non-loopback IP without cookie auth should succeed. Error: {:?}",
        result.err()
    );
    assert!(result
        .unwrap()
        .validator_settings
        .validator_cookie_path
        .is_none());
}

#[test]
fn test_public_ip_still_rejected() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
backend = "fetch"
network = "PubTestnet"
[validator_settings]
validator_jsonrpc_listen_address = "8.8.8.8:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "public_ip.toml");
    let result = load_config(&path);
    assert!(result.is_err());
    if let Err(IndexerError::ConfigError(message)) = result {
        assert!(message.contains("private IP"));
    }
}

#[test]
fn test_sensitive_env_var_overrides_toml() {
    let guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
validator_password = "toml-password"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    guard.set_var(
        "ZAINO_VALIDATOR_SETTINGS__VALIDATOR_PASSWORD",
        "environment-password",
    );
    let path = create_test_config_file(&temp_dir, content, "sensitive_env_test.toml");
    let config = load_config(&path).expect("environment password should override TOML");
    assert_eq!(
        config.validator_settings.validator_password.as_deref(),
        Some("environment-password")
    );
}
