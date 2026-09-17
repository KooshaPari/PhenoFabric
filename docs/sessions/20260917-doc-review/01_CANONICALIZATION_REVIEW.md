# Phenotype Canonicalization Review

**Reviewed:** 2026-09-17 | **Files:** 6 | **Source:** `/Downloads/phenotype-canonicalization/`

## Summary

A research packet (v0.1, September 16, 2026) that audits the Phenotype ecosystem's tooling, architecture patterns, shared consumption, and optimization. It contains **15 findings, 24 proposed decisions, 13 component profiles, and 10 work packages** across 46 discovered repositories (9 actually inspected).

## Per-File Assessment

| File | Rating | Summary |
|------|--------|---------|
| **README.md** | USEFUL | Entry point. Defines reading order, scope (46 repos discovered, 9 inspected), and tool usage. States this is "proposed operationalization, not deployed policy." |
| **REPORT.md** | CRITICAL | Core diagnosis. Identifies broken chain between intent, decision, ownership, implementation, consumer use, and verification. 15 concrete findings (F01-F15) with evidence. Proposes tooling selections: Oxlint, Oxfmt, native TS7, Bun, uv, CPython 3.14t, Ruff, Lefthook, FastMCP. |
| **AGENT-HANDOFF.md** | USEFUL | Worker instructions. Defines 3 parallel work streams: verification (Tracera typecheck), authority/integration (PhenoShared), toolchain (OXC/TS/Bun/uv). Clear prohibited shortcuts list. |
| **AUDIT-RUNBOOK.md** | USEFUL | Step-by-step audit methodology: preserve state, inventory components, trace history, resolve deps, qualify toolchains, qualify verifier, qualify workflows. Highly structured. |
| **SOURCE-INDEX.md** | STALE | Indexes 17 sources (G01-G17) from GitHub with blob SHAs. All dated September 16. Useful for traceability but will rot as repos change. |
| **EXPERIMENT-PROTOCOL.md** | USEFUL | Defines how to evaluate language/library candidates. Measurement matrix, decision criteria, promotion process. Not executed yet. |

## Key Findings Worth Preserving

1. **Tool drift is real**: Tracera uses Oxlint/Oxfmt, HeliosLab uses Biome, OmniRoute uses ESLint. Three different linters across the ecosystem.
2. **False-confidence typechecking**: Tracera's `tsc -p` repeated flags don't guarantee coverage. A TypeScript 5.8.3 counterexample was reproduced.
3. **PhenoShared has conflicting identities**: README describes PhenoMLX/OMLX, npm names `phenodocs`, Cargo identifies substrate workspace.
4. **Broken CI references**: AgilePlus calls `phenotype-tooling/.github/workflows/sbom-monthly.yml@main` which returns 404.
5. **Proposed canonical tooling** (not deployed): Oxlint/Oxfmt, native TS7, Bun, uv, Python 3.14t, Ruff/ty, Lefthook, FastMCP.

## What's Useful for Current Work

- **REPORT.md** findings (F01-F15) are directly actionable for repo-level cleanup
- **AGENT-HANDOFF.md** work packages map to concrete tasks
- **Tooling selections** inform what to standardize on across repos
- **AUDIT-RUNBOOK.md** methodology is reusable for future audits

## What's Stale / Superseded

- **SOURCE-INDEX.md**: All blob SHAs are from Sep 16. Will rot. Reference only.
- The packet explicitly says it's "not deployed policy" -- all decisions are proposed, not accepted.
- References to `phenotype-tooling` are 404. Provider identity unresolved.

## Relationship to docs-5

This canonicalization packet is a **predecessor input** to docs-5. It provides the raw findings and proposed decisions that docs-5 formalizes into ADRs, specs, and federation contracts. The REPORT.md findings are the evidence base for docs-5's program structure.

## Recommendation

**Absorb into repo docs:**
- TOOLING_DECISIONS.md -- the proposed canonical tooling table from REPORT.md
- AUDIT_FINDINGS.md -- the F01-F15 findings with current repair status
- Keep REPORT.md as historical reference in docs/sessions/

**Do not absorb:**
- SOURCE-INDEX.md (rotting blob SHAs)
- AUDIT-RUNBOOK.md (methodology -- keep as reference, not repo canon)
