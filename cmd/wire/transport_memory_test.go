package wire

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"testing"
)

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

// echoServer is a trivial WireServer that echoes the envelope back
// with MsgType changed to "heartbeat" (to make it a valid response).
type echoServer struct {
	// handleErr, if non-nil, is returned by Handle.
	handleErr error
	// lastEnv captures the last envelope received by Handle.
	lastEnv *WireEnvelope
}

func (es *echoServer) Handle(_ context.Context, env WireEnvelope) (*WireEnvelope, error) {
	es.lastEnv = &env
	if es.handleErr != nil {
		return nil, es.handleErr
	}
	// Echo back with the same envelope_id and payload but type unchanged.
	return &env, nil
}

// noopPushServer accepts push envelopes and returns nil (no response).
type noopPushServer struct{}

func (nps *noopPushServer) Handle(_ context.Context, env WireEnvelope) (*WireEnvelope, error) {
	return nil, nil
}

// ---------------------------------------------------------------------------
// MemoryTransport tests
// ---------------------------------------------------------------------------

func TestMemoryTransportSendReceiveRoundTrip(t *testing.T) {
	mt := NewMemoryTransport()
	server := &echoServer{}
	addr := NodeAddress{Scheme: "memory", Host: "node-a"}
	mt.Register(addr, server)

	// Build a valid envelope.
	env, err := NewHeartbeatEnvelope("ops-test", "node-a", 42)
	if err != nil {
		t.Fatalf("NewHeartbeatEnvelope: %v", err)
	}

	resp, err := mt.Send(context.Background(), addr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	if resp == nil {
		t.Fatal("Send returned nil response")
	}
	if resp.EnvelopeID != env.EnvelopeID {
		t.Errorf("response EnvelopeID = %q, want %q", resp.EnvelopeID, env.EnvelopeID)
	}

	// Verify server received the envelope.
	if server.lastEnv == nil {
		t.Fatal("server did not receive envelope")
	}
	if server.lastEnv.MsgType != MsgTypeHeartbeat {
		t.Errorf("server received MsgType = %q, want %q", server.lastEnv.MsgType, MsgTypeHeartbeat)
	}
}

func TestMemoryTransportUnregisteredAddressReturnsError(t *testing.T) {
	mt := NewMemoryTransport()
	addr := NodeAddress{Scheme: "memory", Host: "unknown-node"}

	env, _ := NewHeartbeatEnvelope("ops-test", "node-a", 1)
	_, err := mt.Send(context.Background(), addr, env)
	if err == nil {
		t.Fatal("expected error for unregistered address")
	}
}

func TestMemoryTransportPushReturnsNil(t *testing.T) {
	mt := NewMemoryTransport()
	server := &noopPushServer{}
	addr := NodeAddress{Scheme: "memory", Host: "node-b"}
	mt.Register(addr, server)

	env, _ := NewHeartbeatEnvelope("ops-test", "node-b", 1)
	resp, err := mt.Send(context.Background(), addr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	if resp != nil {
		t.Errorf("expected nil response for push, got %+v", resp)
	}
}

func TestMemoryTransportServerErrorPropagates(t *testing.T) {
	mt := NewMemoryTransport()
	server := &echoServer{handleErr: errors.New("server exploded")}
	addr := NodeAddress{Scheme: "memory", Host: "node-c"}
	mt.Register(addr, server)

	env, _ := NewHeartbeatEnvelope("ops-test", "node-c", 1)
	_, err := mt.Send(context.Background(), addr, env)
	if err == nil {
		t.Fatal("expected error from failing server")
	}
}

func TestMemoryTransportUnregister(t *testing.T) {
	mt := NewMemoryTransport()
	server := &echoServer{}
	addr := NodeAddress{Scheme: "memory", Host: "node-d"}
	mt.Register(addr, server)

	// Send should work.
	env, _ := NewHeartbeatEnvelope("ops-test", "node-d", 1)
	_, err := mt.Send(context.Background(), addr, env)
	if err != nil {
		t.Fatalf("Send before unregister: %v", err)
	}

	// Unregister.
	mt.Unregister(addr)

	// Send should fail now.
	_, err = mt.Send(context.Background(), addr, env)
	if err == nil {
		t.Fatal("expected error after unregister")
	}
}

func TestMemoryTransportRegisterOverwrite(t *testing.T) {
	mt := NewMemoryTransport()
	addr := NodeAddress{Scheme: "memory", Host: "node-e"}

	server1 := &echoServer{}
	server2 := &echoServer{}
	mt.Register(addr, server1)
	mt.Register(addr, server2) // overwrite

	env, _ := NewHeartbeatEnvelope("ops-test", "node-e", 1)
	_, err := mt.Send(context.Background(), addr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}

	// server1 should NOT have received the envelope.
	if server1.lastEnv != nil {
		t.Error("server1 received envelope after overwrite")
	}
	// server2 SHOULD have received it.
	if server2.lastEnv == nil {
		t.Error("server2 did not receive envelope")
	}
}

func TestLoopbackClientSendsToSameTarget(t *testing.T) {
	mt := NewMemoryTransport()
	server := &echoServer{}
	addr := NodeAddress{Scheme: "memory", Host: "self"}
	mt.Register(addr, server)

	lc := NewLoopbackClient(mt, addr)
	env, _ := NewHeartbeatEnvelope("ops-test", "self", 7)

	// LoopbackClient ignores the target argument and uses its configured target.
	otherAddr := NodeAddress{Scheme: "memory", Host: "other"}
	resp, err := lc.Send(context.Background(), otherAddr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	if resp == nil {
		t.Fatal("expected response")
	}
	if server.lastEnv == nil {
		t.Fatal("server did not receive envelope")
	}
}

func TestMemoryTransportMultipleServers(t *testing.T) {
	mt := NewMemoryTransport()

	serverA := &echoServer{}
	serverB := &echoServer{}
	addrA := NodeAddress{Scheme: "memory", Host: "node-a"}
	addrB := NodeAddress{Scheme: "memory", Host: "node-b"}
	mt.Register(addrA, serverA)
	mt.Register(addrB, serverB)

	envA, _ := NewHeartbeatEnvelope("ops-test", "node-a", 1)
	envB, _ := NewHeartbeatEnvelope("ops-test", "node-b", 2)

	// Send to A.
	_, err := mt.Send(context.Background(), addrA, envA)
	if err != nil {
		t.Fatalf("Send to A: %v", err)
	}

	// Send to B.
	_, err = mt.Send(context.Background(), addrB, envB)
	if err != nil {
		t.Fatalf("Send to B: %v", err)
	}

	// Verify each server received its own envelope.
	if serverA.lastEnv == nil || serverA.lastEnv.MsgType != MsgTypeHeartbeat {
		t.Error("serverA did not receive its envelope")
	}
	if serverB.lastEnv == nil || serverB.lastEnv.MsgType != MsgTypeHeartbeat {
		t.Error("serverB did not receive its envelope")
	}
}

// ---------------------------------------------------------------------------
// Multi-hop pipeline test (simulates checker → daemon → checker flow)
// ---------------------------------------------------------------------------

func TestMemoryTransportMultiHopPipeline(t *testing.T) {
	// Scenario: A checker sends a replan.request to a daemon, which
	// processes it and returns a replan.response.
	mt := NewMemoryTransport()
	daemonAddr := NodeAddress{Scheme: "memory", Host: "daemon-1"}

	// Daemon server: receives replan.request, returns replan.response.
	daemon := &replanDaemonServer{}
	mt.Register(daemonAddr, daemon)

	// Build a replan request envelope.
	topoJSON := json.RawMessage(`{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentJSON := json.RawMessage(`{"name":"render","min_trust":"untrusted"}`)
	oldPlanJSON := json.RawMessage(`{"id":"00000000-0000-0000-0000-000000000000","bindings":[]}`)

	env, err := NewEnvelopeFromPayload(MsgTypeReplanRequest, "ops-test", WireReplanRequest{
		RequestID:   newUUIDv4(),
		Topology:    topoJSON,
		Intent:      intentJSON,
		OldPlan:     oldPlanJSON,
		FailedNodes: []string{"node-1"},
	})
	if err != nil {
		t.Fatalf("NewEnvelopeFromPayload: %v", err)
	}

	resp, err := mt.Send(context.Background(), daemonAddr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	if resp == nil {
		t.Fatal("expected response envelope")
	}
	if resp.MsgType != MsgTypeReplanResponse {
		t.Errorf("response MsgType = %q, want %q", resp.MsgType, MsgTypeReplanResponse)
	}

	// Decode the response payload.
	var rr WireReplanResponse
	if err := DecodePayload(*resp, MsgTypeReplanResponse, &rr); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if rr.Outcome != WireReplanOutcomeReplaced {
		t.Errorf("Outcome = %q, want %q", rr.Outcome, WireReplanOutcomeReplaced)
	}
}

// replanDaemonServer is a test WireServer that handles replan.request.
type replanDaemonServer struct{}

func (rds *replanDaemonServer) Handle(_ context.Context, env WireEnvelope) (*WireEnvelope, error) {
	if env.MsgType != MsgTypeReplanRequest {
		return nil, fmt.Errorf("unexpected msg_type: %s", env.MsgType)
	}

	// Parse request to verify it's valid.
	var rr WireReplanRequest
	if err := DecodePayload(env, MsgTypeReplanRequest, &rr); err != nil {
		return nil, err
	}

	// Build response.
	payload := WireReplanResponse{
		RequestID: rr.RequestID,
		Outcome:   WireReplanOutcomeReplaced,
		NewPlan:   json.RawMessage(`{"id":"new-001","bindings":[]}`),
	}

	resp, err := NewEnvelopeFromPayload(MsgTypeReplanResponse, env.TenantID, payload)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}
