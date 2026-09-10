# Plan 025 — Wire transport contract (PF-WP-040, R2 wedge #4)

## Phase 0 — Read authoritative source first (ADR-0028)

Read end-to-end before opening `cmd/wire/wire.go`. The shapes we ship
must match what already exists — we are not inventing a new protocol,
we are codifying the bytes-on-wire for shapes that already exist in
the Go checker + Rust fabric-graph.

### 0.1 Rust fabric-graph types (the underlying types we wrap)

1. `crates/fabric-graph/src/surface.rs` — `SurfaceHandle`, `SurfaceLease`, `SurfaceSpec`, `LeaseExitReason` (already serialized by spec 019 JSON contract)
2. `crates/fabric-graph/src/lease_fsm.rs` — `LeaseState`, `can_transition`, `next_state`
3. `crates/fabric-graph/src/failover.rs` — `FailoverOutcome::{Replaced, NoReplacement}`, `FailoverError::{EmptyIntent, AllCandidatesFailed}` (spec 023 wire shape)
4. `crates/fabric-graph/src/model.rs` — `Topology`, `Intent`, `RoutePlan`, `NodeId` Serialize/Deserialize derives
5. `crates/fabric-capability/src/locality.rs` — `LocalityTier` (spec 023 verified)
6. `crates/fabric-capability/src/trust_root.rs` — `Authority::not_after`, signing shape (for envelope signature field R3)

### 0.2 Existing Go wire shapes (the patterns we follow)

