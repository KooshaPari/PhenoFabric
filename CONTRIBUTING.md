# Documentation and Engineering Contribution Rules

1. Trace every behavioral change to an intent/requirement/spec or propose the missing requirement.
2. Add or update an ADR when ownership, trust, persistence, protocol, performance model or fallback changes.
3. Do not call a benchmark “latency” without naming its start and end boundary.
4. Do not claim a product/adapter capability from marketing alone; link a primary source and reproduce before promotion.
5. Preserve failed experiments and rejected alternatives.
6. Keep privileged code narrow; explain why an unprivileged/platform-supported mechanism is insufficient.
7. Do not add one more top-level service/repository when an adapter or package boundary is adequate.
8. Treat the exact prompts under `intent/` as immutable source records; add new prompts as new entries rather than rewriting history.
9. Update WBS, traceability, tests, risks, source register and compatibility data when behavior changes.
10. Run validation and regenerate index/tree/manifest before packaging.
