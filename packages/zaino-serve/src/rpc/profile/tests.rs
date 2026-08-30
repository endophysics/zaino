use tonic::Code;

use super::{
    metrics::PrivacyWindowTestHarness, EndpointContext, GrpcMethod, MethodRiskClass,
    PrivacyMethodPolicy, RequestLoggingMode,
};

#[test]
fn privacy_context_when_constructed_requires_an_owned_aggregate_recorder() {
    let harness = PrivacyWindowTestHarness::new(10_000, std::time::Duration::from_secs(10));

    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());

    assert_eq!(
        context.request_logging_mode(),
        RequestLoggingMode::AggregateOnly
    );
    harness.close();
}

#[test]
fn legacy_profile_when_enumerated_allows_every_registered_method() {
    let context = EndpointContext::legacy();

    let decisions = GrpcMethod::ALL.map(|method| context.allows(method));

    assert_eq!(decisions, [true; 20]);
}

#[test]
fn privacy_profile_when_flags_vary_matches_each_class_policy() {
    let combinations = [(false, false), (false, true), (true, false), (true, true)];

    for (allow_transaction_specific_reads, allow_transparent_address_reads) in combinations {
        let harness = PrivacyWindowTestHarness::new(10_000, std::time::Duration::from_secs(10));
        let context = EndpointContext::privacy(
            PrivacyMethodPolicy::new(
                allow_transaction_specific_reads,
                allow_transparent_address_reads,
            ),
            harness.recorder(),
        );
        let decisions = GrpcMethod::ALL.map(|method| context.allows(method));
        let expected = GrpcMethod::ALL.map(|method| match method.risk_class() {
            MethodRiskClass::CommonChainData => true,
            MethodRiskClass::TransactionSpecificLookup => allow_transaction_specific_reads,
            MethodRiskClass::TransparentAddressLookup => allow_transparent_address_reads,
            MethodRiskClass::MempoolPersonalization
            | MethodRiskClass::TransactionSubmission
            | MethodRiskClass::AdministrationDebug => false,
        });

        assert_eq!(decisions, expected);
        harness.close();
    }
}

#[test]
fn privacy_profile_when_flags_vary_keeps_immutable_classes_denied() {
    let immutable_denials = [
        GrpcMethod::GetMempoolTx,
        GrpcMethod::GetMempoolStream,
        GrpcMethod::SendTransaction,
        GrpcMethod::Ping,
    ];

    for allow_transaction_specific_reads in [false, true] {
        for allow_transparent_address_reads in [false, true] {
            let harness = PrivacyWindowTestHarness::new(10_000, std::time::Duration::from_secs(10));
            let context = EndpointContext::privacy(
                PrivacyMethodPolicy::new(
                    allow_transaction_specific_reads,
                    allow_transparent_address_reads,
                ),
                harness.recorder(),
            );

            assert_eq!(
                immutable_denials.map(|method| context.allows(method)),
                [false; 4]
            );
            harness.close();
        }
    }
}

#[test]
fn privacy_denial_when_authorized_returns_stable_static_status() {
    let harness = PrivacyWindowTestHarness::new(10_000, std::time::Duration::from_secs(10));
    let context = EndpointContext::privacy(PrivacyMethodPolicy::default(), harness.recorder());

    let status = GrpcMethod::SendTransaction
        .authorize(&context)
        .expect_err("privacy must deny transaction submission");

    assert_eq!(status.code(), Code::PermissionDenied);
    assert_eq!(
        status.message(),
        "endpoint_profile=privacy method=SendTransaction risk_class=transaction_submission"
    );
    harness.close();
}