7. `cmd/checker/types.go` — `Descriptor`, `Capabilities`, `Finding`, `Report`, `Severity`, `Decision` (the JSON shape we wrap in `ProbeResponse`)
8. `cmd/checker/replan.go` — `replanRequest`, `replanResponse`, `reportFromReplan` (the pattern we mirror in `WireReplanRequest`/`WireReplanResponse`)
9. `cmd/checker/main.go` — flag parsing, exit codes (the `cmd/checker` UX we do NOT replicate; wire transport is library code)
10. `cmd/capprobe/main.go` — minimal Go binary reference (style reference for the package layout)
11. `cmd/checker/replan_test.go` — Go test patterns (table-driven, fake binary subprocess)
12. `architecture/openapi.yaml` — the existing API surface (we don't extend it; wire transport is a separate concern)

### 0.3 Documentation baselines

13. `specs/023-fabric-graph-cli-replan/spec.md` — the **directly prior** wire contract (subprocess + JSON). Our spec wraps those types in an envelope.
14. `specs/024-surface-plane-runtime/spec.md` — the **adjacent** contract (SurfaceRegistry). Our `SurfaceInvalidate` payload mirrors its `Invalidation` struct.
15. `releases/2026-09-08-R1.md` §"What's intentionally not in R1" + §"Roadmap to R2" — the explicit PF-WP-040 = R2 wedge positioning.
16. `meta/PHENOTYPE_ARCHITECTURE.md` §"Recommended next wedge" — explicit guidance that the wire-transport wedge "can ship a Go-only stub (similar to `fabric-graph-cli replan`)"

Phase 0 reads above complete before opening `cmd/wire/wire.go`.

## Phase 1 — Scaffold `cmd/wire/`

### 1.1 Directory layout

```
cmd/wire/
├── go.mod                 # module github.com/phenotype/fabric/cmd/wire (go 1.21, no deps)
├── wire.go                # WireEnvelope, WireCodec, 5 WireMessage types, WireError, WireClient/WireServer interfaces
├── wire_test.go           # 13 unit tests (round-trip, validation, determinism, golden-file)
└── testdata/
    ├── heartbeat.json              # golden-file: smallest envelope
    ├── probe_request.json          # golden-file: client → server probe
    ├── probe_response.json         # golden-file: server → client probe
    ├── replan_request.json         # golden-file: client → server replan
    ├── replan_response.json        # golden-file: server → client replan (Replaced)
    ├── surface_invalidate.json     # golden-file: server → client push
    ├── error_response.json         # golden-file: error envelope
    └── envelope_unsupported.json   # negative fixture: unknown msg_type (validate rejects)
```

8 files total (matches meta.json acceptance_signals count of "4 new files" + 4 fixtures).

### 1.2 Module

`cmd/wire/go.mod`:

```go
module github.com/phenotype/fabric/cmd/wire

go 1.21
```

**No dependencies.** Stdlib only (`encoding/json`, `errors`, `fmt`,
`regexp`, `sort`, `time`, `context`). This is the contract — anyone
who imports `cmd/wire` is buying into a stdlib-only contract.

## Phase 2 — Author `cmd/wire/wire.go`

### 2.1 WireEnvelope + Validate

```go
type WireEnvelope struct {
    EnvelopeID   string          `json:"envelope_id"`
    TenantID     string          `json:"tenant_id"`
    MsgType      string          `json:"msg_type"`
    Payload      json.RawMessage `json:"payload"`
    Signature    string          `json:"signature,omitempty"`
    SentAtUnixMs int64           `json:"sent_at_unix_ms"`
}
```

`Validate(env WireEnvelope) error` checks the 6 invariants from spec §3.1.

### 2.2 5 WireMessage types + their typed Payload accessors

Each message type has:

- A constant `MsgType*` string (e.g., `MsgTypeProbeRequest = "probe.request"`)
- A Go struct `ProbeRequest`, `ProbeResponse`, `WireReplanRequest`, `WireReplanResponse`, `SurfaceInvalidate`, `Heartbeat`
- A constructor that takes the struct + envelope fields, returns a `WireEnvelope`

**Note**: There are 5 `MsgType*` constants but 6 payload structs because
`MsgTypeHeartbeat` uses one struct and the other 4 msg_types each have
request/response structs. Wait — actually 5 msg_types × ~1 payload each:

| MsgType | Payload struct |
|---|---|
| `probe.request` | `ProbeRequest` |
| `probe.response` | `ProbeResponse` |
| `replan.request` | `WireReplanRequest` |
| `replan.response` | `WireReplanResponse` |
| `surface.invalidate` | `SurfaceInvalidate` |
| `heartbeat` | `Heartbeat` |

That's 6 payload structs and 5 MsgType constants. Plus the synthetic
`error` MsgType used for error envelopes (not one of the 5 wire types,
per spec §3.4).

### 2.3 WireCodec

```go
type WireCodec struct{}

func (WireCodec) Marshal(env WireEnvelope) ([]byte, error) {
    if err := (WireCodec{}).Validate(env); err != nil {
        return nil, fmt.Errorf("wire: marshal invalid envelope: %w", err)
    }
    return json.Marshal(env) // encoding/json sorts struct tags alphabetically
}

func (WireCodec) Unmarshal(data []byte) (WireEnvelope, error) {
    var env WireEnvelope
    dec := json.NewDecoder(bytes.NewReader(data))
    dec.DisallowUnknownFields() // envelope shape is fixed
    if err := dec.Decode(&env); err != nil {
        return WireEnvelope{}, fmt.Errorf("wire: unmarshal envelope: %w", err)
    }
    if err := (WireCodec{}).Validate(env); err != nil {
        return WireEnvelope{}, fmt.Errorf("wire: unmarshal invalid envelope: %w", err)
    }
    // Reject payloads containing JSON objects (maps) — only structs/arrays/scalars
    if err := rejectJSONMaps(env.Payload); err != nil {
        return WireEnvelope{}, fmt.Errorf("wire: payload contains map: %w", err)
    }
    return env, nil
}
```

`rejectJSONMaps` walks the raw JSON payload and returns an error if it
finds a JSON object that wasn't produced by a Go struct (i.e., that
contains a key like `"_map_marker"` injected by the codec at Marshal
time). **Simpler alternative**: the codec only accepts `json.RawMessage`
that, when re-parsed, contains no `map[string]interface{}` after
`json.Unmarshal` into `interface{}`. We use the simpler alternative.

### 2.4 WireError + MarshalError

