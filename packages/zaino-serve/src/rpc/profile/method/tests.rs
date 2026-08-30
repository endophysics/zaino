use super::{GrpcMethod, MethodRiskClass};

#[test]
fn method_registry_matches_the_generated_twenty_method_contract() {
    let expected = [
        (GrpcMethod::GetLatestBlock, MethodRiskClass::CommonChainData),
        (GrpcMethod::GetBlock, MethodRiskClass::CommonChainData),
        (
            GrpcMethod::GetBlockNullifiers,
            MethodRiskClass::CommonChainData,
        ),
        (GrpcMethod::GetBlockRange, MethodRiskClass::CommonChainData),
        (
            GrpcMethod::GetBlockRangeNullifiers,
            MethodRiskClass::CommonChainData,
        ),
        (
            GrpcMethod::GetTransaction,
            MethodRiskClass::TransactionSpecificLookup,
        ),
        (
            GrpcMethod::SendTransaction,
            MethodRiskClass::TransactionSubmission,
        ),
        (
            GrpcMethod::GetTaddressTxids,
            MethodRiskClass::TransparentAddressLookup,
        ),
        (
            GrpcMethod::GetTaddressTransactions,
            MethodRiskClass::TransparentAddressLookup,
        ),
        (
            GrpcMethod::GetTaddressBalance,
            MethodRiskClass::TransparentAddressLookup,
        ),
        (
            GrpcMethod::GetTaddressBalanceStream,
            MethodRiskClass::TransparentAddressLookup,
        ),
        (
            GrpcMethod::GetMempoolTx,
            MethodRiskClass::MempoolPersonalization,
        ),
        (
            GrpcMethod::GetMempoolStream,
            MethodRiskClass::MempoolPersonalization,
        ),
        (GrpcMethod::GetTreeState, MethodRiskClass::CommonChainData),
        (
            GrpcMethod::GetLatestTreeState,
            MethodRiskClass::CommonChainData,
        ),
        (
            GrpcMethod::GetSubtreeRoots,
            MethodRiskClass::CommonChainData,
        ),
        (
            GrpcMethod::GetAddressUtxos,
            MethodRiskClass::TransparentAddressLookup,
        ),
        (
            GrpcMethod::GetAddressUtxosStream,
            MethodRiskClass::TransparentAddressLookup,
        ),
        (GrpcMethod::GetLightdInfo, MethodRiskClass::CommonChainData),
        (GrpcMethod::Ping, MethodRiskClass::AdministrationDebug),
    ];

    let actual = GrpcMethod::ALL.map(|method| (method, method.risk_class()));

    assert_eq!(GrpcMethod::ALL.len(), 20);
    assert_eq!(actual, expected);
}

#[test]
fn canonical_names_when_enumerated_match_the_generated_service_names() {
    let expected = [
        "GetLatestBlock",
        "GetBlock",
        "GetBlockNullifiers",
        "GetBlockRange",
        "GetBlockRangeNullifiers",
        "GetTransaction",
        "SendTransaction",
        "GetTaddressTxids",
        "GetTaddressTransactions",
        "GetTaddressBalance",
        "GetTaddressBalanceStream",
        "GetMempoolTx",
        "GetMempoolStream",
        "GetTreeState",
        "GetLatestTreeState",
        "GetSubtreeRoots",
        "GetAddressUtxos",
        "GetAddressUtxosStream",
        "GetLightdInfo",
        "Ping",
    ];

    let actual = GrpcMethod::ALL.map(GrpcMethod::canonical_name);

    assert_eq!(actual, expected);
}
