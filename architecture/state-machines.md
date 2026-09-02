# State Machines

## Route lifecycle

```mermaid
stateDiagram-v2
    [*] --> Proposed
    Proposed --> Compiled
    Compiled --> Rejected
    Compiled --> Preparing
    Preparing --> Aborted
    Preparing --> Ready
    Ready --> Active
    Active --> Degraded
    Degraded --> Preparing: replan
    Active --> Draining
    Draining --> Stopped
    Aborted --> [*]
    Rejected --> [*]
    Stopped --> [*]
```

## Exclusive focus lease

```mermaid
stateDiagram-v2
    [*] --> Unowned
    Unowned --> Granted
    Granted --> Renewing
    Renewing --> Granted
    Granted --> Revoking
    Revoking --> Granted: new higher token
    Granted --> Expired
    Expired --> Unowned
```

Endpoint invariant: `token < lastAppliedToken` is rejected.

## Realm lifecycle

```mermaid
stateDiagram-v2
    Requested --> Provisioning
    Provisioning --> Enrolling
    Enrolling --> Ready
    Ready --> Active
    Active --> Idle
    Idle --> Active
    Active --> Expiring
    Idle --> Expiring
    Expiring --> Revoked
    Revoked --> Cleaning
    Cleaning --> Destroyed
    Provisioning --> Failed
    Enrolling --> Failed
```

## Object lifecycle

```text
declared → materializing → resident → replicated/cached
→ pinned/consumed → evictable → spilled/evicted
→ restored or garbage-collected
```

Authority and durable-copy checks gate deletion.

## Workspace activation

```text
load snapshot
→ diff current desired graph
→ authorize
→ compile affected routes
→ admission
→ prepare
→ atomic commit
→ drain old routes
→ persist evidence
```

Activation is idempotent by workspace version and transaction ID.
