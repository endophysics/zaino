use serde::Serialize;
use zaino_consensus::MAX_BLOCK_BYTES;

use super::{CapabilityDocument, MethodCapability, CAPABILITY_VERSION, POLICY_VERSION, SERVICE_ID};

#[derive(Serialize)]
struct JsonCapability<'a> {
    capabilities_version: &'static str,
    service_id: &'static str,
    network: &'a str,
    node: JsonNode<'a>,
    supported_policy_versions: [&'static str; 1],
    private_write: JsonPrivateWrite,
    read_privacy: JsonReadPrivacy<'a>,
    valid_from: &'a str,
    valid_until: &'a str,
    endpoint_profile: &'static str,
    policy_version: &'static str,
    logging_mode: &'static str,
    metric_window_seconds: u32,
    persistent_client_identifiers: bool,
    cookies: bool,
    affinity: bool,
}

#[derive(Serialize)]
struct JsonNode<'a> {
    implementation: &'a str,
    revision: &'a str,
}

#[derive(Serialize)]
struct JsonPrivateWrite {
    enabled: bool,
    // Schema-required descriptor; inactive when `enabled` is false.
    submission_protocols: [&'static str; 1],
    // Schema-required descriptor; inactive when `enabled` is false.
    release_modes: [&'static str; 1],
    // Protocol bound descriptor; inactive when `enabled` is false.
    maximum_transaction_bytes: u64,
    // Schema-required descriptor; inactive when `enabled` is false.
    attestation: JsonAttestation,
}

#[derive(Serialize)]
struct JsonAttestation {
    supported: bool,
    required: bool,
}

#[derive(Serialize)]
struct JsonReadPrivacy<'a> {
    range_bucketing: JsonRangeBucketing,
    canonical_objects: JsonSupported,
    mempool_epochs: JsonMempoolEpochs,
    separate_write_session: bool,
    transparent_local_scan: bool,
    method_policy: Vec<JsonMethodPolicy<'a>>,
}

#[derive(Serialize)]
struct JsonRangeBucketing {
    supported: bool,
    bucket_sizes: [u32; 1],
}

#[derive(Serialize)]
struct JsonSupported {
    supported: bool,
}

#[derive(Serialize)]
struct JsonMempoolEpochs {
    supported: bool,
    epoch_ms: u32,
}

#[derive(Serialize)]
struct JsonMethodPolicy<'a> {
    method: &'a str,
    risk_class: &'a str,
    enabled: bool,
}

pub(super) fn serialize(
    document: &CapabilityDocument,
    valid_from: &str,
    valid_until: &str,
) -> Result<Vec<u8>, serde_json::Error> {
    let supports_writes = document.supports_writes();
    let json = JsonCapability {
        capabilities_version: CAPABILITY_VERSION,
        service_id: SERVICE_ID,
        network: &document.network,
        node: JsonNode {
            implementation: &document.node_implementation,
            revision: &document.node_revision,
        },
        supported_policy_versions: [POLICY_VERSION],
        private_write: JsonPrivateWrite {
            enabled: supports_writes,
            submission_protocols: [if supports_writes {
                "lightwalletd-grpc"
            } else {
                "none (inactive schema-required descriptor)"
            }],
            release_modes: [if supports_writes {
                "immediate"
            } else {
                "none (inactive schema-required descriptor)"
            }],
            maximum_transaction_bytes: MAX_BLOCK_BYTES,
            attestation: JsonAttestation {
                supported: false,
                required: false,
            },
        },
        read_privacy: JsonReadPrivacy {
            range_bucketing: JsonRangeBucketing {
                supported: false,
                bucket_sizes: [1],
            },
            canonical_objects: JsonSupported { supported: false },
            mempool_epochs: JsonMempoolEpochs {
                supported: false,
                epoch_ms: 1,
            },
            separate_write_session: false,
            transparent_local_scan: false,
            method_policy: document.methods.iter().map(method_json).collect(),
        },
        valid_from,
        valid_until,
        endpoint_profile: document.profile.canonical_name(),
        policy_version: POLICY_VERSION,
        logging_mode: document.request_logging.canonical_name(),
        metric_window_seconds: document.metric_window_seconds,
        persistent_client_identifiers: false,
        cookies: false,
        affinity: false,
    };
    serde_json::to_vec(&json)
}

fn method_json(method: &MethodCapability) -> JsonMethodPolicy<'_> {
    JsonMethodPolicy {
        method: method.method.canonical_name(),
        risk_class: method.method.risk_class().canonical_name(),
        enabled: method.enabled,
    }
}
