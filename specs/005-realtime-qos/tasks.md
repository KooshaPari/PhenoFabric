# Tasks: Real-Time QoS, Admission, and Contention Control

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for service-class and deadline schema | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for service-class and deadline schema | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for service-class and deadline schema | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T004 | WP-002 | Define contracts and acceptance criteria for cpu scheduling/isolation adapters | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for cpu scheduling/isolation adapters | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for cpu scheduling/isolation adapters | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T007 | WP-003 | Define contracts and acceptance criteria for gpu/codec/vram pressure and reservations | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for gpu/codec/vram pressure and reservations | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for gpu/codec/vram pressure and reservations | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T010 | WP-004 | Define contracts and acceptance criteria for storage/network/memory pressure control | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for storage/network/memory pressure control | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for storage/network/memory pressure control | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T013 | WP-005 | Define contracts and acceptance criteria for admission and degradation engine | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for admission and degradation engine | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for admission and degradation engine | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T016 | WP-006 | Define contracts and acceptance criteria for ableton rt island prototype | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for ableton rt island prototype | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for ableton rt island prototype | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T019 | WP-007 | Define contracts and acceptance criteria for gaming/frame-time protection prototype | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for gaming/frame-time protection prototype | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for gaming/frame-time protection prototype | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T022 | WP-008 | Define contracts and acceptance criteria for mixed workload/fault validation | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for mixed workload/fault validation | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for mixed workload/fault validation | Planned | PF-FR-056, PF-FR-070, PF-FR-071 |

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
