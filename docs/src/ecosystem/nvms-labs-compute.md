# NVMS and labs-compute Relationship

## NVMS

Prior ecosystem work positioned NVMS as runtime inventory and execution substrate across native, VM, WSL, Lima, Apple container-like and remote paths. Phenotype Fabric materially deepens that scope with real-time I/O, data/object residency, adaptive placement, surface compilation and unified product packaging.

Three viable repository outcomes remain:

1. Fabric becomes the successor/rewrite of NVMS and preserves relevant history/modules.
2. NVMS remains the low-level realm/resource adapter library beneath Fabric.
3. Fabric is initially a docs/research program inside NVMS until a product slice proves independent value.

The decision is intentionally unresolved in ADR-0020/product naming work; duplicate canonical ownership is not acceptable.

## labs-compute

labs-compute holds bounded experiments before product promotion:

- atomic syscall/function/kernel interposition;
- GPU P2P/external-memory and copy-engine experiments;
- CXL/RDMA/GPUDirect transport backends;
- learned placement and speculative execution;
- distributed low-bit inference/device mesh work;
- network DSP feasibility.

A lab item graduates only after the gates in `research/graduation-gates.md`. Failed experiments remain useful negative evidence and do not clutter the stable product API.
