# Spec 025 — Wire transport contract (PF-WP-040, R2 wedge #4)

**WP**: PF-WP-040 · **Release gate**: R2-wedge-contract · **Status**: draft

## 1. Background

The R1 release doc (`releases/2026-09-08-R1.md` §"What's intentionally not in R1") and the spec 024 worklog entry (`WORKLOG.md` §"Spec 024 surface-plane runtime authored") both pin the same sequencing rule:

> **Wire transport (PF-WP-040) — defer until spec 024 + spec 025 wire format are pinned.**

Spec 023 shipped the **first** half (the JSON-over-stdio contract between
`cmd/checker` Go and `fabric-graph-cli` Rust — the inter-process wire for
topology-driven replan). This spec ships the **second** half: the
**inter-node** wire contract — the bytes-on-wire shape for exchanging
descriptors, replan requests, surface invalidation events, and lease state
changes between Fabric nodes over a real network.

Per ADR-0029, **Go is canonical for the wire transport layer**. Rust
fabric-graph code stays the source of truth for graph-domain types
(`Topology`, `Intent`, `RoutePlan`, `SurfaceSpec`, `SurfaceLease`,
`CapabilityDescriptor`); the wire layer is a thin Go façade that
serializes/deserializes those types over JSON.

Per ADR-0028, this turn ships the **contract only** — no actual sockets,
no net/http, no gRPC. The contract is what both client and server will
implement against in R3. Today there is one Go package (`cmd/wire/`)
that defines the wire types and round-trip codec with stdlib only.

## 2. Scope

### 2.1 In scope

| # | Component | Notes |
|---|---|---|
| 1 | `WireEnvelope` | The outermost wire shape (envelope_id, tenant_id, msg_type, payload, signature, sent_at_unix_ms) |
| 2 | `WireMessage` interface | One Go type per msg_type (5 total, see §3.2) |
| 3 | `WireCodec` | `Marshal` / `Unmarshal` / `Validate` — deterministic JSON (sorted keys, RFC3339Nano) |
| 4 | `WireError` taxonomy | Stable error codes per §3.4 |
| 5 | `cmd/wire/` Go package | Pure contract, stdlib only, no runtime deps |
| 6 | 13 unit tests in `wire_test.go` | Round-trip per msg_type + error paths + validation |
| 7 | 5 JSON fixtures in `cmd/wire/testdata/` | One per msg_type, used as golden-file tests |

### 2.2 Out of scope (deferred to R3 or later)

- **Actual transport**: HTTP / gRPC / Unix domain socket / QUIC. R3.
- **TLS / mTLS handshake**: depends on `Authority::not_after` + trust-root chain from spec 021 / ADR-0031. R3.
- **Streaming / backpressure**: event-stream push (surface invalidation, lease state change). R3, separate spec.
- **Compression** (zstd / gzip): only after a measured payload-size problem. R3+.
- **Authentication beyond envelope signature**: WireAuth is a separate spec.
- **Multi-region relay / federation**: not on the R3 roadmap.
- **Rate limiting / QoS shaping**: spec 005 territory.
- **Replacing spec 023 subprocess model**: still subprocess + JSON for the
  `fabric-graph-cli` invocation. Wire transport is a different layer
  (inter-node vs. inter-process).

## 3. Public API

### 3.1 WireEnvelope

The outermost wire shape. Everything crosses the wire as one envelope.

```go
type WireEnvelope struct {
    EnvelopeID   string          `json:"envelope_id"`   // UUIDv4 (lowercase hex, no dashes)
    TenantID     string          `json:"tenant_id"`     // opaque, ASCII, [a-z0-9-]{1,64}
    MsgType      string          `json:"msg_type"`      // one of MsgType* constants
    Payload      json.RawMessage `json:"payload"`       // serialized WireMessage body
    Signature    string          `json:"signature,omitempty"` // base64 Ed25519 over envelope minus signature, or "" if unsigned
    SentAtUnixMs int64           `json:"sent_at_unix_ms"`     // RFC3339-Nano-compatible millis since epoch
}
```

**Invariants** (enforced by `WireCodec.Validate`):

