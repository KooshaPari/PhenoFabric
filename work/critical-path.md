# Critical Path and Release Slices

## Likely packaged-product critical path

```text
PF-WP-000
 → PF-WP-020 graph core
 → PF-WP-050 shell/workspaces
 → PF-WP-060 Windows/macOS endpoints
 → PF-WP-070 LAN media
 → PF-WP-090 HDR/video
 → PF-WP-140 seamless surfaces
 → PF-WP-180 packaging
 → PF-WP-200 acceptance
```

Security and evidence are parallel continuous lanes but become release gates. RT audio is a separate critical branch for the “Ableton-safe” product claim. The object/scheduler branch is critical for the “single distributed compute plane” claim but not for the earliest useful I/O shell.

## Release slices

| Slice | User value | Required WPs | Explicitly deferred |
|---|---|---|---|
| R0 Lab harness | Measured topology and route experiments | 000, 010, 160 skeleton, 170 skeleton | Polished UI, compute mesh |
| R1 Local fabric | One seat across Linux host and local VMs with LG/SPICE/audio | 020, 030, 040, 050 subset | macOS/Windows client parity, WAN |
| R2 Desk/couch | MacBook sink, LAN display/audio/input, workspaces | 060, 070, 090 subset, 180 skeleton | Seamless arbitrary windows, compute mesh |
| R3 Creator/gaming | Protected Ableton/game service classes under mixed load | 080, 090, 120 reservations, 170 | Atomic routing |
| R4 Agent fabric | Ephemeral realms, published surfaces, explicit task placement | 100, 110, 120 | Fine-grained adaptive regions |
| R5 Seamless fabric | Semantic/pixel app surfaces and WAN/OOB | 140, 150 | General cross-OS process migration |
| R6 Adaptive fabric | Fusion/fission, prediction and selected graduated atomic paths | 130 and successful parts of 190 | Unsupported/unprofitable interposition |

## Kill criteria

A release claim is removed rather than weakened silently when its measured path cannot meet acceptance. Example: “Ableton-safe WAN DSP” is not shipped if the admitted graph cannot prevent xruns under the defined impairment envelope.
