use std::{env, path::PathBuf, sync::Mutex};

use tempfile::TempDir;
use zaino_serve::server::config::{GrpcServerConfig, JsonRpcServerConfig};

use super::super::{PrivacyGrpcSettings, ZainodConfig};

const ZAINO_ENV_PREFIX: &str = "ZAINO_";
static TEST_MUTEX: Mutex<()> = Mutex::new(());

pub(super) struct EnvGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
    original_vars: Vec<(String, String)>,
}

impl EnvGuard {
    pub(super) fn new() -> Self {
        let guard = TEST_MUTEX.lock().unwrap_or_else(|error| error.into_inner());
        let original_vars: Vec<_> = env::vars()
            .filter(|(key, _)| key.starts_with(ZAINO_ENV_PREFIX))
            .collect();
        for (key, _) in &original_vars {
            env::remove_var(key);
        }
        Self {
            _guard: guard,
            original_vars,
        }
    }

    pub(super) fn set_var(&self, key: &str, value: &str) {
        env::set_var(key, value);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, _) in env::vars().filter(|(key, _)| key.starts_with(ZAINO_ENV_PREFIX)) {
            env::remove_var(&key);
        }
        for (key, value) in &self.original_vars {
            env::set_var(key, value);
        }
    }
}

pub(super) fn create_test_config_file(
    directory: &TempDir,
    content: &str,
    filename: &str,
) -> PathBuf {
    let path = directory.path().join(filename);
    std::fs::write(&path, content).expect("test config must be written");
    path
}

pub(super) fn json_config_with(address: &str) -> ZainodConfig {
    ZainodConfig {
        json_server_settings: Some(JsonRpcServerConfig {
            json_rpc_listen_address: address.parse().expect("test bind address must parse"),
            cookie_dir: None,
        }),
        ..ZainodConfig::default()
    }
}

pub(super) fn privacy_config_with(address: &str) -> PrivacyGrpcSettings {
    PrivacyGrpcSettings {
        server: GrpcServerConfig {
            listen_address: address.parse().expect("test bind address must parse"),
            tls: None,
        },
        allow_transaction_specific_reads: false,
        allow_transparent_address_reads: false,
        metrics_window_seconds: 60,
    }
}
