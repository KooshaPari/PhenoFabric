# End-to-End Intent Traceability

| Intent ID | Human source meaning | Requirement range | Owning specs | Principal outputs |
|---|---|---|---|---|
| INT-P001 | Survey Parsec/Deskflow/evdev/Looking Glass and at least 25 alternatives each; extreme multi-PC/VM/remote environment; Windows app as a Mac window; global I/O switching, user/realm model, clipboard/files and continuity | PF-FR-001–037, 050–055, 080–088 | 001, 002, 006, 007, 008, 009, 012 | `sota/*`, PRD, domain model, input/surface/files architecture, reference scenarios |
| INT-P002 | Real-time and scaled performance under agent/game/compiler/Ableton load; high video/audio/color quality; C27HG70 and 2021 M1 Pro | PF-FR-040–075 | 004, 005, 007, 011 | real-time audio, HDR/video, QoS, benchmark and acceptance plans |
| INT-P003 | Packaged all-in-one shell despite modular internals; PipeWire/JACK generalized node/port/link model | PF-FR-001–017, 050–056 | 001, 002, 012 | ALD, HLD, domain model, graph runtime, packaging |
| INT-P004 | Same-OS/shared-memory/PCI paths plus LAN/WAN; every stage maximally optimized | PF-FR-010–017, 041, 044, 052, 060–069 | 003, 004 | route compiler, locality tiers, object plane, SOTA memory/transport set |
| INT-P005 | Mesh all compute/data instances; atomic decision potential; best available hardware behind the scenes; surfaces visible anywhere | PF-FR-036, 060–075 | 003, 004, 005, 006 | scheduler, object plane, compute SOTA, work/research program |
| INT-P006 | Absolute atomic granule must be possible but normally avoided/fused because distribution/sharding costs exist | PF-FR-062, 065–067 | 003, 004 | ADR-0005, adaptive-region LLD, cost model, hypotheses/experiments |
| INT-P007 | Connect the substrate to ShareCLI and the existing Phenotype/AGSLAG ecosystem | PF-FR-086–088 | 010 | `ecosystem/*`, GOVERNANCE, integration events and boundaries |
| INT-P008 | Deliver a repository-grade `docs/` tree with PRD/ALD/HLD/LLD, AgilePlus-shaped specs/ADRs, WBS/PERT/DAG, SOTA, exact prompt intent, research, verification and more | documentation deliverable | all | this complete tree, index, validation report and manifest |

## Trace direction

```text
verbatim prompt
 → intent synthesis / non-negotiable / ambiguity
 → PRD job/epic
 → functional and non-functional requirement
 → spec + ADR
 → WorkPackage/task
 → verification/acceptance ID
 → run/artifact/evidence reference
```

The repository currently specifies the chain through verification design. Runtime evidence links become populated during implementation and are emitted for Tracera/SessionLedger rather than fabricated in advance.

---

## INT-P009..012 — ledger model extension (2026-10-12)

| Source | Meaning | Requirements | Spec | Decision | Verification/state |
|---|---|---|---|---|---|
| INT-P009 | Immutable consequence record for bounded autonomous system-level actors | PF-FR-089, PF-FR-092 | [027](specs/027-global-nested-ledger/spec.md) | [ADR 0036](docs/adr/0036-global-nested-transaction-ledger.md) | SQLite local unit model, distributed/native pending |
| INT-P010 | Single global nested/tree-like history | PF-FR-089, PF-FR-091 | 027 | ADR 0036 | Scope ancestry constrained locally; cross-host pending |
| INT-P011 | Human/SoftwareAgent shared Actor superclass | PF-FR-090 | 027 | ADR 0036 | Actor-kind records; authenticated grant runtime pending |
| INT-P012 | ShareCLI and Fabric own integration; Registry indexes | PF-FR-093, PF-FR-094 | 027 | ADR 0015, 0016, 0036; ShareCLI ADR 0009 | Counterpart branch; native joined adoption pending |

These are directly transcribed current-conversation prompt IDs, not inferred from old INT-P001..008. The present implementation is a bounded code slice only.
