# Tasks: Universal I/O Graph, Seats, and Focus

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for port type and graph transaction schemas | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for port type and graph transaction schemas | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for port type and graph transaction schemas | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T004 | WP-002 | Define contracts and acceptance criteria for linux evdev/uinput broker | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for linux evdev/uinput broker | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for linux evdev/uinput broker | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T007 | WP-003 | Define contracts and acceptance criteria for wayland libei/portal adapter | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for wayland libei/portal adapter | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for wayland libei/portal adapter | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T010 | WP-004 | Define contracts and acceptance criteria for windows input endpoint | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for windows input endpoint | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for windows input endpoint | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T013 | WP-005 | Define contracts and acceptance criteria for macos input endpoint | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for macos input endpoint | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for macos input endpoint | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T016 | WP-006 | Define contracts and acceptance criteria for lease, fencing, and pressed-state reconciliation | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for lease, fencing, and pressed-state reconciliation | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for lease, fencing, and pressed-state reconciliation | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T019 | WP-007 | Define contracts and acceptance criteria for hotkeys, osd, and break-glass path | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for hotkeys, osd, and break-glass path | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for hotkeys, osd, and break-glass path | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T022 | WP-008 | Define contracts and acceptance criteria for fuzz, fault, and latency validation | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for fuzz, fault, and latency validation | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for fuzz, fault, and latency validation | Planned | PF-FR-010, PF-FR-011, PF-FR-015 |

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
