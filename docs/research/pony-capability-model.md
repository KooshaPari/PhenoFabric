# Pony Capability Model Analysis for Fabric Type-Level Safety

**Date:** 2026-09-13
**Author:** Fabric Research Team
**Status:** Research proposal

## Abstract

Pony's reference capability system prevents data races at compile time by encoding ownership and aliasing rules in the type system. This document maps Pony's six reference capabilities to Phenotype Fabric's existing concepts (LocalityTier, TrustScope, seat leases), proposes a Rust encoding using PhantomData markers, and evaluates whether the implementation cost is justified.

---

## 1. Pony Reference Capabilities Overview

Pony is an actor-model language where data-race freedom is guaranteed by the type system rather than a garbage collector or runtime lock. Each reference carries one of six **reference capabilities** that constrain what operations are allowed:

| Capability | Aliased? | Mutable? | Sendable? | Description |
|:-----------|:---------|:---------|:----------|:------------|
| `iso` | No | Yes | Yes | Isolated, single owner. Exclusive mutable access. |
| `val` | Yes | No | Yes | Immutable, shareable. Read-only access from any actor. |
| `ref` | Yes | Yes | No | Mutable, local. Multiple aliases, all can mutate (single-thread only). |
| `box` | Yes | No | No | Heap-allocated, no sharing across actors. Internal read-only. |
| `tag` | Yes | No | Yes | Sendable, but carries no data. Token/capability for protocol signaling. |
| `trn` | No | Yes | Yes | Transferable ownership. Exclusive mutable, but can be consumed by transfer. |

The key insight: **data races are impossible** because the type system enforces:
- No two `iso` references to the same object (uniqueness)
- `val` references cannot be mutated
- `ref` references cannot cross actor boundaries
- `tag` carries no data, so sending it cannot leak state

---

## 2. Mapping Pony Capabilities to Fabric Concepts

Fabric's LocalityTier and TrustScope already encode spatial and trust constraints. Pony's capabilities map naturally onto Fabric's seat lease and surface model:

### 2.1 `iso` (Isolated) -> `LocalityTier::L0SameProcess` (Exclusive Seat)

**Pony:** Single owner, exclusive mutable access, sendable.
**Fabric:** An exclusive seat lease at L0 means the caller has sole mutable access to a resource in the same process. No other actor can hold a reference.

```
Pony iso:     var x: iso Foo = Foo.create     // only owner
Fabric L0:    SeatLease { tier: L0SameProcess, exclusive: true }
```

The invariant is the same: **exactly one writer**. In Pony this is enforced by the compiler rejecting duplicate `iso` aliases. In Fabric, the lease FSM (Pending -> Active) guarantees only one active lease per seat.

### 2.2 `val` (Value/Immutable) -> `SurfaceSpec` Shared State (Read-Only Frame Data)

**Pony:** Immutable, shareable across actors. No mutation possible.
**Fabric:** A SurfaceSpec's read-only frame data (e.g., a decoded framebuffer shared with multiple consumers). The data is allocated once, and all consumers see the same immutable snapshot.

```
Pony val:     val x: val Foo = Foo.create     // any actor can read
Fabric:       SharedFrameBuffer { ref_count: AtomicUsize, data: &[u8] }
              // consumers hold &SharedFrameBuffer, never &mut
```

The parallel: `val` guarantees no writer exists anywhere. Fabric enforces this by only exposing `&SurfaceFrame` (immutable reference) to consumers, never `&mut`.

### 2.3 `ref` (Reference/Mutable) -> `ActiveSurface` Mutable State

**Pony:** Multiple aliases, all mutable, single-thread only. Cannot cross actor boundaries.
**Fabric:** An ActiveSurface's mutable state (cursor position, input buffer, rendering context). The surface is bound to a single execution context and cannot be shared across topology nodes.

```
Pony ref:     var x: ref Foo = foo            // multiple mutable aliases, same thread
Fabric:       ActiveSurface { state: RefCell<RenderState> }
              // bound to one RouteStep, not Send across nodes
```

The invariant: `ref` data is **thread-local mutable**. Fabric enforces this by binding ActiveSurface to a RouteStep's execution context. The surface cannot cross LocalityTier boundaries without promotion to a different capability.

### 2.4 `box` (Box) -> Ephemeral Seat Lease

**Pony:** Heap-allocated, no cross-actor sharing. Internal read-only.
**Fabric:** A seat lease that exists only for the duration of a single operation (e.g., a one-shot frame encode). The lease is allocated, used, and dropped within one execution context.

