use tonic::Status;

use super::EndpointContext;

/// A method exposed by the generated `CompactTxStreamer` service.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GrpcMethod {
    /// `GetLatestBlock`.
    GetLatestBlock,
    /// `GetBlock`.
    GetBlock,
    /// `GetBlockNullifiers`.
    GetBlockNullifiers,
    /// `GetBlockRange`.
    GetBlockRange,
    /// `GetBlockRangeNullifiers`.
    GetBlockRangeNullifiers,
    /// `GetTransaction`.
    GetTransaction,
    /// `SendTransaction`.
    SendTransaction,
    /// `GetTaddressTxids`.
    GetTaddressTxids,
    /// `GetTaddressTransactions`.
    GetTaddressTransactions,
    /// `GetTaddressBalance`.
    GetTaddressBalance,
    /// `GetTaddressBalanceStream`.
    GetTaddressBalanceStream,
    /// `GetMempoolTx`.
    GetMempoolTx,
    /// `GetMempoolStream`.
    GetMempoolStream,
    /// `GetTreeState`.
    GetTreeState,
    /// `GetLatestTreeState`.
    GetLatestTreeState,
    /// `GetSubtreeRoots`.
    GetSubtreeRoots,
    /// `GetAddressUtxos`.
    GetAddressUtxos,
    /// `GetAddressUtxosStream`.
    GetAddressUtxosStream,
    /// `GetLightdInfo`.
    GetLightdInfo,
    /// `Ping`.
    Ping,
}

impl GrpcMethod {
    /// Every method in generated service order.
    pub const ALL: [Self; 20] = [
        Self::GetLatestBlock,
        Self::GetBlock,
        Self::GetBlockNullifiers,
        Self::GetBlockRange,
        Self::GetBlockRangeNullifiers,
        Self::GetTransaction,
        Self::SendTransaction,
        Self::GetTaddressTxids,
        Self::GetTaddressTransactions,
        Self::GetTaddressBalance,
        Self::GetTaddressBalanceStream,
        Self::GetMempoolTx,
        Self::GetMempoolStream,
        Self::GetTreeState,
        Self::GetLatestTreeState,
        Self::GetSubtreeRoots,
        Self::GetAddressUtxos,
        Self::GetAddressUtxosStream,
        Self::GetLightdInfo,
        Self::Ping,
    ];

    /// Returns the protobuf method name.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::GetLatestBlock => "GetLatestBlock",
            Self::GetBlock => "GetBlock",
            Self::GetBlockNullifiers => "GetBlockNullifiers",
            Self::GetBlockRange => "GetBlockRange",
            Self::GetBlockRangeNullifiers => "GetBlockRangeNullifiers",
            Self::GetTransaction => "GetTransaction",
            Self::SendTransaction => "SendTransaction",
            Self::GetTaddressTxids => "GetTaddressTxids",
            Self::GetTaddressTransactions => "GetTaddressTransactions",
            Self::GetTaddressBalance => "GetTaddressBalance",
            Self::GetTaddressBalanceStream => "GetTaddressBalanceStream",
            Self::GetMempoolTx => "GetMempoolTx",
            Self::GetMempoolStream => "GetMempoolStream",
            Self::GetTreeState => "GetTreeState",
            Self::GetLatestTreeState => "GetLatestTreeState",
            Self::GetSubtreeRoots => "GetSubtreeRoots",
            Self::GetAddressUtxos => "GetAddressUtxos",
            Self::GetAddressUtxosStream => "GetAddressUtxosStream",
            Self::GetLightdInfo => "GetLightdInfo",
            Self::Ping => "Ping",
        }
    }

    /// Returns the method's single privacy risk classification.
    pub const fn risk_class(self) -> MethodRiskClass {
        match self {
            Self::GetLatestBlock
            | Self::GetBlock
            | Self::GetBlockNullifiers
            | Self::GetBlockRange
            | Self::GetBlockRangeNullifiers
            | Self::GetTreeState
            | Self::GetLatestTreeState
            | Self::GetSubtreeRoots
            | Self::GetLightdInfo => MethodRiskClass::CommonChainData,
            Self::GetTransaction => MethodRiskClass::TransactionSpecificLookup,
            Self::GetTaddressTxids
            | Self::GetTaddressTransactions
            | Self::GetTaddressBalance
            | Self::GetTaddressBalanceStream
            | Self::GetAddressUtxos
            | Self::GetAddressUtxosStream => MethodRiskClass::TransparentAddressLookup,
            Self::GetMempoolTx | Self::GetMempoolStream => MethodRiskClass::MempoolPersonalization,
            Self::SendTransaction => MethodRiskClass::TransactionSubmission,
            Self::Ping => MethodRiskClass::AdministrationDebug,
        }
    }

    /// Authorizes this method for an endpoint context.
    ///
    /// # Errors
    /// Returns `PermissionDenied` when the endpoint policy disables this method.
    pub fn authorize(self, context: &EndpointContext) -> Result<(), Status> {
        if context.allows(self) {
            return Ok(());
        }

        Err(Status::permission_denied(format!(
            "endpoint_profile={} method={} risk_class={}",
            context.profile().canonical_name(),
            self.canonical_name(),
            self.risk_class().canonical_name(),
        )))
    }
}

/// Privacy risk classes for `CompactTxStreamer` methods.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MethodRiskClass {
    /// Chain-wide data that is not wallet-specific.
    CommonChainData,
    /// Lookup of one specific transaction.
    TransactionSpecificLookup,
    /// Lookup keyed by a transparent address.
    TransparentAddressLookup,
    /// Mempool data personalized by client state or timing.
    MempoolPersonalization,
    /// Submission of a transaction.
    TransactionSubmission,
    /// Administrative or debug behavior.
    AdministrationDebug,
}

impl MethodRiskClass {
    /// Returns the stable machine-readable risk class name.
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::CommonChainData => "common_chain_data",
            Self::TransactionSpecificLookup => "transaction_specific_lookup",
            Self::TransparentAddressLookup => "transparent_address_lookup",
            Self::MempoolPersonalization => "mempool_personalization",
            Self::TransactionSubmission => "transaction_submission",
            Self::AdministrationDebug => "administration_debug",
        }
    }
}

#[cfg(test)]
mod tests;
