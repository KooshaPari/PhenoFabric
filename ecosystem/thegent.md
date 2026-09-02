# thegent Integration

## Role

thegent is the labor/execution-intent plane. It decomposes authorized work into agents, tools, retries, dependencies and worker lifecycle. Fabric is the physical substrate available to those workers.

## Interface

```text
thegent Plan/TaskSpec
  ├─ command/tool/model
  ├─ dependency and artifact refs
  ├─ security/realm constraints
  ├─ budget and deadline
  ├─ desired published surfaces
  └─ evidence IDs
          │
          ▼
Fabric execution plan
  ├─ realm/node placement
  ├─ object prefetch/residency
  ├─ CPU/GPU/codec/network reservation
  ├─ interactive surface routes
  ├─ cancellation/fallback
  └─ run/trace identifiers
```

## Boundary

Fabric does not create an agent org chart, choose product strategy or decide that work should exist. thegent does not hardcode “run on GPU 0” when the requirement is a capability/deadline; it may pin physical placement only when policy or experiment demands it.
