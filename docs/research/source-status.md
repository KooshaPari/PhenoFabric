# Source Confidence and Claim Policy

**Status:** canonical (effective 2026-09-01)
**Owner:** spec 013-fabric-program-baseline / PF-WP-000.04
**Cross-references:** `program/identifiers.md`, `sota/INDEX.md`,
`research/hypotheses.md`, `research/source-register.md`,
`risks/legal-licensing.md`

## Purpose

Per `GOVERNANCE.md#evidence-rule`, every external claim that becomes
evidence in Phenotype Fabric must be classified by source confidence
*before* it can be cited. This file is the structured form of that
classification.

A claim is "evidence" if it is used to justify a design decision, a
performance number, a security posture, or a release gate.

## Confidence Levels

| Level | Meaning | Citable as evidence? | Required fields |
|---|---|---|---|
| `verified-by-repro` | We have reproduced the claim on our reference environment and committed the reproduction script + raw data | **Yes** (strongest) | repro script path, raw data path, topology, software versions, date |
| `verified-by-vendor-doc` | A vendor's official documentation says it. We have not reproduced it. | **Yes** (with caveat: "vendor claim, unverified") | primary URL, date_verified, vendor name, version |
| `verified-by-paper` | A peer-reviewed or formal technical paper says it. We have not reproduced it. | **Yes** (with caveat: "academic claim, unverified") | paper URL/DOI, authors, year, venue |
| `verified-by-third-party-bench` | A trustworthy third party benchmark (Phoronix, vendor bench, etc.) | **Yes** (with caveat: "third-party, not our environment") | bench URL, bench author, environment description |
| `claim-only` | A blog post, forum, or general knowledge claim | **No** (must be promoted to one of the above before use) | source URL, author, date, reason not promoted |
| `rejected` | We have evaluated the claim and it does not hold for our use case | **No** | rejection reason, evaluation notes, replacement evidence |

## Source Status Table

This table is the canonical form. It is regenerated from the source
files in `sota/`, `research/hypotheses.md`, and `risks/legal-licensing.md`
when those change. Manual entries are allowed for cross-cutting claims
that span multiple sources.

### SOTA competitive analyses (`sota/INDEX.md`)

| source_id | claim | confidence | primary_url | reproduced | date_verified | decays_after | replacement_evidence | task_trace |
|---|---|---|---|---|---|---|---|---|
| SOTA-001 deskflow | DeskFlow provides low-latency input sharing over LAN | verified-by-vendor-doc | https://deskflow.com/ | false | 2026-08-28 | 2027-08-28 | TBD: Fabric input sharing benchmark | TASK-014 (planned) |
| SOTA-002 parsec | Parsec claims 30ms RTT for game streaming | claim-only | https://parsec.app/ | false | 2026-08-28 | 2026-12-31 | TBD: Fabric RT input benchmark | TASK-014 (planned) |
| SOTA-003 looking-glass | Looking-glass uses shared memory + DXGI capture | verified-by-paper | https://looking-glass.io/ | false | 2026-08-28 | 2027-08-28 | TBD: Fabric surface-plane benchmark | TASK-014 (planned) |
| SOTA-004 evdev | Linux evdev supports raw input injection | verified-by-vendor-doc | https://www.freedesktop.org/software/libevdoc/ | true (kernel headers) | 2026-08-28 | indefinite | n/a (kernel API) | TASK-007 (planned) |
| SOTA-005 seamless-app | seamless-app uses TCP forwarding of X11/Wayland | claim-only | http://seamless-app.org/ | false | 2026-08-28 | 2026-12-31 | TBD: Fabric X11 forwarding test | TASK-014 (planned) |
| SOTA-006 audio-network | PipeWire has built-in network audio (RTP / Opus) | verified-by-vendor-doc | https://pipewire.org/ | true (probe) | 2026-08-28 | 2027-08-28 | n/a (kernel + userspace feature) | TASK-007 (planned) |
| SOTA-007 moat-and-risk |  | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a (analyst memo) |
| SOTA-008 | | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a |
| SOTA-009 | | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a |
| SOTA-010 | | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a |
| SOTA-011 | | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a |
| SOTA-012 | | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a |
| SOTA-013 | | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a |
| SOTA-014 | | claim-only | TBD | false | 2026-08-28 | TBD | TBD | n/a |

The blank rows (SOTA-008 through SOTA-014) are placeholders for the
remaining 7 SOTA entries in `sota/`. They are filled in by a
follow-up PR once each SOTA file has been evaluated for confidence.

