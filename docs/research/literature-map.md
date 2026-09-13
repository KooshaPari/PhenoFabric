# Literature and Prior-Art Map

| Research concern | Prior-art families | What to borrow | What not to assume |
|---|---|---|---|
| Region/data placement | Legion/Realm, Regent, StarPU, PaRSEC | Logical regions, task variants, mapping, replicas, dataflow | That arbitrary binaries expose these semantics automatically |
| Adaptive granularity | Charm++, HPX, overdecomposition, task fusion | Fine units plus runtime aggregation/migration | That migration cost is negligible |
| Object plane | Ray/Plasma, Arrow, Dragon, distributed stores | Immutable refs, local SHM, spill, ownership metadata | That shared-object abstractions meet RT deadlines |
| Transport selection | UCX, libfabric, NIXL, MPI | Capability-driven SHM/TCP/RDMA/GPU paths | That one library covers every OS/device/security domain |
| Local media graph | PipeWire, JACK | Nodes/ports/links, negotiated buffers, RT callback discipline | That video, files and compute share audio timing semantics |
| VM shared surfaces | Looking Glass/KVMFR, Qubes GUI, virtio/vhost-user | Shared memory, fences, isolated service backends | Universal cross-OS GPU handle sharing |
| Semantic apps | RAIL/RemoteApp, WSLg, Xpra, Waypipe | Window ownership, clipboard/IME, persistence | Every app/session supports semantic publishing |
| WAN media | WebRTC, RTP/RTCP, SRT, game streamers | Pacing, timestamps, jitter, congestion, FEC | That video buffering can govern audio/input |
| Professional audio | JACK/NetJACK, Dante/AES67/AVB, AudioGridder | Clocks, period/admission, remote DSP nodes | Ordinary WAN can provide sample-synchronous guarantees |
| Distributed OS research | DSM, PGAS, Plan 9/Sprite/MOSIX-style ideas | Location transparency with explicit locality realities | Uniform remote memory or universal syscall RPC |
| Security | capability systems, mTLS, portal mediation, fencing tokens | Narrow authority and explicit user consent | A trusted LAN or single user eliminates adversaries |
