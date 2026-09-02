# Tasks: Locality-Aware Route Compiler and Transport Plane

## Task Rules

- Every task references a work package and requirement IDs.
- Implementation tasks begin only after their governing contract task.
- “Research” tasks must retain raw results and a falsification conclusion.
- No task may mark a performance claim complete from an idle-host demo alone.
- Platform-specific code includes uninstall/rollback and permission-denial tests.

## Task Catalog

| Task ID | WP | Task | State | Requirement traces |
|---|---|---|---|---|
| T001 | WP-001 | Define contracts and acceptance criteria for locality and stage schema | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T002 | WP-001 | Implement or integrate the minimum vertical slice for locality and stage schema | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T003 | WP-001 | Add unit, contract, failure, and performance evidence for locality and stage schema | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T004 | WP-002 | Define contracts and acceptance criteria for topology/capability probes | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T005 | WP-002 | Implement or integrate the minimum vertical slice for topology/capability probes | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T006 | WP-002 | Add unit, contract, failure, and performance evidence for topology/capability probes | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T007 | WP-003 | Define contracts and acceptance criteria for same-os/same-host route implementations | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T008 | WP-003 | Implement or integrate the minimum vertical slice for same-os/same-host route implementations | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T009 | WP-003 | Add unit, contract, failure, and performance evidence for same-os/same-host route implementations | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T010 | WP-004 | Define contracts and acceptance criteria for kvmfr/ivshmem/virtio adapters | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T011 | WP-004 | Implement or integrate the minimum vertical slice for kvmfr/ivshmem/virtio adapters | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T012 | WP-004 | Add unit, contract, failure, and performance evidence for kvmfr/ivshmem/virtio adapters | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T013 | WP-005 | Define contracts and acceptance criteria for lan direct transport and quality negotiation | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T014 | WP-005 | Implement or integrate the minimum vertical slice for lan direct transport and quality negotiation | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T015 | WP-005 | Add unit, contract, failure, and performance evidence for lan direct transport and quality negotiation | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T016 | WP-006 | Define contracts and acceptance criteria for wan/nat/relay transport | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T017 | WP-006 | Implement or integrate the minimum vertical slice for wan/nat/relay transport | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T018 | WP-006 | Add unit, contract, failure, and performance evidence for wan/nat/relay transport | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T019 | WP-007 | Define contracts and acceptance criteria for compiler cost model and explanations | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T020 | WP-007 | Implement or integrate the minimum vertical slice for compiler cost model and explanations | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T021 | WP-007 | Add unit, contract, failure, and performance evidence for compiler cost model and explanations | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T022 | WP-008 | Define contracts and acceptance criteria for transactional replan and fallback | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T023 | WP-008 | Implement or integrate the minimum vertical slice for transactional replan and fallback | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T024 | WP-008 | Add unit, contract, failure, and performance evidence for transactional replan and fallback | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T025 | WP-009 | Define contracts and acceptance criteria for topology and contention benchmark matrix | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T026 | WP-009 | Implement or integrate the minimum vertical slice for topology and contention benchmark matrix | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |
| T027 | WP-009 | Add unit, contract, failure, and performance evidence for topology and contention benchmark matrix | Planned | PF-FR-012, PF-FR-013, PF-FR-014 |

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
