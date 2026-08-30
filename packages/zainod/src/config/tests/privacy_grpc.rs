use tempfile::TempDir;

use super::{
    super::{load_config, ZainodConfig},
    support::{create_test_config_file, privacy_config_with, EnvGuard},
};

#[test]
fn omitted_privacy_grpc_settings_remain_disabled() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let legacy_toml = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let config_path = create_test_config_file(&temp_dir, legacy_toml, "legacy_only.toml");
    let config = load_config(&config_path).expect("legacy-only config must load");
    assert!(config.privacy_grpc_settings.is_none());
}

#[test]
fn privacy_grpc_settings_use_safe_defaults() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let toml = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
[privacy_grpc_settings]
listen_address = "127.0.0.1:9137"
"#;
    let config_path = create_test_config_file(&temp_dir, toml, "privacy_defaults.toml");
    let config = load_config(&config_path).expect("privacy config must load");
    let privacy = config
        .privacy_grpc_settings
        .expect("privacy endpoint must be configured");
    assert!(!privacy.allow_transaction_specific_reads);
    assert!(!privacy.allow_transparent_address_reads);
    assert_eq!(privacy.metrics_window_seconds, 60);
}

#[test]
fn privacy_grpc_settings_accept_explicit_overrides() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let toml = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
[privacy_grpc_settings]
listen_address = "127.0.0.1:9137"
allow_transaction_specific_reads = true
allow_transparent_address_reads = true
metrics_window_seconds = 120
"#;
    let config_path = create_test_config_file(&temp_dir, toml, "privacy_overrides.toml");
    let config = load_config(&config_path).expect("privacy config must load");
    let privacy = config
        .privacy_grpc_settings
        .expect("privacy endpoint must be configured");
    assert!(privacy.allow_transaction_specific_reads);
    assert!(privacy.allow_transparent_address_reads);
    assert_eq!(privacy.metrics_window_seconds, 120);
}

#[test]
fn privacy_grpc_settings_roundtrip_through_toml() {
    let original = ZainodConfig {
        privacy_grpc_settings: Some(privacy_config_with("127.0.0.1:9137")),
        ..ZainodConfig::default()
    };
    let serialized = toml::to_string_pretty(&original).expect("privacy config must serialize");
    let roundtripped: ZainodConfig =
        toml::from_str(&serialized).expect("privacy config must deserialize");
    assert_eq!(
        serialized,
        toml::to_string_pretty(&roundtripped).expect("roundtripped privacy config must serialize")
    );
}

#[test]
fn privacy_grpc_tls_settings_roundtrip_through_toml() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().expect("temporary directory must be created");
    let cert_path = temp_dir.path().join("privacy-certificate.pem");
    let key_path = temp_dir.path().join("privacy-key.pem");
    std::fs::write(&cert_path, "certificate").expect("certificate fixture must be written");
    std::fs::write(&key_path, "key").expect("key fixture must be written");
    let toml = format!(
        r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
[privacy_grpc_settings]
listen_address = "127.0.0.1:9137"
[privacy_grpc_settings.tls]
cert_path = "{}"
key_path = "{}"
"#,
        cert_path.display(),
        key_path.display(),
    );
    let config_path = create_test_config_file(&temp_dir, &toml, "privacy_tls.toml");
    let config = load_config(&config_path).expect("privacy TLS config must load");
    let serialized = toml::to_string_pretty(&config).expect("privacy TLS config must serialize");
    assert!(serialized.contains("[privacy_grpc_settings.tls]"));
    let roundtripped: ZainodConfig =
        toml::from_str(&serialized).expect("privacy TLS config must deserialize");
    assert_eq!(
        serialized,
        toml::to_string_pretty(&roundtripped)
            .expect("roundtripped privacy TLS config must serialize")
    );
}
