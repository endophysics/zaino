# `zaino-proto` usage

The additive privacy discovery protocol is available as
`zaino_proto::proto::privacy_profile`. Construct a generated
`PrivacyProfileServiceClient` and call `get_privacy_profile` with an empty
`GetPrivacyProfileRequest`.

The response is intentionally dual-form: typed protobuf fields are suitable for
normal clients, while `canonical_json` carries the same capability in stable
capability schema version 1 JSON field order. Consumers should use typed fields
when possible and may ignore additive JSON fields they do not recognize.

Typed fields include semantic versions, service and node metadata, endpoint
profile, all 20 effective method decisions, logging mode, metrics window, write
and identity-state booleans, and five-minute RFC 3339 validity bounds. The JSON
adds `read_privacy.method_policy`. Unsupported capability features retain
inactive schema-required parameter values; their `supported: false` or
`enabled: false` field is authoritative.

`privacy_profile.proto` belongs to Zaino and is generated independently from
the vendored lightwallet protocol. Changes to it must not alter `service.proto`,
`CompactTxStreamer`, or `LightdInfo`. Generated output is committed at
`src/proto/privacy_profile.rs` and should be regenerated and diff-reviewed with
the source proto.
