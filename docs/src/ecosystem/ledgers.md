# Ledger Integrations

| Ledger | Fabric publishes | Fabric consumes | Authority rule |
|---|---|---|---|
| SessionLedger | run/session IDs, stage summaries, logs/artifact bundles, route lifecycle | prior run context and replay references | SessionLedger owns durable session history; Fabric owns current execution state. |
| ResearchLedger | experiment runs, source/version fingerprints, result refs | claims, sources, confidence and open questions | ResearchLedger owns research knowledge; Fabric owns reproduced runtime measurements. |
| RepoLedger | build/test/runtime associations, repo/artifact hashes | repo identity, language/toolchain, cache hints | RepoLedger owns repository inventory; Fabric may cache it. |
| hwLedger | utilization/capability evidence and failure observations | asset identity, acquisition/cost/repair history | hwLedger owns hardware capital record; Fabric probes live capability. |

## Shared identifiers

Identifiers are namespaced and globally unique enough for cross-reference. An event includes source product, schema version, subject ID, causation/correlation IDs, occurred/observed timestamps, content hash and evidence URI. A ledger can reject, reconcile or supersede events without writing Fabric’s store.
