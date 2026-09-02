# Work Breakdown Structure

## Estimation basis

Durations are **engineering-effort weeks for one effective senior lane**, not delivery promises. Agent parallelism can reduce elapsed time but does not remove integration, hardware, signing, benchmark or review effort. The PERT expected value is `(O + 4M + P) / 6`; uncertainty is `(P - O) / 6`.

## Work packages

| ID | Phase | Work package | Depends on | PERT O | PERT M | PERT P | Expected | Std dev | Exit gate | Primary lane |
|---|---|---|---|---|---|---|---|---|---|---|
| PF-WP-000 | P0 | Program baseline and governance | — | 1 | 2 | 4 | 2.2 | 0.5 | Schemas, requirement IDs, ADRs, source register and CI documentation checks accepted. | Architecture/governance |
| PF-WP-010 | P1 | Capability and topology inventory | PF-WP-000 | 2 | 4 | 7 | 4.2 | 0.8 | Linux/Windows/macOS nodes publish signed resource, display, audio, input and transport descriptors. | Runtime/platform |
| PF-WP-020 | P1 | Graph core and transaction engine | PF-WP-000 | 3 | 5 | 8 | 5.2 | 0.8 | Typed nodes/ports/links, schema validation, prepare/commit/abort and workspace persistence pass contracts. | Graph runtime |
| PF-WP-030 | P2 | Linux seat and local media adapters | PF-WP-010, PF-WP-020 | 3 | 6 | 10 | 6.2 | 1.2 | evdev/uinput/libei and PipeWire/JACK adapters pass pressed-state and RT callback tests. | Linux/RT |
| PF-WP-040 | P2 | Same-host VM fast paths | PF-WP-010, PF-WP-020 | 3 | 6 | 10 | 6.2 | 1.2 | Looking Glass/KVMFR, SPICE fallback, virtio input/audio/files and VM lifecycle are one route family. | VFIO/virtualization |
| PF-WP-050 | P2 | Unified shell, workspaces and focus UX | PF-WP-020 | 4 | 7 | 12 | 7.3 | 1.3 | Graph canvas, simple workspace mode, hotkeys, surface palette, explanations and rollback are usable. | Product/UI |
| PF-WP-060 | P3 | Windows and macOS endpoint agents | PF-WP-010, PF-WP-020 | 5 | 9 | 15 | 9.3 | 1.7 | Signed/minimal helpers expose display, audio, input, capture and virtual-device capabilities. | Platform |
| PF-WP-070 | P3 | LAN interactive media transport | PF-WP-030, PF-WP-060 | 4 | 8 | 13 | 8.2 | 1.5 | Sunshine/Moonlight-class adapter plus telemetry and direct QUIC data channels pass wired mixed-load tests. | Media/network |
| PF-WP-080 | P3 | Real-time audio, MIDI and clock domains | PF-WP-030, PF-WP-060 | 5 | 9 | 15 | 9.3 | 1.7 | Ableton reference route meets admitted xrun/latency budgets under background load. | Audio/RT |
| PF-WP-090 | P3 | HDR, color and frame pacing | PF-WP-060, PF-WP-070 | 4 | 8 | 14 | 8.3 | 1.7 | C27HG70 and 2021 M1 Pro paths preserve declared SDR/HDR/color policy with measured frame pacing. | Video/color |
| PF-WP-100 | P4 | Agent realms and surface publishing | PF-WP-050, PF-WP-070 | 3 | 6 | 10 | 6.2 | 1.2 | Agent can create TTL realm and publish desktop/window/report without stealing focus. | Agents/runtime |
| PF-WP-110 | P4 | Object identity, authority and residency plane | PF-WP-010, PF-WP-020 | 5 | 10 | 17 | 10.3 | 2.0 | Versioned objects, local SHM store, replicas, spill, provenance and corruption recovery pass tests. | Data runtime |
| PF-WP-120 | P4 | Explicit TaskSpec placement scheduler | PF-WP-010, PF-WP-110 | 5 | 10 | 16 | 10.2 | 1.8 | Build/test/inference/media tasks choose nodes by measured total cost and explain alternatives. | Scheduler |
| PF-WP-130 | P5 | Adaptive execution regions and prediction | PF-WP-120 | 6 | 12 | 22 | 12.7 | 2.7 | Fusion/fission, hysteresis, prefetch and speculative execution beat explicit baseline without thrash. | Runtime research |
| PF-WP-140 | P5 | Semantic and pixel-proxy application surfaces | PF-WP-060, PF-WP-070, PF-WP-090 | 7 | 14 | 24 | 14.5 | 2.8 | RAIL/Xpra/Waypipe adapters and native proxy fallback handle window graphs, IME, DPI and secure fallbacks. | Surface/platform |
| PF-WP-150 | P5 | WAN, roaming and OOB integration | PF-WP-070, PF-WP-050 | 4 | 8 | 14 | 8.3 | 1.7 | Congestion adaptation, NAT/relay policy, overlay identities and PiKVM-class recovery pass fault tests. | Network/Ops |
| PF-WP-160 | P1-P6 | Security, identity and privileged-boundary program | PF-WP-000 | 5 | 10 | 18 | 10.5 | 2.2 | Threat model, mTLS enrollment, capabilities, helper isolation, signing, audit and red-team gates accepted. | Security |
| PF-WP-170 | P1-P6 | Observability, benchmark and evidence program | PF-WP-000 | 4 | 9 | 16 | 9.3 | 2.0 | Stage timing, topology snapshots, benchmark corpus, Tracera refs and regression gates run continuously. | QE/performance |
| PF-WP-180 | P6 | Packaging, updates and operations | PF-WP-050, PF-WP-060, PF-WP-150, PF-WP-160, PF-WP-170 | 5 | 10 | 18 | 10.5 | 2.2 | One installer per OS, guided setup, rollback, fleet update rings and recovery docs pass clean-machine tests. | Release/operations |
| PF-WP-190 | R | Selective atomic interposition research | PF-WP-110, PF-WP-120, PF-WP-170 | 8 | 18 | 36 | 19.3 | 4.7 | At least one syscall/function/kernel granule experiment shows positive net benefit and safe fallback; otherwise remains research. | Systems research |
| PF-WP-200 | P6 | Release candidate and acceptance | PF-WP-080, PF-WP-090, PF-WP-100, PF-WP-120, PF-WP-140, PF-WP-150, PF-WP-180 | 3 | 6 | 12 | 6.5 | 1.5 | Reference scenarios, security, mixed-load, fault, usability and upgrade gates all pass. | Program/QE |