```
Pony box:     box x: box Foo = Foo.create    // internal, no sharing
Fabric:       EphemeralSeat { ttl: Duration::from_millis(16) }
              // allocated for one frame, then reclaimed
```

The parallel: `box` data is invisible outside its actor. Ephemeral seats are invisible outside their RouteStep.

### 2.5 `tag` (Tag) -> Coordinator Message Passing

**Pony:** Sendable, but carries no data. A pure capability token.
**Fabric:** The coordinator's message-passing tokens (e.g., `SurfaceHandle`, `RoutePlanId`). These are IDs/tokens that can be sent across topology nodes but carry no payload.

```
Pony tag:     tag x: tag Foo                 // sendable, no data
Fabric:       SurfaceHandle(uuid)             // sendable, opaque, no data
              RoutePlanId(uuid)               // sendable, opaque identifier
```

The invariant: tags can be sent freely because they contain no mutable state. Fabric's handles are UUIDs that serve the same purpose.

### 2.6 `trn` (Trn/Transferable) -> Seat Lease Transfer/Revocation

**Pony:** Exclusive mutable, but ownership can be transferred (consumed) by another actor.
**Fabric:** A seat lease that can be transferred between RouteSteps (rebinding) or revoked by the coordinator. The exclusive access is maintained during transfer -- at no point do two holders have mutable access.

```
Pony trn:     var x: trn Foo = Foo.create    // exclusive, transferable
Fabric:       SeatLease::transfer(from_step, to_step)
              // FSM: Active -> Transferring -> Active (new step)
              // or: Active -> Revoked
```

The critical invariant: during transfer, the source loses access before the destination gains it. Fabric's lease FSM enforces this through state transitions.

---

## 3. Proposed Rust Encoding: PhantomData-Typed Markers

### 3.1 Design

We can encode Pony-like compile-time safety in Rust using **PhantomData markers** as type parameters. This gives us:
- Zero runtime cost (PhantomData is zero-sized)
- Compile-time enforcement of LocalityTier and TrustScope constraints
- No garbage collector overhead

```rust
use std::marker::PhantomData;

// --- Locality capability markers ---

/// Marker: data lives in the same process (L0-L2).
/// Corresponds to Pony's `ref` and `box` (thread-local).
pub struct SameProcess;

/// Marker: data lives on the same host but may cross process/NUMA boundaries (L3-L5).
/// Corresponds to Pony's `iso` (isolated, single owner).
pub struct SameHost;

/// Marker: data lives on a remote host (L6-L8).
/// Corresponds to Pony's `tag` (sendable, no data) or `val` (immutable, shared).
pub struct Remote;

// --- Trust scope markers ---

/// Marker: no trust verification (PF-FR-012: no default trust).
pub struct Untrusted;

/// Marker: bootstrap trust (local admin domain).
pub struct BootstrapTrust;

/// Marker: verified attestation (TPM, SEV-SNP).
pub struct AttestedTrust;

// --- Typed seat lease ---

/// A seat lease with compile-time locality and trust guarantees.
///
/// The locality marker ensures the lease can only be used within the
/// appropriate LocalityTier range. The trust marker ensures the lease
/// requires the appropriate TrustLevel.
pub struct TypedSeatLease<L, T> {
    seat_id: uuid::Uuid,
    _locality: PhantomData<L>,
    _trust: PhantomData<T>,
}

impl<L, T> TypedSeatLease<L, T> {
    /// The seat ID.
    pub fn seat_id(&self) -> uuid::Uuid {
        self.seat_id
    }
}

// Only L0-L2 leases can be used for in-process mutation (ref-like)
impl TypedSeatLease<SameProcess, AttestedTrust> {
    /// Get exclusive mutable access to the seat's data (ref-like).
    pub fn exclusive_mut(&mut self) -> &mut [u8] {
        // In a real implementation, this would access shared memory
        // at L0 or L2 locality tier.
        &mut []
    }
}

// Only L3-L5 leases can dispatch to GPU (iso-like)
impl TypedSeatLease<SameHost, AttestedTrust> {
    /// Dispatch a kernel to the GPU (iso-like, exclusive ownership).
    pub fn dispatch_gpu(&self, kernel_id: u64) -> Result<(), String> {
        // In a real implementation, this would dispatch to Mojo GPU.
        let _ = kernel_id;
        Ok(())
    }
}

// Remote leases can only send tokens (tag-like)
impl TypedSeatLease<Remote, BootstrapTrust> {
    /// Send a control token to the remote host (tag-like).
    pub fn send_token(&self, token: u64) -> Result<(), String> {
        // In a real implementation, this would serialize and ship the token.
        let _ = token;
        Ok(())
    }
}

// --- Typed frame buffer ---

/// A frame buffer with compile-time locality constraints.
pub struct TypedFrameBuffer<L> {
    data: Vec<u8>,
    width: u32,
    height: u32,
    _locality: PhantomData<L>,
}

impl TypedFrameBuffer<SameProcess> {
    /// Create an in-process frame buffer (ref-like: mutable, local).
    pub fn new_local(width: u32, height: u32) -> Self {
        Self {
            data: vec![0; (width as usize) * (height as usize) * 4],
            width,
            height,
            _locality: PhantomData,
        }
    }

    /// Mutate pixels in-place (ref-like: multiple mutable aliases, same thread).
    pub fn fill_solid(&mut self, r: u8, g: u8, b: u8) {
        for chunk in self.data.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = 255;
        }
    }
}

impl TypedFrameBuffer<Remote> {
    /// Create a remote frame buffer (val-like: immutable, shared).
    pub fn from_bytes(data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
            _locality: PhantomData,
        }
    }

    /// Get read-only access (val-like: immutable, shareable).
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

// --- Transfer marker (trn-like) ---

/// A transferable seat that can be moved between execution contexts.
/// The source context loses access when the transfer completes.
pub struct TransferableSeat<L, T> {
    inner: TypedSeatLease<L, T>,
}

impl<L, T> TransferableSeat<L, T> {
    /// Transfer ownership to a new context. Consumes self (trn semantics).
    pub fn transfer(self) -> TypedSeatLease<L, T> {
        // In a real implementation, this would:
        // 1. Serialize the seat state
        // 2. Ship to the new context
        // 3. Invalidate the old lease
        self.inner
    }
}
```