- `EnvelopeID` parses as UUIDv4 lowercase hex (32 chars, no dashes).
- `TenantID` matches `^[a-z0-9-]{1,64}$`.
- `MsgType` is one of the 5 `MsgType*` constants.
- `Payload` is non-empty and parses as `json.RawMessage`.
- `SentAtUnixMs` is within ±300s of receiver clock (sanity bound; tight bound = R3).
- `Signature`, if non-empty, is base64 (standard alphabet, no `=` padding).

### 3.2 Message types

Five message types ship in this spec. Each is a Go struct with a stable
JSON wire shape and a `MsgType()` accessor.

| `MsgType` constant | JSON tag | Direction | Payload type | Purpose |
|---|---|---|---|---|
| `MsgTypeProbeRequest` | `probe.request` | client → server | `ProbeRequest` | Ask a node for its `CapabilityDescriptor` |
| `MsgTypeProbeResponse` | `probe.response` | server → client | `ProbeResponse` | Return a `CapabilityDescriptor` (reuses types.go types) |
| `MsgTypeReplanRequest` | `replan.request` | client → server | `WireReplanRequest` | Topology-driven replan (mirrors spec 023 wire shape, but envelope-wrapped) |
| `MsgTypeReplanResponse` | `replan.response` | server → client | `WireReplanResponse` | Outcome of replan (Replaced / NoReplacement / Error) |
| `MsgTypeSurfaceInvalidate` | `surface.invalidate` | server → client (push) | `SurfaceInvalidate` | Notify client that a surface lease was invalidated (per spec 024 SurfaceRegistry::notify_node_failure) |
| `MsgTypeHeartbeat` | `heartbeat` | bidirectional | `Heartbeat` | Liveness probe; payload is `{node_id, epoch}` |

**Note**: `LeaseStateChange` is intentionally **not** in this spec —
it overlaps with `SurfaceInvalidate` and will land in spec 027+ (R3
event-stream wedge).

### 3.3 Payload type shapes

```go
// ProbeRequest — client asks a node for its CapabilityDescriptor.
// Wire fields are minimal; everything the server needs is the request_id
// for correlation.
type ProbeRequest struct {
    RequestID string `json:"request_id"`  // UUIDv4 lowercase hex
}

// ProbeResponse — server returns a CapabilityDescriptor.
// Wraps the existing Descriptor type from cmd/checker/types.go.
type ProbeResponse struct {
    RequestID  string     `json:"request_id"`  // echoes ProbeRequest.RequestID
    Descriptor Descriptor `json:"descriptor"`  // mirrors fabric_capability::CapabilityDescriptor
}

// WireReplanRequest — mirrors spec 023 ReplanRequest shape, but
// envelope-wrapped. Failed nodes is now a stable wire field, not a CLI flag.
type WireReplanRequest struct {
    RequestID   string          `json:"request_id"`
    Topology    json.RawMessage `json:"topology"`     // fabric_graph::model::Topology, full
    Intent      json.RawMessage `json:"intent"`       // fabric_graph::model::Intent, full
    OldPlan     json.RawMessage `json:"old_plan"`     // fabric_graph::model::RoutePlan, full
    FailedNodes []string        `json:"failed_nodes"` // Vec<NodeId> as Vec<String>
}

// WireReplanResponse — mirrors spec 023 ReplanResponse.
type WireReplanResponse struct {
    RequestID string          `json:"request_id"`
    Outcome   string          `json:"outcome"`        // "replaced" | "no_replacement" | "error"
    NewPlan   json.RawMessage `json:"new_plan,omitempty"` // present iff outcome == "replaced"
    Reason    string          `json:"reason,omitempty"`
    Code      string          `json:"code,omitempty"`
    Message   string          `json:"message,omitempty"`
}

// SurfaceInvalidate — server pushes when a lease is invalidated.
// Per spec 024 SurfaceRegistry::notify_node_failure semantics.
type SurfaceInvalidate struct {
    SurfaceHandle string `json:"surface_handle"`  // SurfaceHandle::as_str()
    LeaseID       string `json:"lease_id"`        // unique lease ID
    Reason        string `json:"reason"`          // LeaseExitReason as string ("HostFailure" | "Revoked" | "Expired" | "Failed")
    FailedNode    string `json:"failed_node,omitempty"` // present iff Reason == "HostFailure"
    Epoch         uint64 `json:"epoch"`           // topology epoch at time of invalidation
}

// Heartbeat — liveness probe.
type Heartbeat struct {
    NodeID string `json:"node_id"`
    Epoch  uint64 `json:"epoch"`  // current TopologyEpoch
}
```

