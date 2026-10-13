# Verification and Evidence Set

- [Test strategy](test-strategy.md)
- [Requirements traceability matrix](requirements-traceability-matrix.md)
- [Benchmark plan](benchmark-plan.md)
- [Latency methodology](latency-methodology.md)
- [Audio real-time test plan](audio-rt-test-plan.md)
- [Video/HDR test plan](video-hdr-test-plan.md)
- [Fault injection](fault-injection.md)
- [Security test plan](security-test-plan.md)
- [Compatibility matrix](compatibility-matrix.md)
- [Release acceptance gates](acceptance-gates.md)

Every performance or compatibility claim must have a reproducible evidence bundle. The planned metrics are not presented as achieved results.

---

## Proposed PF-FR-089..094 ledger acceptance

[Spec 027](../specs/027-global-nested-ledger/spec.md) and [proposed traceability rows](requirements-traceability-matrix.md) define the eight transaction acceptance scenarios. Machine-readable counterparts are in [requirements-traceability.json](requirements-traceability.json). No native proof of distributed authority, effect-fencing or physically verified reclamation is claimed by the SQLite unit tests.
