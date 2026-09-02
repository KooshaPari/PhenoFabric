# Documentation Index

**Working product name:** Phenotype Fabric  
**Document baseline:** 0.1.0-draft, 2026-08-28

## Start here

1. [README](README.md) — scope, principles, ecosystem boundary and map.
2. [Exact human prompts](intent/000-source-prompts.md) → [intent synthesis](intent/001-intent-synthesis.md).
3. [PRD](PRD.md) → [normative specification](SPECIFICATION.md) → [functional](FUNCTIONAL_REQUIREMENTS.md) and [non-functional](NON_FUNCTIONAL_REQUIREMENTS.md) requirements.
4. [HLD](HLD.md) → [ALD](ALD.md) → [LLD](LLD.md) → detailed [architecture](architecture/README.md).
5. [AgilePlus-shaped specs](specs/INDEX.md) and [ADRs](adr/INDEX.md).
6. [WBS/PERT/DAG](work/README.md), [roadmap](ROADMAP.md) and [research set](research/README.md).
7. [2026 SOTA/competitive corpus](sota/README.md).
8. [Verification](verification/README.md), [operations](operations/README.md), [risks](risks/README.md) and [examples](examples/README.md).
9. [Traceability](TRACEABILITY.md), [validation](VALIDATION_REPORT.md) and [manifest](MANIFEST.sha256).

## Package summary

- one packaged-product definition with modular data planes;
- exact source-prompt record and formal intent chain;
- PRD, architecture/abstraction/high/low-level design and normative requirements;
- 12 feature specs, 22 ADRs, 21 work packages and 137 tasks;
- SOTA matrices above the requested 25-entry floor for every named class;
- compute/data/I/O/surface architecture, research and falsification program;
- benchmark, real-time audio, HDR/color, fault, security and acceptance plans;
- ecosystem authority contracts, examples, packaging, operations and risk registers.


## Top-level product and program documents

- [`ALD.md`](ALD.md) — Architecture / Abstraction-Level Design — 3,778 bytes
- [`CHANGELOG.md`](CHANGELOG.md) — Documentation Changelog — 862 bytes
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — Documentation and Engineering Contribution Rules — 1,041 bytes
- [`DEVELOPMENT_GUIDE.md`](DEVELOPMENT_GUIDE.md) — Development and Repository Guide — 1,597 bytes
- [`DOMAIN_MODEL.md`](DOMAIN_MODEL.md) — Domain Model — 4,223 bytes
- [`FUNCTIONAL_REQUIREMENTS.md`](FUNCTIONAL_REQUIREMENTS.md) — Functional Requirements — 9,545 bytes
- [`GLOSSARY.md`](GLOSSARY.md) — Glossary — 3,640 bytes
- [`GOVERNANCE.md`](GOVERNANCE.md) — Governance — 3,496 bytes
- [`HLD.md`](HLD.md) — High-Level Design — 5,784 bytes
- [`LLD.md`](LLD.md) — Low-Level Design Overview — 6,084 bytes
- [`MANIFEST.sha256`](MANIFEST.sha256) — SHA-256 manifest — generated during packaging
- [`NON_FUNCTIONAL_REQUIREMENTS.md`](NON_FUNCTIONAL_REQUIREMENTS.md) — Non-Functional Requirements — 6,310 bytes
- [`PLAN.md`](PLAN.md) — Program Plan — 2,241 bytes
- [`PRD.md`](PRD.md) — Phenotype Fabric — Product Requirements Document — 14,891 bytes
- [`README.md`](README.md) — Phenotype Fabric — 8,332 bytes
- [`ROADMAP.md`](ROADMAP.md) — Product and Research Roadmap — 2,120 bytes
- [`SECURITY.md`](SECURITY.md) — Security Policy and Design Entry Point — 857 bytes
- [`SPECIFICATION.md`](SPECIFICATION.md) — Normative System Specification — 2,739 bytes
- [`STATUS.md`](STATUS.md) — Document and Project Status — 2,820 bytes
- [`SYSTEM_REQUIREMENTS.md`](SYSTEM_REQUIREMENTS.md) — System and Platform Requirements — 3,350 bytes
- [`TRACEABILITY.md`](TRACEABILITY.md) — End-to-End Intent Traceability — 2,641 bytes
- [`UX_SPECIFICATION.md`](UX_SPECIFICATION.md) — User Experience Specification — 2,375 bytes
- [`VALIDATION_REPORT.md`](VALIDATION_REPORT.md) — Validation report — generated during packaging
- [`WORKLOG.md`](WORKLOG.md) — Documentation Worklog — 1,018 bytes

