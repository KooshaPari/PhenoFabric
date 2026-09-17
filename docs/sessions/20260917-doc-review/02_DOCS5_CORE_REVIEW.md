# docs-5 Core Documents Review

**Reviewed:** 2026-09-17 | **Files read:** 19 core + 3 product dossiers + roster | **Source:** `/Downloads/docs-5/`

## What Is docs-5?

docs-5 is the **Phenotype Atlas and Assurance Program** (currently v1.4). It is a comprehensive program-level governance corpus covering:

- **166 proposed program requirements** and **132 planned work packages**
- **24 non-zz repositories** in the current roster
- Product dossiers, federation contracts, capability proofs, QA matrices, ADRs, specs, and prompts
- Three changelogs (v1.2, v1.3, v1.4) documenting incremental program additions

It is NOT a codebase. It is NOT deployed. It is a set of proposed work contracts, qualification procedures, and agent dispatch instructions.

## Per-File Assessment

| File | Rating | Key Content |
|------|--------|-------------|
| **README.md** | CRITICAL | Master index. Navigation to all sections. States v1.4 is current. 166 requirements, 132 work packages. |
| **START-HERE.md** | CRITICAL | Entry point for agents. One-chat-per-repository model (supersedes older ten-seat plan). Points to per-product START-HERE.md files. |
| **PRD.md** | CRITICAL | Problem: portfolio has more implementation than operator can comprehend. Goal: reduced maintenance burden and trustworthy products. Defines users, journeys, scope, success measures. |
| **HLD.md** | CRITICAL | High-level design. Not read in detail yet but referenced by START-HERE. |
| **LLD.md** | CRITICAL | Low-level design contracts. Referenced but not read in detail. |
| **ALD.md** | USEFUL | Abstraction-layer design. 6 layers. |
| **INDEX.md** | USEFUL | Master navigation with all document links and categories. |
| **SPECIFICATION.md** | USEFUL | Normative interpretation rules for the program. |
| **REQUIREMENTS.md** | USEFUL | 166 proposed requirements across all products. |
| **ROADMAP.md** | USEFUL | Phase A-D outcomes. No fabricated durations. |
| **DOMAIN_MODEL.md** | USEFUL | Domain model definitions. |
| **SSOT_AUTHORITY.md** | USEFUL | Single source of truth authority rules. Who writes what. |
| **OWNER-QUICKSTART.md** | USEFUL | Quick start for repository owners. |
| **REVISION-1.1.md** | STALE | Ecosystem-first consumer impact amendment. Superseded by v1.3/v1.4. |
| **TRACEABILITY.md** | USEFUL | Requirement-to-implementation traceability rules. |
| **VALIDATION_REPORT.md** | STALE | Validation of the docs-5 package itself (not products). Dated. |
| **VALIDATION-V1.2.md** | STALE | v1.2 validation scope. Superseded by v1.3/v1.4. |
| **CHANGELOG-V1.2.md** | STALE | Historical. Superseded. |
| **CHANGELOG-V1.3.md** | USEFUL | Federation amendment: 56 requirements, 56 acceptance scenarios, 12 work packages. |
| **CHANGELOG-V1.4.md** | USEFUL | Capability/design/proof/grading amendment. Latest changes. |

## Product Dossiers (Fabric, Khostty, Melosviz)

### PhenoFabric
- **Role:** Distributed real-time compute, data and I/O product
- **Status:** `UI_AUTH_INTEGRATION_PRODUCT_PATH_OPEN` (Sep 16 assessment)
- **Next actions:** Qualify auth contract, prove 2-node path, demonstrate locality
- **Current reality:** v0.1.0-nightly installed, SSO via system browser, workspace builds, 49% overall
- **Accuracy:** Dossier is DATED but directionally correct. Auth contract testing is exactly what we've been working on. The "prove 2-node path" is the two-node integration test we completed.

### Khostty
- **Role:** Owned terminal fork or embedded terminal surface
- **Status:** Proposed work program
- **Comparators:** Ghostty, WezTerm, Kitty
- **Current reality:** 80% synced upstream, needs delta decisions

### Melosviz
- **Role:** Music visualization authoring and rendering
- **Status:** Proposed work program
- **Current reality:** DROPPED (40%, handoff written)

### Portfolio Roster
- 24 non-zz repositories, each with 1 owner chat
- Includes .github, PhenoShared as special entries
- Stable IDs assigned (GitHub repo IDs)

## Relationship to docs-3

docs-3 (`~/Downloads/docs-3/`) is the **earlier version** of this same program. docs-5 is a direct evolution:
- docs-3 had 4 product dossiers (PhenoMLX, HeliosLab, PhenoShared, Portage)
- docs-5 expanded to 24 products with full per-product packet structure
- docs-5 added federation (v1.3), capability proof/grading (v1.4)
- docs-3's atlas questions and quality gates are inherited and expanded

**Overlap:** docs-3's `START-HERE.md`, `SSOT_AUTHORITY.md`, `PRD.md`, `REQUIREMENTS.md`, `SPECIFICATION.md` are all superseded by docs-5 versions. The `qa/` directory structure is the same.

## What's Useful for Repo Work

1. **Per-product dossiers** (PhenoFabric especially) -- atlas questions, next actions, delivery gates
2. **Federation contract** -- how apps compose across repos
3. **QA metrics and assurance contract** -- what "done" means
4. **Agent prompts** -- role definitions for workers
5. **One-chat-per-repository model** -- our current operating pattern

## What's Stale

- All STATE.md files are from September 16 snapshots -- will rot
- VALIDATION-V1.2 and VALIDATION_REPORT are superseded by v1.3/v1.4
- REVISION-1.1 is historical
- Many "proposed, not claimed or executed" statuses remain true

## Recommendation

- **Adopt per-product dossiers** as the canonical product documentation structure
- **Use docs-5 START-HERE.md** as the program entry point
- **Supersede docs-3** with docs-5 (mark docs-3 as historical)
- **Keep per-repo state fresh** by updating STATE.md as we make progress
