# Build, Buy, Wrap, or Research Decisions

| Capability | Decision | Rationale | Exit / replacement trigger |
|---|---|---|---|
| Unified graph, workspace, policy and route compiler | **Build** | This is the product-defining layer and does not exist as one system. | None; APIs may evolve. |
| Linux local media graph | **Wrap PipeWire/JACK** | Mature graph, buffers and RT semantics. | Build only missing generalized node types/adapters. |
| Same-host VFIO display | **Wrap Looking Glass/KVMFR** | Proven specialized fast path. | Replace only with measured lower latency/copy count and equivalent robustness. |
| Encoded open desktop | **Wrap Sunshine/Moonlight** | Mature codec/client ecosystem. | Add/replace stages where missing HDR, virtual display, telemetry or API control blocks requirements. |
| Convenient WAN desktop | **Integrate Parsec optionally** | Strong user experience and reach. | Remove if API/licensing/security prevents reliable integration. |
| Windows app semantics | **Wrap RDP/RAIL/FreeRDP** | Semantic window path beats pixels where supported. | Pixel proxy only for unsupported applications/sessions. |
| Linux app semantics | **Wrap Xpra/Waypipe** | Existing application-aware paths. | Build compositor-specific adapter only for missing semantics. |
| Pixel-proxy native windows | **Build selective adapter** | Required universal fallback; no mature open cross-OS solution covers target. | Prefer semantic route whenever it reaches acceptance. |
| Input broker and leases | **Build around evdev/uinput/libei/VHF/CGEvent** | Existing software KVMs do not meet lease/state/realm requirements. | Reuse a project if it exposes the required typed data plane and guarantees. |
| Audio graph | **Wrap platform APIs, build distributed policy/clock layer** | Local drivers/graphs are mature; cross-device integration is product-specific. | Adopt standards/products where they expose needed clocks/API. |
| Compute scheduler/object plane | **Build thin core; integrate runtimes/transports** | Cross-plane cost model and adaptive regions are novel; kernels/transports are not. | Delegate domain-specific workloads to Legion/StarPU/Ray/Slurm/etc. through adapters. |
| OOB KVM | **Buy/wrap hardware** | Reinventing capture/HID/ATX hardware adds no product edge. | Standardize on devices with stable local API/security. |
| Files/sync | **Wrap Syncthing/SMB/NFS/virtiofs/object stores** | Mature data systems. | Build only typed transfer/provenance UX and route selection. |
| Identity/crypto | **Use platform stores and standard cryptography** | Custom crypto is unjustified. | None. |
| Atomic syscall/function interposition | **Research, gated** | Potentially useful but overhead and correctness risk are severe. | Graduate only when a workload shows repeatable net benefit after all costs. |
