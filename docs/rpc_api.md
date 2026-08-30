# Zaino RPC APIs

## Lightwallet gRPC service

Zaino serves `CompactTxStreamer` as defined by the
[Lightwallet Protocol](https://github.com/zcash/lightwallet-protocol/blob/main/walletrpc/service.proto).
The table below is the authoritative public method matrix. Its order and risk
classes match `GrpcMethod::ALL`.

| Method | Request and response | Risk class | Legacy | Privacy v1 default | Privacy v1 override |
|---|---|---|---:|---:|---|
| `GetLatestBlock` | `ChainSpec` to `BlockID` | `common_chain_data` | allow | allow | none |
| `GetBlock` | `BlockID` to `CompactBlock` | `common_chain_data` | allow | allow | none |
| `GetBlockNullifiers` | `BlockID` to `CompactBlock` | `common_chain_data` | allow | allow | none |
| `GetBlockRange` | `BlockRange` to stream of `CompactBlock` | `common_chain_data` | allow | allow | none |
| `GetBlockRangeNullifiers` | `BlockRange` to stream of `CompactBlock` | `common_chain_data` | allow | allow | none |
| `GetTransaction` | `TxFilter` to `RawTransaction` | `transaction_specific_lookup` | allow | deny | `allow_transaction_specific_reads = true` |
| `SendTransaction` | `RawTransaction` to `SendResponse` | `transaction_submission` | allow | deny | never |
| `GetTaddressTxids` | `TransparentAddressBlockFilter` to stream of `RawTransaction` | `transparent_address_lookup` | allow | deny | `allow_transparent_address_reads = true` |
| `GetTaddressTransactions` | `TransparentAddressBlockFilter` to stream of `RawTransaction` | `transparent_address_lookup` | allow | deny | `allow_transparent_address_reads = true` |
| `GetTaddressBalance` | `AddressList` to `Balance` | `transparent_address_lookup` | allow | deny | `allow_transparent_address_reads = true` |
| `GetTaddressBalanceStream` | stream of `Address` to `Balance` | `transparent_address_lookup` | allow | deny | `allow_transparent_address_reads = true` |
| `GetMempoolTx` | `GetMempoolTxRequest` to stream of `CompactTx` | `mempool_personalization` | allow | deny | none in v1 |
| `GetMempoolStream` | `Empty` to stream of `RawTransaction` | `mempool_personalization` | allow | deny | none in v1 |
| `GetTreeState` | `BlockID` to `TreeState` | `common_chain_data` | allow | allow | none |
| `GetLatestTreeState` | `Empty` to `TreeState` | `common_chain_data` | allow | allow | none |
| `GetSubtreeRoots` | `GetSubtreeRootsArg` to stream of `SubtreeRoot` | `common_chain_data` | allow | allow | none |
| `GetAddressUtxos` | `GetAddressUtxosArg` to `GetAddressUtxosReplyList` | `transparent_address_lookup` | allow | deny | `allow_transparent_address_reads = true` |
| `GetAddressUtxosStream` | `GetAddressUtxosArg` to stream of `GetAddressUtxosReply` | `transparent_address_lookup` | allow | deny | `allow_transparent_address_reads = true` |
| `GetLightdInfo` | `Empty` to `LightdInfo` | `common_chain_data` | allow | allow | none |
| `Ping` | `Duration` to `PingResponse` | `administration_debug` | allow | deny | never |

The two privacy overrides are independent. A denial is enforced before Zaino
reads the request body or client stream and returns gRPC `PermissionDenied`.
By contrast, a transaction-specific or transparent-address method enabled by an
override remains sensitive. Enabling one doesn't make the query private or
anonymous. Mempool access, transaction submission, and debug access can't be
enabled on privacy v1.

## Privacy profile capability service

Both endpoint profiles also serve the additive unary method
`zaino.privacy.v1.PrivacyProfileService/GetPrivacyProfile`. It takes an empty
request and is outside the 20-method `CompactTxStreamer` registry. The response
contains:

- capability and policy semantic versions, service ID, network, and node
  implementation and revision;
- endpoint profile and all 20 effective method decisions, each with its
  canonical name, risk class, and enabled state;
- active logging mode and metric-window duration. Privacy responses derive
  `metric_window_seconds` solely from the recorder owned by
  `PrivacyWindowMetrics`; the endpoint context stores that recorder-derived
  duration. Legacy responses report `0` because they don't run a privacy
  aggregate window;
- booleans for write support, persistent client identifiers, cookies, and
  affinity;
- RFC 3339 `valid_from` and `valid_until` timestamps. A response is valid for
  five minutes from its injected construction time;
- `canonical_json`, a deterministic byte representation of the same capability.

Capability discovery accepts only the supported `zebra`, `zakura`, and
`development` node identities. zcashd or MagicBean metadata, unknown identities,
malformed or conflicting fields, and metadata that can't establish a supported
identity return gRPC `FailedPrecondition` and no capability document. Zebra metadata
may pair `zcashd_build="v6.3.0"` with
`zcashd_subversion="/Zebra:6.3.0/"`; that supported identity supplies the
canonical revision.

The canonical JSON conforms to capability schema version 1 and follows its
additive-field rules. Zaino adds `read_privacy.method_policy`. Schema-required
parameters remain present when their parent feature is inactive: unsupported
range bucketing and mempool epochs use inactive `1` placeholders, and disabled
private-write protocols and release modes are labeled
`none (inactive schema-required descriptor)`. The inactive maximum transaction
size is 2,000,000 bytes, the consensus block bound. In every case, the adjacent
`supported: false` or `enabled: false` value is authoritative.

Zaino creates no session ID, generated request ID, persistent client ID, cookie,
`Set-Cookie` metadata, or affinity requirement for the privacy endpoint. This is
an application contract. A proxy, load balancer, operating system, or hosting
platform can still log identity data or add cookies and affinity. Operators must
configure those layers independently.

Capability output describes the Zaino server endpoint. It doesn't describe or
enforce Vizor wallet behavior. Range bucketing, canonical reads,
mempool epochs, transparent local scanning, Tor, and OHTTP are not implemented
by this profile.

## Zcash JSON-RPC service

Zaino also serves the implemented subset of
[Zcash RPC methods](https://zcash.github.io/rpc/) required by wallets and block
explorers. The current method specification is in
[Zaino-zcash-rpcs.pdf](./Zaino-zcash-rpcs.pdf).
