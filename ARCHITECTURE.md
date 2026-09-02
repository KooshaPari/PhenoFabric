# Architecture

## What this is

Phenotype Fabric is a **runtime substrate** for a distributed interactive
operating environment. It exposes one graph, one workspace model, and one
UI/API while compiling each link (input, output, file, process, object,
storage) onto the least-expensive valid execution path: direct calls,
shared memory, DMA-BUF, IVSHMEM/KVMFR, virtio/vhost, PCIe peer paths,
LAN transports, WAN transports, or out-of-band hardware.

It is **not a remote desktop**, **not a cluster scheduler**, and **not a
replacement for adjacent products** (AgilePlus, thegent, AGSLAG, Tracera,
SessionLedger, ShareCLI). It is the physical-execution substrate those
products compose with.

**Lifecycle:** `specified` — architecture baseline. No code is shipped yet.

## Tech stack

| Layer | Choice | Why |
|-------|--------|-----|
| Spec language | Markdown + JSON schemas + OpenAPI 3 + Protobuf | Spec is the artifact; verifiability matters more than runtime |
| Graph schema | `architecture/schemas/graph.schema.json` (typed node/port/link) | Universal I/O across video/audio/HID/file/object/control |
| Event schema | `architecture/schemas/event.proto` | Forward-compatible, language-neutral, signed event stream |
| Policy schema | `architecture/schemas/policy.schema.json` | Declarative; reviewed by humans and tools |
| Workspace schema | `architecture/schemas/workspace.schema.json` | Serializable cross-device state |
| Capability schema | `architecture/schemas/capability.schema.json` | Versioned adapter descriptors |
| Route schema | `architecture/schemas/route.schema.json` | Compiled route plans |
| Task schema | `architecture/schemas/task.schema.json` | Placed work descriptors |
| Process supervisors | `process-compose` (planned) | Cross-platform, well-understood |
| Real-time engine | `pf-rt-engine` (planned) | Pre-allocated rings, no dynamic allocation on RT threads |
| Real OS data planes (planned) | Looking Glass/KVMFR, PipeWire/JACK, evdev/uinput, RDP/RAIL, Sunshine/Moonlight, QUIC, virtio, SMB/NFS, Syncthing, KVM-over-IP | Proven adapters — Fabric composes, doesn't reinvent |
| Verification harness | `verification/` (benchmark, fault, security, compat) | Evidence before claims |

> **Status note:** the implementation language/runtime is not yet chosen.
> HLD and LLD name components (`pf-coordinator`, `pf-endpointd`,
> `pf-rt-engine`, etc.) and contracts but do not lock to a stack. See
> ADR-0020 — provisional name, "the right Rust vs Go vs C vs language-X
> decision is a separate ADR once we know which components must be
> privileged, real-time, portable, or easily inspected."

## High-level diagram

```
                +--------------------+
                |  Human principal   |
                +--------------------+
                          |
                          v
+-----------------+   +-------------------+   +--------------------+
|  Agent/service  |-->|   pf-shell (UI)   |<--|   CLI / SDK / API   |
|   principals    |   +-------------------+   +--------------------+
+-----------------+             |
                                v
+----------------------------------------------------------+
|                  pf-coordinator (control)                |
|  - identity, capability grants                           |
|  - device/realm/resource registry                        |
|  - desired graph, topology epochs                        |
|  - route leases, fencing tokens                          |
|  - audit + evidence refs                                 |
+----------------------------------------------------------+
        |               |               |               |
        v               v               v               v
+------------+  +-----------------+  +---------------+  +-------------+
|  Graph +   |  |  pf-rt-engine   |  | I/O + surface |  | Compute +   |
|  placement |  | (real-time      |  | data planes   |  | data plane  |
|  compiler  |  |  scheduler +    |  | (KVMFR,       |  | (placement, |
|            |  |  admission)     |  |  PipeWire,    |  |  residency, |
|            |  |                 |  |  evdev, etc.) |  |  migration) |
+------------+  +-----------------+  +---------------+  +-------------+
        \             |              |              /
         \            v              v             /
          +----------------------------------------+
          |      Endpoint agents (per device)      |
          |   pf-endpointd + narrowly-scoped       |
          |   helpers: input / display / audio /   |
          |   resource / oob                       |
          +----------------------------------------+
                          |
                          v
+------------------------------------------------------------+
|   Nodes: Linux / Windows / macOS / VMs / remote / OOB hw   |
+------------------------------------------------------------+

        |   Evidence & event flow   |
        v
+----------------------------------------------------------------+
| Ecosystem: AgilePlus · thegent · AGSLAG · Tracera · Ledgers · |
|           ShareCLI · NVMS · labs-compute                       |
+----------------------------------------------------------------+
```

