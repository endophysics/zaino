use time::macros::datetime;
use tonic::Request;
use zaino_proto::proto::privacy_profile::{
    privacy_profile_service_server::PrivacyProfileService, GetPrivacyProfileRequest,
};
use zaino_proto::proto::service::LightdInfo;
use zaino_state::IndexerSubscriber;

use super::{CapabilityDocument, CapabilityService, EndpointContext, RequestLoggingMode};
use crate::rpc::{
    profile::{
        metrics::{ObservedMethod, PrivacyWindowTestHarness},
        PrivacyMethodPolicy, PrivacyOutcome,
    },
    test_support::TestIndexer,
};

#[test]
fn aggregate_mode_when_selected_changes_only_logging_capability_fields() {
    let harness = PrivacyWindowTestHarness::new(11_900, std::time::Duration::from_secs(10));
    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());
    let mut current =
        CapabilityDocument::new(context, lightd_info(), datetime!(2026-08-28 12:34:56 UTC))
            .expect("fixture must build");
    current.request_logging = RequestLoggingMode::MethodLevelRequest;
    let current_wire = current.to_wire().expect("fixture must render");

    current.request_logging = RequestLoggingMode::AggregateOnly;
    let aggregate_wire = current.to_wire().expect("aggregate fixture must render");

    assert_ne!(current_wire.logging_mode, aggregate_wire.logging_mode);
    let mut current_json: serde_json::Value =
        serde_json::from_slice(&current_wire.canonical_json).expect("current JSON must parse");
    let mut aggregate_json: serde_json::Value =
        serde_json::from_slice(&aggregate_wire.canonical_json).expect("aggregate JSON must parse");
    current_json
        .as_object_mut()
        .expect("capability is an object")
        .remove("logging_mode");
    aggregate_json
        .as_object_mut()
        .expect("capability is an object")
        .remove("logging_mode");
    assert_eq!(current_json, aggregate_json);

    let mut current_typed = current_wire;
    current_typed.logging_mode = aggregate_wire.logging_mode;
    current_typed.canonical_json = aggregate_wire.canonical_json.clone();
    assert_eq!(current_typed, aggregate_wire);
    harness.close();
}

#[tokio::test]
async fn privacy_capability_when_called_records_one_ok_observation() {
    let harness = PrivacyWindowTestHarness::new(12_000, std::time::Duration::from_secs(10));
    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());
    let service = CapabilityService::new(IndexerSubscriber::new(TestIndexer::default()), context);

    let result = PrivacyProfileService::get_privacy_profile(
        &service,
        Request::new(GetPrivacyProfileRequest {}),
    )
    .await;
    harness.advance(std::time::Duration::from_secs(10));
    harness.tick();

    assert!(result.is_ok());
    let published = harness.published();
    assert_eq!(published.len(), 1);
    assert_eq!(
        published[0].request_count(ObservedMethod::PrivacyProfileService, PrivacyOutcome::Ok),
        1
    );
    assert_eq!(published[0].total_request_count(), 1);
    harness.close();
}

#[tokio::test]
async fn privacy_capability_when_subscriber_fails_records_one_error_observation() {
    let harness = PrivacyWindowTestHarness::new(12_100, std::time::Duration::from_secs(10));
    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());
    let indexer = TestIndexer::default();
    indexer.set_failure(tonic::Status::internal("capability source failed"));
    indexer.fail_lightd_info();
    let service = CapabilityService::new(IndexerSubscriber::new(indexer), context);

    let result = PrivacyProfileService::get_privacy_profile(
        &service,
        Request::new(GetPrivacyProfileRequest {}),
    )
    .await;
    harness.advance(std::time::Duration::from_secs(10));
    harness.tick();

    assert_eq!(
        result.expect_err("capability source must fail").code(),
        tonic::Code::Internal
    );
    let published = harness.published();
    assert_eq!(published.len(), 1);
    assert_eq!(
        published[0].request_count(ObservedMethod::PrivacyProfileService, PrivacyOutcome::Error),
        1
    );
    assert_eq!(published[0].total_request_count(), 1);
    harness.close();
}

#[tokio::test]
async fn subsecond_metric_window_when_capability_service_is_called_fails_precondition() {
    let harness = PrivacyWindowTestHarness::new(12_200, std::time::Duration::from_millis(500));
    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());
    let service = CapabilityService::new(IndexerSubscriber::new(TestIndexer::default()), context);

    let result = PrivacyProfileService::get_privacy_profile(
        &service,
        Request::new(GetPrivacyProfileRequest {}),
    )
    .await;

    assert_eq!(
        result
            .expect_err("subsecond windows cannot produce a capability")
            .code(),
        tonic::Code::FailedPrecondition
    );
    harness.close();
}

#[tokio::test]
async fn unsupported_validator_metadata_when_capability_service_is_called_fails_precondition_without_response(
) {
    let indexer = TestIndexer::default();
    indexer.unsupported_lightd_info();
    let service =
        CapabilityService::new(IndexerSubscriber::new(indexer), EndpointContext::legacy());

    let result = PrivacyProfileService::get_privacy_profile(
        &service,
        Request::new(GetPrivacyProfileRequest {}),
    )
    .await;

    let status = match result {
        Ok(_) => panic!("unsupported validator metadata must not produce a capability response"),
        Err(status) => status,
    };
    assert_eq!(status.code(), tonic::Code::FailedPrecondition);
}

fn lightd_info() -> LightdInfo {
    LightdInfo {
        chain_name: "main".to_owned(),
        zcashd_build: "Zebra 3.0.0".to_owned(),
        zcashd_subversion: "/Zebra:3.0.0/".to_owned(),
        ..LightdInfo::default()
    }
}
