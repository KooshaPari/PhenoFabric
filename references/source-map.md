# Claim-to-Source Map

| Claim/domain | Load-bearing primary sources | Principal documents |
|---|---|---|
| Graph nodes/ports/links, RT processing and shared buffers | PipeWire docs; JACK2 docs | ALD, HLD, graph-runtime, realtime-audio |
| Same-host VFIO shared-memory/GPU-DMA path | Looking Glass/KVMFR docs; QEMU IVSHMEM | HLD/LLD, route compiler, SOTA, examples |
| Wayland authorized input/capture/clipboard | libei/EIS; XDG portals | platform-linux, input, security |
| Semantic Windows/Linux applications | RDP/RAIL/FreeRDP, WSLg, Xpra, Waypipe | surface-proxy, seamless SOTA |
| Task/data-aware heterogeneous runtimes | Legion/Realm, StarPU, PaRSEC, Ray | scheduler/object plane/research |
| Transport abstraction and direct data movement | UCX, libfabric, NIXL, GPUDirect, DMA-BUF | route compiler/network/object plane |
| Professional audio clocks/network/DSP | JACK/NetJACK, IEEE 1588, AES67/Dante/AudioGridder | realtime-audio, audio SOTA/tests |
| Phenotype authority boundaries | AgilePlus/sharecli repos and user whitepapers/conversation | GOVERNANCE, ecosystem/*, ADR-0015/16 |

Detailed URLs and confidence labels are in `research/source-register.md` and each SOTA matrix.
