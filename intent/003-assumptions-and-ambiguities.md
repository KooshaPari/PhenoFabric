# Assumptions and Ambiguities

## Assumptions adopted

1. **Working name only.** “Phenotype Fabric” is used to make the package coherent; branding remains provisional.
2. **Linux virtualization host is the first deep-integration platform.** The reference main PC is assumed to run Linux with Windows VMs, because evdev/uinput, KVM/QEMU, Looking Glass/KVMFR, PipeWire, cgroups, and topology access make it the strongest initial substrate.
3. **MacBook is both computer and sink.** It must retain native macOS operation while presenting remote desktops/windows.
4. **Windows guest is performance-sensitive.** It may host games, Ableton, and GPU-passthrough workloads.
5. **“User” in the original prompt means interactive target, not identity.** The domain model separates Principal, Realm, Session, Seat, and Surface.
6. **The unified UI/API may wrap proprietary products.** A Parsec adapter is acceptable, but core operation must not require one vendor.
7. **Cross-platform parity means semantic parity, not identical internals.**
8. **“Atomic” means smallest available safe interception boundary, not magical instruction-level migration.**
9. **The first release may be local-first and single-owner while preserving multi-principal contracts.**
10. **The coordinator is logically singular per workspace but may be replicated; the data plane remains peer-to-peer.**

## Material ambiguities retained

| Question | Why it matters | Default for planning |
|---|---|---|
| Is this a new top-level repo or NVMS major evolution? | Affects ownership and public narrative. | New provisional docs package; boundary ADR requires ecosystem decision. |
| Is Ableton expected to execute in a Windows VM or bare metal? | Driver/ASIO and RT guarantees differ sharply. | Support both; validate bare metal first, VM as explicit topology. |
| Which audio interface/controller hardware is used? | Determines ASIO, clock, channel, and round-trip targets. | Use synthetic loopback and generic USB interface until inventory exists. |
| Is the main LAN 1, 2.5, or 10 GbE? | Changes raw/light-compression feasibility. | 1/2.5 GbE baseline, 10+ GbE research tier. |
| Is the 1080 Ti active, repaired, or absent? | Affects spare encoder/GPU scheduling experiments. | Optional resource; architecture does not depend on it. |
| Must macOS host virtual displays or only act as sink/source? | macOS driver/API scope differs. | Initial Mac acts as sink and local seat; host publishing is adapter-dependent. |
| How much proprietary integration is acceptable? | Licensing and packaging risk. | Wrap optional proprietary tools; core paths have open substitutes. |
| Is multi-user collaboration required? | Changes auth and consistency scope. | Multi-principal model, single active human seat in initial product. |
| Is process interposition allowed to require app instrumentation? | Determines feasibility of function/task granularity. | Explicit SDK/instrumented path first; transparent interposition remains research. |
| What degree of WAN relay infrastructure is desired? | Cost and privacy implications. | Self-hostable blind relay, direct-first. |

## Deliberately rejected inferences

- The user did not ask for a cloud-only service.
- The user did not ask to replace each underlying open-source project.
- The user did not claim all connected RAM/VRAM can be made coherent or equally fast.
- The user did not require every syscall to be routed remotely.
- The user did not require hidden or permission-bypassing capture/input on macOS or Wayland.
- The user did not authorize agents to seize focus.
- The user did not ask for a universal enterprise database.