### 3.4 WireError taxonomy

Stable error codes. Wire transport never invents new ones; if you need a
new code, **add it to this list in a new spec**.

| Code | HTTP-equivalent (R3) | Meaning |
|---|---|---|
| `BadEnvelope` | 400 | Envelope itself failed validation (bad UUID, bad tenant_id, unknown msg_type, missing payload) |
| `UnsupportedMsgType` | 405 | Receiver doesn't know how to handle this MsgType (forward compatibility) |
| `AuthFailed` | 401 | Signature verification failed or signature required but missing |
| `UnknownTenant` | 404 | Receiver doesn't know this tenant_id (multi-tenant routing failure) |
| `BadPayload` | 400 | Envelope parsed but payload validation failed |
| `Io` | 503 | Receiver hit a transient I/O or downstream failure |
| `Bug` | 500 | Receiver internal error (always logged with envelope_id for correlation) |

Each `WireError` carries:

```go
type WireError struct {
    Code      string `json:"code"`       // one of the 7 codes above
    Message   string `json:"message"`    // human-readable, NOT machine-parseable
    EnvelopeID string `json:"envelope_id,omitempty"` // for correlation when error is reported back
}
```

The error is the **only** payload of an error response envelope. Error
envelopes have `MsgType` = `error` (a synthetic MsgType the codec
recognizes but is NOT one of the 5 wire msg types — see §3.5).

### 3.5 Codec contract

```go
type WireCodec struct{}

func (WireCodec) Marshal(env WireEnvelope) ([]byte, error)
func (WireCodec) Unmarshal(data []byte) (WireEnvelope, error)
func (WireCodec) Validate(env WireEnvelope) error
func (WireCodec) MarshalError(werr WireError, requestEnv WireEnvelope) (WireEnvelope, error)
```

**Determinism rules** (so two Fabric nodes can hash envelopes for
deduplication / audit):

1. Keys in the JSON output are sorted lexicographically (Go's
   `encoding/json` already does this for struct tags if no maps are
   involved; the codec rejects any payload containing a JSON object
   that itself contains maps — only structs + arrays + scalars).
2. No whitespace in the output (use `json.Marshal`, not `json.MarshalIndent`).
3. `SentAtUnixMs` is int64 milliseconds since Unix epoch.
4. No trailing newline.

