# Phenotype Fabric Identifier Scheme

**Status:** canonical (effective 2026-09-01)
**Owner:** spec 013-fabric-program-baseline / PF-WP-000.02
**Cross-references:** `program/boundaries.json`, `work/tasks.json`,
`verification/requirements-traceability.json`,
`ecosystem/event-contracts.json`

This file is the single source of truth for identifier formats in the
Phenotype Fabric program. Every spec, intent, ADR, task, requirement,
event, and ADR must use the format defined here.

## Identifier Formats

| Prefix | Domain | Format | Example | Source of truth |
|---|---|---|---|---|
| `PF-FR-NNN` | Functional requirement | `PF-FR-` + 3 digits | `PF-FR-001` | `FUNCTIONAL_REQUIREMENTS.md` |
| `PF-NFR-NNN` | Non-functional requirement | `PF-NFR-` + 3 digits | `PF-NFR-001` | `NON_FUNCTIONAL_REQUIREMENTS.md` |
| `PF-SR-NNN` | System requirement (cross-cutting) | `PF-SR-` + 3 digits | `PF-SR-001` | `SYSTEM_REQUIREMENTS.md` |
| `INT-NNN` | Intent entry (source prompt) | `INT-` + 3 digits | `INT-P001` | `intent/source-prompts.md` |
| `ADR-NNNN` | Architecture decision record | `ADR-` + 4 digits | `ADR-0001` | `adr/INDEX.md` |
| `TASK-NNN` | Atomic task | `TASK-` + 3 digits | `TASK-001` | `work/tasks.json` |
| `PF-WP-NNN` | Work package | `PF-WP-` + 3 digits | `PF-WP-010` | `work/wbs.md` |
| `PF-WP-NNN.NN` | Sub-task within WP | `PF-WP-` + 3 digits + `.NN` | `PF-WP-010.02` | `work/tasks.json` |
| `EVT-NNN` | Event contract entry | `EVT-` + 3 digits | `EVT-001` | `ecosystem/event-contracts.json` |
| `RISK-NNN` | Risk register entry | `RISK-` + 3 digits | `RISK-001` | `risks/risk-register.json` |
| `EX-NNN` | Experiment identifier | `EX-` + 3 digits | `EX-001` | `research/experiments/` |
| `HYP-NNN` | Research hypothesis | `HYP-` + 3 digits | `HYP-001` | `research/hypotheses.md` |
| `SOTA-NNN` | SOTA competitive analysis | `SOTA-` + 3 digits | `SOTA-001` | `sota/INDEX.md` |
| `R-N` | Release stage (R0, R1, ...) | `R` + digit | `R0` | `work/build-order.md` |
| `B-NNN` | Build (CI run) | `B-` + 3 digits | `B-001` | `.github/workflows/` |

## Ranges and Current Allocations

### `PF-FR-NNN` (Functional Requirements)

- `PF-FR-001`–`PF-FR-094` — pre-existing, see `FUNCTIONAL_REQUIREMENTS.md`
- `PF-FR-095`–`PF-FR-097` — reserved (capability inventory edge cases)
- `PF-FR-098` — Program-level rules (spec 013)
- `PF-FR-099` — Source confidence policy (spec 013)

### `PF-NFR-NNN` (Non-Functional Requirements)

- `PF-NFR-001`–`PF-NFR-051` — pre-existing, see `NON_FUNCTIONAL_REQUIREMENTS.md`
- `PF-NFR-052`+ — reserved for future expansion

### `INT-NNN` (Intent Entries)

- `INT-P001`–`INT-P008` — pre-existing (from spec 001)
- `INT-P009`–`INT-P011` — reserved (spec 013 program baseline)
- `INT-P012`–`INT-P014` — reserved (spec 014 capability inventory)
- `INT-P015`+ — reserved for future expansion

### `ADR-NNNN` (Architecture Decision Records)

- `ADR-0001`–`ADR-0022` — pre-existing, see `adr/INDEX.md`
- `ADR-0023`+ — new ADRs use this range

### `TASK-NNN` (Atomic Tasks)

- 137 pre-existing tasks in `work/tasks.json`
- Mapping: TASK-NNN = sequential insertion order in `work/tasks.json`
- The WBS ID `PF-WP-NNN.NN` is the source of truth for the WBS
  hierarchy. `TASK-NNN` is the per-row flat identifier for cross-references.

