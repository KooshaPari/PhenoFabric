# Cross-Product Event Contracts

| Event | Producer | Consumers | Minimum payload | Authority |
|---|---|---|---|---|
| fabric.device.capability.changed.v1 | Fabric endpoint | thegent, ShareCLI, hwLedger, UI | device/realm ID, descriptor version/hash, changed capabilities, probe evidence | Fabric live probe |
| fabric.route.committed.v1 | Fabric coordinator | SessionLedger, Tracera, UI | desired link, compiled stages, lease/fencing token, quality/security policy, evidence refs | Fabric |
| fabric.route.failed.v1 | Fabric coordinator/adapter | SessionLedger, Tracera, AgilePlus when gated | stage, reason, fallback, rollback result, trace ID | Fabric execution fact |
| fabric.execution.placed.v1 | Fabric scheduler | thegent, SessionLedger, Tracera, AGSLAG usage view | TaskSpec/region, node, objects, cost terms, alternatives, uncertainty | Fabric |
| fabric.execution.completed.v1 | Fabric runtime | thegent, Tracera, ledgers | result/artifacts, usage, topology, timing, failure/cancel state | Fabric fact; thegent owns task interpretation |
| fabric.object.residency.changed.v1 | Fabric object plane | scheduler, observability | object version, authority, replica/cache locations, transfer cause | Fabric runtime object plane |
| fabric.realm.ready.v1 | Fabric provider | thegent, agent, UI, SessionLedger | realm/seat/surface IDs, TTL, capabilities, fallback console | Fabric provider |
| fabric.attention.requested.v1 | agent/thegent via Fabric | UI/user policy | surface/reason/urgency/expiry; no focus grant | requesting principal; user policy decides |
| agileplus.work.authorized.v1 | AgilePlus | thegent, Fabric budget/policy cache | WorkPackage, requirement/evidence IDs, budget, policy | AgilePlus |
| thegent.task.requested.v1 | thegent | Fabric | TaskSpec, dependencies, desired surfaces, evidence refs | thegent execution intent |
| tracera.evidence.accepted.v1 | Tracera | AgilePlus, AGSLAG, UI | requirement/decision/run/artifact linkage and conclusion | Tracera evidence graph |

## Envelope

```protobuf
message PhenotypeEventEnvelope {
  string event_id = 1;
  string event_type = 2;
  uint32 schema_version = 3;
  string source_product = 4;
  string source_instance = 5;
  string subject_id = 6;
  string correlation_id = 7;
  string causation_id = 8;
  int64 occurred_unix_ns = 9;
  int64 observed_unix_ns = 10;
  bytes payload = 11;
  string content_sha256 = 12;
  repeated string evidence_uris = 13;
}
```

Events are append-only facts or requests. Cross-product commands use explicit APIs and idempotency keys; an event bus is not a shared mutable database.
