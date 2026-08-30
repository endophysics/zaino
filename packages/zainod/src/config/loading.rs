use std::path::Path;

use tracing::info;

use crate::error::IndexerError;

use super::{default_ephemeral_cookie_path, ZainodConfig};

/// Loads configuration from a TOML file with optional environment variable overrides.
///
/// Configuration is layered: Defaults → TOML file → Environment variables (prefix: ZAINO_).
pub fn load_config(file_path: &Path) -> Result<ZainodConfig, IndexerError> {
    load_config_with_env(file_path, "ZAINO")
}

/// Loads configuration with a custom environment variable prefix.
pub fn load_config_with_env(
    file_path: &Path,
    env_prefix: &str,
) -> Result<ZainodConfig, IndexerError> {
    let mut builder = config::Config::builder()
        .set_default("backend", "fetch")
        .map_err(|error| IndexerError::ConfigError(error.to_string()))?;
    builder = builder.add_source(
        config::File::from(file_path)
            .format(config::FileFormat::Toml)
            .required(true),
    );
    builder = builder.add_source(
        config::Environment::with_prefix(env_prefix)
            .prefix_separator("_")
            .separator("__")
            .try_parsing(true),
    );

    let settings = builder.build().map_err(|error| {
        IndexerError::ConfigError(format!("Configuration loading failed: {error}"))
    })?;
    let mut parsed_config: ZainodConfig = settings.try_deserialize().map_err(|error| {
        IndexerError::ConfigError(format!("Configuration parsing failed: {error}"))
    })?;

    if parsed_config
        .json_server_settings
        .as_ref()
        .is_some_and(|json_settings| {
            json_settings
                .cookie_dir
                .as_ref()
                .is_some_and(|dir| dir.as_os_str().is_empty())
        })
    {
        if let Some(ref mut json_config) = parsed_config.json_server_settings {
            json_config.cookie_dir = Some(default_ephemeral_cookie_path());
        }
    }

    parsed_config.check_config()?;
    info!(path = %file_path.display(), "config loaded and validated");
    Ok(parsed_config)
}
