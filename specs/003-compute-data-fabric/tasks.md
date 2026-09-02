# Tasks: Adaptive Heterogeneous Compute and Data Fabric

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for resource/topology descriptor and probes | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for resource/topology descriptor and probes | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for resource/topology descriptor and probes | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T004 | WP-002 | Define contracts and acceptance criteria for object identity, residency, local shared store | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for object identity, residency, local shared store | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for object identity, residency, local shared store | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T007 | WP-003 | Define contracts and acceptance criteria for taskspec and execution-region api | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for taskspec and execution-region api | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for taskspec and execution-region api | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T010 | WP-004 | Define contracts and acceptance criteria for baseline cost model and scheduler | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for baseline cost model and scheduler | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for baseline cost model and scheduler | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T013 | WP-005 | Define contracts and acceptance criteria for sharecli/nvms/thegent execution adapters | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for sharecli/nvms/thegent execution adapters | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for sharecli/nvms/thegent execution adapters | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T016 | WP-006 | Define contracts and acceptance criteria for prefetch, replication, spill, and result routing | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for prefetch, replication, spill, and result routing | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for prefetch, replication, spill, and result routing | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T019 | WP-007 | Define contracts and acceptance criteria for fusion/fission and predictive placement | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for fusion/fission and predictive placement | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for fusion/fission and predictive placement | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T022 | WP-008 | Define contracts and acceptance criteria for selective interposition and mobility experiments | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for selective interposition and mobility experiments | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for selective interposition and mobility experiments | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T025 | WP-009 | Define contracts and acceptance criteria for adversarial mixed-workload evaluation | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T026 | WP-009 | Implement or integrate the minimum vertical slice for adversarial mixed-workload evaluation | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |
| T027 | WP-009 | Add unit, contract, failure, and performance evidence for adversarial mixed-workload evaluation | Planned | PF-FR-060, PF-FR-061, PF-FR-062 |

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