## Human source intent

- [`intent/000-source-prompts.md`](intent/000-source-prompts.md) — Verbatim Human Intent Source — 20,370 bytes
- [`intent/001-intent-synthesis.md`](intent/001-intent-synthesis.md) — Formal Intent Synthesis — 5,091 bytes
- [`intent/002-human-to-requirement-map.md`](intent/002-human-to-requirement-map.md) — Human Intent to Requirement Map — 2,991 bytes
- [`intent/003-assumptions-and-ambiguities.md`](intent/003-assumptions-and-ambiguities.md) — Assumptions and Ambiguities — 3,646 bytes
- [`intent/004-nonnegotiables.md`](intent/004-nonnegotiables.md) — Non-Negotiable Intent — 1,729 bytes
- [`intent/005-alternatives-and-falsification.md`](intent/005-alternatives-and-falsification.md) — Competing Interpretations and Falsification — 4,133 bytes
- [`intent/README.md`](intent/README.md) — Intent Documentation — 1,145 bytes
- [`intent/prompt-map.yaml`](intent/prompt-map.yaml) — prompt-map.yaml — 1,009 bytes

## AgilePlus-shaped feature specifications

- [`specs/001-unified-product-shell/meta.json`](specs/001-unified-product-shell/meta.json) — meta.json — 631 bytes
- [`specs/001-unified-product-shell/plan.md`](specs/001-unified-product-shell/plan.md) — Plan: Unified Product Shell and Workspace Model — 3,616 bytes
- [`specs/001-unified-product-shell/spec.md`](specs/001-unified-product-shell/spec.md) — Unified Product Shell and Workspace Model — 4,989 bytes
- [`specs/001-unified-product-shell/tasks.md`](specs/001-unified-product-shell/tasks.md) — Tasks: Unified Product Shell and Workspace Model — 4,654 bytes
- [`specs/002-universal-io-graph/meta.json`](specs/002-universal-io-graph/meta.json) — meta.json — 718 bytes
- [`specs/002-universal-io-graph/plan.md`](specs/002-universal-io-graph/plan.md) — Plan: Universal I/O Graph, Seats, and Focus — 3,634 bytes
- [`specs/002-universal-io-graph/spec.md`](specs/002-universal-io-graph/spec.md) — Universal I/O Graph, Seats, and Focus — 5,047 bytes
- [`specs/002-universal-io-graph/tasks.md`](specs/002-universal-io-graph/tasks.md) — Tasks: Universal I/O Graph, Seats, and Focus — 4,545 bytes
- [`specs/003-compute-data-fabric/meta.json`](specs/003-compute-data-fabric/meta.json) — meta.json — 709 bytes
- [`specs/003-compute-data-fabric/plan.md`](specs/003-compute-data-fabric/plan.md) — Plan: Adaptive Heterogeneous Compute and Data Fabric — 4,025 bytes
- [`specs/003-compute-data-fabric/spec.md`](specs/003-compute-data-fabric/spec.md) — Adaptive Heterogeneous Compute and Data Fabric — 5,907 bytes
- [`specs/003-compute-data-fabric/tasks.md`](specs/003-compute-data-fabric/tasks.md) — Tasks: Adaptive Heterogeneous Compute and Data Fabric — 5,242 bytes
- [`specs/004-locality-route-compiler/meta.json`](specs/004-locality-route-compiler/meta.json) — meta.json — 673 bytes
- [`specs/004-locality-route-compiler/plan.md`](specs/004-locality-route-compiler/plan.md) — Plan: Locality-Aware Route Compiler and Transport Plane — 3,859 bytes
- [`specs/004-locality-route-compiler/spec.md`](specs/004-locality-route-compiler/spec.md) — Locality-Aware Route Compiler and Transport Plane — 5,003 bytes
- [`specs/004-locality-route-compiler/tasks.md`](specs/004-locality-route-compiler/tasks.md) — Tasks: Locality-Aware Route Compiler and Transport Plane — 5,041 bytes
- [`specs/005-realtime-qos/meta.json`](specs/005-realtime-qos/meta.json) — meta.json — 657 bytes
- [`specs/005-realtime-qos/plan.md`](specs/005-realtime-qos/plan.md) — Plan: Real-Time QoS, Admission, and Contention Control — 3,802 bytes
- [`specs/005-realtime-qos/spec.md`](specs/005-realtime-qos/spec.md) — Real-Time QoS, Admission, and Contention Control — 4,941 bytes
- [`specs/005-realtime-qos/tasks.md`](specs/005-realtime-qos/tasks.md) — Tasks: Real-Time QoS, Admission, and Contention Control — 4,622 bytes
- [`specs/006-seamless-surface-presentation/meta.json`](specs/006-seamless-surface-presentation/meta.json) — meta.json — 735 bytes
- [`specs/006-seamless-surface-presentation/plan.md`](specs/006-seamless-surface-presentation/plan.md) — Plan: Seamless Surface and Application Presentation — 3,747 bytes
- [`specs/006-seamless-surface-presentation/spec.md`](specs/006-seamless-surface-presentation/spec.md) — Seamless Surface and Application Presentation — 4,975 bytes
- [`specs/006-seamless-surface-presentation/tasks.md`](specs/006-seamless-surface-presentation/tasks.md) — Tasks: Seamless Surface and Application Presentation — 4,601 bytes
- [`specs/007-platform-continuity-adapters/meta.json`](specs/007-platform-continuity-adapters/meta.json) — meta.json — 716 bytes
- [`specs/007-platform-continuity-adapters/plan.md`](specs/007-platform-continuity-adapters/plan.md) — Plan: Cross-Platform Continuity, Audio, Clipboard, and Files — 3,848 bytes
- [`specs/007-platform-continuity-adapters/spec.md`](specs/007-platform-continuity-adapters/spec.md) — Cross-Platform Continuity, Audio, Clipboard, and Files — 4,865 bytes
- [`specs/007-platform-continuity-adapters/tasks.md`](specs/007-platform-continuity-adapters/tasks.md) — Tasks: Cross-Platform Continuity, Audio, Clipboard, and Files — 4,658 bytes
- [`specs/008-agent-ephemeral-realms/meta.json`](specs/008-agent-ephemeral-realms/meta.json) — meta.json — 645 bytes
- [`specs/008-agent-ephemeral-realms/plan.md`](specs/008-agent-ephemeral-realms/plan.md) — Plan: Agent-Created Realms and Non-Stealing Surface Publication — 3,732 bytes
- [`specs/008-agent-ephemeral-realms/spec.md`](specs/008-agent-ephemeral-realms/spec.md) — Agent-Created Realms and Non-Stealing Surface Publication — 4,699 bytes
- [`specs/008-agent-ephemeral-realms/tasks.md`](specs/008-agent-ephemeral-realms/tasks.md) — Tasks: Agent-Created Realms and Non-Stealing Surface Publication — 4,607 bytes
- [`specs/009-security-identity-evidence/meta.json`](specs/009-security-identity-evidence/meta.json) — meta.json — 677 bytes
- [`specs/009-security-identity-evidence/plan.md`](specs/009-security-identity-evidence/plan.md) — Plan: Security, Identity, Leases, Audit, and Evidence — 3,756 bytes
- [`specs/009-security-identity-evidence/spec.md`](specs/009-security-identity-evidence/spec.md) — Security, Identity, Leases, Audit, and Evidence — 4,673 bytes
- [`specs/009-security-identity-evidence/tasks.md`](specs/009-security-identity-evidence/tasks.md) — Tasks: Security, Identity, Leases, Audit, and Evidence — 4,528 bytes
- [`specs/010-ecosystem-integration/meta.json`](specs/010-ecosystem-integration/meta.json) — meta.json — 574 bytes
- [`specs/010-ecosystem-integration/plan.md`](specs/010-ecosystem-integration/plan.md) — Plan: Phenotype Ecosystem Integration and Product Boundaries — 3,648 bytes
- [`specs/010-ecosystem-integration/spec.md`](specs/010-ecosystem-integration/spec.md) — Phenotype Ecosystem Integration and Product Boundaries — 4,807 bytes
- [`specs/010-ecosystem-integration/tasks.md`](specs/010-ecosystem-integration/tasks.md) — Tasks: Phenotype Ecosystem Integration and Product Boundaries — 4,313 bytes
- [`specs/011-observability-verification/meta.json`](specs/011-observability-verification/meta.json) — meta.json — 601 bytes
- [`specs/011-observability-verification/plan.md`](specs/011-observability-verification/plan.md) — Plan: Observability, Benchmarking, and Verification — 3,686 bytes
- [`specs/011-observability-verification/spec.md`](specs/011-observability-verification/spec.md) — Observability, Benchmarking, and Verification — 4,567 bytes
- [`specs/011-observability-verification/tasks.md`](specs/011-observability-verification/tasks.md) — Tasks: Observability, Benchmarking, and Verification — 4,574 bytes
- [`specs/012-packaging-operations/meta.json`](specs/012-packaging-operations/meta.json) — meta.json — 641 bytes
- [`specs/012-packaging-operations/plan.md`](specs/012-packaging-operations/plan.md) — Plan: Packaging, Deployment, Upgrade, and Recovery — 3,727 bytes
- [`specs/012-packaging-operations/spec.md`](specs/012-packaging-operations/spec.md) — Packaging, Deployment, Upgrade, and Recovery — 4,795 bytes
- [`specs/012-packaging-operations/tasks.md`](specs/012-packaging-operations/tasks.md) — Tasks: Packaging, Deployment, Upgrade, and Recovery — 4,690 bytes

