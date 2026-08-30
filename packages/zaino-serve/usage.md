# `zaino-serve` usage

`zaino-serve` owns the gRPC and JSON-RPC serving boundary. Its gRPC profile API
classifies every `CompactTxStreamer` method and applies an immutable method
policy before a handler consumes request input or accesses the indexer
subscriber.

## Route construction

Existing callers should continue to use `rpc::grpc_routes(subscriber)`. This is
the legacy-compatible constructor: all 20 `CompactTxStreamer` methods remain
enabled and use their existing response and status behavior.

Callers that need an explicit endpoint policy can use
`rpc::grpc_routes_with_context`:

```rust
use zaino_serve::rpc::{
    grpc_routes_with_context,
    profile::{EndpointContext, PrivacyMethodPolicy, PrivacyWindowMetrics},
};

let metrics = PrivacyWindowMetrics::new(std::time::Duration::from_secs(60))?;
let context = EndpointContext::privacy(PrivacyMethodPolicy::new(
    allow_transaction_specific_reads,
    allow_transparent_address_reads,
), metrics.recorder());
let routes = grpc_routes_with_context(subscriber, context);
```

The context is shared by `Arc` and has no mutation API. Privacy construction
requires a recorder from a live `PrivacyWindowMetrics` owner; there is no
recorder-less aggregate context. The owner must outlive the routes and be closed
after the server. The privacy capability's `metric_window_seconds` comes solely
from the recorder owned by `PrivacyWindowMetrics`; `EndpointContext` stores that
recorder-derived duration. Callers can't provide an independent window value.
Legacy capabilities report `metric_window_seconds = 0` because legacy endpoints
don't run a privacy aggregate window. `zainod` uses this constructor for its
optional privacy listener and uses `grpc_routes` for the legacy listener; both
receive clones of one subscriber.

Both constructors register
`zaino.privacy.v1.PrivacyProfileService/GetPrivacyProfile` beside
`CompactTxStreamer`. The response advertises all 20 methods in registry order,
with typed risk and enabled fields and a byte-identical canonical JSON document.
Node and network metadata come from the subscriber's input-free
`get_lightd_info` query. Service identity and validity never include peer,
listen-address, or request data.

Capability discovery accepts only the supported `zebra`, `zakura`, and
`development` node identities. zcashd or MagicBean metadata, unknown identities,
malformed or conflicting fields, and metadata that can't establish a supported
identity fail closed with gRPC `FailedPrecondition`; no capability document is
returned. Zebra metadata may pair `zcashd_build="v6.3.0"` with
`zcashd_subversion="/Zebra:6.3.0/"`. The explicit supported identity supplies
the canonical revision in that case.

Capability logging mode reports behavior that is active now. Legacy contexts use
`RequestLoggingMode::MethodLevelRequest` and retain the existing per-request log,
cumulative counters, status-code label, and duration histogram. Privacy contexts
use `RequestLoggingMode::AggregateOnly`, emit no per-request application event,
and record only completed fixed windows. `zainod` owns the privacy metrics owner,
passes its recorder into the context, and closes it after the tonic server drains.

The canonical JSON satisfies capability schema version 1 and adds
`read_privacy.method_policy`. The schema requires nonempty parameter arrays even
when a feature is unsupported. When `private_write.enabled` is `false`, every
subordinate value (`submission_protocols`, `release_modes`,
`maximum_transaction_bytes`, and both `attestation` booleans) is an inactive
schema-required descriptor, not an enabled protocol, mode, limit, or
attestation capability. Disabled protocols and release modes are explicitly
labeled `none (inactive schema-required descriptor)`. The reported 2,000,000
bytes comes from `zaino_consensus::MAX_BLOCK_BYTES`, the consensus-backed block
bound that also bounds a transaction, but remains inactive while private writes
are disabled. Unsupported range
bucketing and mempool epochs use schema-valid `1` placeholders solely as
inactive policy parameters; their adjacent `supported: false` is authoritative.