### `PF-WP-NNN` (Work Packages)

- `PF-WP-000` — Program baseline (R0)
- `PF-WP-010` — Capability inventory (R0)
- `PF-WP-020`–`PF-WP-200` — pre-allocated, see `work/wbs.md`

### `EVT-NNN` (Event Contracts)

- To be populated from `ecosystem/event-contracts.json`
- The file currently has prose; the EVT-NNN scheme is introduced by
  spec 013 / PF-WP-000.02. Population happens in a follow-up PR
  once the schema is agreed with thegent, ShareCLI, Tracera.

### `RISK-NNN` (Risk Register)

- To be populated from `risks/risk-register.md`
- See `risks/risk-register.json` for the structured form

## Migration / Renumbering

**No renumbering of existing identifiers.** The pre-existing IDs are
preserved. The TASK-NNN field is added to `work/tasks.json` alongside
the existing `wp` field (which holds `PF-WP-NNN.NN`).

| Old reference | New reference | Notes |
|---|---|---|
| `T001` (in spec 001) | `TASK-001` (in work/tasks.json) | T001 was local to spec 001's tasks table; now points to global index |
| "WP-001" (in spec 001) | `PF-WP-001` | Scope: shells + workspace model |
| Spec-local WP IDs | `PF-WP-NNN` (in work/wbs.md) | Single source of truth |
| `R0` (release) | `R0` (no change) | |

## Identifier Validation

The CI script `program/scripts/check_identifiers.py` (PF-WP-000.02
deliverable) verifies:
- Every `TASK-NNN` in `work/tasks.json` matches the row's
  `PF-WP-NNN.NN` field's first 7 chars
- Every `INT-NNN` referenced in any spec exists in
  `intent/source-prompts.md`
- Every `ADR-NNNN` referenced in any spec exists in `adr/INDEX.md`
- Every `PF-FR-NNN` / `PF-NFR-NNN` / `PF-SR-NNN` referenced in any spec
  exists in the corresponding `*_REQUIREMENTS.md`
- No orphan requirements (defined but never referenced)
- No orphan specs (defined but never referenced from tasks.json)

A CI run that fails this check cannot merge.

## Cross-references and the Web of Identifiers

```
spec.md   →  requirements (PF-FR-NNN, PF-NFR-NNN, PF-SR-NNN)
          →  intent (INT-NNN)
          →  ADRs (ADR-NNNN)
          →  tasks (PF-WP-NNN.NN, TASK-NNN)
tasks.md  →  requirements (PF-FR-NNN, PF-NFR-NNN, PF-SR-NNN)
          →  work packages (PF-WP-NNN)
event-contracts.json  →  events (EVT-NNN)
                      →  source spec(s)
risks/risk-register.json  →  risks (RISK-NNN)
                          →  mitigating ADRs
research/hypotheses.md    →  hypotheses (HYP-NNN)
                          →  experiments (EX-NNN)
                          →  source claims (SOTA-NNN)
sota/INDEX.md    →  SOTA entries (SOTA-NNN)
                  →  source claims with confidence (source-status.md)
```

## When to Add a New Identifier

- A new functional requirement is needed → add a row to
  `FUNCTIONAL_REQUIREMENTS.md` and assign the next free `PF-FR-NNN`.
  Cite it from any spec that depends on it.
- A new intent (user prompt) is needed → add a new numbered entry to
  `intent/source-prompts.md`. Do not rewrite history.
- A new ADR is needed → add a new ADR file in `adr/` and a row to
  `adr/INDEX.md` with the next free `ADR-NNNN`.
- A new atomic task is needed → add a row to `work/tasks.json` and
  assign the next free `TASK-NNN`.
- A new event contract is needed → add an entry to
  `ecosystem/event-contracts.json` with the next free `EVT-NNN`.
- A new SOTA analysis is needed → add a file in `sota/` and a row to
  `sota/INDEX.md` with the next free `SOTA-NNN`.

## Idempotence and Stability

Identifiers, once assigned, do not change. If a number is wrong, file
a corrective ADR (e.g. `ADR-0023-correct-TASK-042-misnumbering`) and
note the historical assignment in the relevant file. Never reuse a
released number for a different concept.