### 3.2 Compile-Time Safety Properties

The PhantomData approach provides these guarantees at compile time:

| Property | How enforced | Pony equivalent |
|:---------|:-------------|:----------------|
| In-process data cannot leave the process | `TypedFrameBuffer<SameProcess>` has no `Send` impl | `ref` cannot cross actor boundaries |
| Remote data is immutable | `TypedFrameBuffer<Remote>::as_slice()` returns `&[u8]` | `val` is immutable |
| Exclusive access during transfer | `transfer()` consumes `self` | `trn` transfer consumes the source |
| GPU dispatch requires attestation | `dispatch_gpu()` only on `TypedSeatLease<SameHost, AttestedTrust>` | `iso` requires single owner |
| Control tokens carry no data | `send_token()` takes a bare `u64`, not a buffer | `tag` carries no data |

### 3.3 Integration with Existing Fabric Types

```rust
use fabric_capability::LocalityTier;
use fabric_graph::TrustLevel;

// Map Fabric LocalityTier to our compile-time markers at the boundary:
fn locality_to_marker<L>(tier: LocalityTier) -> impl Into<TypedSeatLease<L, AttestedTrust>>
where
    L: LocalityMarker,
{
    // This conversion happens at the routing boundary.
    // Once converted, all subsequent operations are type-safe.
    todo!("route boundary conversion")
}

trait LocalityMarker {}
impl LocalityMarker for SameProcess {}
impl LocalityMarker for SameHost {}
impl LocalityMarker for Remote {}
```

---

## 4. Trade-Off Analysis

### 4.1 Benefits

| Benefit | Impact | Confidence |
|:--------|:-------|:-----------|
| **Data race prevention at compile time** | Eliminates an entire class of concurrency bugs | High -- proven in Pony |
| **Zero runtime cost** | PhantomData is zero-sized, erased at monomorphization | High -- Rust compiler guarantee |
| **Self-documenting APIs** | Type signatures encode locality/trust constraints | Medium -- improves on current stringly-typed tags |
| **Refactoring safety** | Changing a tier propagates compile errors to all callers | High -- standard Rust property |
| **No GC overhead** | Unlike Pony, we keep Rust's ownership model | High -- no runtime change needed |

### 4.2 Costs

