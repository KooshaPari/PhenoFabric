# Heterogeneous Scheduler and Adaptive Execution Regions

## Scheduling hierarchy

```text
application
  process
    execution region
      thread/task
        function/RPC
          syscall/I/O operation
            accelerator kernel
```

The runtime chooses the lowest practical boundary available to the workload and topology.

## Explicit first, transparent later

Priority order:

1. explicit TaskSpec/RegionSpec;
2. process/build/test/inference adapters;
3. application plugin/SDK nodes;
4. runtime/compiler instrumentation;
5. selected syscall/FUSE/eBPF/interception;
6. experimental transparent mechanisms.

This avoids making the product depend on opaque binary rewriting.

## Resource descriptor

```yaml
resource:
  kind: cpu|gpu|npu|fpga|memory|storage|network|codec
  topology:
    device: main-pc
    numa_node: 0
    pcie_path: "0000:00/..."
    peers: [...]
  capacity:
    compute: ...
    memory_bytes: ...
    bandwidth: ...
  pressure:
    queue: ...
    utilization: ...
    thermal: ...
  capabilities: [...]
  reservations: [...]
```

## Placement request

```yaml
region:
  code_ref: sha256:...
  inputs: [object://...]
  outputs: [{estimate_bytes: 4096}]
  requirements:
    accelerator: cuda
    vram_bytes: 12GiB
    deadline_ms: 100
    trust_zone: personal
  mobility:
    checkpointable: true
    rematerializable: true
```

## Fusion state

The runtime maintains edge statistics between adjacent operations:

- destination agreement;
- data dependence density;
- shared mutable state;
- marshal bytes;
- decision overhead;
- synchronization;
- failure/cancellation semantics.

When fusion score exceeds threshold, the region is recompiled as one unit. Fission triggers on divergent capability or locality.

## Placement example

An 8 GB tensor is resident on GPU B and the result is 20 bytes. GPU A is 2× faster but requires 8 GB WAN transfer. GPU B wins. If the input already exists on A or the operation repeats enough times to amortize transfer, A may win.

## Protected-workload interaction

The scheduler assigns a shadow price to consuming resources reserved by RT work. A background task may be locally faster yet globally worse if it creates game frame-time or audio xrun risk.

## Preemption and migration

Preferred response order:

1. delay admission;
2. lower concurrency;
3. throttle;
4. pause/checkpoint;
5. move/rematerialize;
6. cancel lower-value work.

Opaque live migration is not the default.

## Speculation

Speculative duplicates are allowed only when:

- outputs are immutable/idempotent;
- duplicate cost fits budget;
- cancellation is safe;
- expected tail reduction exceeds resource impact;
- protected workload is not threatened.

## Explainability output

```text
Placed cargo codegen region on bench-pc-2:
+ 12 idle CPU threads
+ source/cache already present
+ main PC CPU cores reserved for Ableton
- 1.4 GB result transfer
- bench node reliability margin
Net predicted completion: 31 s vs 48 s local
Confidence: 0.72
```
