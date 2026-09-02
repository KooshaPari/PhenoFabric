# Fabric Program Baseline (R0 Foundation)

## Meta

- **ID:** `013-fabric-program-baseline`
- **Created:** 2026-09-01
- **State:** specified
- **Scope:** Program-level rules and contracts that the rest of Fabric is
  built on: product boundary freeze, identifier normalization,
  documentation-as-code enforcement, source-confidence policy, and release
  evidence contract.
- **Release gate:** R0 — must be complete before PF-WP-010 begins coding.
- **Requirement traces:** `PF-FR-098`, `PF-FR-099`
- **Intent traces:** `INT-P009`–`INT-P011`
- **Work package:** `PF-WP-000` (5 tasks: PF-WP-000.01 through PF-WP-000.05)

## Context

Phenotype Fabric is a 137-task program with 21 work packages across 7 release
stages (R0–R6). The first release, R0 ("Measurement and adapter lab"),
depends on five program-level rules that every other work package will
inherit. These are not implementation tasks; they are contracts.

Without these baselines, downstream WPs would each invent their own ID
schemes, release criteria, and evidence rules, leading to an unmanageable
program. This WP produces the *rules of the game*.

## Problem Statement

Before any runtime code is written, Fabric must:
1. Freeze what is in scope vs out of scope (per `ecosystem/boundaries.md`)
2. Establish a single identifier scheme (PF-FR-NNN, INT-NNN, ADR-NNN,
   TASK-NNN) so every spec, intent, ADR, and task in the program can be
   cross-referenced unambiguously.
3. Define what "valid documentation" means — broken cross-doc links,
   malformed JSON schemas, and missing manifests must fail CI.
4. Establish how confident we are in each external claim (SOTA benchmark,
   vendor doc, blog post) so the evidence rule (`GOVERNANCE.md`) is
   enforceable.
5. Define what a release *is* — what evidence must exist for an
   R-channel promotion.

## Goals

- One canonical, machine-checked product boundary map.
- One identifier scheme applied uniformly to all 137 existing tasks and
  all future tasks.
- A documentation-as-code pipeline that fails the build on any error.
- A source-confidence policy in `research/source-register.md` and a
  `references/source-status.md` table.
- A release evidence contract that gates R-channel promotion on a
  documented evidence bundle.

## Non-Goals

- Authoring any runtime code (no Rust, no Python — this is docs and
  schemas only).
- Resolving ADR-0020 (provisional name) — that requires PF-WP-010
  evidence.