| Cost | Impact | Confidence |
|:-----|:-------|:-----------|
| **API verbosity** | Every seat/frame type gains two extra type parameters | Medium -- mitigated by type aliases |
| **Boundary conversions** | Converting from runtime LocalityTier to compile-time markers adds code | Medium -- one-time cost at routing boundary |
| **Ecosystem friction** | Libraries expecting `Vec<u8>` won't accept `TypedFrameBuffer<...>` without conversion | High -- real ergonomic cost |
| **Incomplete coverage** | TrustScope is not currently a first-class type in Fabric (it's a runtime check) | Medium -- requires adding TrustScope as a type |
| **Dynamic tiers break the model** | If a route can change tiers at runtime (failover), the compile-time markers become stale | High -- Fabric's re-planning model conflicts with static tiers |
| **Testing overhead** | Each combination of locality + trust markers needs test coverage | Medium -- combinatorial explosion (3 localities x 3 trust levels = 9 impls) |

### 4.3 Key Risk: Dynamic Tier Re-Planning

Fabric's failover system (PF-WP-021) can re-plan routes when nodes fail. A route originally at L3PcieP2P might be re-planned to L6Lan. If the seat lease is typed as `TypedSeatLease<SameHost, _>`, the re-planning breaks the type-level guarantee.

**Mitigation:** The markers apply only to the *current* route step, not the lifetime of the connection. At each re-plan boundary, the type is re-validated. This means:
- Typed wrappers exist only within a single route step's execution
- Re-plan boundaries use runtime `LocalityTier` (the current model)
- The compile-time markers are a local optimization, not a global invariant

### 4.4 Decision Matrix

| Criterion | Weight | Without markers | With markers | Winner |
|:----------|:-------|:----------------|:-------------|:-------|
| Data race safety | High | Runtime checks (leases) | Compile-time | Markers |
| API ergonomics | High | Simple types | Verbose generics | Without |
| Runtime performance | Medium | No overhead | Zero-cost (PhantomData) | Tie |
| Dynamic re-planning | High | Works naturally | Breaks types | Without |
| Code maintainability | Medium | Straightforward | More complex | Without |
| New contributor onboarding | Low | Standard Rust | Requires understanding Pony model | Without |

### 4.5 Recommendation

**Do not implement the full PhantomData marker system at this time.**

The costs outweigh the benefits given Fabric's current architecture:

1. **Dynamic re-planning is incompatible** with static locality markers. Fabric's core value proposition is runtime re-planning, and locking types to locality tiers undermines this.

2. **The existing lease FSM already prevents data races.** The `Pending -> Active -> Completed/Revoked/Expired` state machine, combined with exclusive seat leases, provides the same safety guarantee at runtime without the API overhead.

3. **The ergonomics cost is real.** Every function that takes a seat lease or frame buffer would need generic parameters, making the API harder to use and document.

**However, consider a narrower application:**

- **`Send`/`!Send` markers for ActiveSurface**: Since ActiveSurface is bound to a RouteStep and should never cross thread boundaries, a `!Send` marker (using `PhantomData<*mut ()>`) would provide a cheap compile-time guarantee that surfaces don't accidentally get sent across threads. This is a single-marker, zero-ergonomics-cost improvement.

- **Sealed trait pattern for TrustLevel**: Instead of PhantomData markers, use a sealed trait pattern where `AttestedSeat` and `UntrustedSeat` are distinct types with different method sets. This gives the same safety benefit without generic parameters.

---

## 5. Proposed Narrow Implementation: Send Barrier

If any part of this analysis is worth implementing immediately, it is the `!Send` barrier for surfaces:

```rust
use std::marker::PhantomData;

/// A surface that must not be sent across threads.
/// The PhantomData<*mut ()> makes this !Send automatically.
pub struct ThreadLocalSurface {
    inner: SurfaceState,
    _not_send: PhantomData<*mut ()>,
}

impl ThreadLocalSurface {
    /// Access the surface's render state (must be called from the owning thread).
    pub fn render_state(&mut self) -> &mut RenderState {
        &mut self.inner.render_state
    }
}

// This will fail to compile if someone tries to send a surface across threads:
// let handle = tokio::spawn(async { surface.render_state(); }); // ERROR: !Send
```

This is a one-line PhantomData addition that prevents an entire class of bugs with zero API cost. It should be implemented as part of the ActiveSurface redesign.

---

## 6. References

- [Pony reference capabilities](https://www.ponylang.io/reference-capabilities/)
- [Pony tutorial: Reference capabilities](https://tutorial.ponylang.io/capabilities/reference-capabilities.html)
- [Fabric PF-FR-002: Locality tier invariants](../specs/)
- [Fabric PF-WP-015: Surface plane spec](../specs/)
- [Fabric PF-WP-021: Failover and re-planning](../specs/)
- [Rustonomicon: PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)