## Architecture Decision Records

- [`adr/0001-fabric-product-boundary.md`](adr/0001-fabric-product-boundary.md) — ADR 0001 — Define Phenotype Fabric as the execution/data/I-O substrate — 2,909 bytes
- [`adr/0002-one-shell-modular-runtime.md`](adr/0002-one-shell-modular-runtime.md) — ADR 0002 — One packaged product shell over modular adapters — 2,832 bytes
- [`adr/0003-universal-typed-graph.md`](adr/0003-universal-typed-graph.md) — ADR 0003 — Use a PipeWire/JACK-inspired typed node-port-link graph — 2,835 bytes
- [`adr/0004-locality-first-route-selection.md`](adr/0004-locality-first-route-selection.md) — ADR 0004 — Search locality tiers before remote transports — 2,799 bytes
- [`adr/0005-atomic-capability-adaptive-regions.md`](adr/0005-atomic-capability-adaptive-regions.md) — ADR 0005 — Permit atomic placement but execute fused regions by default — 2,904 bytes
- [`adr/0006-multi-projection-shared-identity.md`](adr/0006-multi-projection-shared-identity.md) — ADR 0006 — Keep compute, data, I/O, resource, and product graphs as projections — 2,757 bytes
- [`adr/0007-no-universal-database.md`](adr/0007-no-universal-database.md) — ADR 0007 — Do not introduce a universal database or ontology — 2,748 bytes
- [`adr/0008-protected-realtime-islands.md`](adr/0008-protected-realtime-islands.md) — ADR 0008 — Protect local real-time islands before distributing critical loops — 2,799 bytes
- [`adr/0009-shared-memory-before-codecs.md`](adr/0009-shared-memory-before-codecs.md) — ADR 0009 — Prefer shared memory and device DMA before compression — 2,803 bytes
- [`adr/0010-semantic-before-pixel.md`](adr/0010-semantic-before-pixel.md) — ADR 0010 — Prefer semantic application remoting before pixel proxies — 2,792 bytes
- [`adr/0011-mobility-hierarchy.md`](adr/0011-mobility-hierarchy.md) — ADR 0011 — Use a mobility hierarchy instead of one process-migration claim — 2,785 bytes
- [`adr/0012-exclusive-route-leases.md`](adr/0012-exclusive-route-leases.md) — ADR 0012 — Use leases and fencing tokens for exclusive focus and routes — 2,753 bytes
- [`adr/0013-independent-media-clocks.md`](adr/0013-independent-media-clocks.md) — ADR 0013 — Treat input, audio, video, and device clocks independently — 2,872 bytes
- [`adr/0014-color-hdr-first-class.md`](adr/0014-color-hdr-first-class.md) — ADR 0014 — Make color and HDR intent authoritative graph metadata — 2,836 bytes
- [`adr/0015-sharecli-boundary.md`](adr/0015-sharecli-boundary.md) — ADR 0015 — Keep ShareCLI standalone and integrate it as a privileged runtime client — 2,848 bytes
- [`adr/0016-ecosystem-authority.md`](adr/0016-ecosystem-authority.md) — ADR 0016 — Preserve AGSLAG, AgilePlus, thegent, Tracera, and ledger authority — 2,833 bytes
- [`adr/0017-agent-non-stealing.md`](adr/0017-agent-non-stealing.md) — ADR 0017 — Agents may publish and request attention but not steal focus — 2,752 bytes
- [`adr/0018-oob-independent-recovery.md`](adr/0018-oob-independent-recovery.md) — ADR 0018 — Maintain an independent out-of-band recovery plane — 2,673 bytes
- [`adr/0019-mutual-pairing-privileged-isolation.md`](adr/0019-mutual-pairing-privileged-isolation.md) — ADR 0019 — Use mutual identity and narrow privileged helpers — 2,734 bytes
- [`adr/0020-provisional-name.md`](adr/0020-provisional-name.md) — ADR 0020 — Use Phenotype Fabric as a working name only — 2,685 bytes
- [`adr/0021-object-authority-immutability.md`](adr/0021-object-authority-immutability.md) — ADR 0021 — Prefer immutable objects and explicit mutable authority — 2,726 bytes
- [`adr/0022-control-data-plane-separation.md`](adr/0022-control-data-plane-separation.md) — ADR 0022 — Keep coordinator off payload and RT hot paths — 2,742 bytes

