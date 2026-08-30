use serde_json::Value;
use time::OffsetDateTime;
use zaino_proto::proto::privacy_profile::{EndpointProfile, LoggingMode};
use zaino_serve::rpc::profile::{
    EndpointContext, GrpcMethod, PrivacyMethodPolicy, RequestLoggingMode,
};

use super::{privacy_wire, wire, PrivacyFixture, FIXED_TIME};

#[test]
fn legacy_profile_when_rendered_has_typed_allow_all_response() {
    let response = wire(EndpointContext::legacy());

    assert_eq!(response.endpoint_profile, EndpointProfile::Legacy as i32);
    assert_eq!(response.methods.len(), 20);
    assert!(response.methods.iter().all(|method| method.enabled));
    assert!(response.supports_writes);
}

#[test]
fn legacy_profile_when_rendered_reports_no_privacy_metric_window() {
    let response = wire(EndpointContext::legacy());
    let canonical: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");

    assert_eq!(response.metric_window_seconds, 0);
    assert_eq!(canonical["metric_window_seconds"], 0);
}

#[test]
fn default_privacy_profile_when_rendered_has_typed_fail_closed_response() {
    let response = privacy_wire(PrivacyMethodPolicy::default());

    assert_eq!(response.endpoint_profile, EndpointProfile::Privacy as i32);
    assert_eq!(response.methods.len(), 20);
    assert_eq!(
        response
            .methods
            .iter()
            .filter(|method| method.enabled)
            .count(),
        9
    );
}

#[test]
fn privacy_route_when_advertised_reports_aggregate_only_logging() {
    let fixture = PrivacyFixture::new(PrivacyMethodPolicy::default());
    let context = fixture.context();
    let response = wire(context.clone());
    let json: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");

    assert_eq!(
        response.logging_mode,
        LoggingMode::PrivacyAggregateOnly as i32
    );
    assert_eq!(
        context.request_logging_mode(),
        RequestLoggingMode::AggregateOnly
    );
    assert_eq!(json["logging_mode"], "privacy_aggregate_only");
}

#[test]
fn legacy_route_when_advertised_reports_method_level_request_logging() {
    let context = EndpointContext::legacy();
    let response = wire(context.clone());

    assert_eq!(
        response.logging_mode,
        LoggingMode::MethodLevelRequest as i32
    );
    assert_eq!(
        context.request_logging_mode(),
        RequestLoggingMode::MethodLevelRequest
    );
}

#[test]
fn sensitive_flags_when_toggled_change_only_their_risk_classes() {
    for transaction_reads in [false, true] {
        for transparent_reads in [false, true] {
            let fixture = PrivacyFixture::new(PrivacyMethodPolicy::new(
                transaction_reads,
                transparent_reads,
            ));
            let context = fixture.context();
            let response = wire(context.clone());

            for (method, advertised) in GrpcMethod::ALL.iter().zip(&response.methods) {
                assert_eq!(advertised.name, method.canonical_name());
                assert_eq!(advertised.enabled, context.allows(*method));
            }
        }
    }
}

#[test]
fn privacy_profile_when_rendered_has_no_write_or_identity_state() {
    let response = privacy_wire(PrivacyMethodPolicy::default());

    assert!(!response.supports_writes);
    assert!(!response.persistent_client_identifiers);
    assert!(!response.cookies);
    assert!(!response.affinity);
}

#[test]
fn disabled_private_write_when_rendered_uses_only_inactive_schema_descriptors() {
    let response = privacy_wire(PrivacyMethodPolicy::default());
    let json: Value =
        serde_json::from_slice(&response.canonical_json).expect("canonical JSON must parse");
    let private_write = &json["private_write"];

    assert_eq!(private_write["enabled"], false);
    assert_eq!(
        private_write["submission_protocols"],
        serde_json::json!(["none (inactive schema-required descriptor)"])
    );
    assert_eq!(
        private_write["release_modes"],
        serde_json::json!(["none (inactive schema-required descriptor)"])
    );
    assert_eq!(
        private_write["maximum_transaction_bytes"],
        zaino_consensus::MAX_BLOCK_BYTES
    );
    assert_eq!(private_write["attestation"]["supported"], false);
    assert_eq!(private_write["attestation"]["required"], false);
}

#[test]
fn fixed_clock_when_rendered_produces_valid_rfc3339_interval() {
    let response = privacy_wire(PrivacyMethodPolicy::default());

    let valid_from = OffsetDateTime::parse(
        &response.valid_from,
        &time::format_description::well_known::Rfc3339,
    )
    .expect("valid_from must be RFC3339");
    let valid_until = OffsetDateTime::parse(
        &response.valid_until,
        &time::format_description::well_known::Rfc3339,
    )
    .expect("valid_until must be RFC3339");
    assert_eq!(valid_from, FIXED_TIME);
    assert!(valid_until > valid_from);
}

#[test]
fn same_input_when_rendered_twice_has_byte_identical_canonical_json() {
    let fixture = PrivacyFixture::new(PrivacyMethodPolicy::new(true, false));
    let context = fixture.context();

    let first = wire(context.clone()).canonical_json;
    let second = wire(context).canonical_json;

    assert_eq!(first, second);
}
