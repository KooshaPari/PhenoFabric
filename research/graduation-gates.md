# Research Graduation Gates

A research mechanism becomes an automatic product route only when all gates pass.

1. **Net benefit:** measured total cost beats the current valid route by a workload-specific minimum margin.
2. **Tail safety:** p99/worst and failure behavior pass, not just median.
3. **Mixed-load safety:** foreground game/audio SLOs remain protected.
4. **Correctness:** object authority, event ordering, input state and application behavior remain correct.
5. **Security:** privilege and cross-device threat review passes.
6. **Fallback:** route can be aborted/replanned without stranding input, surfaces or authoritative data.
7. **Explainability:** plan and rejection reasons are inspectable.
8. **Compatibility:** supported hardware/OS matrix is explicit and probed.
9. **Operationality:** install, update, downgrade, logs and cleanup are defined.
10. **Independent reproduction:** another agent/human can reproduce from a clean setup.

## Outcomes

- `graduated`: automatic candidate within declared matrix.
- `preview`: explicit opt-in and telemetry required.
- `narrowed`: useful only for named workloads/topologies.
- `rejected`: evidence retained; not re-opened without a changed assumption.
