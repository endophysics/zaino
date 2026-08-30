# Every CompactTxStreamer method has one privacy risk class

## Status

accepted

## Context and decision

Zaino's legacy gRPC endpoint exposes the complete `CompactTxStreamer` service.
The privacy profile needs a smaller default surface, but a list of method names
inside individual handlers would be easy to miss when the protobuf changes. It
would also let enforcement, capability reporting, and documentation disagree.

We classify all 20 current `CompactTxStreamer` methods in one exhaustive typed
registry. Each method has exactly one of six risk classes:

| Risk class | Methods |
|---|---|
| Common chain data | `GetLatestBlock`, `GetBlock`, `GetBlockNullifiers`, `GetBlockRange`, `GetBlockRangeNullifiers`, `GetTreeState`, `GetLatestTreeState`, `GetSubtreeRoots`, `GetLightdInfo` |
| Transaction-specific lookup | `GetTransaction` |
| Transparent-address lookup | `GetTaddressTxids`, `GetTaddressTransactions`, `GetTaddressBalance`, `GetTaddressBalanceStream`, `GetAddressUtxos`, `GetAddressUtxosStream` |
| Mempool personalization | `GetMempoolTx`, `GetMempoolStream` |
| Transaction submission | `SendTransaction` |
| Administration/debug | `Ping` |

The registry is total rather than a fallback map. Protobuf regeneration that
adds a `CompactTxStreamer` method must fail the registry completeness test until
the new method is named and classified. Authorization and capability output
consume this same registry.

Privacy authorization happens before a request body or client stream reaches
the indexer. A denied call returns gRPC `PermissionDenied` with a stable message
containing only the endpoint profile, canonical method name, and risk class. It
doesn't call `Request::into_inner`, consume a client stream, spawn a forwarding
task, or touch the subscriber.

`PrivacyProfileService/GetPrivacyProfile` is an additive discovery method, not
one of the 20 `CompactTxStreamer` methods. It accepts no wallet-derived input,
is always available on both profiles, and has a fixed static observability key.

## Alternatives considered

**Handler-local allowlists.** Rejected because the 19 generated handler paths
and the separate `GetTaddressBalanceStream` path could drift. They would also
duplicate policy across enforcement and capability reporting.

**Classify only denied methods.** Rejected because an unknown method could then
be allowed by omission. Privacy policy must fail closed.

**Classify from method-name strings at runtime.** Rejected because string
matching doesn't give an exhaustive compile-time contract and invites spelling
differences between consumers.

## Approved defaults and non-goals

This ADR defines classification, early authorization, and the shared registry.
ADR-0013 defines which classes are enabled by each profile. Classification does
not claim that an enabled sensitive method is anonymous.

The registry does not add canonical reads, range bucketing, mempool epochs,
transparent local scanning, Tor, OHTTP, transaction relay, or new
`CompactTxStreamer` methods. It doesn't change the request or response semantics
of an allowed method.

## Legacy preservation

Every one of the 20 methods remains allowed on the legacy endpoint. Legacy
clients keep the existing generated service, status codes, response bodies,
streams, and subscriber behavior. The typed authorization path is shared, but
the legacy policy is allow-all.

## Consequences

Adding or removing a generated method now requires an explicit registry and
classification update. This is intentional review friction.

One policy source controls denial, capability output, bounded metric labels,
and documentation. The separate client-streaming balance handler remains a
distinct enforcement site, but it consumes the same typed decision before
reading its stream.

Denied privacy calls do less work and reveal only static policy information.
They cannot reach chain state or expose request-derived data through the denial
path.
