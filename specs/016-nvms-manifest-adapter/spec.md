# Spec 016 — NVMS Manifest Adapter (R0.5)

**Release gate:** R0.5 (intermediate, after R0 capability inventory; before R1 route compiler).
**Source spec:** odin.nvms v0.2 (archived at `nanovms/crates/phenotype-manifest/`).
**Owner:** Fabric runtime team.

---

## 1. Purpose

Bridge the **odin.nvms v0.2** application manifest format (a YAML document describing what an application requires) into Fabric's `CapabilityDescriptor` (a JSON document describing what a host offers).

This is the missing link between two adjacent systems:
- **NVMS** is a per-application document (one per workload)
- **Fabric capability** is a per-host document (one per machine)

The adapter produces a **synthetic host descriptor** whose offer set is the upper bound required by the NVMS application. A match between the synthetic descriptor and a real Fabric probe is a candidate placement.

---

## 2. Mapping

| NVMS v0.2 field | Fabric field | Notes |
|:--|:--|:--|
| `cpu.cores` | `compute.cpu.physical_cores` | exact |
| `cpu.threads` | `compute.cpu.logical_cores` | exact |
| `cpu.model` | `compute.cpu.model` | exact |
| `cpu.features` | `compute.cpu.features` | subset; missing features = `WARN` |
| `memory.min_bytes` | `compute.memory.bytes` (lower bound) | `>=` |
| `agent.id` | `topology.canonical_host` | synthetic |
| `agent.command` | `topology.scheduling_class` | mapped (`agent.scheduler` → `agent-class`) |
| `network.bind` | `network.bind_addresses` | subset |
| `network.ports` | `network.listen_ports` | translated to `Range` |
| `nvms.version` | `metadata.nvms_version` | preserved as provenance |

Fields not in NVMS (`accelerator`, `display`, `pcie`, `audio`, `input`, `storage`, `link_metrics`) are **explicitly omitted** — the adapter does not invent host capabilities that aren't in the manifest.

---

## 3. Sub-tasks

### 3.1 PF-WP-011.01 — Define the adapter API

- `nvms_to_capability_descriptor(yaml: &str) -> Result<CapabilityDescriptor, AdapterError>`
- `nvms_to_capability_descriptor_from_path(p: &Path) -> Result<CapabilityDescriptor, AdapterError>` (for test fixtures)
- `RequiredCapabilities` (YAML struct) with the exact field set from NVMS v0.2
- `CapabilityBounds` (synthesized field set) with provenance linking back to NVMS via `metadata.nvms_version`

### 3.2 PF-WP-011.02 — Validation and error taxonomy

- `AdapterError::UnknownField(String)` — strict mode
- `AdapterError::InvalidType { field, expected, found }`
- `AdapterError::MissingRequired(String)`
- `AdapterError::NanosVersionMismatch { expected, found }` (when NVMS adds a version field we don't know)

### 3.3 PF-WP-011.03 — Round-trip and provenance

- `descriptor_to_nvms_bounds(d: &CapabilityDescriptor) -> CapabilityBounds` (reverse direction for verification)
- Both functions produce a `metadata` block with:
  - `provenance.kind = "nvms-manifest-adapter"`
  - `provenance.source = "<sha256 of the input YAML>"`
  - `provenance.adapter_version = env!("CARGO_PKG_VERSION")`

### 3.4 PF-WP-011.04 — Test fixtures from the archived `phenotype-manifest`

- Load `nanovms/crates/phenotype-manifest/tests/fixtures/minimal.yaml`
- Load `nanovms/crates/phenotype-manifest/tests/fixtures/full.yaml`
- Verify the adapter produces valid descriptors for both
- Verify the round-trip `nvms → descriptor → nvms` preserves the source fields exactly

---

## 4. Acceptance

- All four sub-tasks implemented in `crates/phenotype-nvms-adapter/`
- ≥ 12 unit tests passing (parse, field validation, bounds, roundtrip)
- Two integration tests against the archived NVMS fixtures pass
- The adapter reports the version mismatch and refuses to convert when given a NVMS document with a future version number
- The descriptor `metadata.provenance` block points back to the source YAML by SHA-256
