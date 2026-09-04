# 0027 — Checker Decision Taxonomy & Reason Codes

## Status

Accepted — 2026-09-03. **Reference implementation is Go-only** (`cmd/checker`,
stdlib-only, 9 tests passing). The Rust implementation was attempted twice
in spec 018 but hit cascading type-mismatch loops; the Go path was a clean
break and is now the authoritative contract.

## Date

2026-09-02 (originally Proposed). Accepted 2026-09-03.

## Deciders

GLM-backed Codex chat continuation session (2026-09-01 → 2026-09-02).

## Supersedes

None.

## Traceability

- spec `018-fabric-checker-pf-wp-011` — defines the contract
- ADR-0024 (NVMS adapter mapping) — defines the manifest side
- `architecture/schemas/check-decision.schema.json` — wire format
- PRD section 6.3 (admission control)

## Context

`fabric-checker` (PF-WP-011) is the boundary between a Fabric
`CapabilityDescriptor` (what a host actually has, probed at runtime)
and an NVMS `BoundManifest` (what an application claims to need). Both
descriptions may be honest; they may still disagree. The checker is
the only place that arbitrates admission.

Every check outcome must be:
1. **Structured** — machine-readable code, not prose.
2. **Non-overlapping** — two reasons should never simultaneously apply
   to the same (capability, requirement) pair.
3. **Severity-graded** — `Error` (reject), `Warn` (admit with notes),
   `Info` (admit, log only).
