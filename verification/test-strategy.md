# Test and Evidence Strategy

## Pyramid is insufficient

This product crosses kernel, driver, VM, media, network, hardware and human interaction boundaries. Verification uses a mesh:

1. **Schema/unit/property tests** for graph objects, state machines, policies, cost terms and serialization.
2. **Contract tests** for every adapter and cross-product event.
3. **Model/simulation tests** for scheduling, partitions, lease fencing, object authority and degradation.
4. **Hardware-in-loop tests** on the main PC, VFIO VM, C27HG70, M1 Pro and bench targets.
5. **Performance tests** reporting distributions and stage traces under idle and mixed load.
6. **Fault injection** at every prepare/commit/data-plane boundary.
7. **Security tests** for enrollment, capability scope, privileged helpers, clipboard/input/surfaces and OOB.
8. **User scenario acceptance** for desk, couch, game, Ableton, agent realm and remote recovery.
9. **Long soaks** for audio clocks, memory/cache pressure, reconnects and resource leaks.
10. **Counterfactual tests** proving the compiler/scheduler chose better than valid alternatives.

## Evidence bundle

Each promoted route or scheduler policy records:

- requirement and intent IDs;
- source/target capability snapshots;
- code/config/build identifiers;
- route graph before and after compilation;
- stage-level raw samples and clock calibration;
- foreground/background workload descriptors;
- failure and aborted runs;
- quality/color/audio settings;
- security and privilege review;
- cleanup/rollback outcome;
- Tracera-compatible evidence URI and SessionLedger run reference.

## Statistical rule

Report sample count, median, p95, p99, maximum, confidence interval where meaningful and every exclusion. Averages alone are prohibited for real-time claims. Warm and cold states are separate cohorts.
