# Normative System Specification

## Status and language

This document is the normative index for the 0.1 architecture baseline. “Shall” is mandatory for a promoted capability, “should” is a preferred default with a documented exception path, and “may” is optional or research-gated. Planned behavior is not an implementation claim.

## Normative sources

1. [`intent/004-nonnegotiables.md`](intent/004-nonnegotiables.md)
2. [`FUNCTIONAL_REQUIREMENTS.md`](FUNCTIONAL_REQUIREMENTS.md)
3. [`NON_FUNCTIONAL_REQUIREMENTS.md`](NON_FUNCTIONAL_REQUIREMENTS.md)
4. [`SYSTEM_REQUIREMENTS.md`](SYSTEM_REQUIREMENTS.md)
5. accepted [`adr/`](adr/) records
6. feature contracts under [`specs/`](specs/)
7. acceptance gates under [`verification/`](verification/)

If documents conflict, exact human intent and accepted ADRs trigger a correction; later accepted decisions supersede earlier architecture prose but cannot silently weaken a non-negotiable.

## Mandatory invariants

- One user-facing product shell with modular replaceable data planes.
- Desired intent and compiled physical route remain separate.
- Hard security, authority, real-time and quality constraints prune candidates before scoring.
- Same-process/OS/host/PCIe paths are examined before LAN/WAN paths.
- Every copy, serialization, codec, transform, resample, relay and migration is represented and costed.
- Input/output ownership uses exclusive leases and fencing; held input state is reconciled.
- Audio, input, video, control and bulk may coordinate but retain independent scheduling/queues.
- Objects expose authority, version, consistency and residency; remote capacity is not presented as uniform local memory.
- Atomic placement is supported only at actual interposition points and normally fused into profitable execution regions.
- Agents cannot seize human focus without explicit grant.
- Unsupported secure/protected surfaces and impossible deadlines fail explicitly with fallback.
- Evidence names the topology, version, workload, settings, distribution and failed runs.

## Conformance

A component conforms to an adapter contract when it:

1. advertises a versioned capability descriptor;
2. supports prepare/commit/abort/stop and idempotency where applicable;
3. reports memory domains, formats, clocks, resource claims and failure modes;
4. enforces route identity, authorization and fencing;
5. exposes stage telemetry without blocking real-time paths;
6. cleans up after crash/timeout or provides a deterministic recovery tool;
7. passes the declared compatibility, benchmark, fault and security gates.

A product release conforms only when the complete reference scenarios pass; individually conforming adapters do not prove the integrated product.
