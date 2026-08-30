use tempfile::TempDir;
use zaino_common::Network;

use super::{
    super::{load_config, BackendType, ZainodConfig},
    support::{create_test_config_file, EnvGuard},
};

#[test]
fn test_deserialize_full_valid_config() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let cert_file = temp_dir.path().join("test_cert.pem");
    let key_file = temp_dir.path().join("test_key.pem");
    let validator_cookie_file = temp_dir.path().join("validator.cookie");
    let zaino_cookie_dir = temp_dir.path().join("zaino_cookies_dir");
    let zaino_db_dir = temp_dir.path().join("zaino_db_dir");
    let zebra_db_dir = temp_dir.path().join("zebra_db_dir");
    std::fs::write(&cert_file, "mock cert content").unwrap();
    std::fs::write(&key_file, "mock key content").unwrap();
    std::fs::write(&validator_cookie_file, "mock validator cookie content").unwrap();
    std::fs::create_dir_all(&zaino_cookie_dir).unwrap();
    std::fs::create_dir_all(&zaino_db_dir).unwrap();
    std::fs::create_dir_all(&zebra_db_dir).unwrap();
    let toml_content = format!(
        r#"
backend = "fetch"
zebra_db_path = "{}"
network = "Mainnet"
[storage.database]
path = "{}"
[validator_settings]
validator_jsonrpc_listen_address = "192.168.1.10:18232"
validator_cookie_path = "{}"
validator_user = "user"
validator_password = "password"
[json_server_settings]
json_rpc_listen_address = "127.0.0.1:8000"
cookie_dir = "{}"
[grpc_settings]
listen_address = "0.0.0.0:9000"
[grpc_settings.tls]
cert_path = "{}"
key_path = "{}"
"#,
        zebra_db_dir.display(),
        zaino_db_dir.display(),
        validator_cookie_file.display(),
        zaino_cookie_dir.display(),
        cert_file.display(),
        key_file.display(),
    );
    let config_path = create_test_config_file(&temp_dir, &toml_content, "full_config.toml");
    let config = load_config(&config_path).expect("load_config failed");
    assert_eq!(config.backend, BackendType::Rpc);
    assert!(config.json_server_settings.is_some());
    assert_eq!(
        config
            .json_server_settings
            .as_ref()
            .unwrap()
            .json_rpc_listen_address,
        "127.0.0.1:8000".parse().unwrap()
    );
    assert_eq!(config.network, Network::Mainnet);
    assert_eq!(
        config.grpc_settings.listen_address,
        "0.0.0.0:9000".parse().unwrap()
    );
    assert!(config.grpc_settings.tls.is_some());
    assert_eq!(
        config.validator_settings.validator_user,
        Some("user".to_string())
    );
    assert_eq!(
        config.validator_settings.validator_password,
        Some("password".to_string())
    );
}

#[test]
fn test_deserialize_optional_fields_missing() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let toml_content = r#"
backend = "state"
network = "PubTestnet"
zebra_db_path = "/opt/zebra/data"
[storage.database]
path = "/opt/zaino/data"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let config_path = create_test_config_file(&temp_dir, toml_content, "optional_missing.toml");
    let config = load_config(&config_path).expect("load_config failed");
    let defaults = ZainodConfig::default();
    assert_eq!(config.backend, BackendType::Direct);
    assert_eq!(config.network, Network::PubTestnet);
    assert!(config.json_server_settings.is_none());
    assert_eq!(
        config.validator_settings.validator_user,
        defaults.validator_settings.validator_user
    );
    assert_eq!(
        config.storage.cache.capacity,
        defaults.storage.cache.capacity
    );
}

#[test]
fn legacy_only_config_defaults_serialize_and_validate() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let legacy_toml = r#"
backend = "fetch"
network = "PubTestnet"
zebra_db_path = "/opt/zebra/data"
[storage.database]
path = "/opt/zaino/data"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let config_path = create_test_config_file(&temp_dir, legacy_toml, "legacy_only.toml");
    let loaded = load_config(&config_path).expect("legacy-only config must load");
    loaded
        .check_config()
        .expect("legacy-only config must validate");
    let serialized_default =
        toml::to_string_pretty(&ZainodConfig::default()).expect("default config must serialize");
    assert!(serialized_default.contains("[grpc_settings]"));
    assert!(serialized_default.contains("listen_address = \"127.0.0.1:8137\""));
    assert!(!serialized_default.contains("privacy_grpc_settings"));
    let serialized_legacy = toml::to_string_pretty(&loaded).expect("legacy config must serialize");
    assert!(!serialized_legacy.contains("privacy_grpc_settings"));
    let roundtripped: ZainodConfig =
        toml::from_str(&serialized_default).expect("default config TOML must deserialize");
    assert_eq!(
        serialized_default,
        toml::to_string_pretty(&roundtripped).expect("roundtripped default must serialize")
    );
}

#[test]
fn legacy_testnet_spelling_parses_as_the_pub_testnet() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let toml_content = r#"
backend = "state"
network = "Testnet"
zebra_db_path = "/opt/zebra/data"
[storage.database]
path = "/opt/zaino/data"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let config_path = create_test_config_file(&temp_dir, toml_content, "legacy_testnet.toml");
    let config = load_config(&config_path).expect("load_config failed");
    assert_eq!(config.network, Network::PubTestnet);
}

#[test]
fn test_deserialize_empty_string_yields_default() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let toml_content = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let config_path = create_test_config_file(&temp_dir, toml_content, "empty.toml");
    let config = load_config(&config_path).expect("Empty TOML load failed");
    let defaults = ZainodConfig::default();
    assert_eq!(config.network, defaults.network);
    assert_eq!(config.backend, defaults.backend);
    assert_eq!(
        config.storage.cache.capacity,
        defaults.storage.cache.capacity
    );
}
