# Tasks: Agent-Created Realms and Non-Stealing Surface Publication

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for realm and publication schemas | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for realm and publication schemas | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for realm and publication schemas | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T004 | WP-002 | Define contracts and acceptance criteria for provisioning adapter contract | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for provisioning adapter contract | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for provisioning adapter contract | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T007 | WP-003 | Define contracts and acceptance criteria for enrollment and short-lived identity | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for enrollment and short-lived identity | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for enrollment and short-lived identity | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T010 | WP-004 | Define contracts and acceptance criteria for surface catalog and attention workflow | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for surface catalog and attention workflow | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for surface catalog and attention workflow | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T013 | WP-005 | Define contracts and acceptance criteria for ttl, budget, cleanup, and fallback | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for ttl, budget, cleanup, and fallback | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for ttl, budget, cleanup, and fallback | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T016 | WP-006 | Define contracts and acceptance criteria for thegent/sharecli/nvms integration | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for thegent/sharecli/nvms integration | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for thegent/sharecli/nvms integration | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T019 | WP-007 | Define contracts and acceptance criteria for sessionledger/tracera event export | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for sessionledger/tracera event export | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for sessionledger/tracera event export | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T022 | WP-008 | Define contracts and acceptance criteria for abuse, scale, and failure testing | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for abuse, scale, and failure testing | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for abuse, scale, and failure testing | Planned | PF-FR-026, PF-FR-037, PF-FR-085 |

## Verification Checklist

- [ ] Schema/API contract tests.
- [ ] Simulator/loopback tests.
- [ ] Reference topology happy path.
- [ ] Concurrent gaming/build/agent load where relevant.
- [ ] Ableton/audio RT impact where relevant.
- [ ] Network loss/jitter/partition where relevant.
- [ ] Endpoint/adapter crash and rollback.
- [ ] Permission denied/revoked.
- [ ] Evidence bundle with FR/NFR/WP/task IDs.
- [ ] Documentation and operator runbook.