**Forward compatibility**: when a receiver sees a `MsgType` it does not
recognize, it returns an `UnsupportedMsgType` error envelope. When the
**envelope** itself parses but a field is unknown to the receiver, the
field is ignored (per Go's `encoding/json` default). Unknown payload
fields inside a known `WireMessage` are ignored at the codec level; the
typed Go receiver decides whether to reject.

### 3.6 Client / Server interfaces (R3 only — interfaces shipped now)

The client and server **interfaces** ship in this spec so R3 implementers
have a target. The actual implementations are R3.

```go
// WireClient is the interface a Fabric node uses to send envelopes.
type WireClient interface {
    // Send a wire envelope to a target node. Returns the response envelope
    // (if the msg_type is request/response) or nil (if push-style).
    Send(ctx context.Context, target NodeAddress, env WireEnvelope) (*WireEnvelope, error)
    Close() error
}

// WireServer is the interface a Fabric node implements to receive envelopes.
type WireServer interface {
    // Handle is called once per incoming envelope. The implementation is
    // expected to return either a response envelope or nil (for push msgs).
    Handle(ctx context.Context, env WireEnvelope) (*WireEnvelope, error)
}

// NodeAddress identifies a target node. R3 implementation chooses
// transport (HTTP / gRPC / UDS); for the contract stub it's just an
// opaque string.
type NodeAddress struct {
    Scheme string `json:"scheme"` // "https" | "grpc" | "unix" | "memory" — R3 decision
    Host   string `json:"host"`
    Port   int    `json:"port,omitempty"`     // 0 for UDS
    Path   string `json:"path,omitempty"`     // UDS socket path or HTTP URL prefix
}
```

## 4. Wire format examples

### 4.1 Heartbeat (smallest envelope)

```json
{
  "envelope_id": "7d3a1b8c4e9f0123a5b6c7d8e9f01234",
  "tenant_id": "ops-phenotype-default",
  "msg_type": "heartbeat",
  "payload": {"node_id":"host-1","epoch":42},
  "sent_at_unix_ms": 1725900000000
}
```

### 4.2 ProbeRequest / ProbeResponse

ProbeRequest:
```json
{
  "envelope_id": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
  "tenant_id": "ops-phenotype-default",
  "msg_type": "probe.request",
  "payload": {"request_id":"11223344556677889900aabbccddeeff"},
  "sent_at_unix_ms": 1725900001000
}
```

ProbeResponse:
```json
{
  "envelope_id": "f1e2d3c4b5a69788796a5b4c3d2e1f00",
  "tenant_id": "ops-phenotype-default",
  "msg_type": "probe.response",
  "payload": {
    "request_id":"11223344556677889900aabbccddeeff",
    "descriptor":{ /* full CapabilityDescriptor from cmd/checker/types.go */ }
  },
  "sent_at_unix_ms": 1725900001050
}
```

### 4.3 SurfaceInvalidate push

```json
{
  "envelope_id": "99887766554433221100ffeeddccbbaa",
  "tenant_id": "ops-phenotype-default",
  "msg_type": "surface.invalidate",
  "payload": {
    "surface_handle":"surf-abc",
    "lease_id":"lease-xyz",
    "reason":"HostFailure",
    "failed_node":"host-1",
    "epoch":43
  },
  "sent_at_unix_ms": 1725900002000
}
```

### 4.4 WireError envelope (error response)

```json
{
  "envelope_id": "554433221100ffeeddccbbaa99887766",
  "tenant_id": "ops-phenotype-default",
  "msg_type": "error",
  "payload": {
    "code":"UnsupportedMsgType",
    "message":"receiver does not know msg_type 'foo.bar'",
    "envelope_id":"a1b2c3d4e5f60718293a4b5c6d7e8f90"
  },
  "sent_at_unix_ms": 1725900001500
}
```

## 5. Determinism guarantees

These rules make `WireCodec.Marshal(WireCodec.Unmarshal(x)) == x` a
soundness property, not a hopeful claim:

| Property | Enforcement |
|---|---|
| Sorted keys | `json.Marshal` (struct tags drive order; codec rejects map payloads) |
| No whitespace | `json.Marshal`, not `MarshalIndent` |
| Stable number format | Int64 / Float64 only; no `Number` type in payload |
| Stable timestamp | `int64` Unix milliseconds, never string |
| Stable UUID | Lowercase hex, no dashes, 32 chars |
| Stable tenant_id | Regex `^[a-z0-9-]{1,64}$` enforced by `Validate` |
| Stable base64 | Standard alphabet, no `=` padding (envelope `signature` field) |

## 6. Acceptance criteria

1. `cmd/wire/wire.go` compiles with `go build ./cmd/wire/` (stdlib only).
2. `go test ./cmd/wire/` passes — 13 tests:
   - `envelope_round_trip_for_each_msg_type` (5 subtests, one per msg_type)
   - `envelope_validate_rejects_bad_uuid`
   - `envelope_validate_rejects_bad_tenant_id`
   - `envelope_validate_rejects_unknown_msg_type`
   - `envelope_validate_rejects_empty_payload`
   - `envelope_validate_rejects_clock_skew`
   - `codec_rejects_payload_containing_json_map`
   - `marshal_error_wraps_request_envelope_id`
   - `unmarshal_rejects_malformed_envelope`
   - `deterministic_marshal_same_input_same_bytes` (golden file)
   - `heartbeat_is_smallest_envelope` (sanity, < 200 bytes)
3. `go test ./cmd/capprobe` — 0 regressions
4. `go test ./cmd/checker` — 0 regressions (the `replan.go` subprocess
   shape is independent of the wire envelope)
5. `cargo test --workspace` — 0 regressions
6. `check_manifest.py` — 0 mismatches (4 new files: `cmd/wire/wire.go`,
   `cmd/wire/wire_test.go`, `cmd/wire/go.mod`, `cmd/wire/testdata/heartbeat.json`)
7. `check_json_schemas.py` / `check_openapi.py` / `check_links.py` — all green
8. The 5 JSON fixtures in `cmd/wire/testdata/` are byte-identical to
   what `WireCodec.Marshal` produces for the matching Go fixture.

## 7. Cross-deps

- **Depends on**:
  - spec 019 (`SurfaceSpec`, `SurfaceLease`, `SurfaceHandle`, `LeaseExitReason`) — JSON shapes borrowed
  - spec 020 (`RebindOutcome`, `rebind_or_fail` semantics) — drives wire error mapping
  - spec 023 (`ReplanRequest` / `ReplanResponse` shape) — wire equivalent is `WireReplanRequest` / `WireReplanResponse`
  - spec 024 (`SurfaceRegistry::notify_node_failure`, `Invalidation {handle, reason}`) — drives `SurfaceInvalidate` payload shape
  - ADR-0030 (route failover model) — invalidation triggers
  - ADR-0029 (Go is canonical for wire transport)
  - ADR-0028 (read-authoritative-source-first; spec-only when stuck)
- **Blocks**:
  - spec 026 (tenant-scoped wire envelopes) — depends on this contract
  - spec 027 (event-stream wire shape) — depends on this contract
  - R3 actual transport (HTTP / gRPC / UDS) implementations
  - Fleet-ops integration of fabric-agents (R3)

## 8. Out-of-scope horizons

- **R3 — actual transport**: pick one of HTTP/2 + JSON, gRPC + protobuf,
  or Unix domain socket + JSON. The contract in this spec is
  transport-agnostic. Decision deferred until 1+ consumer is wired up
  and we can measure latency / deployment friction.
- **R3 — TLS / mTLS**: handshake driven by `Authority::not_after` +
  RevocationList per spec 021 / ADR-0031. The envelope signature field
  already accommodates per-message signing; mTLS is a transport-layer
  concern on top.
- **R3 — streaming push**: `WireServer.Handle` currently returns a
  single response. R3 may add `Stream(env) <-chan WireEnvelope` for
  long-lived push subscriptions (e.g., SurfaceInvalidate feed).
- **R3+ — compression**: only after measured payload-size problems.
  Heartbeat is 200 bytes; replan request can be 10–50 KB; surface
  invalidation is <300 bytes. None justify compression today.
- **R3+ — multi-region relay / federation**: separate spec. Adds
  envelope routing by `tenant_id` across regions.
- **Spec 027 — event-stream wire shape**: building block for the
  fleet-ops dashboard live feed (per `releases/2026-09-08-R1.md`
  §"Adoption plan").

## 9. Why this is a stub and not an implementation

Per ADR-0028: "ship spec + ADR + stub source untracked when stuck." This
turn applies the same rule to **forward motion**, not stuckness. The
`cmd/wire/` package is small (one Go file, ~300 LoC) and ships real code
with real tests, but it deliberately omits the network layer:

| Layer | This spec (R2 wedge #4) | R3 |
|---|---|---|
| Wire types | ✅ | ✅ |
| Codec (Marshal/Unmarshal/Validate) | ✅ | ✅ |
| 5 msg_type payloads | ✅ | ✅ |
| WireError taxonomy | ✅ | ✅ |
| WireClient / WireServer interfaces | ✅ (interfaces only) | ✅ |
| `memory://` WireClient impl | ❌ (R3 test helper) | ✅ |
| `http://` WireClient impl | ❌ | ✅ |
| `grpc://` WireClient impl | ❌ | ✅ |
| `unix://` WireClient impl | ❌ | ✅ |
| TLS / mTLS handshake | ❌ | ✅ |
| Stream / push subscription | ❌ | ✅ |

This matches the spec 023 wedge shape exactly: that spec shipped a thin
binary that calls one function; this spec ships a thin package that
defines one contract. The actual transport waits for a measured reason
to pick HTTP / gRPC / UDS.

## 10. Refs

- ADR-0028 (read-authoritative-source-first; spec-only when stuck)
- ADR-0029 (Go is canonical for wire transport)
- ADR-0030 (route failover model)
- ADR-0031 (trust-root for descriptor signatures; covers envelope signing)
- specs/019-surface-plane (PF-WP-015)
- specs/020-route-lease-integration (PF-WP-022)
- specs/023-fabric-graph-cli-replan (PF-WP-040 wedge #1)
- specs/024-surface-plane-runtime (PF-WP-030)
- `releases/2026-09-08-R1.md` §"What's intentionally not in R1" (wire transport R2)
- `meta/PHENOTYPE_ARCHITECTURE.md` §"Spec 024 surface-plane runtime authored" (recommended next wedge)