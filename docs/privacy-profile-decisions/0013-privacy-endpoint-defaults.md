# The privacy gRPC endpoint is opt-in and fail-closed

## Status

accepted

## Context and decision

Zaino must offer a reduced-metadata endpoint without changing the endpoint
existing operators and clients already use. A privacy label is meaningful only
if its defaults are explicit and sensitive capabilities cannot appear through
an omitted setting.

The existing `grpc_settings` table remains the required legacy-compatible
endpoint. A separate optional `privacy_grpc_settings` table enables the privacy
endpoint. When that table is absent, Zaino starts and runs exactly as it does
today, with no privacy listener.

The privacy v1 policy uses the risk classes from ADR-0012:

| Risk class | Default | Override |
|---|---:|---|
| Common chain data | allow | none |
| Transaction-specific lookup | deny | `allow_transaction_specific_reads = true` |
| Transparent-address lookup | deny | `allow_transparent_address_reads = true` |
| Mempool personalization | deny | none in v1 |
| Transaction submission | deny | never configurable |
| Administration/debug | deny | never configurable |

The two sensitive-read flags default to `false` and act independently. Enabling
one changes only its own class. `GetMempoolTx`, `GetMempoolStream`,
`SendTransaction`, and `Ping` remain denied under every privacy v1 setting.

The default privacy metrics window is 60 seconds. A configured privacy
endpoint uses the same transport safety rules as legacy gRPC. Legacy and
privacy gRPC addresses must differ, and neither may collide with JSON-RPC.

The privacy endpoint creates no persistent client session identifier, generated
client identifier, cookie, or affinity requirement. It emits no `Set-Cookie`
metadata and doesn't require load-balancer stickiness. These are application
boundaries, not claims about infrastructure an operator places in front of
Zaino.

## Alternatives considered

**Replace the legacy endpoint with privacy defaults.** Rejected because it
would silently remove methods from existing clients and change deployed
configuration behavior.

**Enable privacy by default.** Rejected because a second listener requires an
explicit address and an operator decision.

**Use one broad sensitive-read switch.** Rejected because transaction lookup
and transparent-address lookup reveal different data and must be enabled
separately.

**Make every denial configurable.** Rejected. Mempool requests and streams are
personalizing, transaction submission is a write, and `Ping` is a debug method.
Privacy v1 has no switch for these classes.

## Approved defaults and non-goals

Approved defaults are privacy opt-in, common chain data allowed, both sensitive
read classes denied, both mempool methods denied, transaction submission denied,
administration/debug denied, and a 60-second metrics window. Submission, debug,
and mempool denials are immutable in privacy v1.

An enabled transaction-specific or transparent-address read is supported but
still sensitive. The profile does not turn that read into a private query.

This decision doesn't add sessions, cookies, affinity, transaction relay,
transparent local scanning, canonical-read behavior, range bucketing, Tor,
OHTTP, or deployment and proxy configuration.

## Legacy preservation

Legacy configuration, serialization, environment precedence, startup behavior,
method availability, request handling, and response/status semantics remain
unchanged. The legacy endpoint keeps all 20 methods allowed. Omitting
`privacy_grpc_settings` preserves the current single-endpoint process.

## Consequences

Operators must deliberately configure a second listener to offer the privacy
profile. Capability output can state the effective flags and immutable denials
without guessing from transport behavior.

Clients can rely on common chain reads by default, but must inspect capability
output or configuration before using either sensitive-read class. No client can
enable mempool, submission, or debug methods on privacy v1.

Running both profiles adds another tonic server, but not another indexer,
subscriber, chain state, or validator connection.
