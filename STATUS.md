# Document and Project Status

**As of:** 2026-08-28  
**Lifecycle state:** `specified`  
**Confidence:** Architecture baseline is strong; several fine-grained execution mechanisms remain research hypotheses.

## Readiness by capability

| Capability | Status | Evidence required before implementation claim |
|---|---|---|
| Unified graph/object model | Specified | Schema validation, reference implementation, round-trip persistence |
| Packaged UI/API shell | Specified | Install/upgrade prototype on Linux, Windows, macOS |
| Same-host input routing | Integration-ready | evdev/uinput/libei latency and stuck-key stress tests |
| Same-host VFIO display | Integration-ready | Looking Glass/KVMFR benchmarks under concurrent GPU/CPU load |
| LAN desktop streaming | Integration-ready | Sunshine/Moonlight/Parsec adapter comparison |
| WAN desktop streaming | Integration-ready | NAT traversal, congestion, relay, loss, and security tests |
| Per-window semantic remoting | Partly integration-ready | RAIL/Xpra/Waypipe capability matrix |
| Universal pixel-proxy windows | Research/prototype | owned-window graph, IME, DPI, protected surfaces, encoder scaling |
| Audio/MIDI graph | Integration-ready locally | PipeWire/JACK/WASAPI/CoreAudio adapter prototypes |
| Network live-audio routing | Research/prototype | clock drift, ASRC, jitter, xrun, round-trip measurement |
| Data/object locality plane | Specified | immutable object store and residency benchmark |
| Process/task placement | Prototype-ready | ShareCLI/NVMS adapter, build farm, agent task demonstrations |
| Atomic syscall/function placement | Research | interposition overhead and region-fusion proof |
| Cross-OS arbitrary process migration | Explicitly non-general | use semantic handoff, checkpoint, VM migration, or rematerialization |
| Hard real-time isolation | Platform-specific research | measured deadline protection under adversarial contention |
| Agent-created ephemeral realms | Specified | enrollment, TTL, permissions, fallback-console demo |
| Full evidence/trace integration | Specified boundary | Tracera/SessionLedger event contracts |

## Document quality gates

- [x] Human intent captured verbatim.
- [x] Synthesis maps prompts to requirements and decisions.
- [x] PRD, FRs, NFRs, HLD, LLD, domain model, and API baseline included.
- [x] AgilePlus-style specifications, plans, and task catalogs included.
- [x] ADRs record accepted and rejected alternatives.
- [x] WBS, DAG, PERT, critical path, research plan, and acceptance gates included.
- [x] Competitive/SOTA families exceed the requested 25 entries where a meaningful class exists.
- [x] User-facing and technical differentiation are separated.
- [x] Current claims are tagged by source confidence.
- [x] File hashes and validation report are generated with the archive.

---

## Proposed global ledger extension — status 2026-10-12

| Capability | Evidence class | Status |
|---|---|---|
| User intent provenance INT-P009..012 | Verbatim current conversation | Captured |
| Nested local scope / actor-kind record / finite capacity / SQL event-outbox | `fabric-persist` migration V4 and in-crate tests | Candidate implementation; branch CI pending |
| Human/agent authenticated effective grant and delegation verification | ADR/FR only | Not implemented |
| ShareCLI standalone root adoption to Fabric | ADR/FR only | Not implemented |
| Cross-host target publication fencing for this ledger | ADR/FR only | Not independently demonstrated |
| Verified physical reclaim receipt | SQL caller-supplied reference only | Not independently demonstrated |
| Two-consumer native scenario and crash/partition evidence | Verification plan only | Not run |

**Source:** [intent](intent/000-source-prompts.md), [spec 027](specs/027-global-nested-ledger/spec.md), [verification](verification/requirements-traceability-matrix.md). Earlier status/measurement pins are historical and must not be assumed valid for the new branch.
