# phenotype-fabric scout report — 2026-10-14

**Worker:** session_mouse (gpt-6-sol, completed)
**Repo:** `KooshaPari/PhenoFabric` (NOTE: requested `phenotype-fabric`, actual is `PhenoFabric` — case mismatch)
**HEAD:** 4a16747 on main

## State
- Main clean
- Latest 10 commits inspected
- **0 open PRs**

## Actionlint failures
- 7 release.yml findings:
  - `macos-13` (deprecated, twice)
  - unused `TAG` (3 times)
  - invalid `needs.identity` (twice)

## Polish
- No changes needed
- Doc link scan: 159 missing local references
  - Predominantly systematic links to absent document trees, not safe isolated typos

## Status
- Read-only maintained, no files changed
- Validation: git log -10, git status --short --branch, gh pr list, actionlint, scanned 178 tracked docs markdown files for local links

## Blockers / follow-ups
- **Repository name mismatch** between requested (`phenotype-fabric`) and actual (`PhenoFabric`)
- Release workflow lint findings need separate scoped remediation
- Missing docs targets require provenance assessment before changing links