## Detailed architecture and schemas

- [`architecture/README.md`](architecture/README.md) — Architecture Index — 2,550 bytes
- [`architecture/api.md`](architecture/api.md) — Public API and Operator Surface — 2,223 bytes
- [`architecture/control-plane.md`](architecture/control-plane.md) — Control Plane — 3,841 bytes
- [`architecture/files-clipboard.md`](architecture/files-clipboard.md) — Clipboard, Files, Storage, and Continuity — 1,967 bytes
- [`architecture/graph-runtime.md`](architecture/graph-runtime.md) — Graph Runtime — 3,083 bytes
- [`architecture/input.md`](architecture/input.md) — Input and Seat Architecture — 2,242 bytes
- [`architecture/network-lan-wan.md`](architecture/network-lan-wan.md) — LAN, WAN, Relay, and Clock Transport — 2,255 bytes
- [`architecture/object-plane.md`](architecture/object-plane.md) — Data and Object Plane — 2,412 bytes
- [`architecture/openapi.yaml`](architecture/openapi.yaml) — openapi.yaml — 2,677 bytes
- [`architecture/platform-linux.md`](architecture/platform-linux.md) — Linux Platform Design — 2,097 bytes
- [`architecture/platform-macos.md`](architecture/platform-macos.md) — macOS Platform Design — 1,682 bytes
- [`architecture/platform-windows.md`](architecture/platform-windows.md) — Windows Platform Design — 1,940 bytes
- [`architecture/protocols.md`](architecture/protocols.md) — Protocol Suite — 1,909 bytes
- [`architecture/realtime-audio.md`](architecture/realtime-audio.md) — Real-Time Audio, MIDI, and Clock Architecture — 3,130 bytes
- [`architecture/route-compiler.md`](architecture/route-compiler.md) — Route Compiler — 2,737 bytes
- [`architecture/scheduler.md`](architecture/scheduler.md) — Heterogeneous Scheduler and Adaptive Execution Regions — 2,986 bytes
- [`architecture/schemas/capability.schema.json`](architecture/schemas/capability.schema.json) — capability.schema.json — 1,526 bytes
- [`architecture/schemas/event.proto`](architecture/schemas/event.proto) — event.proto — 1,041 bytes
- [`architecture/schemas/graph.schema.json`](architecture/schemas/graph.schema.json) — graph.schema.json — 3,529 bytes
- [`architecture/schemas/policy.schema.json`](architecture/schemas/policy.schema.json) — policy.schema.json — 1,222 bytes
- [`architecture/schemas/route.schema.json`](architecture/schemas/route.schema.json) — route.schema.json — 2,549 bytes
- [`architecture/schemas/task.schema.json`](architecture/schemas/task.schema.json) — task.schema.json — 2,397 bytes
- [`architecture/schemas/workspace.schema.json`](architecture/schemas/workspace.schema.json) — workspace.schema.json — 1,252 bytes
- [`architecture/security.md`](architecture/security.md) — Security Architecture — 2,345 bytes
- [`architecture/state-machines.md`](architecture/state-machines.md) — State Machines — 1,625 bytes
- [`architecture/surface-proxy.md`](architecture/surface-proxy.md) — Surface Presentation and Native Proxy Windows — 2,534 bytes
- [`architecture/system-context.md`](architecture/system-context.md) — System Context and Plane Model — 3,104 bytes
- [`architecture/video-hdr.md`](architecture/video-hdr.md) — Video, HDR, Color, and Frame-Pacing Architecture — 3,003 bytes