## Directory map

| Path | Purpose | Touch when... |
|------|---------|---------------|
| `PRD.md` | Product Requirements (vision, JTBDs, epics, success metrics) | revising scope or releasing |
| `SPECIFICATION.md` | Normative index — "shall/should/may" language, mandatory invariants | resolving a behavioral conflict |
| `HLD.md` | High-level system architecture + locality tiers L0–L8 | changing component boundaries |
| `ALD.md` | Architecture/abstraction lowering (intent → physical) | extending the compilation pipeline |
| `LLD.md` | Low-level component + hot-path design (cost model, fusion/fission, RT rules) | implementing a data-plane component |
| `UX_SPECIFICATION.md` | Workspace + graph interaction contracts | designing the UI |
| `GOVERNANCE.md` | Decision authority, lifecycle, evidence rule, ADR rule, security triggers | proposing an architecture change |
| `GOVERNANCE.md#specification-lifecycle` | Captured → Synthesized → Specified → Researched → Planned → Implementing → Validated → Shipped → Retrospected | promoting any document/release |
| `intent/` | Verbatim human prompts + intent synthesis + non-negotiables | referencing the original goals |
| `specs/001..012/` | 12 AgilePlus-shaped feature specs (E1–E12 epics) | implementing a feature |
| `adr/` | 22 architecture decision records + INDEX | recording a decision or rejection |
| `architecture/` | 14 component docs + openapi.yaml + 6 JSON schemas + 1 proto | editing a component contract |
| `architecture/schemas/` | graph, capability, route, policy, task, workspace (JSON) + event.proto | changing the wire schema |
| `ecosystem/` | boundaries.md, sharecli.md, thegent.md, agileplus.md, agslag.md, tracera.md, ledgers.md, nvms-labs-compute.md, event-contracts.{md,json}, integration-matrix.md | integrating with adjacent products |
| `examples/` | 8 reference scenarios + 2 graph-config YAML examples | demonstrating a feature to a user |
| `operations/` | config-and-policy, deployment-topologies, incident-response, observability, packaging, recovery, slo-sla, support-matrix, upgrades | deploying or operating Fabric |
| `references/` | BIBLIOGRAPHY, source-map, source-status, url-inventory | citing evidence or research |
| `research/` | hypotheses, experiments, graduation-gates, literature-map, open-questions, research-program, source-register | designing a new mechanism |
| `risks/` | feasibility, legal-licensing, risk-register.{md,json}, threat-model, vendor-dependency | assessing feasibility/lock-in |
| `sota/` | 14 competitive-class analyses (deskflow, parsec, looking-glass, evdev, seamless-app, audio-network, etc.) | understanding existing solutions |
| `verification/` | acceptance-gates, audio-rt-test-plan, benchmark-plan, compatibility-matrix, fault-injection, latency-methodology, requirements-traceability.{md,json}, security-test-plan, video-hdr-test-plan, benchmark-catalog.json, fault-catalog.json | proving a claim |
| `work/` | WBS, DAG, PERT, critical-path, milestones, build-order, dependency-register, staffing-agent-plan, release-plan, cost-model, tasks.json (137 tasks), wbs.csv | planning the program |
| `INDEX.md`, `TREE.txt`, `MANIFEST.sha256` | Navigation + file hash manifest | finding a file / verifying integrity |

## Key abstractions

The non-negotiable objects new contributors must understand to read any
component of Fabric:

1. **Node / port / link** — the universal I/O graph (E2). A *node* is a
   realm, device, application, process, surface, or object. A *port* is a
   typed I/O endpoint. A *link* connects ports with a policy. The
   compiler consumes a desired link and emits a route plan.
2. **Locality tier (L0–L8)** — the physical execution boundary the
   compiler reasons about. L0 same-process, L1 same-OS, L2 same-host
   different realm, L3 same-PCIe, L4 same-host isolated, L5 wired LAN,
   L6 routed LAN/Wi-Fi, L7 WAN, L8 OOB/preboot. See `HLD.md`.
3. **Stage** — a unit of work the compiler composes into a route.
   Stages are `capture|transform|copy|encode|transport|decode|compose|inject|store`
   and advertise fixed cost, per-byte cost, queue model, locality,
   resource claims, timing class, and failure modes. See `LLD.md`.
4. **Topology epoch** — a signed, versioned snapshot of the device/realm
   registry. Every route plan is bound to an epoch; topology changes
   force recompilation, never silent mid-stream changes.
5. **Fencing token** — a monotonically increasing token that proves a
   route lease is still valid. Stale tokens cannot produce input or
   output after lease transfer. See `architecture/state-machines.md`.
