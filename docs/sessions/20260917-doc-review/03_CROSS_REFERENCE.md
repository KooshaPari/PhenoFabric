# Cross-Reference: Canonicalization vs docs-5

**Reviewed:** 2026-09-17

## Relationship

| Aspect | Canonicalization | docs-5 |
|--------|-----------------|--------|
| **Date** | Sep 16, 2026 | Sep 16, 2026 (same day) |
| **Version** | v0.1 (research) | v1.4 (program) |
| **Purpose** | Raw findings + proposed tooling decisions | Formalized program governance |
| **Scope** | 46 repos discovered, 9 inspected | 24 non-zz repos with dossiers |
| **Authority** | "Not deployed policy" | "Proposed work contracts" |
| **Status** | Predecessor input | Successor formalization |

## What Overlaps

1. **Tool drift findings** (canonicalization F05) inform docs-5's tooling specs
2. **PhenoShared identity confusion** (canonicalization F01) is the root of docs-5's SSOT_AUTHORITY rules
3. **CI reference breakage** (canonicalization F09) maps to docs-5's federation WORK-PLAN
4. **Tracera typecheck hazard** (canonicalization F02/F03) is the subject of SPEC-001

## What's Unique to Each

**Canonicalization only:**
- Concrete blob SHAs and source indexes
- Specific tool versions (TypeScript 5.8.3 counterexample)
- Python 3.14t qualification protocol
- Oxlint/Oxfmt/Bun selection rationale

**docs-5 only:**
- 166 formalized requirements
- Federation contract (app composition)
- Capability proof/grading system
- Per-product dossiers with atlas questions
- Agent prompt roles (Master Coordinator, Product Owner, etc.)
- QA matrix with 85% coverage floors

## Recommendation

Both directories are valuable and should be preserved:
- **canonicalization/** = research evidence base (read-only, dated)
- **docs-5/** = program governance (living, evolving)

Neither should be deleted. Canonicalization findings are cited by docs-5 ADRs and specs.
