use std::net::SocketAddr;

use tonic::metadata::KeyRef;
use zaino_proto::proto::privacy_profile::{
    privacy_profile_service_client::PrivacyProfileServiceClient, GetPrivacyProfileRequest,
    GetPrivacyProfileResponse, MethodRiskClass,
};

pub(super) const PRIVACY_POLICY: [(&str, MethodRiskClass, bool); 20] = [
    ("GetLatestBlock", MethodRiskClass::CommonChainData, true),
    ("GetBlock", MethodRiskClass::CommonChainData, true),
    ("GetBlockNullifiers", MethodRiskClass::CommonChainData, true),
    ("GetBlockRange", MethodRiskClass::CommonChainData, true),
    (
        "GetBlockRangeNullifiers",
        MethodRiskClass::CommonChainData,
        true,
    ),
    (
        "GetTransaction",
        MethodRiskClass::TransactionSpecificLookup,
        false,
    ),
    (
        "SendTransaction",
        MethodRiskClass::TransactionSubmission,
        false,
    ),
    (
        "GetTaddressTxids",
        MethodRiskClass::TransparentAddressLookup,
        false,
    ),
    (
        "GetTaddressTransactions",
        MethodRiskClass::TransparentAddressLookup,
        false,
    ),
    (
        "GetTaddressBalance",
        MethodRiskClass::TransparentAddressLookup,
        false,
    ),
    (
        "GetTaddressBalanceStream",
        MethodRiskClass::TransparentAddressLookup,
        false,
    ),
    (
        "GetMempoolTx",
        MethodRiskClass::MempoolPersonalization,
        false,
    ),
    (
        "GetMempoolStream",
        MethodRiskClass::MempoolPersonalization,
        false,
    ),
    ("GetTreeState", MethodRiskClass::CommonChainData, true),
    ("GetLatestTreeState", MethodRiskClass::CommonChainData, true),
    ("GetSubtreeRoots", MethodRiskClass::CommonChainData, true),
    (
        "GetAddressUtxos",
        MethodRiskClass::TransparentAddressLookup,
        false,
    ),
    (
        "GetAddressUtxosStream",
        MethodRiskClass::TransparentAddressLookup,
        false,
    ),
    ("GetLightdInfo", MethodRiskClass::CommonChainData, true),
    ("Ping", MethodRiskClass::AdministrationDebug, false),
];

pub(super) async fn capability(address: SocketAddr) -> GetPrivacyProfileResponse {
    let response = PrivacyProfileServiceClient::connect(format!("http://{address}"))
        .await
        .expect("capability client must connect")
        .get_privacy_profile(GetPrivacyProfileRequest {})
        .await
        .expect("capability request must succeed");
    assert_identity_free(response.metadata());
    response.into_inner()
}

pub(super) fn assert_identity_free(metadata: &tonic::metadata::MetadataMap) {
    for key in metadata.keys() {
        let name = match key {
            KeyRef::Ascii(key) => key.as_str(),
            KeyRef::Binary(key) => key.as_str(),
        };
        assert!(
            !["set-cookie", "affinity", "session", "request-id"]
                .iter()
                .any(|forbidden| name.contains(forbidden)),
            "response metadata exposed identity-bearing key {name}"
        );
    }
}
