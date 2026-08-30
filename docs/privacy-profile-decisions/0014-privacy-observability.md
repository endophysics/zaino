# Privacy observability is identity-free and windowed

## Status

accepted

## Context and decision

Per-request logs and duration samples can reconstruct a client's activity even
when transport hides the client's network path. Unbounded or request-derived
metric labels create the same risk. The privacy profile still needs enough
aggregate data to show health and denied/error rates.

Privacy calls emit no per-request application log. They are counted in
non-overlapping fixed windows keyed only by these bounded static dimensions:

1. endpoint profile;
2. canonical method;
3. method risk class;
4. coarse outcome, one of `ok`, `denied`, or `error`.

Each window records aggregate request count, error count, and total duration.
It doesn't publish individual duration samples. Labels come from the typed
endpoint and method registries, never from maps keyed by request data.

The default window is 60 seconds and `metrics_window_seconds` may configure a
different positive duration. Windows don't overlap. A timer closes and
publishes a completed window at its boundary even when no later request arrives.
Prometheus gauges expose the last completed window with explicit window-start
and window-duration metadata.

Shutdown deliberately discards the active partial window. It may not publish or
relabel that partial interval as a complete window. Every window visible as
complete therefore covers its full configured duration. Test code uses an
injected clock and tick source rather than real sleeps.

The privacy observability path prohibits every identity and request field
family below, whether as a log field, log message value, metric label, metric
key, or capability snapshot value:

- source IP, peer IP, peer socket, and peer address;
- request metadata, headers, request bodies, request values, and stream items;
- wallet data, Zcash addresses, transparent addresses, and other address values;
- transaction data, transaction IDs, and `txid` values;
- user-agent values;
- request IDs, generated per-request identifiers, and persistent client IDs;
- stream IDs and generated per-stream identifiers;
- cookies, cookie values, and `Set-Cookie` metadata;
- load-balancer affinity, sticky-session keys, and other affinity data.

Denied calls are recorded only with the four static dimensions. Request
metadata and body values are neither read for observability nor recorded.

## Alternatives considered

**Keep per-request logs but remove source IP.** Rejected because request timing,
method sequences, request IDs, addresses, and transaction values can still
identify or fingerprint a client.

**Use cumulative privacy counters and duration histograms.** Rejected because
WP12 requires bounded fixed-window aggregates and no individual duration
samples. Completed windows also give inspection a clear time boundary.

**Key metrics by peer, request, address, transaction, or user agent.** Rejected
because these dimensions are identity-bearing or unbounded.

**Flush a partial shutdown window as complete.** Rejected because it would
misstate the observation duration and make windows incomparable.

## Approved defaults and non-goals

The approved privacy default is aggregate-only observability in configurable
60-second, non-overlapping windows. Label cardinality is bounded by static
profiles, methods, risk classes, and three outcomes. Complete windows are
published on time; partial shutdown windows are discarded.

This boundary covers Zaino application logs, Zaino metrics, and profile
capability snapshots. It doesn't claim to suppress transport, operating-system,
reverse-proxy, load-balancer, or hosting-provider logs. Those systems remain the
operator's responsibility.

This ADR doesn't add tracing correlation, client diagnostics, session tracking,
request sampling, dynamic labels, or per-request latency export.

## Legacy preservation

The legacy endpoint retains its current method-level request logs and cumulative
Prometheus metric behavior, including existing method/status dimensions and
duration observations. Privacy suppression and fixed windows are selected by
endpoint context and do not change legacy output.

## Consequences

Privacy metrics answer aggregate operational questions but can't reconstruct a
single call or client history. Debugging a specific privacy request through
application telemetry is intentionally unsupported.

Completed-window gauges need explicit start and duration metadata, and process
shutdown loses the current partial window. That loss is preferable to
publishing misleading data.

The static registry places a hard bound on label cardinality. A new method or
outcome requires an explicit code and review change before it can appear in
privacy telemetry.
