# Privacy profiles attach at the serving and daemon seams

## Status

accepted

## Context and decision

The privacy profile changes which gRPC methods are admitted and how calls are
observed. It doesn't change chain interpretation, consensus, validator
protocols, or wallet behavior. Keeping that distinction visible makes the work
reviewable upstream and prevents profile policy from spreading into Zaino's
indexing layers.

The isolation seam has two parts:

1. `zaino-serve` owns immutable endpoint profile context. Its profile modules
   hold method classification, authorization, capability rendering, and
   privacy fixed-window metrics. Route assembly gives each tonic server its own
   shared `EndpointContext`, and handlers authorize before reading a body or
   stream.
2. `zainod` owns dual-server wiring. It reads the optional privacy settings,
   builds named legacy and privacy route sets, binds and supervises both tonic
   servers, combines their status, and closes both during rollback or graceful
   shutdown.

Both servers use clones of one subscriber from one indexer service. There is one
chain index, one chain state, and one set of validator connections. The privacy
profile must not create a second indexer or duplicate synchronization work.

Capability reporting is an additive
`PrivacyProfileService/GetPrivacyProfile` service routed beside
`CompactTxStreamer` on both profiles. It derives method decisions from the same
typed registry used by enforcement. It doesn't modify `CompactTxStreamer`,
`LightdInfo`, existing generated lightwallet client semantics, or the Zebra
protocol.

## Alternatives considered

**Fork the indexer for the privacy endpoint.** Rejected because duplicate chain
state and validator connections would waste resources and could let the two
profiles observe different tips.

**Put privacy policy in chain-index or validator adapters.** Rejected because
method admission and request observability are serving concerns. Lower layers
shouldn't know which public endpoint issued a permitted query.

**Modify `CompactTxStreamer` or `LightdInfo` for capability discovery.**
Rejected because changing the legacy protocol would couple profile discovery to
existing client semantics. A separate service is additive.

**Implement the policy in Vizor or Zebra.** Rejected because WP12 is a Zaino
server-profile change. Wallet choices and validator protocols are separate
work.

## Approved defaults and non-goals

The approved seam is limited to `zaino-serve` endpoint context and `zainod`
dual-server lifecycle wiring. It uses one indexer and adds one capability
service. Existing source, chain-head, mempool, persistence, and validator ports
remain profile-agnostic.

No Vizor change is included. There are also no Zcash consensus changes, Zebra
protocol changes, validator behavior changes, chain-index semantic changes,
transaction-relay changes, load-balancer or deployment changes, Tor or OHTTP
work, canonical-read implementation, range bucketing, mempool epochs, or
transparent local scanning.

## Legacy preservation

The existing legacy route remains present with its current service semantics.
`grpc_settings` keeps its meaning, and no second server exists unless
`privacy_grpc_settings` is configured. The additive capability service doesn't
rename or alter legacy `CompactTxStreamer` methods or `LightdInfo` fields.

Startup returns a running daemon only after every configured bind succeeds. If
privacy binding fails after legacy binding, `zainod` closes the legacy server
before returning the typed startup error. An absent privacy endpoint is neutral
for status and shutdown rather than being reported as offline.

## Consequences

Most privacy work stays in a small serving-profile module and explicit daemon
wiring. Reviewers can evaluate method policy without auditing consensus or
storage code.

The daemon must track two named handles when privacy is configured, including
readiness, critical errors, rollback, and graceful shutdown. This is added
lifecycle complexity, but chain synchronization remains singular.

Future wallet integration can consume capability output without being part of
WP12. Any future policy that requires new chain semantics or validator support
must be proposed outside this seam rather than hidden in endpoint context.