## Method policy

`GrpcMethod::ALL` is the exhaustive 20-method registry. Each method has one
`MethodRiskClass`, and both `EndpointContext::allows` and handler authorization
consume that classification.

| Risk class | Methods | Legacy | Privacy default | Privacy override |
|---|---|---:|---:|---|
| Common chain data | `GetLatestBlock`, `GetBlock`, `GetBlockNullifiers`, `GetBlockRange`, `GetBlockRangeNullifiers`, `GetTreeState`, `GetLatestTreeState`, `GetSubtreeRoots`, `GetLightdInfo` | allow | allow | none |
| Transaction-specific lookup | `GetTransaction` | allow | deny | `allow_transaction_specific_reads` |
| Transparent-address lookup | `GetTaddressTxids`, `GetTaddressTransactions`, `GetTaddressBalance`, `GetTaddressBalanceStream`, `GetAddressUtxos`, `GetAddressUtxosStream` | allow | deny | `allow_transparent_address_reads` |
| Mempool personalization | `GetMempoolTx`, `GetMempoolStream` | allow | deny | none in privacy v1 |
| Transaction submission | `SendTransaction` | allow | deny | never |
| Administration/debug | `Ping` | allow | deny | never |

The two privacy flags are independent. Mempool, submission, and debug methods
remain denied for every flag combination. A denied handler returns
`PermissionDenied` with only the endpoint profile, canonical method name, and
risk class:

```text
endpoint_profile=privacy method=SendTransaction risk_class=transaction_submission
```

Authorization runs before `Request::into_inner`, client-stream polling,
stream-forwarding task creation, or subscriber access. Unknown methods cannot
be represented by `GrpcMethod`; adding a generated service method requires an
explicit enum variant, class mapping, and registry test update.

An override changes enforcement from denied to enabled only for its named risk
class. It doesn't make a transaction or transparent-address lookup private. The
profile makes no privacy promise for an explicitly enabled sensitive method.

## Privacy aggregate dimensions

Every privacy endpoint runs the fixed-window aggregator and lifecycle, including
default builds. Gauge publication is compiled only with the optional
`prometheus` feature. When enabled, each completed window publishes gauges for
`request_count`, `error_count`, and `duration_seconds_sum`, plus numeric
window-start and window-duration metadata.
The duration sum observes each RPC from before authorization through response
creation. Server-streaming calls measure setup/admission rather than stream
consumption, while capability calls include metadata lookup and document
rendering.
The only tuple labels are `endpoint_profile="privacy"`, canonical `method`, its
static `risk_class`, and `outcome` (`ok`, `denied`, or `error`). The registry has
the 20 `GrpcMethod` values plus `PrivacyProfileService/GetPrivacyProfile`, whose
non-policy risk label is `capability_discovery`. All 63 method/outcome tuples are
published, including zeros, when Prometheus support is enabled. Privacy emits no
counter or histogram. A partial shutdown window is discarded in every build and
is never published as complete.

The feature is off in default `zainod` builds, so those builds aggregate and
close windows but expose none of the privacy gauges. Operators enable export
through the existing `zainod` build feature (`--features prometheus`) and the
optional top-level `metrics_endpoint` daemon setting. See the established build
and configuration example in the
[`zaino-bench` usage guide](../zaino-bench/usage.md#the-node-under-test). Without that
feature, the configured endpoint is ignored and no metrics listener binds.

Metric keys, labels, logs, and capability snapshots must never contain source or
peer IP/socket/address; metadata, headers, bodies, values, or stream items;
wallet or Zcash addresses; transaction data or txids; user agents; request,
stream, or persistent client identifiers; cookies or `Set-Cookie`; or affinity
and sticky-session data. These restrictions cover Zaino application telemetry,
not external proxies, load balancers, operating systems, or hosting platforms.
Those layers must be configured independently to avoid identity logs, cookies,
and affinity.
