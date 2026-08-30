use crate::error::IndexerError;

use super::ZainodConfig;

/// Header for generated configuration files.
pub const GENERATED_CONFIG_HEADER: &str = r#"# Zaino Configuration
#
# Generated with `zainod generate-config`
#
# Configuration sources are layered (highest priority first):
#   1. Environment variables (prefix: ZAINO_)
#   2. TOML configuration file
#   3. Built-in defaults
#
# For detailed documentation, see:
#   https://github.com/zingolabs/zaino

"#;

/// Generate default configuration file content.
///
/// Returns the full config file content including header and TOML-serialized defaults.
pub fn generate_default_config() -> Result<String, IndexerError> {
    let config = ZainodConfig::default();
    let toml_content = toml::to_string_pretty(&config).map_err(|error| {
        IndexerError::ConfigError(format!("Failed to serialize config: {error}"))
    })?;

    Ok(format!("{GENERATED_CONFIG_HEADER}{toml_content}"))
}
