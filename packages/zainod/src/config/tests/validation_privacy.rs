use tempfile::TempDir;
use zaino_serve::server::config::{GrpcTls, JsonRpcServerConfig};

use crate::error::IndexerError;

use super::{
    super::{PrivacyGrpcSettings, ZainodConfig},
    support::{json_config_with, privacy_config_with},
};

#[test]
fn zero_privacy_metrics_window_is_rejected() {
    let mut config = ZainodConfig {
        privacy_grpc_settings: Some(privacy_config_with("127.0.0.1:9137")),
        ..ZainodConfig::default()
    };
    config
        .privacy_grpc_settings
        .as_mut()
        .expect("privacy endpoint must be configured")
        .metrics_window_seconds = 0;
    config
        .check_config()
        .expect_err("zero privacy metrics window must be rejected");
}

#[test]
fn privacy_metrics_window_above_u32_max_is_rejected() {
    let mut config = ZainodConfig {
        privacy_grpc_settings: Some(privacy_config_with("127.0.0.1:9137")),
        ..ZainodConfig::default()
    };
    config
        .privacy_grpc_settings
        .as_mut()
        .expect("privacy endpoint must be configured")
        .metrics_window_seconds = u64::from(u32::MAX) + 1;
    let error = config
        .check_config()
        .expect_err("oversized privacy metrics window must be rejected");
    assert!(matches!(
        error,
        IndexerError::ConfigError(message)
            if message == "privacy_grpc_settings.metrics_window_seconds must fit in uint32."
    ));
}

#[test]
fn duplicate_legacy_and_privacy_grpc_binds_are_rejected() {
    let config = ZainodConfig {
        privacy_grpc_settings: Some(privacy_config_with("127.0.0.1:8137")),
        ..ZainodConfig::default()
    };
    config
        .check_config()
        .expect_err("legacy and privacy gRPC servers must use different addresses");
}

#[test]
fn privacy_grpc_and_json_rpc_bind_collision_is_rejected() {
    let config = ZainodConfig {
        json_server_settings: Some(JsonRpcServerConfig {
            json_rpc_listen_address: "127.0.0.1:9137"
                .parse()
                .expect("test bind address must parse"),
            cookie_dir: None,
        }),
        privacy_grpc_settings: Some(privacy_config_with("127.0.0.1:9137")),
        ..ZainodConfig::default()
    };
    config
        .check_config()
        .expect_err("privacy gRPC and JSON-RPC servers must use different addresses");
}

#[test]
fn legacy_grpc_and_json_rpc_bind_collision_remains_rejected() {
    json_config_with("127.0.0.1:8137")
        .check_config()
        .expect_err("legacy gRPC and JSON-RPC servers must use different addresses");
}

#[test]
fn privacy_tls_paths_are_validated_independently() {
    let temp_dir = TempDir::new().expect("temporary directory must be created");
    let cert_path = temp_dir.path().join("certificate.pem");
    let key_path = temp_dir.path().join("key.pem");
    std::fs::write(&cert_path, "certificate").expect("certificate fixture must be written");
    std::fs::write(&key_path, "key").expect("key fixture must be written");
    let mut config = ZainodConfig {
        privacy_grpc_settings: Some(PrivacyGrpcSettings {
            server: zaino_serve::server::config::GrpcServerConfig {
                listen_address: "127.0.0.1:9137"
                    .parse()
                    .expect("test bind address must parse"),
                tls: Some(GrpcTls {
                    cert_path: temp_dir.path().join("missing-certificate.pem"),
                    key_path: key_path.clone(),
                }),
            },
            ..privacy_config_with("127.0.0.1:9137")
        }),
        ..ZainodConfig::default()
    };
    config.grpc_settings.tls = Some(GrpcTls {
        cert_path: cert_path.clone(),
        key_path: key_path.clone(),
    });
    config
        .check_config()
        .expect_err("missing privacy certificate must be rejected independently");
    config
        .privacy_grpc_settings
        .as_mut()
        .expect("privacy endpoint must be configured")
        .server
        .tls = Some(GrpcTls {
        cert_path,
        key_path,
    });
    config.grpc_settings.tls = Some(GrpcTls {
        cert_path: temp_dir.path().join("missing-legacy-certificate.pem"),
        key_path: temp_dir.path().join("missing-legacy-key.pem"),
    });
    config
        .check_config()
        .expect_err("missing legacy certificate must remain independently rejected");
}

#[cfg(not(feature = "no_tls_use_unencrypted_traffic"))]
#[test]
fn public_plaintext_privacy_grpc_bind_is_rejected() {
    let config = ZainodConfig {
        privacy_grpc_settings: Some(privacy_config_with("8.8.8.8:9137")),
        ..ZainodConfig::default()
    };
    config
        .check_config()
        .expect_err("public plaintext privacy gRPC bind must be rejected");
}

#[cfg(feature = "no_tls_use_unencrypted_traffic")]
#[test]
fn public_plaintext_privacy_grpc_bind_is_accepted_with_override() {
    let config = ZainodConfig {
        privacy_grpc_settings: Some(privacy_config_with("8.8.8.8:9137")),
        ..ZainodConfig::default()
    };
    config
        .check_config()
        .expect("public plaintext privacy gRPC bind must be accepted with the override");
}
