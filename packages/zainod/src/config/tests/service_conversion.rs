use tempfile::TempDir;
use zaino_state::{NodeBackedIndexerServiceConfig, ValidatorConnectionType};

use super::{
    super::{load_config, BackendType, ZainodConfig},
    support::{create_test_config_file, EnvGuard},
};

#[test]
fn donation_address_valid_is_accepted() {
    use zcash_address::{ToAddress as _, ZcashAddress};
    use zcash_protocol::consensus::NetworkType;

    let _guard = EnvGuard::new();
    let directory = TempDir::new().unwrap();
    let valid_address = ZcashAddress::from_transparent_p2pkh(NetworkType::Main, [1u8; 20]).encode();
    let content = format!(
        "donation_address = {valid_address:?}\n[grpc_settings]\nlisten_address = \"127.0.0.1:8232\"\n"
    );
    let path = create_test_config_file(&directory, &content, "valid_donation.toml");
    let config = load_config(&path).unwrap();
    assert_eq!(config.donation_address.unwrap().to_string(), valid_address);
}

#[test]
fn donation_address_invalid_is_rejected() {
    let _guard = EnvGuard::new();
    let directory = TempDir::new().unwrap();
    let content = "donation_address = \"not_a_zcash_address\"\n\
         [grpc_settings]\n\
         listen_address = \"127.0.0.1:8232\"\n";
    let path = create_test_config_file(&directory, content, "invalid_donation.toml");
    assert!(load_config(&path).is_err());
}

#[test]
fn indexer_version_is_zainod_pkg_version() {
    let _guard = EnvGuard::new();
    let service_config = NodeBackedIndexerServiceConfig::try_from(ZainodConfig::default())
        .expect("service config conversion should succeed for default ZainodConfig");
    assert_eq!(
        service_config.common.indexer_version,
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn common_payload_is_connection_independent() {
    let _guard = EnvGuard::new();
    let rpc_config = NodeBackedIndexerServiceConfig::try_from(ZainodConfig {
        backend: BackendType::Rpc,
        ..ZainodConfig::default()
    })
    .expect("Rpc conversion should succeed for default ZainodConfig");
    let direct_config = NodeBackedIndexerServiceConfig::try_from(ZainodConfig {
        backend: BackendType::Direct,
        ..ZainodConfig::default()
    })
    .expect("Direct conversion should succeed for default ZainodConfig");
    assert!(matches!(
        rpc_config.connection,
        ValidatorConnectionType::Rpc
    ));
    assert!(matches!(
        direct_config.connection,
        ValidatorConnectionType::Direct(_)
    ));
    assert_eq!(
        format!("{:#?}", rpc_config.common),
        format!("{:#?}", direct_config.common)
    );
    let ephemeral_config = NodeBackedIndexerServiceConfig::try_from(ZainodConfig {
        ephemeral_finalised_state: true,
        ..ZainodConfig::default()
    })
    .expect("conversion should succeed for ephemeral finalised state");
    assert!(ephemeral_config.common.ephemeral_finalised_state);
}

#[test]
fn test_ephemeral_finalised_state_config_is_deserialized() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let toml_content = r#"
backend = "fetch"
network = "PubTestnet"
ephemeral_finalised_state = true
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"
[storage.database]
path = "/zaino/db"
[grpc_settings]
listen_address = "127.0.0.1:8137"
"#;
    let config_path =
        create_test_config_file(&temp_dir, toml_content, "ephemeral_finalised_state.toml");
    let config = load_config(&config_path).expect("load_config failed");
    assert!(config.ephemeral_finalised_state);
    let service_config = NodeBackedIndexerServiceConfig::try_from(config)
        .expect("service config conversion should succeed");
    assert!(service_config.common.ephemeral_finalised_state);
}
