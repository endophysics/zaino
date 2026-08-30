use tempfile::TempDir;

use crate::error::IndexerError;

use super::{
    super::load_config,
    support::{create_test_config_file, EnvGuard},
};

#[test]
fn test_deserialize_invalid_backend_type() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
backend = "invalid_type"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "invalid_backend.toml");
    let result = load_config(&path);
    assert!(result.is_err());
    if let Err(IndexerError::ConfigError(message)) = result {
        assert!(message.contains("unknown variant") || message.contains("invalid_type"));
    }
}

#[test]
fn test_deserialize_invalid_socket_address() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
[json_server_settings]
json_rpc_listen_address = "not-a-valid-address"
cookie_dir = ""
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "invalid_socket.toml");
    assert!(load_config(&path).is_err());
}

#[test]
fn test_unknown_fields_rejected() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
unknown_field = "value"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "unknown_fields.toml");
    assert!(load_config(&path).is_err());
}

#[test]
fn stale_sync_write_batch_bytes_key_is_rejected() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
sync_write_batch_bytes = 2147483648
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "stale_batch_key.toml");
    assert!(
        load_config(&path).is_err(),
        "stale `sync_write_batch_bytes` key must be rejected by deny_unknown_fields"
    );
}

#[test]
fn current_database_budget_keys_parse() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let content = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
sync_write_batch_size = 2
accumulator_rebuild_memory_size = 1
sync_checkpoint_interval = 30
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let path = create_test_config_file(&temp_dir, content, "current_budget_keys.toml");
    let config = load_config(&path).expect("current budget keys must parse");
    assert_eq!(config.storage.database.sync_write_batch_size.0, 2);
    assert_eq!(config.storage.database.accumulator_rebuild_memory_size.0, 1);
    assert_eq!(config.storage.database.sync_checkpoint_interval, 30);
}
