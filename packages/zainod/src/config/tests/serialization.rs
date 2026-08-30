use super::super::{generate_default_config, ZainodConfig, GENERATED_CONFIG_HEADER};

#[test]
fn test_generate_default_config_produces_valid_toml() {
    let content = generate_default_config().expect("should generate config");
    assert!(content.starts_with(GENERATED_CONFIG_HEADER));

    let toml_part = content.strip_prefix(GENERATED_CONFIG_HEADER).unwrap();
    let parsed: Result<toml::Value, _> = toml::from_str(toml_part);
    assert!(
        parsed.is_ok(),
        "Generated config is not valid TOML: {:?}",
        parsed.err()
    );
}

#[test]
fn test_config_roundtrip_serialize_deserialize() {
    let original = ZainodConfig::default();
    let toml_str = toml::to_string_pretty(&original).expect("should serialize");
    let roundtripped: ZainodConfig = toml::from_str(&toml_str).expect("should deserialize");
    let toml_str_again = toml::to_string_pretty(&roundtripped).expect("should serialize again");

    assert_eq!(
        toml_str, toml_str_again,
        "config roundtrip should be stable"
    );
}
