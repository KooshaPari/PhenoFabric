# Tasks: Security, Identity, Leases, Audit, and Evidence

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for threat model and capability matrix | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for threat model and capability matrix | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for threat model and capability matrix | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T004 | WP-002 | Define contracts and acceptance criteria for device identity and pairing | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for device identity and pairing | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for device identity and pairing | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T007 | WP-003 | Define contracts and acceptance criteria for authorization token and policy engine | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for authorization token and policy engine | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for authorization token and policy engine | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T010 | WP-004 | Define contracts and acceptance criteria for privileged helper isolation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for privileged helper isolation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for privileged helper isolation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T013 | WP-005 | Define contracts and acceptance criteria for lease/fencing service | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for lease/fencing service | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for lease/fencing service | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T016 | WP-006 | Define contracts and acceptance criteria for audit/evidence bundle format | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for audit/evidence bundle format | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for audit/evidence bundle format | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T019 | WP-007 | Define contracts and acceptance criteria for oob management isolation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for oob management isolation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for oob management isolation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T022 | WP-008 | Define contracts and acceptance criteria for red-team, partition, and recovery validation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for red-team, partition, and recovery validation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for red-team, partition, and recovery validation | Planned | PF-FR-080, PF-FR-081, PF-FR-082 |

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