## 2026 SOTA and differentiation

- [`sota/README.md`](sota/README.md) — State of the Art and Competitive Analysis — 1,828 bytes
- [`sota/audio-network-class.md`](sota/audio-network-class.md) — Real-Time Audio, MIDI, Clock, and Media-Network Class — 11,056 bytes
- [`sota/build-buy-wrap.md`](sota/build-buy-wrap.md) — Build, Buy, Wrap, or Research Decisions — 2,812 bytes
- [`sota/compute-runtime-class.md`](sota/compute-runtime-class.md) — Distributed and Heterogeneous Compute Runtime Class — 12,928 bytes
- [`sota/deskflow-class.md`](sota/deskflow-class.md) — Deskflow-Class Input Continuity, Software KVM, and OOB Control — 13,190 bytes
- [`sota/evdev-class.md`](sota/evdev-class.md) — evdev-Class Input APIs, Routers, Virtual Devices, and Passthrough — 17,455 bytes
- [`sota/executive-differentiation.md`](sota/executive-differentiation.md) — Executive Differentiation — 3,010 bytes
- [`sota/looking-glass-class.md`](sota/looking-glass-class.md) — Looking Glass-Class Same-Host VM Display and Adjacent Alternatives — 12,133 bytes
- [`sota/moat-and-risk.md`](sota/moat-and-risk.md) — Defensibility, Moat, and Competitive Risk — 1,777 bytes
- [`sota/parsec-class.md`](sota/parsec-class.md) — Parsec-Class Interactive Desktop and Workstation Transports — 19,409 bytes
- [`sota/seamless-application-class.md`](sota/seamless-application-class.md) — Seamless Application and Native-Window Presentation Class — 7,987 bytes
- [`sota/standards-and-primitives.md`](sota/standards-and-primitives.md) — Standards and Primitive Map — 2,563 bytes
- [`sota/storage-memory-fabric-class.md`](sota/storage-memory-fabric-class.md) — Storage, Shared Memory, GPU Memory, and Object-Fabric Class — 11,098 bytes
- [`sota/user-facing-differentiators.md`](sota/user-facing-differentiators.md) — User-Facing Differentiators and Product Requirements — 1,940 bytes

