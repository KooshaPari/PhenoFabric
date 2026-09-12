// Package wire — in-memory transport for WireClient/WireServer.
//
// MemoryTransport is a fast, dependency-free transport for unit and
// integration tests. It routes envelopes between WireServer instances
// by NodeAddress, enabling full pipeline tests without network I/O.
//
// MemoryTransport is NOT safe for concurrent use across goroutines —
// callers must serialise access or wrap with a mutex. This matches the
// intended test usage (single goroutine driving the transport).
package wire

import (
	"context"
	"fmt"
	"sync"
)

// MemoryTransport routes WireEnvelopes between in-memory WireServer
// instances keyed by NodeAddress.
type MemoryTransport struct {
	mu      sync.Mutex
	servers map[string]WireServer
}

// NewMemoryTransport creates an empty MemoryTransport.
func NewMemoryTransport() *MemoryTransport {
	return &MemoryTransport{
		servers: make(map[string]WireServer),
	}
}

// Register binds a WireServer to the given NodeAddress. If an address
// is already registered, the old server is replaced (idempotent).
func (mt *MemoryTransport) Register(addr NodeAddress, server WireServer) {
	mt.mu.Lock()
	defer mt.mu.Unlock()
	mt.servers[addr.String()] = server
}

// Unregister removes a WireServer for the given NodeAddress.
func (mt *MemoryTransport) Unregister(addr NodeAddress) {
	mt.mu.Lock()
	defer mt.mu.Unlock()
	delete(mt.servers, addr.String())
}

// Send implements WireClient. It looks up the target address in the
// registry and calls server.Handle synchronously. Returns the response
// envelope (for request/response msg_types) or nil (for push types).
//
// Returns an error if the target address is not registered or the
// server returns an error.
func (mt *MemoryTransport) Send(ctx context.Context, target NodeAddress, env WireEnvelope) (*WireEnvelope, error) {
	mt.mu.Lock()
	key := target.String()
	server, ok := mt.servers[key]
	mt.mu.Unlock()

	if !ok {
		return nil, fmt.Errorf("wire: no server registered at %s", target)
	}

	resp, err := server.Handle(ctx, env)
	if err != nil {
		return nil, fmt.Errorf("wire: server handle: %w", err)
	}
	return resp, nil
}

// Close is a no-op for MemoryTransport (no resources to release).
func (mt *MemoryTransport) Close() error {
	return nil
}

// Compile-time check: MemoryTransport implements WireClient.
var _ WireClient = (*MemoryTransport)(nil)

// ---------------------------------------------------------------------------
// Convenience: loopback client (same node sends to itself)
// ---------------------------------------------------------------------------

// LoopbackClient wraps a MemoryTransport and always sends to the same
// target address. Useful for single-node test scenarios.
type LoopbackClient struct {
	transport *MemoryTransport
	target    NodeAddress
}

// NewLoopbackClient creates a client that routes all Send calls to
// the given target address via the provided MemoryTransport.
func NewLoopbackClient(transport *MemoryTransport, target NodeAddress) *LoopbackClient {
	return &LoopbackClient{
		transport: transport,
		target:    target,
	}
}

// Send sends env to the loopback target.
func (lc *LoopbackClient) Send(ctx context.Context, target NodeAddress, env WireEnvelope) (*WireEnvelope, error) {
	return lc.transport.Send(ctx, lc.target, env)
}

// Close is a no-op.
func (lc *LoopbackClient) Close() error {
	return nil
}

// Compile-time check: LoopbackClient implements WireClient.
var _ WireClient = (*LoopbackClient)(nil)