```go
type WireError struct {
    Code       string `json:"code"`
    Message    string `json:"message"`
    EnvelopeID string `json:"envelope_id,omitempty"`
}

func (WireCodec) MarshalError(werr WireError, requestEnv WireEnvelope) (WireEnvelope, error) {
    payload, err := json.Marshal(werr)
    if err != nil {
        return WireEnvelope{}, fmt.Errorf("wire: marshal error: %w", err)
    }
    return WireEnvelope{
        EnvelopeID:   newUUIDv4(),     // local helper, deterministic in tests
        TenantID:     requestEnv.TenantID,
        MsgType:      MsgTypeError,    // synthetic, not one of the 5
        Payload:      payload,
        SentAtUnixMs: time.Now().UnixMilli(),
    }, nil
}
```

### 2.5 WireClient / WireServer interfaces + NodeAddress

Per spec §3.6. Interfaces only — no implementations in this wedge.
Document that `cmd/wire` ships `MemoryWireClient` / `MemoryWireServer`
**R3 helpers** for in-process testing.

## Phase 3 — Author `cmd/wire/wire_test.go`

13 tests, table-driven where it makes sense. Use `t.TempDir()` for any
filesystem needs (none — wire transport is pure Go).

| # | Test | Purpose |
|---|---|---|
| 1 | `TestRoundTripHeartbeat` | Envelope round-trip preserves all fields |
| 2 | `TestRoundTripProbeRequest` | Same, for ProbeRequest |
| 3 | `TestRoundTripProbeResponse` | Same, for ProbeResponse |
| 4 | `TestRoundTripWireReplanRequest` | Same, for WireReplanRequest |
| 5 | `TestRoundTripWireReplanResponse` | Same, for WireReplanResponse |
| 6 | `TestRoundTripSurfaceInvalidate` | Same, for SurfaceInvalidate |
| 7 | `TestValidateRejectsBadUUID` | `EnvelopeID` of "not-a-uuid" → error |
| 8 | `TestValidateRejectsBadTenantID` | `TenantID` of "UPPER_AND_DOTS..." → error |
| 9 | `TestValidateRejectsUnknownMsgType` | `MsgType` of "foo.bar" → error |
| 10 | `TestValidateRejectsEmptyPayload` | `Payload` of `null` → error |
| 11 | `TestValidateRejectsClockSkew` | `SentAtUnixMs` of `time.Now().UnixMilli() + 10*60*1000` → error |
| 12 | `TestDeterministicMarshal` | Two marshals of identical input → identical bytes (golden-file style) |
| 13 | `TestMarshalErrorWrapsRequestEnvelopeID` | Error envelope echoes `requestEnv.EnvelopeID` in payload |

Plus 5 **golden-file** checks: each `testdata/*.json` matches the
output of `WireCodec.Marshal(envelopeFromFixture(t, "..."))`.

## Phase 4 — Author 8 fixtures in `cmd/wire/testdata/`

Hand-written JSON files matching the wire shapes from spec §4. Each
fixture is loaded by the test and verified to round-trip exactly.

| File | Size (approx) | Purpose |
|---|---|---|
| `heartbeat.json` | 175 B | Smallest envelope, sanity baseline |
| `probe_request.json` | 220 B | Client → server probe |
| `probe_response.json` | 1.2 KB | Server → client probe (with CapabilityDescriptor) |
| `replan_request.json` | 600 B | Client → server replan |
| `replan_response.json` | 450 B | Server → client replan (Replaced outcome) |
| `surface_invalidate.json` | 250 B | Server → client push |
| `error_response.json` | 230 B | Error envelope (BadEnvelope) |
| `envelope_unsupported.json` | 240 B | Negative fixture: msg_type = "foo.bar" (codec rejects on Unmarshal) |

## Phase 5 — Verify

