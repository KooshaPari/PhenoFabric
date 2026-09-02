# Program Plan

## Objective

Deliver a packaged all-in-one interactive fabric in useful slices while separately maturing the deeper heterogeneous compute/data plane. The program is organized around evidence-bearing WorkPackages in `work/` and AgilePlus-shaped feature specs in `specs/`.

## Phase map

| Phase | Objective | Principal deliverables | Exit gate |
|---|---|---|---|
| P0 Truth and governance | Freeze boundaries, schemas, measurements and source discipline | intent corpus, requirements, ADRs, topology/latency lab | docs/schema checks; calibrated probes |
| P1 Core | Build graph transactions, capability inventory, identity/security and evidence spine | control plane, node/port/link model, leases, descriptors | graph/property/fault contracts |
| P2 Local product | Make host+VMs feel like one reliable system | Linux broker, Looking Glass/SPICE/virtio, shell/workspaces | local seat and rollback acceptance |
| P3 Cross-device RT media | Add Windows/macOS endpoints, MacBook sink, LAN media, audio and HDR | desk↔couch, creator/gaming profiles | mixed-load reference gates |
| P4 Agent and compute baseline | Publish ephemeral realms; place explicit tasks/objects | realm API, object plane, TaskSpec scheduler | agent and placement counterfactuals |
| P5 Seamless/adaptive fabric | Add semantic/pixel app surfaces, WAN/OOB and adaptive regions | app portal, roaming, fusion/fission | compatibility/fault/security gates |
| P6 Packaged release | Signed installation, updates, recovery and acceptance | one installer per OS, support matrix, release evidence | RC gates in verification set |
| R Research | Evaluate atomic interposition and advanced fabrics | FUSE/syscall/function/kernel, RDMA/CXL/GPU experiments | graduate/narrow/reject by evidence |

## Work and dependency details

- [`work/wbs.md`](work/wbs.md)
- [`work/dag.md`](work/dag.md)
- [`work/pert.md`](work/pert.md)
- [`work/critical-path.md`](work/critical-path.md)
- [`work/build-order.md`](work/build-order.md)

## Planning rule

The research layer may improve the product but cannot hold local seat/workspace, MacBook display, agent-surface or explicit-placement value hostage. Conversely, early UI demos cannot be used to claim the compute mesh exists.
