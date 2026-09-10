# Tasks 025 — Wire transport contract (PF-WP-040, R2 wedge #4)

## Definition of done

- [ ] `specs/025-wire-transport/{meta.json, spec.md, plan.md, tasks.md}` authored
- [ ] `cmd/wire/wire.go` — WireEnvelope + WireCodec + 5 WireMessage types + WireError + WireClient/WireServer interfaces + NodeAddress (stdlib only)
- [ ] `cmd/wire/wire_test.go` — 13 unit tests (round-trip per msg_type, validation paths, determinism, error wrapping)
- [ ] `cmd/wire/go.mod` — separate module, `github.com/phenotype/fabric/cmd/wire`, `go 1.21`, zero deps
- [ ] `cmd/wire/testdata/{heartbeat,probe_request,probe_response,replan_request,replan_response,surface_invalidate,error_response,envelope_unsupported}.json` — 8 fixtures (5 wire shapes + 1 negative + 1 error + 1 smallest)
- [ ] `go test ./cmd/wire/` — all 13 tests pass
- [ ] `go test ./cmd/capprobe` — 0 regressions (6 PASS unchanged)
- [ ] `go test ./cmd/checker` — 0 regressions (26 PASS unchanged)
- [ ] `cargo test --workspace` — 0 regressions (177 pass unchanged)
- [ ] `python3 program/scripts/check_manifest.py` — 8 new files match
- [ ] `python3 program/scripts/check_json_schemas.py` / `check_openapi.py` / `check_links.py` — all green
- [ ] `MANIFEST.sha256` regenerated for the 8 new files
- [ ] `specs/INDEX.md` updated with row 025
- [ ] `WORKLOG.md` 2026-09-09 entry appended
- [ ] `meta/PHENOTYPE_ARCHITECTURE.md` spec 025 addendum appended
- [ ] Single commit per ADR-0028 (preferred: split into feat + docs if reviewer asks)

## Sub-tasks

### S025-01 — Phase 0 read (ADR-0028)
- [ ] Read `cmd/checker/replan.go` end-to-end
- [ ] Read `cmd/checker/types.go` end-to-end
- [ ] Read `cmd/checker/replan_test.go` end-to-end
- [ ] Read `cmd/checker/main.go` end-to-end (style reference)
- [ ] Read `cmd/capprobe/main.go` end-to-end (style reference)
- [ ] Read `specs/023-fabric-graph-cli-replan/spec.md` (subprocess wire contract we wrap)
- [ ] Read `specs/024-surface-plane-runtime/spec.md` (SurfaceRegistry semantics we mirror)
- [ ] Read `architecture/openapi.yaml` (existing API surface, untouched)
- [ ] Read `crates/fabric-graph/src/surface.rs` (`SurfaceHandle`, `LeaseExitReason`)
- [ ] Read `crates/fabric-graph/src/failover.rs` (`FailoverOutcome`, `FailoverError`)
- [ ] Read `crates/fabric-capability/src/trust_root.rs` (`Authority::not_after`, signing shape)
- [ ] Document verified Go types in `cmd/checker/types.go` that `WireProbeResponse.Descriptor` reuses
- [ ] Document the 5 wire msg types and their payload structs in `cmd/wire/wire.go` header comment

### S025-02 — Spec 025 artifacts
- [ ] `specs/025-wire-transport/meta.json` — 54 LoC, fields per spec 024 meta shape
- [ ] `specs/025-wire-transport/spec.md` — wire transport contract (§1–§10, ~430 LoC)
- [ ] `specs/025-wire-transport/plan.md` — phased build plan (Phase 0–6 + risk register)
- [ ] `specs/025-wire-transport/tasks.md` — this file
- [ ] `specs/INDEX.md` row 025 inserted

### S025-03 — `cmd/wire/` scaffold
- [ ] `mkdir cmd/wire/testdata`
- [ ] Author `cmd/wire/go.mod` — `module github.com/phenotype/fabric/cmd/wire`, `go 1.21`, zero deps
- [ ] Author `cmd/wire/wire.go` header comment explaining the contract

### S025-04 — WireEnvelope + Validate
- [ ] `WireEnvelope` struct (6 fields per spec §3.1)
- [ ] `Validate(env WireEnvelope) error` checks 6 invariants
- [ ] UUID regex (`^[0-9a-f]{32}$`)
- [ ] Tenant regex (`^[a-z0-9-]{1,64}$`)
- [ ] `MsgType` is in the 5-constant set
- [ ] `Payload` is non-empty JSON
- [ ] `SentAtUnixMs` is within ±300s of `time.Now().UnixMilli()`