1. `go build ./cmd/wire/` — must succeed, no warnings
2. `go test ./cmd/wire/` — all 13 tests pass
3. `go test ./cmd/capprobe/` — 0 regressions
4. `go test ./cmd/checker/` — 0 regressions (the spec 023 subprocess model is unchanged)
5. `cargo test --workspace` — 0 regressions (no Rust changes this turn)
6. `python3 program/scripts/check_manifest.py` — 8 new files must appear in MANIFEST
7. `python3 program/scripts/check_json_schemas.py` — green
8. `python3 program/scripts/check_openapi.py` — green
9. `python3 program/scripts/check_links.py` — green
10. Operator smoke:
    ```bash
    cd cmd/wire
    go test -v ./...
    diff <(jq -S . testdata/heartbeat.json) <(go run . encode testdata/heartbeat.json | jq -S .)
    ```
    (encode CLI is R3; for now golden files are validated by tests)

## Phase 6 — Commit + docs

1. Regenerate `MANIFEST.sha256` (8 new entries: `cmd/wire/{wire.go, wire_test.go, go.mod}` + 5 fixtures in `testdata/`)
2. Update `specs/INDEX.md` (+1 row for spec 025)
3. Append `WORKLOG.md` 2026-09-09 entry
4. Append `meta/PHENOTYPE_ARCHITECTURE.md` spec 025 addendum
5. Commit (1 feat + 1 docs, or 1 combined commit if reviewer prefers)
6. Update ROADMAP.md R2 wedge #4 marker if such a section exists (it does not yet — defer until R2 closes)

## Risk register

- **R1**: `go test ./cmd/wire/` hits a determinism bug (e.g., map ordering
  creeps in via `map[string]NodeAddress` somewhere). **Mitigation**:
  the codec rejects any payload containing a JSON map (validated by
  `TestDeterministicMarshal`). If it fails, the fix is to convert any
  map field to a struct or a sorted slice.

- **R2**: `cmd/checker` or `cmd/capprobe` accidentally imports
  `cmd/wire` and pulls it into their build. **Mitigation**: `cmd/wire`
  is a separate Go module (`go.mod` per-directory), so an explicit
  `replace` or `require` is needed. We don't add it from the checker
  side this turn.

- **R3**: Operator wants actual transport NOW, not in R3. **Mitigation**:
  spec §9 explains the wedge shape explicitly; the contract is what
  unblocks R3 implementation, so shipping it now is forward motion.

- **R4**: Spec 023 subprocess model needs to be replaced by wire
  transport (i.e., `fabric-graph-cli` should be invoked over the wire
  envelope instead of stdin/stdout). **Mitigation**: explicitly out of
  scope for this spec (spec §2.2). If a future spec wants to do this
  replacement, it would wrap the existing `fabric-graph-cli` invocation
  in a `WireReplanRequest` envelope — same wire shape, different
  transport. That's a follow-up spec, not this one.

## Definition of done

- All 6 phases above complete.
- HEAD on phenotype-fabric is one or two commits ahead of `02c0ec0` (the
  spec 024 worklog).
- Both repos clean.
- All 4 spec checks green.
- 13+ Go tests pass under `cmd/wire/`.
- 0 regressions in `cmd/checker` (26 PASS → 26 PASS), `cmd/capprobe` (6 PASS → 6 PASS),
  `cargo test --workspace` (177 pass → 177 pass).

## Phase 0 (ADR-0028) read order

`cmd/checker/replan.go` → `cmd/checker/types.go` → `cmd/checker/replan_test.go`
→ `cmd/capprobe/main.go` (style) → `specs/023-fabric-graph-cli-replan/spec.md` (subprocess contract)
→ `specs/024-surface-plane-runtime/spec.md` (SurfaceRegistry semantics)
→ `architecture/openapi.yaml` (existing API surface, untouched)
→ `crates/fabric-graph/src/surface.rs` → `crates/fabric-graph/src/failover.rs`
→ `crates/fabric-capability/src/trust_root.rs` (signature shape for envelope.signature)

(Go-side shapes first because that's what we're wrapping; Rust-side
types second as cross-reference. After Phase 0, the only open question
is whether `cmd/wire` should be a separate Go module or share
`cmd/checker`'s — separate wins on the dep-light rule.)