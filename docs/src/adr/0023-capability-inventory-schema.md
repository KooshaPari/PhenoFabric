# ADR-0023: Capability Inventory — Signed Descriptor Schema and FFI

**Status:** Accepted
**Date:** 2026-09-01
**Deciders:** Session 01a04c3e continuation (2026-09-01)
**Supersedes:** None
**Traceability:** PF-FR-007, PF-FR-008, PF-FR-009, INT-P012, INT-P013, INT-P014

---

## Context

Fabric needs a canonical, signed, machine-readable representation of the physical
capabilities of a node — the "capability descriptor." This descriptor is the
fundamental unit of topology, route planning, and placement decisions.

Before this ADR, the architecture described capability discovery in prose. This ADR
locks in: (1) the descriptor schema, (2) the canonical-bytes algorithm, (3) the
signing model, and (4) the FFI surface.

Requirements:
- **Stable across serializations:** `serde_json` map ordering must not affect identity.
- **Tamper-evident:** Descriptors carry Ed25519 signatures; verification rejects
  modified content.
- **Multi-language callable:** C ABI required for Go, C, and Python consumers.
- **Schema-validatable:** A JSON Schema for the descriptor allows offline validation.
- **Zero-copy top-level ID:** The top-level `descriptor_id` is the blake3 of a
  stable subgraph — not serialized JSON, not the full descriptor.

---

## Decision

### Descriptor Schema

The `CapabilityDescriptor` is a JSON object with this top-level shape:

```json
{
  "descriptor_version": "0.1.0",
  "descriptor_id": "blake3-hex",
  "issued_at": "2026-09-01T00:00:00Z",
  "expires_at": "2036-09-01T00:00:00Z",
  "processor": { ... },
  "accelerator": { ... },
  "display": { ... },
  "pcie": { ... },
  "audio": { ... },
  "input": { ... },
  "storage": { ... },
  "network": { ... },
  "topology": { ... },
  "signatures": [ ... ],
  "meta": { ... }
}
```

`descriptor_id` is the blake3 of the hash-stable subgraph (everything except
`descriptor_id` itself and `signatures`). See `canonical_bytes()` in
`crates/fabric-capability/src/descriptor.rs`.

Every optional sub-struct is nullable. Absence means "unknown" (not "none").

### Canonical Bytes Algorithm

To produce a stable identity:

1. Clone the descriptor, strip `descriptor_id` and `signatures`.
2. Sort every top-level key alphabetically.
3. Serialize to canonical JSON (`serde_json::to_string`, which sorts map keys).
4. Prepend `b"FABRIC-CAPABILITY-v1:"` as a domain separation prefix.
5. Hash with blake3.
6. The hex digest is `descriptor_id`.

This means:
- Adding or removing a `null` field changes identity (null ≠ absent).
- Map ordering never affects identity.
- The prefix prevents collisions with other blake3 uses.

### Signing Model

- **Algorithm:** Ed25519 (ed25519-dalek 2.x)
- **Canonical bytes source:** The same `canonical_bytes()` as used for `descriptor_id`
  — i.e., the descriptor stripped of its own `signatures` field before hashing.
- **Key lifecycle:** Out of scope for this ADR. Assume keys are provisioned via a
  secure channel and stored in platform secret stores per ADR-0019.
- **Multi-sig:** Supported via an array of `Signature` entries with `trusted: true/false`.
- **Fingerprint:** blake3 of the 32-byte public key (hex string).

### JSON Schema

`architecture/schemas/capability.schema.json` is the canonical machine-readable schema.
It is generated from Rust types via `schemars` 0.8 (`#[derive(JsonSchema)]`) and
checked into source. Validation uses `jsonschema` 0.27.

### FFI Surface

`crates/fabric-capability-ffi/include/fabric_capability.h` — hand-authored C header
matching the Rust `lib.rs` surface:

| Function | Purpose |
|---|---|
| `fabric_capability_sign_json` | Sign a descriptor JSON with a 32-byte key |
| `fabric_capability_verify_json` | Verify an Ed25519 signature |
| `fabric_capability_generate_key` | Generate a new signing key |
| `fabric_capability_key_id` | Get the blake3 key fingerprint |
| `fabric_capability_to_json` | Re-serialize (validates + canonicalizes) |
| `fabric_capability_free_string` | Free returned strings |
| `fabric_capability_free_key` | Free raw key buffers |
| `fabric_capability_error_message` | Human-readable error for `FabricError` code |

The FFI is intentionally thin — it wraps the Rust library directly, no additional
logic.

---

## Alternatives Considered

### HMAC-SHA256 instead of Ed25519

Pros: Simpler, standard library only. Cons: Requires a shared secret, which
requires secure distribution. Ed25519's public-key model fits Fabric's trust
model better (signing keys are local; verification keys are shared).

### Hash of serialized JSON for identity

Pros: Simpler. Cons: `serde_json` map ordering is stable within a process but
not guaranteed across versions or language implementations. The explicit
`canonical_bytes()` with domain separation is unambiguous.

### Automatic header generation with cbindgen

Rejected: cbindgen does not work cleanly with workspace crates. The FFI surface
is small enough to maintain by hand. The header is documented and tested against
the Rust implementation.

### JSON-LD / RDF for capability representation

Rejected: Overkill for the initial implementation. The flat JSON object is
human-readable, widely supported, and sufficient for the current scope.
JSON-LD can be added as a projection layer later.

---

## Consequences

- **Positive:** Stable, tamper-evident descriptors; multi-language consumers via FFI;
  schema validation in CI.
- **Negative:** Ed25519 signing is CPU-bound (negligible for descriptor-sized
  payloads). The FFI requires `unsafe` Rust.
- **Neutral:** `descriptor_id` is derived from content, not assigned. This means
  two identical machines produce identical IDs — which is correct behavior for
  capability comparison but may surprise observers expecting unique IDs.

---

## References

- Implementation: `crates/fabric-capability/`
- FFI: `crates/fabric-capability-ffi/`
- Schema: `architecture/schemas/capability.schema.json`
- Spec: `specs/014-capability-inventory/`
