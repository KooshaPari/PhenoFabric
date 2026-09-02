# Human and Agent Execution Plan

## Lanes

| Lane | Human accountability | Agent leverage | Non-delegable gate |
|---|---|---|---|
| Architecture/governance | Authority boundaries, accepted ADRs, scope | Source synthesis, traceability, alternative generation | Product/authority decisions |
| Kernel/platform | Privileged helpers, driver design, OS APIs | Code generation, test scaffolding, compatibility research | Signing, threat review, physical testing |
| Media/RT | Clock, buffer, codec and display policy | Bench automation, data analysis, adapter implementation | Listening/visual acceptance and RT safety |
| Runtime/data | Scheduler, object authority, cost model | Simulation, trace analysis, backend adapters | Correctness model and promotion decisions |
| Product/UI | Simple workflows, accessibility, error recovery | UI implementation and test generation | User acceptance and interaction choices |
| QE/security | Evidence, fault, adversarial testing | Massive matrix execution and regression triage | Release gate and risk acceptance |

## Agent operating rule

Agents receive bounded WorkPackage/TaskSpec objects with file scope, hardware scope, requirement IDs, evidence contract and rollback. Agents may run parallel research or implementation, but privileged changes, performance claims and release transitions require reproduced evidence.

## Parallelism ceiling

Parallelism is constrained by integration surfaces and physical test fixtures, not available model calls. No more than one agent lane edits a privileged adapter or schema authority at once; independent platform adapters and benchmark families may fan out.