## Work/plan/WBS/PERT/DAG

- [`work/README.md`](work/README.md) — Work, Planning, and Delivery Set — 834 bytes
- [`work/backlog.md`](work/backlog.md) — Initial Backlog — 1,036 bytes
- [`work/build-order.md`](work/build-order.md) — Build Order Rationale — 921 bytes
- [`work/cost-model.md`](work/cost-model.md) — Engineering and Runtime Cost Models — 1,150 bytes
- [`work/critical-path.md`](work/critical-path.md) — Critical Path and Release Slices — 1,944 bytes
- [`work/dag.md`](work/dag.md) — Program Dependency DAG — 1,961 bytes
- [`work/dependency-register.md`](work/dependency-register.md) — External Dependency and Assumption Register — 1,853 bytes
- [`work/milestones.md`](work/milestones.md) — Milestones and Exit Gates — 1,796 bytes
- [`work/pert.md`](work/pert.md) — PERT and Uncertainty Model — 3,170 bytes
- [`work/release-plan.md`](work/release-plan.md) — Release and Versioning Plan — 950 bytes
- [`work/staffing-agent-plan.md`](work/staffing-agent-plan.md) — Human and Agent Execution Plan — 1,642 bytes
- [`work/tasks.json`](work/tasks.json) — tasks.json — 28,293 bytes
- [`work/wbs.csv`](work/wbs.csv) — wbs.csv — 4,373 bytes
- [`work/wbs.md`](work/wbs.md) — Work Breakdown Structure — 22,275 bytes