### S025-05 — 5 WireMessage types
- [ ] `MsgTypeProbeRequest = "probe.request"` constant
- [ ] `MsgTypeProbeResponse = "probe.response"` constant
- [ ] `MsgTypeReplanRequest = "replan.request"` constant
- [ ] `MsgTypeReplanResponse = "replan.response"` constant
- [ ] `MsgTypeSurfaceInvalidate = "surface.invalidate"` constant
- [ ] `MsgTypeHeartbeat = "heartbeat"` constant
- [ ] `MsgTypeError = "error"` synthetic constant (for error envelopes)
- [ ] `ProbeRequest`, `ProbeResponse`, `WireReplanRequest`, `WireReplanResponse`, `SurfaceInvalidate`, `Heartbeat` Go structs
- [ ] Constructor for each (`NewXxxRequest(...) WireEnvelope`)

### S025-06 — WireCodec
- [ ] `WireCodec` empty struct
- [ ] `Marshal(env WireEnvelope) ([]byte, error)` — calls Validate, then `json.Marshal`
- [ ] `Unmarshal(data []byte) (WireEnvelope, error)` — uses `json.NewDecoder` + `DisallowUnknownFields` + Validate
- [ ] `Validate(env WireEnvelope) error` — exported wrapper for the private validate logic
- [ ] `MarshalError(werr WireError, requestEnv WireEnvelope) (WireEnvelope, error)` — wraps in error envelope

### S025-07 — WireError
- [ ] `WireError` struct (code, message, envelope_id)
- [ ] 7 stable error code constants: `WireErrorBadEnvelope`, `WireErrorUnsupportedMsgType`, `WireErrorAuthFailed`, `WireErrorUnknownTenant`, `WireErrorBadPayload`, `WireErrorIo`, `WireErrorBug`
- [ ] `Validate() error` checks Code is one of the 7

### S025-08 — WireClient / WireServer interfaces + NodeAddress
- [ ] `WireClient` interface (Send + Close)
- [ ] `WireServer` interface (Handle)
- [ ] `NodeAddress` struct (Scheme, Host, Port, Path)
- [ ] All in `cmd/wire/wire.go` — interfaces only, no impls

### S025-09 — UUIDv4 helper
- [ ] `newUUIDv4() string` — returns lowercase hex, 32 chars, no dashes
- [ ] Implementation: `crypto/rand` reads 16 bytes, hex-encoded
- [ ] Used by `MarshalError` and constructor helpers

### S025-10 — Unit tests (13)
- [ ] `TestRoundTripHeartbeat`
- [ ] `TestRoundTripProbeRequest`
- [ ] `TestRoundTripProbeResponse`
- [ ] `TestRoundTripWireReplanRequest`
- [ ] `TestRoundTripWireReplanResponse`
- [ ] `TestRoundTripSurfaceInvalidate`
- [ ] `TestValidateRejectsBadUUID`
- [ ] `TestValidateRejectsBadTenantID`
- [ ] `TestValidateRejectsUnknownMsgType`
- [ ] `TestValidateRejectsEmptyPayload`
- [ ] `TestValidateRejectsClockSkew`
- [ ] `TestDeterministicMarshal`
- [ ] `TestMarshalErrorWrapsRequestEnvelopeID`

### S025-11 — Golden-file fixtures (8)
- [ ] `cmd/wire/testdata/heartbeat.json` — smallest envelope
- [ ] `cmd/wire/testdata/probe_request.json` — client → server probe
- [ ] `cmd/wire/testdata/probe_response.json` — server → client probe (with CapabilityDescriptor subset)
- [ ] `cmd/wire/testdata/replan_request.json` — client → server replan
- [ ] `cmd/wire/testdata/replan_response.json` — server → client replan (Replaced)
- [ ] `cmd/wire/testdata/surface_invalidate.json` — server → client push
- [ ] `cmd/wire/testdata/error_response.json` — error envelope (BadEnvelope)
- [ ] `cmd/wire/testdata/envelope_unsupported.json` — negative fixture, msg_type = "foo.bar"

### S025-12 — Verification
- [ ] `go build ./cmd/wire/` — clean
- [ ] `go test -v ./cmd/wire/` — all 13 pass
- [ ] `go test ./cmd/capprobe` — 6 PASS unchanged
- [ ] `go test ./cmd/checker` — 26 PASS unchanged
- [ ] `cargo test --workspace` — 177 pass unchanged
- [ ] `python3 program/scripts/check_manifest.py` — 8 new files
- [ ] `python3 program/scripts/check_json_schemas.py` — green
- [ ] `python3 program/scripts/check_openapi.py` — green
- [ ] `python3 program/scripts/check_links.py` — green

