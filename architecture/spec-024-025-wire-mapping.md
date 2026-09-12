# Spec 024 ↔ 025 Wire Mapping

**Status:** R2 Implementation Reference
**Date:** 2026-09-12
**Specs:** PF-WP-024 (surface-plane runtime) → PF-WP-025 (wire transport)

## Overview

This document defines how the Rust `surface_runtime` (spec 024) produces
`Invalidation` events that are serialized to the spec 025 `SurfaceInvalidate`
wire format for transmission over the Go checker's wire layer.

The mapping is tested in:
- `crates/fabric-graph/src/surface_runtime.rs` (Rust, 11 tests)
- `cmd/wire/wire.go` (Go wire constructors, 18 tests)
- `cmd/checker/integration_test.go` (Go E2E, 4 tests)

## Invalidation → SurfaceInvalidate Mapping

| `LeaseExitReason` (Rust) | `SurfaceInvalidate.reason` (Wire) | `failed_node` field | Notes |
|---|---|---|---|
| `HostFailure { node_id }` | `"HostFailure"` | Present: `node_id` | Only variant with `failed_node` |
| `OperatorRevoked` | `"Revoked"` | Absent (omitted) | Manual operator action |
| `Expired` | `"Expired"` | Absent (omitted) | Lease TTL exceeded |
| `WorkloadReported { code, message }` | `"Failed"` | Absent (omitted) | App-level failure |
| `NormalCompletion` | `"NormalCompletion"` | Absent (omitted) | Clean exit |
| `EpochDrift { previous_epoch, new_epoch }` | `"EpochDrift"` | Absent (omitted) | Topology epoch changed |

## Wire Shape (spec 025)

```json
{
  "surface_handle": "550e8400-e29b-41d4-a716-446655440000",
  "lease_id": "550e8400-e29b-41d4-a716-446655440001",
  "reason": "HostFailure",
  "failed_node": "node-alpha",
  "epoch": 3
}
```

### Fields

| Field | Type | Source | Notes |
|---|---|---|---|
| `surface_handle` | UUID string | `SurfaceHandle::new()` | Opaque handle for the surface plane |
| `lease_id` | UUID string | `RouteBinding.binding_id` | Identifies the failed lease |
| `reason` | string enum | `LeaseExitReason` mapping | One of 6 values |
| `failed_node` | string (optional) | `HostFailure.node_id` | Only present for `HostFailure` |
| `epoch` | uint64 | `Invalidation.epoch` | Topology epoch when invalidation occurred |

## Go Wire Constructors (cmd/wire/wire.go)

```go
// Type-safe constructor for SurfaceInvalidate
env, err := NewSurfaceInvalidateEnvelope(
    handle, leaseID, "HostFailure", failedNode, epoch)

// Available reason constants:
ReasonHostFailure       = "HostFailure"
ReasonRevoked           = "Revoked"
ReasonExpired           = "Expired"
ReasonFailed            = "Failed"
ReasonNormalCompletion  = "NormalCompletion"
ReasonEpochDrift        = "EpochDrift"

// Mapping from Rust → Go wire
wireReason := InvalidationReasonToWire("HostFailure") → "HostFailure"
wireReason := InvalidationReasonToWire("OperatorRevoked") → "Revoked"
```

## Checker Integration Pipeline

```
┌──────────────┐     ┌───────────────┐     ┌──────────────────┐
│ surface_     │     │ to_wire_json()│     │ Go wire layer    │
│ runtime      │────▶│ spec 025      │────▶│ (cmd/wire)       │
│ (Rust)       │     │ format        │     │                  │
└──────────────┘     └───────────────┘     └────────┬─────────┘
                                                    │
                                                    ▼
                                          ┌──────────────────┐
                                          │ cmd/checker      │
                                          │ -trust-root      │
                                          │ -replan-binary   │
                                          │ (Report output)  │
                                          └──────────────────┘
```

### Data flow

1. **Node failure detected** → `SurfaceRuntime::notify_node_failure(node_id)`
2. **Invalidations created** → `Invalidation { handle, binding_id, reason, epoch }`
3. **Wire serialization** → `invalidation.to_wire_json()` → spec 025 `SurfaceInvalidate`
4. **Go envelope** → `NewSurfaceInvalidateEnvelope()` wraps for transport
5. **Checker processes** → trust verification + resource check → `Report{Decision, Findings}`
6. **Replan** (optional) → `buildReplanRequest()` + `invokeReplan()` → new plan

## Trust Root Verification (Go)

The Go checker loads a root Authority JSON and verifies descriptor
signatures before running placement checks:

```go
// Load trust root
root, err := loadTrustRoot("/path/to/trust-root.json")

// Verify descriptor signature
if err := verifyDescriptorSignature(descriptor, root); err != nil {
    // → DecisionReject, ReasonUntrusted
}
```

### Trust root JSON shape

```json
{
  "key_id": "<base64 public key>",
  "name": "root-authority",
  "issued_at": "2026-09-01T00:00:00Z",
  "not_after": "2099-12-31T23:59:59Z",
  "verification_key": {
    "bytes": "<32-byte Ed25519 public key, base64>",
    "key_id": "<same as key_id>"
  }
}
```

### Canonical bytes

Before signature verification, the descriptor is canonicalized:
1. JSON-serialize the descriptor
2. Strip the `"signatures"` field
3. Emit compact bytes (no whitespace)

This matches Rust `canonical_bytes()` (ADR-0031).

## Test Coverage

| Layer | Test Count | Focus |
|---|---|---|
| Rust surface_runtime | 11 | Wire reason mapping, invalidation creation, topology binding |
| Go wire constructors | 18 | Envelope construction, round-trip, reason mapping |
| Go checker (trust) | 7 | Signature verification, trust root loading, expiry |
| Go checker (E2E) | 4 | Full pipeline: trust → check → replan → report |
| Go checker (unit) | 16 | Resource checks, replan, blacklist |
| **Total** | **56** | |

## File Locations

| Component | Path | Language |
|---|---|---|
| Surface runtime | `crates/fabric-graph/src/surface_runtime.rs` | Rust |
| Wire constructors | `cmd/wire/wire.go` | Go |
| Wire tests | `cmd/wire/wire_test.go` | Go |
| Checker (main) | `cmd/checker/main.go` | Go |
| Trust verification | `cmd/checker/trust.go` | Go |
| Trust tests | `cmd/checker/trust_test.go` | Go |
| Replan integration | `cmd/checker/replan.go` | Go |
| E2E integration | `cmd/checker/integration_test.go` | Go |
