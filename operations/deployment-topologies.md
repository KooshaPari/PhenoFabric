# Deployment Topologies

## T0 — One machine, host plus VMs

```text
Linux coordinator + endpoint
  ├─ evdev/uinput/PipeWire
  ├─ QEMU/VFIO Windows VM
  ├─ Looking Glass/KVMFR
  └─ SPICE/OOB fallback
```

All control/data stays local. This topology proves graph/lease/route semantics without networking.

## T1 — Main PC plus MacBook

Main PC may host Linux and Windows realms. MacBook is simultaneously an independent realm, display/audio sink and input source. Wired LAN media routes are direct; coordinator can run on either device with one elected authority.

## T2 — Room/lab fleet

1–5 bench PCs join a trusted but capability-scoped LAN. Software paths are preferred after boot; important systems also expose OOB KVM on an isolated management network.

## T3 — Worldwide peers

Direct encrypted peer path over overlay/NAT traversal where possible; explicit relay fallback. WAN routes are coarse by default, quality-adaptive and never allowed to hold an RT0 local audio island hostage.

## T4 — Agent-created ephemeral realms

Realm providers may be local VMs, native sandboxes, remote machines or cloud workers. Each has TTL, owner, budget, fallback console and published surfaces. The user sees one surface catalog rather than provider-specific consoles.

## Coordinator availability

The first deployment uses one local coordinator and optional warm replica. Data planes continue within their leases during brief coordinator loss. Strongly exclusive state uses fenced leases; discovery and telemetry can reconcile eventually.