### S025-13 — Docs + commit
- [ ] `MANIFEST.sha256` regenerated for 8 new files
- [ ] `specs/INDEX.md` row 025 inserted
- [ ] `WORKLOG.md` 2026-09-09 entry appended
- [ ] `meta/PHENOTYPE_ARCHITECTURE.md` spec 025 addendum appended
- [ ] Single commit message: `feat(wire): PF-WP-040 wire transport contract (Go-only stub, R2 wedge #4)`

## Cross-dependencies

- **Uses**:
  - `cmd/checker/types.go::Descriptor` — borrowed as the `Descriptor` field of `WireProbeResponse` payload (single import, no coupling)
  - `specs/019-surface-plane::SurfaceHandle` — wire shape borrowed for `SurfaceInvalidate.surface_handle` string
  - `specs/019-surface-plane::LeaseExitReason` — wire shape borrowed for `SurfaceInvalidate.reason` string
  - `specs/023-fabric-graph-cli-replan::ReplanRequest/ReplanResponse` — mirrored in `WireReplanRequest/WireReplanResponse`
  - `specs/024-surface-plane-runtime::Invalidation {handle, reason}` — drives `SurfaceInvalidate` payload shape
- **Does NOT use** (out of scope):
  - `crates/fabric-graph::failover::replan` directly — that's spec 023 territory (subprocess + JSON)
  - `crates/fabric-capability::trust_root::Authority` — R3 (envelope signature verification)
  - `cmd/checker::replan.go` — we do NOT replace spec 023 subprocess model
- **Unblocks**:
  - R3 actual transport (HTTP / gRPC / UDS) implementations
  - spec 026 (tenant-scoped wire envelopes)
  - spec 027 (event-stream wire shape)
  - Fleet-ops integration of fabric-agents (R3)
- **Blocks**: nothing — this is additive

## Out of scope

- **No actual transport** (HTTP / gRPC / UDS / QUIC). R3.
- **No TLS / mTLS handshake**. R3 (depends on trust_root per spec 021).
- **No streaming / backpressure**. R3 (event-stream wedge).
- **No compression**. R3+ (only after measured payload-size problem).
- **No replacement of spec 023 subprocess model**. The `fabric-graph-cli`
  invocation continues to use stdin/stdout JSON (spec 023 contract
  unchanged).
- **No authentication beyond envelope signature**. R3 WireAuth spec.
- **No multi-region relay / federation**. R3+.
- **No rate limiting / QoS shaping**. spec 005 territory.
- **No LeaseStateChange msg type**. Overlaps with `SurfaceInvalidate`;
  lands in spec 027+ (R3 event-stream wedge).

## Verification commands (copy-pasteable)

```bash
# Build the wire package
go build ./cmd/wire/

# Run wire tests
go test -v ./cmd/wire/

# Verify no regressions in other Go packages
go test ./cmd/capprobe
go test ./cmd/checker

# Verify no regressions in Rust workspace
cargo test --workspace

# Spec checks
python3 program/scripts/check_manifest.py
python3 program/scripts/check_json_schemas.py
python3 program/scripts/check_openapi.py
python3 program/scripts/check_links.py
```

## Definition of done — checklist summary

| Check | Status |
|---|---|
| spec 025 meta/spec/plan/tasks authored | ☐ |
| cmd/wire/{wire.go, wire_test.go, go.mod} committed | ☐ |
| cmd/wire/testdata/*.json (8 files) committed | ☐ |
| go test ./cmd/wire/ — 13/13 pass | ☐ |
| go test ./cmd/capprobe — 6/6 unchanged | ☐ |
| go test ./cmd/checker — 26/26 unchanged | ☐ |
| cargo test --workspace — 177/177 unchanged | ☐ |
| check_manifest.py — 8 new files | ☐ |
| check_json_schemas / openapi / links — 3/3 green | ☐ |
| MANIFEST.sha256 regenerated | ☐ |
| specs/INDEX.md row 025 inserted | ☐ |
| WORKLOG.md 2026-09-09 entry appended | ☐ |
| meta ARCHITECTURE spec 025 addendum appended | ☐ |
| Single commit per ADR-0028 | ☐ |
| HEAD ahead of 02c0ec0 by 1-2 commits | ☐ |
| Both repos clean (phenotype-fabric + meta) | ☐ |