## Task catalog

| Task ID | WP | Task | Depends on | Required evidence | State |
|---|---|---|---|---|---|
| PF-WP-000.01 | PF-WP-000 | Freeze provisional product boundary and authority map | — | Accepted ADR set and ecosystem boundary review | Planned |
| PF-WP-000.02 | PF-WP-000 | Normalize requirement, intent, evidence and event identifiers | 000.01 | Schema tests and traceability lint | Planned |
| PF-WP-000.03 | PF-WP-000 | Establish documentation-as-code checks | 000.02 | Link/schema/source validation in CI | Planned |
| PF-WP-000.04 | PF-WP-000 | Create source confidence and claim policy | 000.01 | Research source register | Planned |
| PF-WP-000.05 | PF-WP-000 | Define release evidence contract | 000.02 | Acceptance gate manifest | Planned |
| PF-WP-010.01 | PF-WP-010 | Define capability descriptor schemas | 000.02 | JSON/protobuf contract tests | Planned |
| PF-WP-010.02 | PF-WP-010 | Probe CPU/NUMA/cache/memory topology | 010.01 | Cross-platform topology snapshots | Planned |
| PF-WP-010.03 | PF-WP-010 | Probe GPU/NPU/codec/display/PCIe topology | 010.01 | Reference hardware inventory | Planned |
| PF-WP-010.04 | PF-WP-010 | Probe audio/MIDI/input/storage/NIC endpoints | 010.01 | Endpoint capability snapshots | Planned |
| PF-WP-010.05 | PF-WP-010 | Measure link RTT/jitter/loss/bandwidth and copy paths | 010.02,010.03,010.04 | Calibrated topology benchmark | Planned |
| PF-WP-010.06 | PF-WP-010 | Sign and publish descriptor deltas | 010.01 | Identity/replay tests | Planned |
| PF-WP-020.01 | PF-WP-020 | Implement canonical node/port/link/domain objects | 000.02 | Schema/property tests | Planned |
| PF-WP-020.02 | PF-WP-020 | Implement capability and format negotiation | 020.01 | Negotiation matrix | Planned |
| PF-WP-020.03 | PF-WP-020 | Implement graph prepare/commit/abort/rollback | 020.01 | Fault/property tests | Planned |
| PF-WP-020.04 | PF-WP-020 | Implement lease/fencing primitives | 020.03 | Partition and stale-token tests | Planned |
| PF-WP-020.05 | PF-WP-020 | Implement workspace snapshots and diff | 020.01 | Round-trip/recovery tests | Planned |
| PF-WP-020.06 | PF-WP-020 | Implement route recursion/hop prevention | 020.03 | Loop adversarial tests | Planned |
| PF-WP-020.07 | PF-WP-020 | Expose CLI/SDK/API graph operations | 020.01-020.06 | Contract and usability tests | Planned |
| PF-WP-030.01 | PF-WP-030 | Build Linux evdev/libevdev acquisition service | 010.04 | Device hotplug/grab tests | Planned |
| PF-WP-030.02 | PF-WP-030 | Build uinput endpoint pool and pressed-state ledger | 030.01 | One-million randomized switch test | Planned |
| PF-WP-030.03 | PF-WP-030 | Integrate libei/EIS and XDG portals | 030.01 | Wayland compositor matrix | Planned |
| PF-WP-030.04 | PF-WP-030 | Integrate PipeWire node/port discovery and links | 010.04,020.02 | Audio/video graph tests | Planned |
| PF-WP-030.05 | PF-WP-030 | Integrate JACK compatibility and RT callbacks | 030.04 | xrun/priority tests | Planned |
| PF-WP-030.06 | PF-WP-030 | Implement emergency local control path | 030.01 | Kill/partition recovery tests | Planned |
| PF-WP-040.01 | PF-WP-040 | Automate Looking Glass/KVMFR discovery and launch | 010.03 | VFIO reference scenario | Planned |
| PF-WP-040.02 | PF-WP-040 | Integrate QEMU/libvirt input endpoint binding | 030.02 | Hot switch and rollback test | Planned |
| PF-WP-040.03 | PF-WP-040 | Integrate SPICE/noVNC recovery console | 040.01 | Guest failure recovery | Planned |
| PF-WP-040.04 | PF-WP-040 | Integrate virtio audio/file/clipboard service paths | 020.02 | Copy/latency benchmark | Planned |
| PF-WP-040.05 | PF-WP-040 | Model VM lifecycle, display and seat as graph nodes | 020.01 | Graph contract test | Planned |
| PF-WP-040.06 | PF-WP-040 | Probe PCIe/IOMMU/P2P constraints | 010.03 | Topology truth report | Planned |
| PF-WP-050.01 | PF-WP-050 | Implement device/realm/surface navigator | 020.07 | Usability scenario | Planned |
| PF-WP-050.02 | PF-WP-050 | Implement graph canvas with nested nodes | 020.07 | Graph editing test | Planned |
| PF-WP-050.03 | PF-WP-050 | Implement simple workspace mode and templates | 050.01 | Desk/couch/bench templates | Planned |
| PF-WP-050.04 | PF-WP-050 | Implement global input/output/coupled hotkeys | 030.02,020.04 | No-blind/no-stuck test | Planned |
| PF-WP-050.05 | PF-WP-050 | Implement route and placement explanation UI | 020.02 | Explanation acceptance | Planned |
| PF-WP-050.06 | PF-WP-050 | Implement adapter health/fallback UI | 020.03 | Injected adapter failure | Planned |
| PF-WP-050.07 | PF-WP-050 | Implement accessibility and keyboard-only workflows | 050.01-050.06 | Accessibility audit | Planned |
| PF-WP-060.01 | PF-WP-060 | Windows endpoint service and capability probe | 010.01 | Windows clean-machine test | Planned |
| PF-WP-060.02 | PF-WP-060 | Windows capture/virtual display/audio/input adapters | 060.01 | Signed helper and media tests | Planned |
| PF-WP-060.03 | PF-WP-060 | macOS endpoint service and capability probe | 010.01 | macOS clean-machine test | Planned |
| PF-WP-060.04 | PF-WP-060 | macOS capture/presentation/audio/input adapters | 060.03 | TCC/secure-input matrix | Planned |
| PF-WP-060.05 | PF-WP-060 | Implement per-platform privileged helper split | 060.01,060.03 | Privilege/threat review | Planned |
| PF-WP-060.06 | PF-WP-060 | Implement sleep/wake/hotplug recovery | 060.02,060.04 | Lifecycle fault tests | Planned |
| PF-WP-070.01 | PF-WP-070 | Wrap Sunshine/Moonlight session lifecycle | 030.04,060.02,060.04 | Adapter contract | Planned |
| PF-WP-070.02 | PF-WP-070 | Implement direct media/control QUIC channels | 020.03 | Loss/reorder tests | Planned |
| PF-WP-070.03 | PF-WP-070 | Implement stage timestamps and queue telemetry | 170.01 | Latency calibration | Planned |
| PF-WP-070.04 | PF-WP-070 | Implement congestion/pacing/FEC policy hooks | 070.02 | Network impairment suite | Planned |
| PF-WP-070.05 | PF-WP-070 | Implement client virtual display matching | 060.02 | MacBook sink test | Planned |
| PF-WP-070.06 | PF-WP-070 | Implement input/audio independent channels | 030.02,080.01 | Queue isolation test | Planned |
| PF-WP-080.01 | PF-WP-080 | Define audio clock/format/latency port schema | 020.02 | Schema and negotiation test | Planned |
| PF-WP-080.02 | PF-WP-080 | Integrate ASIO/WASAPI, CoreAudio and PipeWire/JACK nodes | 030.04,060.02,060.04 | Endpoint matrix | Planned |
| PF-WP-080.03 | PF-WP-080 | Implement drift estimator, ASRC and elastic buffer | 080.01 | Long soak drift test | Planned |
| PF-WP-080.04 | PF-WP-080 | Implement timestamped MIDI/OSC routes | 080.01 | Jitter/ordering test | Planned |
| PF-WP-080.05 | PF-WP-080 | Implement RT admission and CPU/IRQ affinity policy | 010.02 | Ableton mixed-load test | Planned |
| PF-WP-080.06 | PF-WP-080 | Implement audio-only and split-monitor routes | 080.02 | Workspace scenarios | Planned |
| PF-WP-080.07 | PF-WP-080 | Implement xrun diagnosis and fallback | 080.03,080.05 | Fault injection | Planned |
| PF-WP-090.01 | PF-WP-090 | Inventory EDID/ICC/HDR/refresh/VRR capabilities | 010.03 | C27HG70/MacBook profiles | Planned |
| PF-WP-090.02 | PF-WP-090 | Define color intent and transform graph | 020.02 | Golden image/vector tests | Planned |
| PF-WP-090.03 | PF-WP-090 | Implement 8/10-bit and HDR metadata negotiation | 090.01 | HDR metadata capture | Planned |
| PF-WP-090.04 | PF-WP-090 | Implement workload chroma/quality policies | 090.02 | Text/game/video comparisons | Planned |
| PF-WP-090.05 | PF-WP-090 | Implement frame pacing and refresh adaptation | 070.03 | High-speed capture evidence | Planned |
| PF-WP-090.06 | PF-WP-090 | Implement tone/gamut mapping fallback | 090.02 | Instrumented visual tests | Planned |
| PF-WP-100.01 | PF-WP-100 | Define RealmRequest, SurfacePublish and TTL contracts | 020.01 | API contract | Planned |
| PF-WP-100.02 | PF-WP-100 | Integrate VM/native/sandbox providers | 040.05 | Provider tests | Planned |
| PF-WP-100.03 | PF-WP-100 | Implement non-stealing surface catalog | 050.01 | Focus safety test | Planned |
| PF-WP-100.04 | PF-WP-100 | Implement budget/TTL cleanup and grace restore | 100.01 | Lifecycle fault test | Planned |
| PF-WP-100.05 | PF-WP-100 | Publish semantic terminal/report/artifact nodes | 100.01 | Agent scenario | Planned |
| PF-WP-100.06 | PF-WP-100 | Emit thegent/AgilePlus/Tracera/SessionLedger IDs | 000.02 | Cross-product contract tests | Planned |
| PF-WP-110.01 | PF-WP-110 | Define immutable/versioned object references and authority | 020.01 | Object schema/property tests | Planned |
| PF-WP-110.02 | PF-WP-110 | Implement local shared-memory object store | 110.01 | Zero-copy/copy-count benchmark | Planned |
| PF-WP-110.03 | PF-WP-110 | Implement residency catalog and transfer plans | 010.05,110.01 | Residency correctness tests | Planned |
| PF-WP-110.04 | PF-WP-110 | Implement spill to files/object stores | 110.01 | Crash/recovery tests | Planned |
| PF-WP-110.05 | PF-WP-110 | Implement replica/cache eviction and provenance | 110.03 | Eviction/corruption tests | Planned |
| PF-WP-110.06 | PF-WP-110 | Integrate virtiofs/SMB/NFS/Syncthing adapters | 110.03 | Compatibility tests | Planned |
| PF-WP-110.07 | PF-WP-110 | Implement data policy and sensitivity constraints | 110.01 | Security tests | Planned |
| PF-WP-120.01 | PF-WP-120 | Define TaskSpec/RegionSpec capabilities and constraints | 010.01,110.01 | Schema tests | Planned |
| PF-WP-120.02 | PF-WP-120 | Build candidate pruning by security/capability/locality | 120.01 | Correctness corpus | Planned |
| PF-WP-120.03 | PF-WP-120 | Build total completion cost model | 010.05,110.03 | Calibration benchmark | Planned |
| PF-WP-120.04 | PF-WP-120 | Implement reservations/admission across resource types | 120.03 | Mixed-load test | Planned |
| PF-WP-120.05 | PF-WP-120 | Integrate ShareCLI/NVMS/thegent execution adapters | 120.01 | Cross-product run test | Planned |
| PF-WP-120.06 | PF-WP-120 | Implement placement explanation and uncertainty | 120.03 | Counterfactual test | Planned |
| PF-WP-120.07 | PF-WP-120 | Implement cancellation/result routing/retry | 120.02 | Failure tests | Planned |
| PF-WP-130.01 | PF-WP-130 | Collect decision traces and neighboring affinity | 120.03 | Trace corpus | Planned |
| PF-WP-130.02 | PF-WP-130 | Implement region fusion with amortization threshold | 130.01 | Micro/macro benchmark | Planned |
| PF-WP-130.03 | PF-WP-130 | Implement fission on contention/locality change | 130.02 | Dynamic load test | Planned |
| PF-WP-130.04 | PF-WP-130 | Implement hysteresis/cooldown/minimum residency | 130.02,130.03 | Thrash adversarial test | Planned |
| PF-WP-130.05 | PF-WP-130 | Implement predictive prefetch and cache warming | 110.03,130.01 | Build/game/agent prediction tests | Planned |
| PF-WP-130.06 | PF-WP-130 | Implement bounded speculative execution | 120.07 | Waste/correctness test | Planned |
| PF-WP-130.07 | PF-WP-130 | Compare learned versus analytic policy | 130.01-130.06 | A/B evidence | Planned |
| PF-WP-140.01 | PF-WP-140 | Integrate FreeRDP/RAIL application adapter | 060.02 | Windows app matrix | Planned |
| PF-WP-140.02 | PF-WP-140 | Integrate Xpra and Waypipe adapters | 030.03 | Linux app matrix | Planned |
| PF-WP-140.03 | PF-WP-140 | Define owned-window graph and local proxy model | 020.01 | Popup/modal model tests | Planned |
| PF-WP-140.04 | PF-WP-140 | Implement Windows capture and Mac native proxy fallback | 060.02,060.04,140.03 | Reference demo | Planned |
| PF-WP-140.05 | PF-WP-140 | Implement IME/raw keys/clipboard/drag-drop | 140.03 | International/input tests | Planned |
| PF-WP-140.06 | PF-WP-140 | Implement mixed-DPI/z-order/minimize semantics | 140.03 | Multi-monitor matrix | Planned |
| PF-WP-140.07 | PF-WP-140 | Implement secure/protected surface detection and fallback | 140.01,140.04 | UAC/DRM/security tests | Planned |
| PF-WP-140.08 | PF-WP-140 | Implement stream atlas/session budgeting | 140.04 | Encoder saturation test | Planned |
| PF-WP-150.01 | PF-WP-150 | Implement overlay/direct/relay path discovery | 010.05 | Roaming tests | Planned |
| PF-WP-150.02 | PF-WP-150 | Implement WAN congestion and loss adaptation | 070.04 | Network emulator suite | Planned |
| PF-WP-150.03 | PF-WP-150 | Implement device roaming and session resume | 150.01 | IP/sleep transition tests | Planned |
| PF-WP-150.04 | PF-WP-150 | Integrate Parsec optional adapter | 050.06 | Adapter test | Planned |
| PF-WP-150.05 | PF-WP-150 | Integrate PiKVM/JetKVM-class OOB APIs | 010.01 | BIOS/crash scenario | Planned |
| PF-WP-150.06 | PF-WP-150 | Implement management VLAN/ACL policy templates | 150.05 | Security review | Planned |
| PF-WP-160.01 | PF-WP-160 | Threat model all trust and privilege boundaries | 000.01 | Reviewed threat model | Planned |
| PF-WP-160.02 | PF-WP-160 | Implement ECDH-confirmed mutual enrollment and rotation | 010.06 | Protocol tests | Planned |
| PF-WP-160.03 | PF-WP-160 | Implement capability-scoped authorization and leases | 020.04 | Policy/property tests | Planned |
| PF-WP-160.04 | PF-WP-160 | Implement OS key-store/TPM/Keychain integration | 160.02 | Clean-machine security tests | Planned |
| PF-WP-160.05 | PF-WP-160 | Implement signed helper/update pipeline | 060.05 | Supply-chain evidence | Planned |
| PF-WP-160.06 | PF-WP-160 | Implement audit/event integrity and redaction | 000.02 | Audit tests | Planned |
| PF-WP-160.07 | PF-WP-160 | Red-team input capture, surface, clipboard, OOB and relay paths | 160.01-160.06 | Adversarial report | Planned |
| PF-WP-170.01 | PF-WP-170 | Define stage taxonomy, clocks and trace context | 000.05 | Telemetry schema | Planned |
| PF-WP-170.02 | PF-WP-170 | Build capture-to-enqueue and input-to-photon lab | 170.01 | Calibration report | Planned |
| PF-WP-170.03 | PF-WP-170 | Build audio loopback/xrun/drift lab | 170.01 | Calibration report | Planned |
| PF-WP-170.04 | PF-WP-170 | Build mixed-load and contention corpus | 170.01 | Reference scenarios | Planned |
| PF-WP-170.05 | PF-WP-170 | Build network impairment/fault lab | 170.01 | Repro scripts | Planned |
| PF-WP-170.06 | PF-WP-170 | Build requirement-to-evidence matrix and gates | 000.02 | Traceability report | Planned |
| PF-WP-170.07 | PF-WP-170 | Implement regression dashboards and artifact retention | 170.02-170.06 | CI evidence | Planned |
| PF-WP-180.01 | PF-WP-180 | Define per-OS installer and component manifests | 050.01,060.05 | Manifest validation | Planned |
| PF-WP-180.02 | PF-WP-180 | Implement guided permissions/driver/device enrollment | 180.01 | Clean-machine UX test | Planned |
| PF-WP-180.03 | PF-WP-180 | Implement update rings, compatibility checks and rollback | 180.01 | Upgrade fault tests | Planned |
| PF-WP-180.04 | PF-WP-180 | Implement local-first coordinator and replica recovery | 020.05 | Coordinator loss test | Planned |
| PF-WP-180.05 | PF-WP-180 | Ship workspace templates and diagnostics bundle | 050.03,170.07 | Operator acceptance | Planned |
| PF-WP-180.06 | PF-WP-180 | Define support/security response and deprecation policy | 160.06 | Operations review | Planned |
| PF-WP-190.01 | PF-WP-190 | Select high-value interposition workloads and boundaries | 120.06 | Experiment charter | Planned |
| PF-WP-190.02 | PF-WP-190 | Prototype FUSE/filesystem-operation relocation | 190.01 | Net-benefit benchmark | Planned |
| PF-WP-190.03 | PF-WP-190 | Prototype syscall/eBPF/seccomp mediation where safe | 190.01 | Overhead/correctness report | Planned |
| PF-WP-190.04 | PF-WP-190 | Prototype runtime/library function offload | 190.01 | Cooperative benchmark | Planned |
| PF-WP-190.05 | PF-WP-190 | Prototype accelerator-kernel routing/graph capture | 190.01 | GPU benchmark | Planned |
| PF-WP-190.06 | PF-WP-190 | Implement automatic fusion fallback | 190.02-190.05 | No-regression test | Planned |
| PF-WP-190.07 | PF-WP-190 | Graduate or reject each mechanism by evidence | 190.02-190.06 | ADR update | Planned |
| PF-WP-200.01 | PF-WP-200 | Run desk/couch/MacBook-third-display acceptance | 050.03,070.05 | Scenario evidence | Planned |
| PF-WP-200.02 | PF-WP-200 | Run gaming mixed-load acceptance | 090.05,120.04 | Frame/input evidence | Planned |
| PF-WP-200.03 | PF-WP-200 | Run Ableton mixed-load acceptance | 080.05 | Audio evidence | Planned |
| PF-WP-200.04 | PF-WP-200 | Run agent game-E2E and ephemeral realm acceptance | 100.03,120.05 | Agent evidence | Planned |
| PF-WP-200.05 | PF-WP-200 | Run WAN/OOB and fault acceptance | 150.02,150.05 | Recovery evidence | Planned |
| PF-WP-200.06 | PF-WP-200 | Run security/red-team and upgrade acceptance | 160.07,180.03 | Release report | Planned |
| PF-WP-200.07 | PF-WP-200 | Publish release manifest and known limits | 200.01-200.06 | Signed release evidence | Planned |

## Completion definition

A task is done only when its implementation, rollback, requirement trace, benchmark/fault evidence, operator notes and cleanup are present. A work package is done only when its exit gate is reproduced from a clean environment.
