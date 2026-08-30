use std::sync::Arc;

use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use zaino_proto::proto::{
    privacy_profile::{
        EndpointProfile as WireEndpointProfile, GetPrivacyProfileResponse,
        LoggingMode as WireLoggingMode, MethodPolicy as WireMethodPolicy,
        MethodRiskClass as WireMethodRiskClass, NodeMetadata as WireNodeMetadata,
    },
    service::LightdInfo,
};

use super::{EndpointContext, EndpointProfile, GrpcMethod, MethodRiskClass, RequestLoggingMode};

mod json;
pub(super) mod service;
mod validator;
pub(crate) use service::CapabilityService;

#[cfg(test)]
mod tests;

const CAPABILITY_VERSION: &str = "1.0.0";
const POLICY_VERSION: &str = "1.0.0";
const SERVICE_ID: &str = "zaino-privacy-profile";
const VALIDITY: Duration = Duration::minutes(5);

#[derive(Debug, thiserror::Error)]
/// Errors produced while constructing a capability document.
pub enum CapabilityError {
    /// The privacy metrics duration cannot be represented in whole seconds.
    #[error("privacy metric window must be a whole number of seconds")]
    NonWholeSecondMetricWindow,
    /// The privacy metrics duration exceeds the wire representation.
    #[error("privacy metric window must fit in uint32 seconds")]
    MetricWindowTooLarge,
    /// Validator metadata does not identify a supported validator implementation.
    #[error("validator metadata does not identify a supported validator implementation")]
    UnsupportedValidatorMetadata,
    /// Validator metadata fields identify different validator implementations.
    #[error("validator metadata fields identify conflicting validator implementations")]
    ConflictingValidatorMetadata,
    /// Validator metadata does not provide a sufficiently long revision.
    #[error("validator metadata revision must be at least 7 characters")]
    ShortValidatorRevision,
    /// An RFC3339 timestamp could not be formatted.
    #[error("capability timestamp formatting failed: {0}")]
    Timestamp(#[from] time::error::Format),
    /// The typed canonical JSON model could not be serialized.
    #[error("canonical capability JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MethodCapability {
    method: GrpcMethod,
    enabled: bool,
}

/// Immutable capability document shared by typed protobuf and canonical JSON output.
#[derive(Clone, Debug)]
pub struct CapabilityDocument {
    profile: EndpointProfile,
    request_logging: RequestLoggingMode,
    network: String,
    node_implementation: String,
    node_revision: String,
    methods: Vec<MethodCapability>,
    metric_window_seconds: u32,
    valid_from: OffsetDateTime,
    valid_until: OffsetDateTime,
}

impl CapabilityDocument {
    /// Builds a capability from endpoint policy, node metadata, and a typed instant.
    pub fn new(
        context: Arc<EndpointContext>,
        lightd_info: LightdInfo,
        valid_from: OffsetDateTime,
    ) -> Result<Self, CapabilityError> {
        let metric_window_seconds = match context.metric_window() {
            None => 0,
            Some(window) if window.subsec_nanos() == 0 => u32::try_from(window.as_secs())
                .map_err(|_| CapabilityError::MetricWindowTooLarge)?,
            Some(_) => return Err(CapabilityError::NonWholeSecondMetricWindow),
        };
        let profile = context.profile();
        let request_logging = context.request_logging_mode();
        let network = canonical_network(&lightd_info.chain_name).to_owned();
        let (node_implementation, node_revision) = validator::classify(&lightd_info)?;
        let methods = GrpcMethod::ALL
            .iter()
            .map(|method| MethodCapability {
                method: *method,
                enabled: context.allows(*method),
            })
            .collect();
        Ok(Self {
            profile,
            request_logging,
            network,
            node_implementation: node_implementation.to_owned(),
            node_revision: node_revision.to_owned(),
            methods,
            metric_window_seconds,
            valid_from,
            valid_until: valid_from + VALIDITY,
        })
    }

    /// Converts the business model to the additive protobuf response.
    pub fn to_wire(&self) -> Result<GetPrivacyProfileResponse, CapabilityError> {
        let valid_from = self.valid_from.format(&Rfc3339)?;
        let valid_until = self.valid_until.format(&Rfc3339)?;
        let canonical_json = json::serialize(self, &valid_from, &valid_until)?;
        Ok(GetPrivacyProfileResponse {
            capability_version: CAPABILITY_VERSION.to_owned(),
            policy_version: POLICY_VERSION.to_owned(),
            service_id: SERVICE_ID.to_owned(),
            network: self.network.clone(),
            node: Some(WireNodeMetadata {
                implementation: self.node_implementation.clone(),
                revision: self.node_revision.clone(),
            }),
            endpoint_profile: wire_profile(self.profile) as i32,
            methods: self.methods.iter().map(MethodCapability::to_wire).collect(),
            logging_mode: wire_logging_mode(self.request_logging) as i32,
            metric_window_seconds: self.metric_window_seconds,
            supports_writes: self.supports_writes(),
            persistent_client_identifiers: false,
            cookies: false,
            affinity: false,
            valid_from,
            valid_until,
            canonical_json,
        })
    }

    const fn supports_writes(&self) -> bool {
        matches!(self.profile, EndpointProfile::Legacy)
    }
}

impl MethodCapability {
    fn to_wire(&self) -> WireMethodPolicy {
        WireMethodPolicy {
            name: self.method.canonical_name().to_owned(),
            risk_class: wire_risk_class(self.method.risk_class()) as i32,
            enabled: self.enabled,
        }
    }
}

const fn canonical_network(chain_name: &str) -> &str {
    match chain_name.as_bytes() {
        b"main" | b"mainnet" => "mainnet",
        b"regtest" => "regtest",
        _ => "testnet",
    }
}

const fn wire_profile(profile: EndpointProfile) -> WireEndpointProfile {
    match profile {
        EndpointProfile::Legacy => WireEndpointProfile::Legacy,
        EndpointProfile::Privacy => WireEndpointProfile::Privacy,
    }
}

const fn wire_logging_mode(mode: RequestLoggingMode) -> WireLoggingMode {
    match mode {
        RequestLoggingMode::MethodLevelRequest => WireLoggingMode::MethodLevelRequest,
        RequestLoggingMode::AggregateOnly => WireLoggingMode::PrivacyAggregateOnly,
    }
}

const fn wire_risk_class(risk: MethodRiskClass) -> WireMethodRiskClass {
    match risk {
        MethodRiskClass::CommonChainData => WireMethodRiskClass::CommonChainData,
        MethodRiskClass::TransactionSpecificLookup => {
            WireMethodRiskClass::TransactionSpecificLookup
        }
        MethodRiskClass::TransparentAddressLookup => WireMethodRiskClass::TransparentAddressLookup,
        MethodRiskClass::MempoolPersonalization => WireMethodRiskClass::MempoolPersonalization,
        MethodRiskClass::TransactionSubmission => WireMethodRiskClass::TransactionSubmission,
        MethodRiskClass::AdministrationDebug => WireMethodRiskClass::AdministrationDebug,
    }
}