6. **RT class (RT0–Bulk)** — explicit service class. RT0 = hard deadline
   (input, audio), RT1 = soft (UI), RT2 = streaming, RT3 = bulk, Bulk =
   background. RT0 reservations are admitted before activation; no
   silent overcommit. See `architecture/scheduler.md`.
7. **Ecosystem authority** — Fabric owns *physical execution substrate*;
   it explicitly does **not** own governed intent (AgilePlus), agent
   dispatch (thegent), economic allocation (AGSLAG), evidence graph
   (Tracera), session history (SessionLedger), or process supervision
   (ShareCLI). Event contracts are the only shared boundary. See
   `ecosystem/boundaries.md`.

## Data flow for the 3 most important user actions

### 1. User moves input focus from desk PC to couch MacBook

```
User presses "global input cycle" hotkey
  -> pf-shell posts focus change to pf-coordinator
  -> coordinator validates identity + capability + topology epoch
  -> graph compiler selects new route plan:
       input source = desk-PC-evdev-broker
       input sink    = macbook-uinput-helper
       candidates ordered by locality tier:
         L2 IVSHMEM (rejected: not available)
         L5 LAN direct  (rejected: pressure on link)
         L7 WAN relay   (rejected: deadline)
         L0 same-host   (rejected: across host boundary)
       winner: L5 LAN direct QUIC datagram with pressed-key ledger
  -> RT admission reserves input-class slice on both endpoints
  -> coordinator allocates fencing token N+1
  -> endpoints COMMIT(token=N+1, activation_time=T)
  -> previous route drains, releases its lease
  -> coordinator updates desired graph version to N+1
  -> evidence exporter logs route + cost to Tracera
```

### 2. Agent creates a VM and publishes a desktop surface

```
Agent calls API: create_ephemeral_realm(spec, ttl=30m, surfaces=["desktop"])
  -> pf-coordinator validates principal identity + capability grant
  -> coordinator requests capability inventory from endpointd on chosen host
  -> graph compiler allocates resources: CPU/GPU/memory/storage slice
  -> pf-workerd spawns the VM (via QEMU/KVM + virtio)
  -> VM lifecycle wired into a transient realm; certificate + TTL issued
  -> VM advertises a virtual display + window via Looking Glass/KVMFR
  -> agent publishes the surface to the user's graph canvas
  -> non-stealing attention: the surface appears pinned to a corner;
     user opts in to take focus
  -> TTL countdown starts; on expiry, the realm is torn down
     and the surface is retracted; evidence ref retained
```

### 3. User launches Ableton while a Rust build is running

```
User starts Ableton Live 12 Suite
  -> pf-coordinator classifies the workload: RT0 (audio hard deadline)
  -> RT admission checks current reservations:
       RT0 input: 200us budget
       RT0 audio: 1500us buffer at 48kHz
       Bulk: rust build currently consuming 12 cores + encoder
  -> admission prunes the Bulk class first:
       CPU affinity pinned to non-RT cores (E9.2)
       GPU encoder reservation capped (E9.3)
       memory bandwidth share reduced
  -> Ableton launches with audio class reservation
  -> scheduler reports xrun count = 0; p99 buffer fill = 78%
  -> if xrun threshold exceeded: Bulk further degraded before audio breaks
  -> evidence: topology snapshot + p50/p95/p99/xrun count
     -> exported to Tracera, attached to the work-package ID in AgilePlus
```

## How to run locally

There is no implementation to run yet. Spec validation:

```bash
# Verify all 6 JSON schemas are well-formed
for s in architecture/schemas/*.json; do
  python3 -c "import json; json.load(open('$s'))" && echo "OK: $s"
done

# Verify event.proto is syntactically valid
which protoc && protoc --proto_path=architecture/schemas \
  --descriptor_set_out=/dev/null architecture/schemas/event.proto

# Verify openapi.yaml is well-formed
python3 -c "import yaml; yaml.safe_load(open('architecture/openapi.yaml'))"

# Verify the doc tree, manifest, and traceability are consistent
diff <(find . -type f | sort) <(python3 -c "
import json
m = json.load(open('MANIFEST.sha256'))
print('\n'.join(sorted(m.keys())))")
```

Future:

```bash
# Once the implementation lands:
cargo build --release  # (or go build, etc. — see ADR for stack choice)
./target/release/pf-coordinator --config /etc/pf/coordinator.toml &
./target/release/pf-endpointd --config /etc/pf/endpointd.toml &
./target/release/pf-shell
```

## How tests work

There are no unit tests yet (no implementation). The verification
artifacts in `verification/` define the gates a future implementation
must pass:

