# Data and Object Plane

## Purpose

Execution placement is meaningless without data authority and residency. The object plane is not a replacement for filesystems or databases; it is the runtime's data-movement and locality contract.

## Object record

```yaml
object:
  id: sha256:...
  logical_name: optional
  version: 7
  size: 8.2GiB
  type: tensor|artifact|file_chunk|frame_set|build_output|checkpoint
  mutability: immutable
  authority: pf://realm/main-pc/worker-1
  consistency: immutable
  residency:
    - resource: pf://resource/main-pc/gpu0/vram
      state: resident
      verified_at: ...
    - resource: pf://resource/bench2/nvme
      state: cached
  durability:
    required: true
    locations: [...]
```

## Mutability modes

1. **Immutable value:** freely replicated; content hash is identity.
2. **Single-writer versioned:** one authority creates new immutable versions.
3. **Actor/service state:** accessed through owner operations; no direct replica mutation.
4. **Explicit shared consistency:** only for a named protocol with measured cost.
5. **Ephemeral buffer:** lifetime bound to route/region, no durability promise.

## Local storage tiers

- GPU/device memory;
- pinned host memory;
- shared-memory object store;
- normal RAM;
- NVMe cache;
- durable local filesystem;
- remote peer cache;
- object/blob store.

Spill is policy-driven and never allowed to block RT threads directly.

## Transfer plan

Transfers are chunked, content-verified, resumable, and peer-to-peer. Candidate mechanisms include:

- shared memory/memfd;
- DMA-BUF/device handles;
- IVSHMEM/virtiofs;
- direct file clone/reflink;
- QUIC streams;
- RDMA/UCX/NIXL experiments;
- conventional SMB/NFS/SFTP;
- object storage.

## Prefetch

Prediction inputs:

- task DAG;
- workspace activation;
- recent execution sequence;
- application launch;
- agent plan;
- file/object access;
- mobility/handoff intent.

Prefetch may not evict protected working sets or saturate RT paths.

## Garbage collection

Objects use:

- distributed references/leases;
- explicit pinning;
- workspace/run retention;
- evidence retention;
- cache pressure/priority;
- durable authority check.

A node may evict a cache only after confirming it is not the sole durable copy.

## Security

Objects carry labels and allowed principals/zones. A placement cannot move data to a faster node outside the permitted trust/residency boundary.
