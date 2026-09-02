# Risk Register

| ID | Category | Risk | Impact | Likelihood | Mitigation / decision | State |
|---|---|---|---|---|---|---|
| R-001 | Scope | Everything-product never reaches useful release | High | High | Slice releases; one shell over existing adapters first; research off critical path | Open |
| R-002 | Performance | Idle demo collapses under real background load | High | High | Mixed-load gates from first transport; reserve RT resources | Open |
| R-003 | Performance | Atomic routing overhead exceeds benefit | High | High | Explicit TaskSpec baseline; fusion; graduate bounded cases only | Open |
| R-004 | Correctness | Remote memory/VRAM presented as uniform and causes pathological latency/coherence | High | Medium | Explicit locality/consistency/cost in every object descriptor | Mitigated by design |
| R-005 | Input | Stuck keys/buttons or dual focus damages user state | Critical | Medium | Pressed-state ledger, prepare/commit, fencing, break-glass input | Open |
| R-006 | Audio | Ableton recording/monitoring xruns under contention | Critical | Medium | RT admission, isolation, local authoritative path, mixed-load soak | Open |
| R-007 | Video | High average FPS hides bad frame-age tail | High | High | Frame age/p99/bursts and hardware visible latency, not average FPS | Open |
| R-008 | Color | HDR/color transforms silently clip or mis-map | High | Medium | Explicit intent/capability/transform graph and reference patterns | Open |
| R-009 | Platform | macOS permissions/secure input block seamless behavior | High | High | Use supported APIs, explicit consent, feature fallback | Open |
| R-010 | Platform | Windows driver/helper signing and maintenance becomes dominant | High | High | Minimize privileged code, defer true VHF/virtual display where existing adapters suffice | Open |
| R-011 | Virtualization | IOMMU/PCIe/P2P topology differs from assumptions | High | High | Probe and benchmark exact topology; no name-based capability claims | Open |
| R-012 | Network | WAN jitter/loss creates unsafe input/audio state | High | Medium | Independent queues, fencing, adaptation, local RT independence | Open |
| R-013 | Security | Global input/capture service becomes keylogger/surveillance vector | Critical | High | Narrow privileged helper, capabilities, local indicators, audit, red team | Open |
| R-014 | Security | Pairing/relay compromise grants remote control | Critical | Medium | ECDH-confirmed enrollment, mTLS, revocation, no plain-secret bootstrap | Open |
| R-015 | Security | Malicious endpoint advertises false capabilities/cost | High | Medium | Signed descriptors plus active probes and policy bounds | Open |
| R-016 | Security | Clipboard/file route exfiltrates secrets | Critical | Medium | Typed sensitivity/provenance, WAN prompt/deny, limits and redaction | Open |
| R-017 | Security | OOB KVM exposed to public network | Critical | Medium | Management VLAN/overlay ACL, no port-forward default, separate credentials | Open |
| R-018 | Data | Object authority split or stale replica returned | Critical | Medium | Version/epoch authority, immutable replication, fail closed for mutable writes | Open |
| R-019 | Data | Object plane duplicates filesystem/database badly | High | High | Limit to runtime objects; bridge conventional authoritative stores | Mitigated by design |
| R-020 | Scheduler | Prediction causes migration/cache thrash | High | High | Hysteresis, cooldown, min residency, uncertainty margin, regret telemetry | Open |
| R-021 | Scheduler | Resource model misses memory/PCIe/encoder externality | High | High | Probe and pressure telemetry; counterfactual mixed-load benchmark | Open |
| R-022 | Scheduler | Moving background work slows it without protecting foreground | Medium | Medium | Measure both SLO benefit and displaced completion cost | Open |
| R-023 | Surface | Owned windows/popups/IME/DPI fail for arbitrary apps | High | High | Semantic route first; per-app compatibility and full-desktop fallback | Open |
| R-024 | Surface | Protected/DRM/UAC surface appears blank at critical time | High | Medium | Detect/fail explicitly and offer console/OOB/local confirmation | Open |
| R-025 | Media | Encoder-session exhaustion with many app proxies | High | Medium | Atlas, dedicated-stream selection, session admission and fallback | Open |
| R-026 | Clock | Cross-device clock drift causes audio glitches or bad timestamps | High | High | Clock domains, drift estimator, ASRC, uncertainty bounds, soak | Open |
| R-027 | Operations | Upgrade mismatches core/helper/adapter and strands control | Critical | Medium | Compatibility manifest, canary, transactional upgrade and rollback | Open |
| R-028 | Operations | Coordinator loss leaves routes in split-brain state | Critical | Low-Medium | Fenced lease epoch, data-plane lease expiry and warm replica | Open |
| R-029 | Operations | Recovery depends on same failed software stack | High | Medium | Independent OOB and local emergency route | Open |
| R-030 | Product | Expert graph overwhelms ordinary workflow | High | High | Simple workspace mode; patchbay optional; scenario usability tests | Open |
| R-031 | Product | One shell hides important quality/security changes | High | Medium | Route explanation and explicit profile/fallback indicators | Open |
| R-032 | Ecosystem | Fabric duplicates thegent/NVMS/ShareCLI authority | High | High | Boundary ADRs, shared contracts, repo consolidation decision | Open |
| R-033 | Ecosystem | Universal DB/ontology creates coupling and contested truth | High | Medium | Independent stores/authority and event/API reconciliation | Mitigated by design |
| R-034 | Vendor | Parsec/Apple/Microsoft/NVIDIA API or licensing changes | High | High | Adapter isolation, open alternatives, support matrix and exit plan | Open |
| R-035 | Open source | License combinations prevent packaged distribution | High | Medium | SBOM/license gate, process-boundary adapters, legal review before release | Open |
| R-036 | Anti-cheat/DRM | Input/capture/VM mechanisms conflict with games/content | High | High | Per-title/app compatibility; do not bypass protections; local/full console fallback | Open |
| R-037 | Research | Benchmark cherry-picking overstates novelty | High | Medium | Predeclared hypotheses, failed runs, counterfactuals and raw data | Open |
| R-038 | Hardware | Reference devices insufficient for general claims | Medium | High | Fingerprint-scoped support and broadened preview matrix over time | Open |
| R-039 | Thermal/power | Sustained media+compute throttles laptops/GPUs | High | High | Thermal/power as resource dimensions; profile adaptation | Open |
| R-040 | Privacy | Evidence bundles capture titles, audio, clipboard or files | Critical | Medium | Content-off default, redaction, explicit test consent and retention | Open |
| R-041 | Availability | External adapter project is archived/abandoned | Medium | High | Version pin, minimal fork policy, replaceable adapter contract | Open |
| R-042 | Business | No clear adoption wedge outside full ecosystem | High | Medium | Ship standalone seat/workspace/VM/Mac sink utility first | Open |

Risks close only with evidence or an accepted ADR, not because implementation has started.
