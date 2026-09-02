# Upgrade, Compatibility, and Rollback

## Independent version axes

- graph/control schema;
- endpoint daemon;
- privileged helper/driver;
- adapter protocol;
- workspace file format;
- cross-product event schema.

A single marketing version maps to a tested compatibility manifest but does not erase component versions.

## Upgrade transaction

1. Fetch and verify signed manifests/artifacts.
2. Check OS/hardware/driver and peer compatibility.
3. Quiesce only affected routes.
4. Snapshot workspace, policy and credential metadata.
5. Upgrade unprivileged component, then helper if required.
6. Run local self-test and selected route canary.
7. Commit version lease or roll back.

## Safety

- Unknown graph nodes are preserved but disabled.
- A helper cannot auto-upgrade across an incompatible core range.
- Current valid route remains until replacement is prepared.
- Rollback includes schema migration reversal or forward-compatible snapshot restore.
- Preview adapters never force stable peers to upgrade.
