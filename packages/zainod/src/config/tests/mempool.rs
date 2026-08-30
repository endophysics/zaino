use std::time::Duration;

use tempfile::TempDir;

use super::{
    super::{load_config, ZainodConfig},
    support::{create_test_config_file, EnvGuard},
};

#[test]
fn mempool_section_overrides_only_what_it_sets() {
    let _guard = EnvGuard::new();
    let temp_dir = TempDir::new().unwrap();
    let toml_content = r#"
[validator_settings]
validator_jsonrpc_listen_address = "127.0.0.1:18232"

[storage.database]
path = "/zaino/db"

[grpc_settings]
listen_address = "127.0.0.1:8137"

[mempool]
max_cost_bytes = 67108864
poll_interval_ms = 250
"#;
    let config_path = create_test_config_file(&temp_dir, toml_content, "mempool.toml");
    let config = load_config(&config_path).expect("load_config failed");
    assert_eq!(config.mempool.max_cost_bytes, Some(67_108_864));
    assert_eq!(config.mempool.poll_interval_ms, Some(250));

    let mempool = config.mempool.to_mempool_config();
    assert_eq!(mempool.max_cost_bytes(), 67_108_864);
    assert_eq!(mempool.poll_interval(), Duration::from_millis(250));
    assert_eq!(mempool.metadata_min_interval(), mempool.poll_interval());
    assert_eq!(
        mempool.max_exclude_count(),
        zaino_mempool::MempoolConfig::default().max_exclude_count()
    );
}

#[test]
fn absent_mempool_section_keeps_the_built_in_bounds() {
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
    let config_path = create_test_config_file(&temp_dir, toml_content, "no_mempool.toml");
    let config = load_config(&config_path).expect("load_config failed");
    let mempool = config.mempool.to_mempool_config();
    let defaults = zaino_mempool::MempoolConfig::default();
    assert_eq!(mempool.max_cost_bytes(), defaults.max_cost_bytes());
    assert_eq!(mempool.poll_interval(), defaults.poll_interval());
    assert_eq!(
        mempool.metadata_min_interval(),
        defaults.metadata_min_interval()
    );
}

#[test]
fn mempool_bound_below_the_zip401_floor_is_rejected() {
    let mut config = ZainodConfig::default();
    config.mempool.max_cost_bytes =
        Some(zaino_mempool::config::MEMPOOL_TRANSACTION_COST_THRESHOLD - 1);
    config
        .check_config()
        .expect_err("a sub-floor mempool bound must be rejected");
    config.mempool.max_cost_bytes = Some(zaino_mempool::config::MEMPOOL_TRANSACTION_COST_THRESHOLD);
    config
        .check_config()
        .expect("a bound at the floor admits one transaction and is accepted");
}

#[test]
fn a_zero_mempool_poll_interval_is_rejected() {
    let mut config = ZainodConfig::default();
    config.mempool.poll_interval_ms = Some(0);
    config
        .check_config()
        .expect_err("a zero poll interval must be rejected");
    config.mempool.poll_interval_ms = Some(1);
    config
        .check_config()
        .expect("a non-zero poll interval is the operator's business");
}

#[test]
fn a_zero_metadata_floor_is_accepted() {
    let mut config = ZainodConfig::default();
    config.mempool.metadata_min_interval_ms = Some(0);
    config
        .check_config()
        .expect("a zero metadata floor is legal: it means no additional coalescing");
    let mempool = config.mempool.to_mempool_config();
    assert_eq!(mempool.metadata_min_interval(), Duration::ZERO);
}