### Research hypotheses (`research/hypotheses.md`)

| hypothesis_id | claim | confidence | primary_url | reproduced | date_verified | decays_after | replacement_evidence | task_trace |
|---|---|---|---|---|---|---|---|---|
| HYP-001 | Stage elimination at L0–L1 reduces copy cost to zero for same-NUMA traffic | verified-by-paper | research/literature-map.md#paper-3 | false | 2026-08-28 | 2027-08-28 | PF-WP-030 placement benchmark | TASK-030 (planned) |
| HYP-002 | GPU memory can be zero-copy transferred via NVLink for up to 4 GPUs | verified-by-vendor-doc | NVIDIA NVLink spec | false | 2026-08-28 | 2027-08-28 | PF-WP-030 GPU benchmark | TASK-030 (planned) |
| HYP-003 | PipeWire's graph-based scheduling avoids glitches under contention | verified-by-paper | research/literature-map.md#paper-7 | false | 2026-08-28 | 2027-08-28 | PF-WP-050 RT audio benchmark | TASK-050 (planned) |
| HYP-004 | Wayland presentation-time protocol enables sub-frame input-to-photon latency | verified-by-vendor-doc | Wayland spec | false | 2026-08-28 | 2027-08-28 | PF-WP-050 RT display benchmark | TASK-050 (planned) |
| HYP-005 | Thunderbolt 4 peer-to-peer tunneling allows <1µs PCIe-equivalent latency over TB4 cable | claim-only | TBD | false | 2026-08-28 | 2026-12-31 | TBD: Fabric link probe | TASK-105 (planned) |
| HYP-006 | Linux cgroup v2 freezer can be replaced with eBPF-based freezer for faster snapshots | claim-only | TBD | false | 2026-08-28 | 2026-12-31 | TBD: PF-WP-040 freeze benchmark | TASK-040 (planned) |
| HYP-007 | Vulkan VK_KHR_synchronization2 enables deterministic RT submission | verified-by-vendor-doc | Vulkan spec | false | 2026-08-28 | 2027-08-28 | PF-WP-060 GPU presentation benchmark | TASK-060 (planned) |
| HYP-008 | AMD ROCm supports CUDA-equivalent zero-copy via unified memory | verified-by-vendor-doc | AMD ROCm docs | false | 2026-08-28 | 2027-08-28 | PF-WP-060 AMD GPU benchmark | TASK-060 (planned) |

### Legal / licensing claims (`risks/legal-licensing.md`)

| source_id | claim | confidence | primary_url | reproduced | date_verified | decays_after | replacement_evidence | task_trace |
|---|---|---|---|---|---|---|---|---|
| RISK-001 SPDX | All deps in `Cargo.lock` and `go.mod` have SPDX identifiers | verified-by-repo-scan | https://spdx.org/ | true (script) | 2026-08-28 | indefinite | n/a (script runs in CI) | n/a |

## Promotion Path for `claim-only` Sources

A `claim-only` source cannot be cited as evidence. To promote it:

1. **Reproduce it** on the reference environment.
2. Commit the reproduction script under `research/experiments/EX-NNN-*`
3. Capture the raw data in the same directory.
4. Add an `EX-NNN` row to `research/source-register.md`.
5. Update this table: change `confidence` to `verified-by-repro` and
   fill in the `reproduced` field with the path to the raw data.
6. Add the `reproduction` cross-link in the originating
   `sota/SOTA-NNN-*` or `research/hypotheses.md#HYP-NNN` file.

This promotion is itself an atomic task (`TASK-NNN`).

## Decay and Re-verification

The `decays_after` field is a calendar date. After that date, the
claim must be re-verified by re-running the reproduction. This is
required because:
- Vendor claims can change with new versions.
- Kernel APIs can change.
- Hardware revisions can change performance characteristics.

The CI script `program/scripts/check_source_decay.py` (PF-WP-000.04
deliverable) runs weekly and emits a warning for any source whose
`decays_after` is in the past. The warning does not fail CI but does
create a maintenance task in `work/tasks.json` (TASK-NNN auto-assigned).

## Cross-references

- `GOVERNANCE.md#evidence-rule` — the upstream rule
- `GOVERNANCE.md#research-rule` — research must be reproducible
- `verification/requirements-traceability.json` — which claims are
  used for which requirements
- `work/release-evidence-contract.md` — how this table gates releases
