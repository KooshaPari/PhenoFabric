# Competing Interpretations and Falsification

The architecture assumes it may be wrong. Each selected direction is paired with plausible alternatives and conditions under which the alternative wins.

## A. One graph versus separate product modes

**Selected:** one typed graph with simplified modes layered above it.

**Alternatives:**
- separate remote desktop, KVM, audio, and scheduler products with loose launch integration;
- workflow scripts without a shared graph;
- a fixed “host” and many clients.

**Falsification:** If a common graph forces lowest-common-denominator semantics, creates measurable hot-path overhead, or makes ordinary operations materially harder, retain shared identity/policy but split graph runtimes by media/compute domain.

## B. Runtime route compiler versus user-selected backend

**Selected:** compiler chooses the least-expensive valid path and explains it.

**Alternatives:**
- user always selects Looking Glass/Parsec/Moonlight/RDP;
- static per-device profiles;
- one universal codec transport.

**Falsification:** If path prediction is unstable or route churn harms UX, pin routes by workspace and use automatic selection only during initial setup or failure.

## C. Fine-grained placement versus coarse workload scheduling

**Selected:** atomic capability with adaptive fusion/fission.

**Alternatives:**
- process/container/VM-only scheduling;
- explicit application-level RPC tasks only;
- instruction-level distributed shared memory.

**Falsification:** If all transparent interposition paths exceed 5–10% overhead outside narrow kernels, keep atomic mechanisms as observability/research tools and expose explicit task/region APIs as the product path.

## D. Shared object plane versus network filesystem

**Selected:** immutable object references and residency plus conventional mounts.

**Alternatives:**
- SMB/NFS only;
- distributed POSIX filesystem;
- database/blob-store only.

**Falsification:** If object identity/version semantics duplicate application storage and fail to create placement value, restrict the object plane to runtime artifacts, tensors, build outputs, and media buffers.

## E. Semantic application remoting versus pixel proxies

**Selected:** semantic protocols first, pixel proxy fallback.

**Alternatives:**
- always stream entire desktop;
- always capture individual windows as pixels;
- require application-native web frontends.

**Falsification:** If semantic protocols are unavailable/inconsistent for the target workload, use pixel proxies. If pixel proxies fail on popups, IME, protected surfaces, or encoder scaling, fall back to whole desktop.

## F. Distributed professional audio

**Selected:** local RT island by default; remote DSP only when the measured budget fits.

**Alternatives:**
- always distribute plugins/DSP to spare CPUs;
- never route audio over the network;
- use a commercial AoIP system only.

**Falsification:** Any repeatable xrun or unacceptable round-trip latency under the required buffer size disables remote DSP for that topology. Presentation and control can remain remote while DSP stays local.

## G. Control-plane replication

**Selected:** one authority per workspace with fencing; warm replica later.

**Alternatives:**
- fully decentralized eventual-consistency mesh;
- cloud central service;
- local-only independent nodes.

**Falsification:** If a coordinator adds failure/latency with no cross-device consistency benefit, move more policy to local nodes. Exclusive route ownership still requires an authoritative lease service or explicit physical local priority.

## H. Product boundary

**Selected:** separate fabric substrate integrated with NVMS/ShareCLI/thegent.

**Alternatives:**
- absorb into ShareCLI;
- make it the new NVMS major version;
- make it a thegent internal implementation;
- split compute and I/O into separate products.

**Falsification:** Decide based on external user story and independent adoption. If users primarily install it for interactive multi-device computing, it deserves its own product surface. If its only consumer is thegent, keep it internal to runtime infrastructure.
