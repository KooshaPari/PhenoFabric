# Plan — Spec 027 (proposed)

## 0. Documentation and provenance

Record INT-P009..012 verbatim, synthesized meaning, requirement IDs, competing alternatives, ADR and traceability. Preserve prior FR/ADR IDs.

## 1. Local authority-domain baseline

Use existing Fabric SQLite migration and a short atomic transaction for scope/reservation/event/outbox. Pure ShareCLI guards validate selected local invariants. Verify migration and tests on exact branch. A database trigger does not protect against a malicious database owner.

## 2. Authenticating and enforcing effects

Bind actor, grant, scope, exact effect and epoch; use independent trusted policy and OS/provider adapters. Verify reclamation physically, not by the existence of a receipt string.

## 3. ShareCLI two-consumer acceptance

Two actors request compatible work, share one producer and keep separate permissions/deadlines. Cancel one, join another concurrently, restart supervisor, and independently accept outputs. Compare to native cache/queue baselines.

## 4. Fabric cross-host acceptance

Prepare/commit capacity and residency, fence stale workers at object/effect targets, preserve WorkItem across attempts, reconcile uncertain external commits and offline-rights partitions. Detect parent membership conflict under concurrency.

## 5. Release gate and evidence

Run fmt, clippy, cargo test and native faults, evidence/traceability checks and SOTA comparison. No upgrade to accepted claims until exact commit receipts and independent verifier acceptance are recorded.
