# AgilePlus Integration

## Current identity

AgilePlus is the governed intent/work plane: requirements, feature lifecycle, WorkPackages, DAG dependencies, acceptance criteria, governance contracts, evidence requirements and audit transitions.

## Contract with Fabric

AgilePlus emits an authorized WorkPackage or execution constraint set. Fabric does not infer product requirements from process behavior.

```yaml
work_package:
  id: AP-WP-483
  requirement_ids: [PF-FR-056, PF-FR-072]
  acceptance_contract: evidence://ableton-mixed-load-v1
  budget:
    gpu_seconds: 1800
    cpu_core_hours: 12
  execution_policy:
    may_use_preview_routes: false
    foreground_class: RT0
```

Fabric returns run, topology, route, placement and artifact references. AgilePlus uses Tracera evidence links to decide whether its transition gate is satisfied.

## Boundary

- AgilePlus says what must be true and what evidence closes work.
- thegent says what labor/task plan to run.
- Fabric says where/how the physical work and presentation execute.
- Tracera says what evidence links prove or contradict the requirement.
