# ADR-0024: NVMS Adapter — Mapping `odin.nvms` to Fabric `CapabilityDescriptor`

**Status:** Proposed
**Date:** 2026-09-01
**Deciders:** Session 01a04c3e continuation (2026-09-01)
**Supersedes:** None
**Traceability:** PF-FR-007, PF-FR-009, INT-P012, R0.5

---

## Context

The `odin.nvms` manifest format (v0.2) is a YAML application manifest that
describes the **application's resource requirements** — CPU cores, memory, network
ports, GPU compute — and a list of agent IDs that run on the machine.

Fabric needs to map these manifests into `CapabilityDescriptor` instances to feed
the topology graph and route compiler. This ADR defines the mapping rules and
establishes `crates/phenotype-nvms-adapter/` as the canonical translation layer.

**Key distinction:** `odin.nvms` describes an **application deployment**, not a
**machine's raw hardware**. The adapter translates application requirements into
a synthetic Fabric descriptor for the host, annotated with the manifest's claims.

---

## Decision

### Architecture

```
odin.nvms v0.2 YAML
      │
      ▼
crates/phenotype-nvms-adapter/
  ├─ lib.rs          — top-level API (nvms_to_capability_descriptor)
  ├─ required.rs     — RequiredCapabilities, bounds (min/max/default)
  └─ bound.rs        — capability lower-bounds from required
          │
          ▼
CapabilityDescriptor (Fabric)
  ├─ processor      — from required.compute.cores + nvms metadata
  ├─ accelerator    — from required.compute.gpu (if any)
  ├─ storage        — from required.storage (sum of mounts)
  ├─ network        — from required.network.ports
  └─ topology       — populated with nvms-specific locality annotations
```

### Mapping Rules

| NVMS field | Fabric field | Notes |
|---|---|---|
| `metadata.name` | `meta.identifiers.nvms_name` | Identity link |
| `metadata.version` | `meta.identifiers.nvms_version` | |
| `required.compute.cores` | `processor.core_count_min` | |
| `required.compute.memory` | `processor.memory_bytes_min` | |
| `required.compute.gpu.count` | `accelerator.device_count` | `None` if absent |
| `required.compute.gpu.memory` | `accelerator.memory_bytes_min` | |
| `required.storage` | `storage` | Per-mount; sum for total |
| `required.network.ports` | `network` | Port ranges + protocol |
| `agent_ids[]` | `meta.nvms_agent_ids` | |
| `bounds.compute.cores` | `processor.core_count_max` | |
| `bounds.compute.memory` | `processor.memory_bytes_max` | |
| `bounds.compute.gpu.memory` | `accelerator.memory_bytes_max` | |

### Stability Model

`CapabilityDescriptor.stability` is set to `nvms_compatible` when the adapter
produces it. This signals to the route compiler that the descriptor reflects
an application manifest rather than a direct probe.

### Bounds vs. Requirements

- **Required** (`RequiredCapabilities`): the minimum viable configuration for the
  application to function. The route compiler must satisfy these or reject placement.
- **Bounds** (`CapabilityBounds`): the range of acceptable hardware. Used for
  scoring and feasibility checks. When no bound is specified, the bound equals
  the required value.

### Error Handling

| Condition | Behavior |
|---|---|
| Malformed YAML | `Error::Parse` with source |
| Missing required fields | `Error::MissingRequired` listing missing keys |
| Unknown NVMS version | `Error::UnsupportedVersion` with version string |
| Storage mount with no path | `Error::InvalidStorage` |

### Test Fixtures

Two fixtures in `crates/phenotype-nvms-adapter/tests/`:
- `testdata/minimal.yaml`: CPU-only, no GPU, single port
- `testdata/full.yaml`: GPU, multi-mount storage, multi-port, bounds

Both are from the archived `nanovms` repo's `phenotype-manifest` test suite,
adapted to the current `odin.nvms` schema.

---

## Alternatives Considered

### Pull from NVMS runtime directly

Rejected: The NVMS runtime (`nanovms/nvms-runtime`) is archived and not
actively maintained. The manifest adapter approach is more portable and works
offline.

### Embed NVMS manifest in `CapabilityDescriptor.meta`

Rejected: This loses the structural translation. A separate adapter crate with
typed input/output makes the boundary explicit and testable.

### Derive `descriptor_id` from manifest content

Rejected: `descriptor_id` is a hardware capability fingerprint. Using manifest
content would cause the same machine to have different IDs depending on which
manifest was applied. The adapter annotates the hardware descriptor with
manifest metadata; it does not replace the hardware identity.

---

## Consequences

- **Positive:** Enables Fabric route planning to consider NVMS-registered
  applications as first-class participants. Clear, typed, testable boundary.
- **Negative:** Dual source of truth for "what can this machine do" — probe results
  and NVMS manifests may diverge. A reconciliation policy is needed (future work,
  PF-WP-010.09).
- **Neutral:** The adapter does not modify the original NVMS manifest. It is a
  one-way translation layer.

---

## References

- Implementation: `crates/phenotype-nvms-adapter/`
- NVMS schema origin: `nanovms/crates/phenotype-manifest/` (archived)
- Capability schema: `architecture/schemas/capability.schema.json`