## Research and falsification

- [`research/README.md`](research/README.md) — Research Set — 531 bytes
- [`research/experiments.json`](research/experiments.json) — experiments.json — 7,242 bytes
- [`research/experiments.md`](research/experiments.md) — Experiment Catalog — 5,766 bytes
- [`research/graduation-gates.md`](research/graduation-gates.md) — Research Graduation Gates — 1,216 bytes
- [`research/hypotheses.json`](research/hypotheses.json) — hypotheses.json — 9,425 bytes
- [`research/hypotheses.md`](research/hypotheses.md) — Research Hypotheses and Falsification Register — 7,657 bytes
- [`research/literature-map.md`](research/literature-map.md) — Literature and Prior-Art Map — 1,952 bytes
- [`research/open-questions.md`](research/open-questions.md) — Open Questions — 2,567 bytes
- [`research/research-program.md`](research/research-program.md) — Research Program — 1,593 bytes
- [`research/source-register.md`](research/source-register.md) — Source Register — 5,930 bytes

## Verification and evidence

- [`verification/README.md`](verification/README.md) — Verification and Evidence Set — 659 bytes
- [`verification/acceptance-gates.md`](verification/acceptance-gates.md) — Release Acceptance Gates — 1,354 bytes
- [`verification/audio-rt-test-plan.md`](verification/audio-rt-test-plan.md) — Real-Time Audio and MIDI Test Plan — 1,529 bytes
- [`verification/benchmark-catalog.json`](verification/benchmark-catalog.json) — benchmark-catalog.json — 4,899 bytes
- [`verification/benchmark-plan.md`](verification/benchmark-plan.md) — Benchmark Plan — 4,396 bytes
- [`verification/compatibility-matrix.md`](verification/compatibility-matrix.md) — Compatibility Matrix and Support Policy — 1,718 bytes
- [`verification/fault-catalog.json`](verification/fault-catalog.json) — fault-catalog.json — 2,762 bytes
- [`verification/fault-injection.md`](verification/fault-injection.md) — Fault-Injection Catalog — 2,190 bytes
- [`verification/latency-methodology.md`](verification/latency-methodology.md) — Latency Measurement Methodology — 1,463 bytes
- [`verification/requirements-traceability-matrix.md`](verification/requirements-traceability-matrix.md) — Requirements Traceability Matrix — 20,497 bytes
- [`verification/requirements-traceability.json`](verification/requirements-traceability.json) — requirements-traceability.json — 29,483 bytes
- [`verification/security-test-plan.md`](verification/security-test-plan.md) — Security Verification Plan — 1,339 bytes
- [`verification/test-strategy.md`](verification/test-strategy.md) — Test and Evidence Strategy — 1,834 bytes
- [`verification/video-hdr-test-plan.md`](verification/video-hdr-test-plan.md) — Video, HDR, Color, and Frame-Pacing Test Plan — 1,464 bytes

## Phenotype ecosystem integration

- [`ecosystem/README.md`](ecosystem/README.md) — Phenotype Ecosystem Integration — 555 bytes
- [`ecosystem/agileplus.md`](ecosystem/agileplus.md) — AgilePlus Integration — 1,110 bytes
- [`ecosystem/agslag.md`](ecosystem/agslag.md) — AGSLAG Integration — 992 bytes
- [`ecosystem/boundaries.md`](ecosystem/boundaries.md) — Ecosystem Authority and Product Boundaries — 3,066 bytes
- [`ecosystem/event-contracts.json`](ecosystem/event-contracts.json) — event-contracts.json — 2,856 bytes
- [`ecosystem/event-contracts.md`](ecosystem/event-contracts.md) — Cross-Product Event Contracts — 2,589 bytes
- [`ecosystem/integration-matrix.md`](ecosystem/integration-matrix.md) — Integration Matrix — 1,327 bytes
- [`ecosystem/ledgers.md`](ecosystem/ledgers.md) — Ledger Integrations — 1,248 bytes
- [`ecosystem/nvms-labs-compute.md`](ecosystem/nvms-labs-compute.md) — NVMS and labs-compute Relationship — 1,322 bytes
- [`ecosystem/sharecli.md`](ecosystem/sharecli.md) — ShareCLI Integration — 1,771 bytes
- [`ecosystem/thegent.md`](ecosystem/thegent.md) — thegent Integration — 1,006 bytes
- [`ecosystem/tracera.md`](ecosystem/tracera.md) — Tracera Integration — 1,303 bytes