- Building the product shell (that's `001-unified-product-shell`).
- Touching any other repo besides `phenotype-fabric` and the
  `meta/` cross-repo architecture file.

## User and System Outcomes

After this WP completes, the Fabric repo has:
- A `program/boundaries.json` machine-readable product boundary.
- A `program/identifiers.md` canonical scheme with renumbering of all
  137 existing tasks (preserved via a 1-to-1 mapping in the same file).
- A `program/doc-checks.yml` GitHub Action that runs the validation
  scripts in `CONTRIBUTING.md` and fails the build on errors.
- A `research/source-status.md` confidence table with at least the 14
  SOTA references and the 8 research hypotheses classified.
- A `work/release-evidence-contract.md` defining what evidence must
  exist for an R0 → R1 promotion.

## Functional Requirements

- `PF-FR-098` — Program rules and contracts are enforced in CI before
  any code merge.
- `PF-FR-099` — Every external claim is classified by source confidence
  before being cited as evidence.

## Technical Approach

### PF-WP-000.01 — Freeze provisional product boundary and authority map

- Author `program/boundaries.json` containing:
  - The 12 in-scope product specs (001–012)
  - The 13 adjacent products (AgilePlus, thegent, AGSLAG, Tracera,
    SessionLedger, ShareCLI, NVMS, labs-compute, ResearchLedger,
    RepoLedger, hwLedger, no-mistakes, fleet-ops) with
    `in_scope: false` and a one-line `responsibility` field
  - The 5 new specs in this WP and the next (013, 014) with
    `in_scope: true`
- Cross-link this file from `ecosystem/boundaries.md` (replace the
  prose with a reference to the JSON).
- Acceptance: any future PR that adds a new product to the boundary
  without updating this file fails CI.

### PF-WP-000.02 — Normalize requirement, intent, evidence, and event identifiers

- Adopt the schema:
  - `PF-FR-NNN` for functional requirements (existing 001–094 stays)
  - `PF-NFR-NNN` for non-functional requirements (existing 001–051 stays)
  - `INT-NNN` for intent entries (existing 001–050 stays)
  - `ADR-NNNN` for ADRs (existing 0001–0022 stays)
  - `TASK-NNN` for atomic tasks (existing 137 tasks stay; renumbering is
    cosmetic; the WBS ID `PF-WP-NNN.NN` is the source of truth and
    does NOT renumber)
  - `EVT-NNN` for event-contracts entries (new; populate from
    `ecosystem/event-contracts.json` — expected ~30 entries)
- Create `program/identifiers.md` as the canonical reference.
- Update `work/tasks.json` field `id` to `TASK-NNN` format and add
  field `wp` (already present) and `requirement_traces` (new).
- Acceptance: `python3 program/scripts/check_identifiers.py` exits 0.

### PF-WP-000.03 — Establish documentation-as-code checks

- Create `.github/workflows/doc-checks.yml` invoking the validation
  scripts already described in `CONTRIBUTING.md`:
  - JSON schema well-formedness
  - `event.proto` syntactic validation
  - `openapi.yaml` well-formedness
  - MANIFEST.sha256 vs file tree diff
  - Broken cross-doc link detection
- Add a `program/scripts/` directory with these scripts (currently
  inlined in CONTRIBUTING.md; extract for reuse).
- Acceptance: the GitHub Action runs and passes on the current archive
  state. Any commit that breaks a link, schema, or manifest entry
  fails the action.

### PF-WP-000.04 — Create source confidence and claim policy

- Create `research/source-status.md` with columns:
  `source_id | claim | confidence | primary_url | reproduced |
   date_verified | decays_after | replacement_evidence`
- Classify all 14 SOTA entries (deskflow, parsec, looking-glass, evdev,
  seamless-app, audio-network, moat-and-risk, etc.) with confidence
  `verified-by-paper`, `verified-by-vendor-doc`, `verified-by-repro`,
  or `claim-only`.
- Cross-link each SOTA entry's claim to a TASK-NNN that must reproduce
  it before the claim is used as evidence.
- Acceptance: every claim under `sota/` and `risks/legal-licensing.md`
  has a row in `source-status.md`.

### PF-WP-000.05 — Define release evidence contract

- Author `work/release-evidence-contract.md` with the template that
  every R-channel promotion must complete:
  - Architecture and authority claim evidence (refs to specs, ADRs)
  - Performance evidence (refs to `verification/benchmark-catalog.json`
    entries with full p50/p95/p99/worst results)
  - Failure and degradation evidence (refs to
    `verification/fault-catalog.json` entries)
  - Security evidence (refs to `risks/threat-model.md` mitigations)
  - Compatibility evidence (refs to
    `verification/compatibility-matrix.md`)
  - Rollback evidence (refs to `operations/recovery.md` procedures
    exercised on the reference environment)
- Acceptance: the file is referenced by `work/release-plan.md` and
  the R0 → R1 gate in `work/build-order.md` cannot be passed without
  the template fields populated.

## Risks and Open Questions

- **Risk:** Renumbering TASK-NNN might break downstream consumers
  (AgilePlus WBS integration). **Mitigation:** preserve the
  `PF-WP-NNN.NN` field as the source of truth; `TASK-NNN` is cosmetic.
- **Risk:** The doc-checks GitHub Action might catch many existing
  issues in the archive on first run. **Mitigation:** stage the
  action as `continue-on-error: true` for the first commit, then
  fix issues incrementally and re-enable strict mode.
- **Open:** The exact `EVT-NNN` schema for event contracts needs to
  match the consumers (thegent, ShareCLI, Tracera). Coordinate before
  populating the table.

## Exit Gate (R0 → R1)

This WP is done when:
1. All 5 sub-tasks above are complete.
2. The doc-checks GitHub Action runs green.
3. `work/build-order.md` is updated to mark PF-WP-000 as Done.
4. The `R0` label can be applied to the release channel.
