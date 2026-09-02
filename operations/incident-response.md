# Incident Response

## Severity

| Severity | Examples | Immediate action |
|---|---|---|
| SEV-0 | unauthorized input/control, credential compromise, destructive object authority split | isolate affected endpoints, revoke credentials, return local control, preserve evidence |
| SEV-1 | RT audio recording corruption, route split-brain, failed rollback, widespread black surfaces | freeze new routes, revert adapter/core, switch to safe local/OOB path |
| SEV-2 | repeated latency regression, one platform adapter failure, cache/object recovery issue | disable automatic candidate, use fallback, collect trace |
| SEV-3 | UI/workspace issue with safe workaround | record and repair in normal release lane |

## First five actions

1. Preserve human local/break-glass control.
2. Revoke or fence stale routes/credentials.
3. Protect authoritative recordings/data; stop unsafe writes.
4. Fall back to simpler known path—local, full desktop, SPICE or OOB.
5. Capture topology/version/route/evidence bundle before restarting where safe.

## Post-incident

Every SEV-0/1 creates an ADR or risk-register update, requirement/evidence link, regression test and explicit affected compatibility fingerprints.
