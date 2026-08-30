use crate::rpc::profile::GrpcMethod;

/// A fixed observability key for every endpoint handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObservedMethod {
    Grpc(GrpcMethod),
    PrivacyProfileService,
}

impl ObservedMethod {
    pub(crate) const ALL: [Self; 21] = [
        Self::Grpc(GrpcMethod::GetLatestBlock),
        Self::Grpc(GrpcMethod::GetBlock),
        Self::Grpc(GrpcMethod::GetBlockNullifiers),
        Self::Grpc(GrpcMethod::GetBlockRange),
        Self::Grpc(GrpcMethod::GetBlockRangeNullifiers),
        Self::Grpc(GrpcMethod::GetTransaction),
        Self::Grpc(GrpcMethod::SendTransaction),
        Self::Grpc(GrpcMethod::GetTaddressTxids),
        Self::Grpc(GrpcMethod::GetTaddressTransactions),
        Self::Grpc(GrpcMethod::GetTaddressBalance),
        Self::Grpc(GrpcMethod::GetTaddressBalanceStream),
        Self::Grpc(GrpcMethod::GetMempoolTx),
        Self::Grpc(GrpcMethod::GetMempoolStream),
        Self::Grpc(GrpcMethod::GetTreeState),
        Self::Grpc(GrpcMethod::GetLatestTreeState),
        Self::Grpc(GrpcMethod::GetSubtreeRoots),
        Self::Grpc(GrpcMethod::GetAddressUtxos),
        Self::Grpc(GrpcMethod::GetAddressUtxosStream),
        Self::Grpc(GrpcMethod::GetLightdInfo),
        Self::Grpc(GrpcMethod::Ping),
        Self::PrivacyProfileService,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Grpc(GrpcMethod::GetLatestBlock) => 0,
            Self::Grpc(GrpcMethod::GetBlock) => 1,
            Self::Grpc(GrpcMethod::GetBlockNullifiers) => 2,
            Self::Grpc(GrpcMethod::GetBlockRange) => 3,
            Self::Grpc(GrpcMethod::GetBlockRangeNullifiers) => 4,
            Self::Grpc(GrpcMethod::GetTransaction) => 5,
            Self::Grpc(GrpcMethod::SendTransaction) => 6,
            Self::Grpc(GrpcMethod::GetTaddressTxids) => 7,
            Self::Grpc(GrpcMethod::GetTaddressTransactions) => 8,
            Self::Grpc(GrpcMethod::GetTaddressBalance) => 9,
            Self::Grpc(GrpcMethod::GetTaddressBalanceStream) => 10,
            Self::Grpc(GrpcMethod::GetMempoolTx) => 11,
            Self::Grpc(GrpcMethod::GetMempoolStream) => 12,
            Self::Grpc(GrpcMethod::GetTreeState) => 13,
            Self::Grpc(GrpcMethod::GetLatestTreeState) => 14,
            Self::Grpc(GrpcMethod::GetSubtreeRoots) => 15,
            Self::Grpc(GrpcMethod::GetAddressUtxos) => 16,
            Self::Grpc(GrpcMethod::GetAddressUtxosStream) => 17,
            Self::Grpc(GrpcMethod::GetLightdInfo) => 18,
            Self::Grpc(GrpcMethod::Ping) => 19,
            Self::PrivacyProfileService => 20,
        }
    }

    #[cfg(any(test, feature = "prometheus"))]
    pub(crate) const fn static_labels(self) -> StaticLabels {
        match self {
            Self::Grpc(method) => StaticLabels {
                endpoint_profile: "privacy",
                method: method.canonical_name(),
                risk_class: method.risk_class().canonical_name(),
            },
            Self::PrivacyProfileService => StaticLabels {
                endpoint_profile: "privacy",
                method: "PrivacyProfileService/GetPrivacyProfile",
                risk_class: "capability_discovery",
            },
        }
    }
}

#[cfg(any(test, feature = "prometheus"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StaticLabels {
    pub(crate) endpoint_profile: &'static str,
    pub(crate) method: &'static str,
    pub(crate) risk_class: &'static str,
}
