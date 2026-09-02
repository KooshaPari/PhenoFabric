# Tasks: Packaging, Deployment, Upgrade, and Recovery

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for component/privilege manifest | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for component/privilege manifest | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for component/privilege manifest | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T004 | WP-002 | Define contracts and acceptance criteria for linux packaging and service lifecycle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for linux packaging and service lifecycle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for linux packaging and service lifecycle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T007 | WP-003 | Define contracts and acceptance criteria for windows packaging and driver lifecycle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for windows packaging and driver lifecycle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for windows packaging and driver lifecycle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T010 | WP-004 | Define contracts and acceptance criteria for macos packaging/notarization/permissions | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for macos packaging/notarization/permissions | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for macos packaging/notarization/permissions | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T013 | WP-005 | Define contracts and acceptance criteria for enrollment, config, and schema migration | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for enrollment, config, and schema migration | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for enrollment, config, and schema migration | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T016 | WP-006 | Define contracts and acceptance criteria for update, repair, rollback, uninstall | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for update, repair, rollback, uninstall | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for update, repair, rollback, uninstall | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T019 | WP-007 | Define contracts and acceptance criteria for recovery/oob and diagnostic bundle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for recovery/oob and diagnostic bundle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for recovery/oob and diagnostic bundle | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T022 | WP-008 | Define contracts and acceptance criteria for release channels and clean-machine acceptance | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for release channels and clean-machine acceptance | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for release channels and clean-machine acceptance | Planned | PF-FR-001, PF-FR-003, PF-FR-006 |

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
