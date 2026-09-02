# PERT and Uncertainty Model

## Work-package estimates

| ID | Work package | O | M | P | Expected effort-weeks | σ | Dependencies |
|---|---|---|---|---|---|---|---|
| PF-WP-000 | Program baseline and governance | 1 | 2 | 4 | 2.2 | 0.5 | — |
| PF-WP-010 | Capability and topology inventory | 2 | 4 | 7 | 4.2 | 0.8 | PF-WP-000 |
| PF-WP-020 | Graph core and transaction engine | 3 | 5 | 8 | 5.2 | 0.8 | PF-WP-000 |
| PF-WP-030 | Linux seat and local media adapters | 3 | 6 | 10 | 6.2 | 1.2 | PF-WP-010, PF-WP-020 |
| PF-WP-040 | Same-host VM fast paths | 3 | 6 | 10 | 6.2 | 1.2 | PF-WP-010, PF-WP-020 |
| PF-WP-050 | Unified shell, workspaces and focus UX | 4 | 7 | 12 | 7.3 | 1.3 | PF-WP-020 |
| PF-WP-060 | Windows and macOS endpoint agents | 5 | 9 | 15 | 9.3 | 1.7 | PF-WP-010, PF-WP-020 |
| PF-WP-070 | LAN interactive media transport | 4 | 8 | 13 | 8.2 | 1.5 | PF-WP-030, PF-WP-060 |
| PF-WP-080 | Real-time audio, MIDI and clock domains | 5 | 9 | 15 | 9.3 | 1.7 | PF-WP-030, PF-WP-060 |
| PF-WP-090 | HDR, color and frame pacing | 4 | 8 | 14 | 8.3 | 1.7 | PF-WP-060, PF-WP-070 |
| PF-WP-100 | Agent realms and surface publishing | 3 | 6 | 10 | 6.2 | 1.2 | PF-WP-050, PF-WP-070 |
| PF-WP-110 | Object identity, authority and residency plane | 5 | 10 | 17 | 10.3 | 2.0 | PF-WP-010, PF-WP-020 |
| PF-WP-120 | Explicit TaskSpec placement scheduler | 5 | 10 | 16 | 10.2 | 1.8 | PF-WP-010, PF-WP-110 |
| PF-WP-130 | Adaptive execution regions and prediction | 6 | 12 | 22 | 12.7 | 2.7 | PF-WP-120 |
| PF-WP-140 | Semantic and pixel-proxy application surfaces | 7 | 14 | 24 | 14.5 | 2.8 | PF-WP-060, PF-WP-070, PF-WP-090 |
| PF-WP-150 | WAN, roaming and OOB integration | 4 | 8 | 14 | 8.3 | 1.7 | PF-WP-070, PF-WP-050 |
| PF-WP-160 | Security, identity and privileged-boundary program | 5 | 10 | 18 | 10.5 | 2.2 | PF-WP-000 |
| PF-WP-170 | Observability, benchmark and evidence program | 4 | 9 | 16 | 9.3 | 2.0 | PF-WP-000 |
| PF-WP-180 | Packaging, updates and operations | 5 | 10 | 18 | 10.5 | 2.2 | PF-WP-050, PF-WP-060, PF-WP-150, PF-WP-160, PF-WP-170 |
| PF-WP-190 | Selective atomic interposition research | 8 | 18 | 36 | 19.3 | 4.7 | PF-WP-110, PF-WP-120, PF-WP-170 |
| PF-WP-200 | Release candidate and acceptance | 3 | 6 | 12 | 6.5 | 1.5 | PF-WP-080, PF-WP-090, PF-WP-100, PF-WP-120, PF-WP-140, PF-WP-150, PF-WP-180 |

## Interpretation

- `O` assumes reusable adapters behave, reference hardware is available, and no signing/platform blocker appears.
- `M` assumes ordinary integration and one redesign cycle.
- `P` includes driver/API incompatibility, hardware-specific faults, security redesign and benchmark failures.
- These estimates cannot be added as calendar time because lanes overlap and staffing is not fixed.
- Research WP-190 is option-valued: failure to graduate is a valid result and does not block the packaged product.

## Risk-adjusted sequencing

Front-load high-information work: capability probes, same-host path validation, MacBook HDR/decode behavior, Ableton clock/latency lab, and platform permission/driver feasibility. A failed assumption discovered after proxy-window or scheduler implementation is substantially more expensive.
