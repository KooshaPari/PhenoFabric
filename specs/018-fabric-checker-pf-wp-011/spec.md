# Spec 018 — Fabric checker: capability probe vs NVMS manifest cross-check

**WP**: PF-WP-011  
**Status**: specified (0.1.0-draft)  
**Depends on**: specs/014 (capability inventory), specs/016 (NVMS adapter)  
**Blocks**: specs/017 (workspace persistence), PF-WP-022 (runtime migration)  
**Release gate**: R1

## Problem

A `CapabilityDescriptor` describes **what a host actually has** (what the probe saw
right now). An NVMS `Manifest` describes **what an application needs** to run
(bandwidth, audio, RAM, GPU, topology constraints). Today they are
*constructed* from the same source of physical truth (the machine) but compared
*implicitly* only at compile time inside `BoundManifestBuilder` — which fails
on "memory not enough" but not on the more interesting cases:

1. **A host claims to have GPU X, but the manifest needs GPU Y.** The host's
   capability was probed at a moment when Y was unplugged; the manifest was
   authored against a different host.
2. **A host claims CPU=4 cores, the manifest needs CPU=8.** The probe was
   before a hot-plug event; the manifest is stale.
3. **A host has two GPUs and the manifest requires both** (e.g. one for
   encode, one for display), but the probe only sees one because the other
   is in a different NUMA node and the probe path was NUMA-blind.
4. **An NVMS manifest was authored against host A, but the workspace is
   running on host B.** Today's compiler happily produces a route plan and
   the agent crashes 30s into the route.

The Fabric checker is the **explicit, structured, deterministic** answer to
"given this host's probed capabilities and this application's required
capabilities, should we admit this workspace, admit with notes, or reject?"

## Goals (PF-FR-007..009)

- **PF-FR-007** — The checker MUST be the *single source of truth* for
  admit/admit-with-notes/reject decisions. No caller may reason about
  sufficiency independently.
- **PF-FR-008** — Every decision MUST carry structured reason codes
  (categorical + numeric severity) so that downstream consumers (UI, audit
  log, agent policy engine) can reason about them.
- **PF-FR-009** — The checker MUST be deterministic and side-effect free.
  Same inputs MUST produce same decision.

## Decisions

The checker produces a `CheckOutcome` with a `Decision` and a `Vec<ReasonCode`:

| Decision      | Meaning                                                      | Caller action        |
|:--|:--|:--|
| `Admit`       | Host satisfies all hard requirements. No notes.              | Proceed.             |
| `AdmitWithNotes` | Host satisfies all hard requirements, but at least one **soft** requirement is unmet. | Proceed, log the notes, optionally surface a warning to the user. |
| `Reject`      | At least one **hard** requirement is unmet.                  | Block workspace creation. |

## Reason codes (categorical)

A `ReasonCode` is one of:

- `InsufficientMemory { required_bytes, available_bytes }` — physical RAM
- `InsufficientCpuCores { required, available }`
- `InsufficientGpuMemoryBytes { required, available, gpu_index }`
- `MissingAccelerator { required_kind, available_kinds }`
- `MissingDisplay { required, available }` (e.g. monitor count < required)
- `MissingAudioDevice { required_kind, available_kinds }`
- `MissingInputDevice { required_kind, available_kinds }`
- `MissingStorage { required_bytes, available_bytes }`
- `TopologyConstraintUnmet { constraint }` (e.g. `same_host` violated)
- `BandwidthShortfall { required_mbps, available_mbps, link_index }`
- `LatencyBudgetExceeded { required_us, observed_us, link_index }`
- `LossAboveThreshold { required_loss, observed_loss, link_index }`
- `StaleManifest { manifest_age_seconds, max_age_seconds }`
- `StaleDescriptor { descriptor_age_seconds, max_age_seconds }`
- `UntrustedSignature { expected, observed, key_id }` (NVMS manifest is signed
  but the signing key isn't in the trust set)
- `UntrustedDescriptor` (probe signature)
- `IncompatibleManifestVersion { required, observed }`
- `IncompatibleDescriptorVersion { required, observed }`
- `CustomPolicyViolation { policy_id, detail }`

Each reason code carries `severity: Severity { Hard | Soft | Info }`. Soft
reasons do not block admission; they populate the notes on an
`AdmitWithNotes`. Hard reasons make the decision `Reject`.

## Inputs

- `descriptor: &CapabilityDescriptor` — host capability snapshot
- `manifest: &Manifest` (or `BoundManifest`) — application's requirements
- `policies: &CheckPolicies` — thresholds (bandwidth soft floor, manifest max
  age, trust roots, severity overrides per reason code)

## Output

```rust
pub struct CheckOutcome {
    pub decision: Decision,
    pub reasons: Vec<ReasonCode>,
    pub checked_at: DateTime<Utc>,
    pub descriptor_id: Uuid,
    pub manifest_id: Uuid,
    pub policy_id: String,
    pub notes: Vec<String>, // human-readable summary of the AdmitWithNotes path
}
```

The `notes` field is the *output* of the checker for UI / log; the `reasons`
are the *structured data* for downstream automation.

## What is explicitly out of scope

- **Migration of running workspaces** when capabilities change underneath
  (covered by PF-WP-022 runtime migration, R1.5).
- **Identity** of the manifest author (covered by
  `009-security-identity-evidence`, R2).
- **Telemetry aggregation** across many checks (covered by
  `011-observability-verification`, R1).

## Non-goals (explicit, to prevent scope creep)

- The checker does NOT probe. It consumes probe output.
- The checker does NOT compile route plans. It produces admit/reject
  decisions. Route planning is `fabric-graph::compile` (specs/015).
- The checker does NOT manage workspaces. That's `fabric-workspace`
  (specs/017).
- The checker does NOT enforce policies at runtime. It's a pre-flight
  gate, not a watchdog.

## Acceptance criteria

A change to the checker is "done" when:

1. The same `(descriptor, manifest, policies)` triple produces the same
   `(decision, reasons)` across all runs (PF-FR-009).
2. Every `ReasonCode` variant in the spec is reachable by at least one
   test fixture (positive or negative).
3. The `Go` reference implementation in `cmd/checker` produces bit-identical
   `CheckOutcome` for the spec's canonical fixtures.
4. The checker's public surface area is `CheckOutcome` + the enums; no
   internal types leak.
5. All reason codes are listed in ADR-0027 with a normative definition
   and a test fixture.
