use once_cell::sync::Lazy;
use std::path::PathBuf;

fn binary_path(binary_name: &str) -> Option<PathBuf> {
    std::env::var("TEST_BINARIES_DIR")
        .ok()
        .map(|dir| PathBuf::from(dir).join(binary_name))
}

/// Create a local URI from a port.
pub fn make_uri(indexer_port: u16) -> http::Uri {
    format!("http://127.0.0.1:{indexer_port}")
        .try_into()
        .unwrap()
}

/// Path for zcashd binary.
#[cfg(feature = "zcashd_support")]
pub static ZCASHD_BIN: Lazy<Option<PathBuf>> = Lazy::new(|| binary_path("zcashd"));

/// Path for zcash-cli binary.
#[cfg(feature = "zcashd_support")]
pub static ZCASH_CLI_BIN: Lazy<Option<PathBuf>> = Lazy::new(|| binary_path("zcash-cli"));

/// Path for zebrad binary.
pub static ZEBRAD_BIN: Lazy<Option<PathBuf>> = Lazy::new(|| binary_path("zebrad"));

/// Path for lightwalletd binary.
pub static LIGHTWALLETD_BIN: Lazy<Option<PathBuf>> = Lazy::new(|| binary_path("lightwalletd"));

/// Path for zainod binary.
pub static ZAINOD_BIN: Lazy<Option<PathBuf>> = Lazy::new(|| binary_path("zainod"));

/// Path for zcashd chain cache.
#[cfg(feature = "zcashd_support")]
pub static ZCASHD_CHAIN_CACHE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let mut workspace_root_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    workspace_root_path.pop();
    Some(workspace_root_path.join("live-tests/chain_cache/client_rpc_tests"))
});

/// Path for zebrad chain cache.
pub static ZEBRAD_CHAIN_CACHE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let mut workspace_root_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    workspace_root_path.pop();
    Some(workspace_root_path.join("live-tests/chain_cache/client_rpc_tests_large"))
});

/// Path for the Zebra chain cache of the public testnet.
pub static ZEBRAD_HOME_CACHE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let home_path = PathBuf::from(std::env::var("HOME").unwrap());
    Some(home_path.join(".cache/zebra"))
});

/// Path for the Zebra public-testnet state directory.
pub static ZEBRAD_TESTNET_STATE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let home_path = PathBuf::from(std::env::var("HOME").unwrap());
    Some(home_path.join(".cache/zebra/state/testnet"))
});
