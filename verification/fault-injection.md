# Fault-Injection Catalog

| ID | Injected fault | Expected invariant / recovery |
|---|---|---|
| FI-001 | Kill coordinator before route prepare | No data-plane impact; request fails/retries safely. |
| FI-002 | Kill coordinator after prepare before commit | Prepared resources expire/abort; old route remains authoritative. |
| FI-003 | Partition coordinator from one endpoint | Stale fencing token rejected; no dual focus. |
| FI-004 | Kill source adapter mid-stream | Sink freezes/marks stale; audio/input return to safe policy; fallback offered. |
| FI-005 | Kill sink/compositor | Source virtual display/stream cleaned or parked after timeout. |
| FI-006 | Hold every modifier/button during switch | Old target receives releases; new target starts defined state. |
| FI-007 | Raw-relative game/CAD capture then network loss | Emergency chord/local control releases capture. |
| FI-008 | Encoder/decoder exhaustion | Compiler chooses valid lower path or rejects; no unbounded queue. |
| FI-009 | GPU reset/driver crash | Route fails explicit; VM/OOB fallback remains reachable. |
| FI-010 | Audio device disappears | Graph reconfigures or stops recording/monitoring explicitly; no silent wrong sink. |
| FI-011 | Clock jump/drift spike | Estimator rejects outlier; buffer stays bounded or route fails clearly. |
| FI-012 | Object authority node fails | Promote only valid replica by policy; no split writer. |
| FI-013 | Corrupt cached object | Hash/version detects and refetches; authority remains intact. |
| FI-014 | Storage full | Admission/spill fails before corrupting state; cleanup guidance emitted. |
| FI-015 | Network bandwidth step/loss burst | Declared quality ladder; RT queues protected. |
| FI-016 | Agent realm provider hangs | TTL/cancel/fallback console and resource reclamation. |
| FI-017 | Upgrade interrupted | Previous compatible core/helpers remain bootable or rollback. |
| FI-018 | OOB device unreachable | Secondary physical/local recovery path documented. |

Faults are injected at stage boundaries and under mixed load. “Reconnect succeeded” is not enough: leases, authoritative objects, held input state, audio recording and cleanup must be checked.