4. **Remediable when possible** — a reason code carries a `remediation`
   hint so callers can surface it to operators (e.g. "use a host with
   32 GiB RAM").
5. **Stable across versions** — once a code is published, its
   meaning does not change. New reasons get new codes; old reasons
   never get repurposed.

## Decision

The checker emits a `Decision` per (descriptor, manifest) pair. The
`Decision` is one of three top-level outcomes:

| Outcome    | Meaning                                                                                  |
|:--|:--|
| `Admit`    | All `must` requirements satisfied, no warnings raised.                                 |
| `AdmitWithNotes` | All `must` requirements satisfied, but one or more `Warn` notes raised.          |
| `Reject`   | One or more `Error`-severity reasons raised. The decision is *blocking*.              |

`AdmitWithNotes` is a first-class outcome (not just a flag) because
operators need to distinguish "fit perfectly" from "fit but
marginally" when triaging manifests at scale.

### Reason code taxonomy

Each reason code has a stable string form `PF-CK-<AREA>-<NNN>`
and a fixed severity. The taxonomy is grouped by area:

**Memory** (`PF-CK-MEM-`)
- `PF-CK-MEM-001` `InsufficientTotalMemory` (Error)
- `PF-CK-MEM-002` `MemoryMarginBelowTenPercent` (Warn) — host RAM is
  within 10 % of manifest `must_total_memory_bytes`
- `PF-CK-MEM-003` `MemoryBoundsMismatch` (Error) — manifest `must_total_memory_bytes`
  falls outside the host's `min_total_memory_bytes`..`max_total_memory_bytes`
  bounds

**CPU** (`PF-CK-CPU-`)
- `PF-CK-CPU-001` `InsufficientCores` (Error)
- `PF-CK-CPU-002` `CpuBoundsMismatch` (Error)
- `PF-CK-CPU-003` `ArchitectureMismatch` (Error) — manifest `cpu_arch`
  not in host's supported architectures

**GPU** (`PF-CK-GPU-`)
- `PF-CK-GPU-001` `GpuAbsent` (Error) — manifest `must_gpu != None` but host has none
- `PF-CK-GPU-002` `VramInsufficient` (Error)
- `PF-CK-GPU-003` `GpuDriverMissing` (Warn) — driver probe failed, but
  hardware is present (admit with note; runtime may fail)
- `PF-CK-GPU-004` `GpuVulkanMismatch` (Error) — manifest `must_gpu.vulkan`
  version not in host's supported list

**Audio** (`PF-CK-AUD-`)
- `PF-CK-AUD-001` `AudioBackendAbsent` (Error)
- `PF-CK-AUD-002` `SampleRateUnsupported` (Error)
- `PF-CK-AUD-003` `RtThreadPriorityDenied` (Warn) — host advertises
  RT-capable audio but kernel permission denied at probe time
- `PF-CK-AUD-004` `AudioRtBoundsMismatch` (Error)

**Display** (`PF-CK-DSP-`)
- `PF-CK-DSP-001` `NoDisplayBackend` (Error) — manifest requires a
  display surface but host has no display server
- `PF-CK-DSP-002` `HdrAbsent` (Warn) — manifest wants HDR, host is SDR
- `PF-CK-DSP-003` `RefreshRateInsufficient` (Error)

**Network** (`PF-CK-NET-`)
- `PF-CK-NET-001` `LinkMetricsMissing` (Error) — manifest requires a
  network capability but host did not probe any link
- `PF-CK-NET-002` `BandwidthInsufficient` (Error)
- `PF-CK-NET-003` `RttExceedsBudget` (Error)
- `PF-CK-NET-004` `LossRateAboveThreshold` (Error)

**Storage** (`PF-CK-STR-`)
- `PF-CK-STR-001` `FreeDiskInsufficient` (Error)
- `PF-CK-STR-002` `FilesystemUnsupported` (Error)
- `PF-CK-STR-003` `IoLatencyExceedsRtBudget` (Warn) — manifest wants
  RT I/O, host I/O latency budget exceeds 1 ms p99

**Trust** (`PF-CK-TRU-`)
- `PF-CK-TRU-001` `DescriptorUnsigned` (Warn) — descriptor is valid
  but no signature is present (admit with note; trust is `SelfReported`)
- `PF-CK-TRU-002` `SignatureUntrusted` (Warn) — signature present but
  the key is not in the trust set
- `PF-CK-TRU-003` `DescriptorStale` (Warn) — `probed_at` older than
  the freshness budget (default 1 hour)

**Staleness** (`PF-CK-STL-`)
- `PF-CK-STL-001` `SchemaVersionAhead` (Warn) — manifest references
  a capability schema version newer than the checker supports
- `PF-CK-STL-002` `SchemaVersionBehind` (Info) — manifest references
  an older schema version than the checker supports (admit; log only)

### Wire format

Every reason code carries a JSON object:

```json
{
  "code": "PF-CK-MEM-001",
  "severity": "error",
  "subject": "host.com.example.lab-01",
  "expected": { "total_memory_bytes": 34359738368 },
  "actual":   { "total_memory_bytes": 17179869184 },
  "remediation": "Provision a host with at least 32 GiB RAM, or lower the manifest's must_total_memory_bytes",
  "evidence_pointer": "capability_descriptor.json#capabilities.compute.total_memory_bytes"
}
```

The `evidence_pointer` is a JSON pointer into the source
`CapabilityDescriptor` so an operator can jump straight to the
mismatched field.

### Composition rules

The checker composes all check outcomes into a single `Decision`:

1. Run all checks (one per reason code that *might* apply).
2. If any `Error` reason fires → `Decision::Reject` with all reasons.
3. Else if any `Warn` reason fires → `Decision::AdmitWithNotes` with
   the warn reasons (and any info reasons, which are still surfaced).
4. Else → `Decision::Admit` with all info reasons (none in practice
   for a clean admit, but the field is reserved).

**Determinism**: given the same `CapabilityDescriptor` and
`BoundManifest`, the checker MUST emit the same `Decision` and the
same set of reasons. No external network calls; no time-based
freshness judgments. (Staleness is checked against a caller-provided
`now_unix_ms`, not `SystemTime::now()`.)

### Cross-implementation stability

The Go reference checker (PF-WP-011.06) MUST emit the same JSON
shape and the same reason codes for the same inputs. Tests in
`crates/fabric-checker/tests/parity.rs` (Rust) and
`cmd/checker/parity_test.go` (Go) lock this in.

## Consequences

Positive:
- Every admission decision is reproducible and auditable.
- Operators get a stable vocabulary for triaging manifest/host
  mismatches.
- The 1,200-host fleet is operable without bespoke human judgment on
  each manifest admission.

Negative:
- Adding a new check requires a new reason code, a schema bump, and
  test fixtures — a higher cost than a free-form warning string.
- The taxonomy is R0-frozen; we will need a deprecation path when
  a check is retired.

## Open questions

- Should `AdmitWithNotes` require operator confirmation, or proceed
  with the note in an audit log? (Proposed default: audit log only,
  not interactive confirmation.)
- What is the freshness budget for `PF-CK-TRU-003`? (Proposed
  default: 1 hour; configurable per checker invocation.)
- Should the checker reject on `PF-CK-TRU-001` in
  `TrustMode::Audited`, or still admit? (Proposed: `Reject` in
  `Audited`, `AdmitWithNotes` in `Verified` and below.)
