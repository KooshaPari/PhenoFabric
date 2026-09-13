# Tracera Integration

## Role

Tracera is the evidence/digital-thread graph, not generic telemetry. Fabric supplies evidence-ready facts about physical execution; Tracera connects them to requirements, decisions, code, tests and artifacts.

## Cross-linked projections

```text
Tracera semantic graph                  Fabric runtime graph
----------------------                  --------------------
Requirement PF-FR-056 ───────────────▶  RoutePlan RP-918
Test PF-AT-ABLETON-057 ───────────────▶ ExecutionRun RUN-204
Evidence EV-8001 ◀────────────────────  audio xrun trace
Artifact ART-77 ◀─────────────────────  topology + raw samples
Decision ADR-0008 ◀──────────────────── admission outcome
```

## Evidence event

Fabric emits content-addressed evidence references and summaries, not an assertion that a product requirement passed. Tracera evaluates/records the linkage under the owning governance contract.

## Boundary

High-volume raw telemetry may live in Fabric/observability storage. Tracera stores durable semantic references, provenance, hashes and conclusions appropriate to the evidence graph.