## Reference scenarios and configuration

- [`examples/README.md`](examples/README.md) — Reference Scenarios and Configurations — 644 bytes
- [`examples/ableton.md`](examples/ableton.md) — Scenario: Ableton Live 12 Suite Protected Real-Time Island — 1,608 bytes
- [`examples/agent-game-e2e.md`](examples/agent-game-e2e.md) — Scenario: Agent-Created Game E2E Realm — 1,210 bytes
- [`examples/desk-to-couch.md`](examples/desk-to-couch.md) — Scenario: Desk to Couch and Back — 1,381 bytes
- [`examples/gaming.md`](examples/gaming.md) — Scenario: Gaming Under Background Agent Load — 1,581 bytes
- [`examples/graph-config.yaml`](examples/graph-config.yaml) — graph-config.yaml — 1,505 bytes
- [`examples/macbook-third-display.md`](examples/macbook-third-display.md) — Scenario: M1 Pro MacBook as a Third Display — 1,426 bytes
- [`examples/multi-vm.md`](examples/multi-vm.md) — Scenario: Main Host with Multiple Disjoint VMs — 1,254 bytes
- [`examples/user-environment.md`](examples/user-environment.md) — Reference User Environment — 1,528 bytes
- [`examples/wan.md`](examples/wan.md) — Scenario: Remote Worldwide PC — 1,105 bytes
- [`examples/workspace-config.yaml`](examples/workspace-config.yaml) — workspace-config.yaml — 1,125 bytes

## Packaging and operations

- [`operations/README.md`](operations/README.md) — Packaging and Operations — 569 bytes
- [`operations/config-and-policy.md`](operations/config-and-policy.md) — Configuration and Policy Model — 1,053 bytes
- [`operations/deployment-topologies.md`](operations/deployment-topologies.md) — Deployment Topologies — 1,562 bytes
- [`operations/incident-response.md`](operations/incident-response.md) — Incident Response — 1,203 bytes
- [`operations/observability.md`](operations/observability.md) — Observability and Evidence Operations — 1,214 bytes
- [`operations/packaging.md`](operations/packaging.md) — Packaging and Installation Architecture — 2,002 bytes
- [`operations/recovery.md`](operations/recovery.md) — Recovery Playbooks — 1,322 bytes
- [`operations/slo-sla.md`](operations/slo-sla.md) — Service-Level Objectives and Product Profiles — 2,449 bytes
- [`operations/support-matrix.md`](operations/support-matrix.md) — Support and Deprecation Policy — 795 bytes
- [`operations/upgrades.md`](operations/upgrades.md) — Upgrade, Compatibility, and Rollback — 1,014 bytes

## Risk, threat and feasibility

- [`risks/README.md`](risks/README.md) — Risk and Constraint Set — 438 bytes
- [`risks/feasibility-boundaries.md`](risks/feasibility-boundaries.md) — Feasibility Boundaries and Honest Claims — 1,636 bytes
- [`risks/legal-licensing.md`](risks/legal-licensing.md) — Legal, Licensing, and Distribution Risk — 1,430 bytes
- [`risks/risk-register.json`](risks/risk-register.json) — risk-register.json — 11,807 bytes
- [`risks/risk-register.md`](risks/risk-register.md) — Risk Register — 7,234 bytes
- [`risks/threat-model.md`](risks/threat-model.md) — Threat Model — 2,196 bytes
- [`risks/vendor-dependency.md`](risks/vendor-dependency.md) — Vendor and Upstream Dependency Register — 1,500 bytes

## Sources and bibliography

- [`references/BIBLIOGRAPHY.md`](references/BIBLIOGRAPHY.md) — Bibliography and Primary Reference Categories — 3,149 bytes
- [`references/README.md`](references/README.md) — References — 303 bytes
- [`references/source-map.md`](references/source-map.md) — Claim-to-Source Map — 1,190 bytes
- [`references/source-status.md`](references/source-status.md) — Source and Claim Status — 1,228 bytes
- [`references/url-inventory.txt`](references/url-inventory.txt) — url-inventory.txt — 14,512 bytes
