use std::time::Duration;

use serde_json::Value;
use zaino_proto::proto::service::LightdInfo;
use zaino_serve::rpc::profile::{
    CapabilityDocument, CapabilityError, EndpointContext, PrivacyMethodPolicy,
};

use super::{privacy_wire, PrivacyFixture, FIXED_TIME};

/// RFC 5737 TEST-NET-3 address used in malformed validator metadata.
const RFC_5737_TEST_NET_3_ADDRESS: &str = "203.0.113.4";

#[test]
fn canonical_json_when_validated_matches_current_capability_schema_and_extension() {
    let response = privacy_wire(PrivacyMethodPolicy::default());
    let schema: Value =
        serde_json::from_str(include_str!("../fixtures/privacy-capabilities.schema.json"))
            .expect("vendored capability schema must be valid JSON");
    let instance: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");
    let validator = jsonschema::validator_for(&schema).expect("capability schema must compile");

    validator
        .validate(&instance)
        .expect("canonical JSON must satisfy capability schema");
    assert!(instance["read_privacy"]["method_policy"].is_array());
    assert_eq!(
        instance["read_privacy"]["method_policy"]
            .as_array()
            .map(Vec::len),
        Some(20)
    );
}

#[test]
fn old_reader_when_denying_unknown_fields_can_select_capability_schema_required_fields() {
    let response = privacy_wire(PrivacyMethodPolicy::default());
    let instance: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");

    for required in [
        "capabilities_version",
        "service_id",
        "network",
        "node",
        "supported_policy_versions",
        "private_write",
        "read_privacy",
        "valid_from",
        "valid_until",
    ] {
        assert!(instance.get(required).is_some(), "missing {required}");
    }
}

#[test]
fn typed_and_json_outputs_when_compared_have_identical_policy_decisions() {
    let response = privacy_wire(PrivacyMethodPolicy::new(true, false));
    let json: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");
    let json_methods = json["read_privacy"]["method_policy"]
        .as_array()
        .expect("method_policy must be an array");

    assert_eq!(json["service_id"], response.service_id);
    assert_eq!(json["network"], response.network);
    assert_eq!(json["valid_from"], response.valid_from);
    assert_eq!(json["valid_until"], response.valid_until);
    for (typed, canonical) in response.methods.iter().zip(json_methods) {
        assert_eq!(canonical["method"], typed.name);
        assert_eq!(canonical["enabled"], typed.enabled);
    }
}

#[test]
fn subsecond_metric_window_when_constructed_is_rejected() {
    let fixture =
        PrivacyFixture::new_with_window(PrivacyMethodPolicy::default(), Duration::from_millis(500));
    let result = CapabilityDocument::new(fixture.context(), super::lightd_info(), FIXED_TIME);

    assert!(matches!(
        result,
        Err(CapabilityError::NonWholeSecondMetricWindow)
    ));
}

#[test]
fn oversized_metric_window_when_constructed_is_rejected() {
    let fixture = PrivacyFixture::new_with_window(
        PrivacyMethodPolicy::default(),
        Duration::from_secs(u64::from(u32::MAX) + 1),
    );
    let result = CapabilityDocument::new(fixture.context(), super::lightd_info(), FIXED_TIME);

    assert!(matches!(result, Err(CapabilityError::MetricWindowTooLarge)));
}

#[test]
fn malformed_metadata_when_rendered_is_rejected() {
    let fixture = PrivacyFixture::new(PrivacyMethodPolicy::default());
    let info = LightdInfo {
        chain_name: format!("peer={RFC_5737_TEST_NET_3_ADDRESS}:8232"),
        zcashd_build: "x".to_owned(),
        zcashd_subversion: "y".to_owned(),
        ..LightdInfo::default()
    };
    let result = CapabilityDocument::new(fixture.context(), info, FIXED_TIME);

    assert!(matches!(
        result,
        Err(CapabilityError::UnsupportedValidatorMetadata)
    ));
}

#[test]
fn privacy_recorder_window_when_rendered_uses_recorder_duration_not_independent_argument() {
    let fixture =
        PrivacyFixture::new_with_window(PrivacyMethodPolicy::default(), Duration::from_secs(73));
    let response = CapabilityDocument::new(fixture.context(), super::lightd_info(), FIXED_TIME)
        .expect("capability fixture must build")
        .to_wire()
        .expect("capability fixture must render");
    let canonical: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");

    assert_eq!(response.metric_window_seconds, 73);
    assert_eq!(canonical["metric_window_seconds"], 73);
}

