# Tasks: Phenotype Ecosystem Integration and Product Boundaries

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for authority and entity ownership matrix | Planned | PF-FR-086, PF-FR-087 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for authority and entity ownership matrix | Planned | PF-FR-086, PF-FR-087 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for authority and entity ownership matrix | Planned | PF-FR-086, PF-FR-087 |
| T004 | WP-002 | Define contracts and acceptance criteria for shared id/event envelope schema | Planned | PF-FR-086, PF-FR-087 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for shared id/event envelope schema | Planned | PF-FR-086, PF-FR-087 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for shared id/event envelope schema | Planned | PF-FR-086, PF-FR-087 |
| T007 | WP-003 | Define contracts and acceptance criteria for sharecli adapter | Planned | PF-FR-086, PF-FR-087 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for sharecli adapter | Planned | PF-FR-086, PF-FR-087 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for sharecli adapter | Planned | PF-FR-086, PF-FR-087 |
| T010 | WP-004 | Define contracts and acceptance criteria for nvms/thegent runtime contracts | Planned | PF-FR-086, PF-FR-087 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for nvms/thegent runtime contracts | Planned | PF-FR-086, PF-FR-087 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for nvms/thegent runtime contracts | Planned | PF-FR-086, PF-FR-087 |
| T013 | WP-005 | Define contracts and acceptance criteria for agileplus/agslag policy inputs | Planned | PF-FR-086, PF-FR-087 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for agileplus/agslag policy inputs | Planned | PF-FR-086, PF-FR-087 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for agileplus/agslag policy inputs | Planned | PF-FR-086, PF-FR-087 |
| T016 | WP-006 | Define contracts and acceptance criteria for tracera/sessionledger evidence outputs | Planned | PF-FR-086, PF-FR-087 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for tracera/sessionledger evidence outputs | Planned | PF-FR-086, PF-FR-087 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for tracera/sessionledger evidence outputs | Planned | PF-FR-086, PF-FR-087 |
| T019 | WP-007 | Define contracts and acceptance criteria for labs-compute graduation process | Planned | PF-FR-086, PF-FR-087 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for labs-compute graduation process | Planned | PF-FR-086, PF-FR-087 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for labs-compute graduation process | Planned | PF-FR-086, PF-FR-087 |
| T022 | WP-008 | Define contracts and acceptance criteria for end-to-end intent-to-evidence demonstration | Planned | PF-FR-086, PF-FR-087 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for end-to-end intent-to-evidence demonstration | Planned | PF-FR-086, PF-FR-087 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for end-to-end intent-to-evidence demonstration | Planned | PF-FR-086, PF-FR-087 |

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