- `test-strategy.md` — overall strategy
- `acceptance-gates.md` — release-blocking criteria
- `benchmark-plan.md` + `benchmark-catalog.json` — performance evidence
- `fault-injection.md` + `fault-catalog.json` — failure modes
- `security-test-plan.md` — security evidence
- `audio-rt-test-plan.md` — real-time audio evidence
- `video-hdr-test-plan.md` — HDR/video evidence
- `latency-methodology.md` — measurement boundary discipline
- `compatibility-matrix.md` — adapter / OS / device support
- `requirements-traceability.{md,json}` — every claim linked to FR/NFR

## Where the risk lives

The 5 areas most likely to bite new contributors, in order of severity:

1. **Lying about "transparent" / "zero-copy" / "hard real-time"** — the
   evidence rule (`GOVERNANCE.md#evidence-rule`) is non-negotiable.
   These claims require topology, version, workload, p50/p95/p99/worst,
   failure behavior, and reproducible scripts. No claim is accepted
   based on architecture intent alone.
2. **Adding a 13th top-level service or repo** — `CONTRIBUTING.md`
   forbids this. The integration boundary must be an adapter or
   package, not a new product. ShareCLI, thegent, AgilePlus, AGSLAG,
   Tracera, SessionLedger already own adjacent responsibilities.
3. **Coupling presentation to execution** — non-goal. A window may live
   on the MacBook while its process, GPU kernels, and data execute
   elsewhere. The shared-memory/data plane (`architecture/object-plane.md`)
   and surface-proxy (`architecture/surface-proxy.md`) keep these
   independent.
4. **Privileged code scope** — `pf-input-helper`, `pf-display-helper`,
   `pf-audio-helper`, `pf-resource-helper`, `pf-oob-adapter` are the
   only narrowly-scoped privileged components. Anything that requires
   kernel/driver installation, input capture/injection, screen/audio
   capture, virtual display/audio/HID creation, or remote wake must
   trigger a security review per `GOVERNANCE.md#security-review-triggers`.
5. **Topology / configuration drift** — the spec lifecycle requires
   every behavioral change to update WBS, traceability, tests, risks,
   source register, and compatibility data. A change that touches
   behavior without touching these is rejected at review.

## Open questions / known gaps

These are the explicit open questions at the end of the 0.1 baseline.
A new contributor should investigate in roughly this order:

1. **Implementation language** — ADR-0020 leaves the Rust/Go/C/
   language-X choice undecided. The right answer depends on which
   components must be privileged, real-time, portable, or easily
   inspected. The next ADR is "choose the language for the first
   three components: pf-coordinator, pf-endpointd, pf-rt-engine."
2. **Atomic placement vs fused regions** — the cost model and
   fusion/fission rules in `LLD.md` are theory. The `research/`
   directory has open hypotheses; the next R&D step is to *measure*
   the local baseline and prove the coordination tax before any
   interposition code is written.
3. **Worst-case latency bound** — `NON_FUNCTIONAL_REQUIREMENTS.md`
   names targets (input p95 < 50ms, prepared video first frame < 250ms)
   but the source of those numbers is the user's reference environment,
   not measurement. Real p50/p95/p99/worst numbers must be collected
   before any release can claim "shipped."
4. **WAN real-time guarantees** — explicitly a non-goal. WAN
   routes are best-effort with adaptive degradation. The fallback
   for an audio stream that misses deadlines is graceful
   concealment, not retransmission.
5. **R0 scope** — the first independently-valuable release is
   "Measurement and adapter lab" (topology inventory + benchmark
   harness + Looking Glass/PipeWire/evdev adapters). It has not been
   broken into atomic tasks beyond `work/tasks.json` PF-WP-000 (5)
   and PF-WP-010 (6) tasks. The rest of the WBS is "Planned" but not
   sized for a single engineer-week.
6. **Spec lifecycle** — the spec is at `Specified` (per
   `GOVERNANCE.md#specification-lifecycle`). Promotion to `Researched`
   requires the research program to produce measurable evidence for
   the data-locality and adaptive-granularity claims. The
   `research/research-program.md` defines the next experiments.

## What to read first

In order, for a new contributor:

1. `PRD.md` — the why
2. `README.md` — the non-negotiables
3. `SPECIFICATION.md` — the mandatory invariants
4. `HLD.md` — the components + locality tiers
5. `LLD.md` — the hot-path rules + cost model
6. `GOVERNANCE.md` — the rules of engagement
7. `adr/INDEX.md` + `work/wbs.md` — the decisions and the plan
8. The single spec that matches the work you're doing under `specs/`
9. The single component doc under `architecture/` for the component
   you're touching
10. The matching `verification/` test plan + `verification/requirements-traceability.json`
    before claiming a feature is "done"