#[test]
fn zebra_metadata_when_rendered_matches_capability_schema_node_identity() {
    let response = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("zebra-v6.0.0-abcdef0", "/Zebra:6.0.0/"),
        FIXED_TIME,
    )
    .expect("Zebra metadata must be supported")
    .to_wire()
    .expect("Zebra metadata must render");
    let canonical: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");
    let node = response
        .node
        .expect("capability response must include a node");

    assert_eq!(node.implementation, "zebra");
    assert_eq!(node.revision, "zebra-v6.0.0-abcdef0");
    assert_eq!(canonical["node"]["implementation"], "zebra");
    assert_eq!(canonical["node"]["revision"], "zebra-v6.0.0-abcdef0");
}

#[test]
fn zebra_version_build_with_explicit_subversion_when_rendered_matches_capability_schema_node_identity(
) {
    let response = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("v6.3.0", "/Zebra:6.3.0/"),
        FIXED_TIME,
    )
    .expect("Zebra's version-only build metadata must be supported")
    .to_wire()
    .expect("Zebra metadata must render");
    let canonical: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");
    let node = response
        .node
        .expect("capability response must include a node");

    assert_eq!(node.implementation, "zebra");
    assert_eq!(node.revision, "/Zebra:6.3.0/");
    assert_eq!(canonical["node"]["implementation"], "zebra");
    assert_eq!(canonical["node"]["revision"], "/Zebra:6.3.0/");
}

#[test]
fn zakura_metadata_when_rendered_matches_capability_schema_node_identity() {
    let response = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("zakura-v1.0.0-abcdef0", "/Zakura:1.0.0/"),
        FIXED_TIME,
    )
    .expect("Zakura metadata must be supported")
    .to_wire()
    .expect("Zakura metadata must render");
    let canonical: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");
    let node = response
        .node
        .expect("capability response must include a node");

    assert_eq!(node.implementation, "zakura");
    assert_eq!(node.revision, "zakura-v1.0.0-abcdef0");
    assert_eq!(canonical["node"]["implementation"], "zakura");
    assert_eq!(canonical["node"]["revision"], "zakura-v1.0.0-abcdef0");
}

#[test]
fn explicit_development_metadata_when_rendered_matches_capability_schema_node_identity() {
    let response = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("development-abcdef0", "/Development:abcdef0/"),
        FIXED_TIME,
    )
    .expect("explicit development metadata must be supported")
    .to_wire()
    .expect("explicit development metadata must render");
    let canonical: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");
    let node = response
        .node
        .expect("capability response must include a node");

    assert_eq!(node.implementation, "development");
    assert_eq!(node.revision, "development-abcdef0");
    assert_eq!(canonical["node"]["implementation"], "development");
    assert_eq!(canonical["node"]["revision"], "development-abcdef0");
}

#[test]
fn magicbean_metadata_when_rendered_is_rejected() {
    let result = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("v6.0.0-abcdef0", "/MagicBean:6.0.0/"),
        FIXED_TIME,
    );

    assert!(
        result.is_err(),
        "legacy zcashd MagicBean metadata must not produce a capability schema"
    );
}

#[test]
fn unknown_validator_metadata_when_rendered_is_rejected() {
    let result = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("unknown-validator-abcdef0", "/UnknownValidator:1.0.0/"),
        FIXED_TIME,
    );

    assert!(
        result.is_err(),
        "unknown validator metadata must not produce a capability schema"
    );
}

#[test]
fn mismatched_validator_metadata_when_rendered_is_rejected() {
    let result = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("zebra-v6.0.0-abcdef0", "/Zakura:1.0.0/"),
        FIXED_TIME,
    );

    assert!(matches!(
        result,
        Err(CapabilityError::ConflictingValidatorMetadata)
    ));
}

#[test]
fn short_validator_revision_when_rendered_is_rejected() {
    let result = CapabilityDocument::new(
        EndpointContext::legacy(),
        validator_metadata("zebra-", "/Zebra:6.0.0/"),
        FIXED_TIME,
    );

    assert!(matches!(
        result,
        Err(CapabilityError::ShortValidatorRevision)
    ));
}

fn validator_metadata(build: &str, subversion: &str) -> LightdInfo {
    LightdInfo {
        zcashd_build: build.to_owned(),
        zcashd_subversion: subversion.to_owned(),
        ..super::lightd_info()
    }
}
