# Genesis — global immutable transaction model

**2026-10-12 source:** [INT-P009..012](../../intent/000-source-prompts.md).

Initial user problem: system-level actors require autonomy while individual actions may be beneficial or harmful depending on context. The user corrected the architecture from potentially agent-local history to **one globally logical nested tree**, and required **Human/SoftwareAgent to share an Actor parent**. This research belongs to ShareCLI and PhenoFabric, with registry oversight of product owners.

The derived approach scopes authorization to the enclosing commitment and shared physical resource facts. A correct work plan may be split, reused, cancelled or migrated without multiplying grant authority or changing what is owed to each consumer.

Alternative interpretations still worth testing: after-the-fact queues/logs, domain-specific SQL plus outbox and effect-broker, a long-running workflow engine, or a bespoke global distributed ledger. One global semantic tree does not settle physical data placement, conflict resolution, access privacy, or irreversible compensation.

Do not label the exact protocol/SQL implementation a verbatim user demand. It is a falsifiable engineering proposal bound to [ADR 0036](../adr/0036-global-nested-transaction-ledger.md).
