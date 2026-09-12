package wire

import (
	"context"
	"encoding/json"
	"net"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// TCPWireServer + TCPTransport integration tests
// ---------------------------------------------------------------------------

// echoWireServer is a WireServer that echoes the envelope back.
type echoWireServer struct{}

func (ews *echoWireServer) Handle(_ context.Context, env WireEnvelope) (*WireEnvelope, error) {
	return &env, nil
}

// replanWireServer handles replan.request and returns a replaced response.
type replanWireServer struct{}

func (rws *replanWireServer) Handle(_ context.Context, env WireEnvelope) (*WireEnvelope, error) {
	if env.MsgType != MsgTypeReplanRequest {
		return nil, nil
	}
	var rr WireReplanRequest
	if err := DecodePayload(env, MsgTypeReplanRequest, &rr); err != nil {
		return nil, err
	}
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

func startTestServer(t *testing.T, handler WireServer) *TCPWireServer {
	t.Helper()
	// Bind to :0 to get a random port.
	srv, err := NewTCPWireServer("127.0.0.1:0", handler)
	if err != nil {
		t.Fatalf("NewTCPWireServer: %v", err)
	}
	go srv.Serve()
	t.Cleanup(func() { srv.Close() })
	return srv
}

func serverAddr(t *testing.T, srv *TCPWireServer) NodeAddress {
	t.Helper()
	addr := srv.Addr().(*net.TCPAddr)
	return NodeAddress{
		Scheme: "tcp",
		Host:   "127.0.0.1",
		Port:   addr.Port,
	}
}

func TestTCPTransportEchoRoundTrip(t *testing.T) {
	srv := startTestServer(t, &echoWireServer{})
	addr := serverAddr(t, srv)

	client := NewTCPTransport()
	defer client.Close()

	// Use probe.request (request/response type, not push type).
	env, _ := NewProbeRequestEnvelope("ops-test")

	// Allow server a moment to start accepting.
	time.Sleep(50 * time.Millisecond)

	resp, err := client.Send(context.Background(), addr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	if resp == nil {
		t.Fatal("expected response")
	}
	if resp.EnvelopeID != env.EnvelopeID {
		t.Errorf("response EnvelopeID = %q, want %q", resp.EnvelopeID, env.EnvelopeID)
	}
}

func TestTCPTransportReplanRoundTrip(t *testing.T) {
	srv := startTestServer(t, &replanWireServer{})
	addr := serverAddr(t, srv)

	client := NewTCPTransport()
	defer client.Close()

	env, _ := NewEnvelopeFromPayload(MsgTypeReplanRequest, "ops-test", WireReplanRequest{
		RequestID: newUUIDv4(),
		Topology:  json.RawMessage(`{"nodes":[]}`),
		Intent:    json.RawMessage(`{"name":"test"}`),
		OldPlan:   json.RawMessage(`{"id":"00000000-0000-0000-0000-000000000000"}`),
	})

	time.Sleep(50 * time.Millisecond)

	resp, err := client.Send(context.Background(), addr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	if resp == nil {
		t.Fatal("expected response")
	}
	if resp.MsgType != MsgTypeReplanResponse {
		t.Errorf("MsgType = %q, want %q", resp.MsgType, MsgTypeReplanResponse)
	}

	var rr WireReplanResponse
	if err := DecodePayload(*resp, MsgTypeReplanResponse, &rr); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if rr.Outcome != WireReplanOutcomeReplaced {
		t.Errorf("Outcome = %q, want %q", rr.Outcome, WireReplanOutcomeReplaced)
	}
}

func TestTCPTransportConnectionReuse(t *testing.T) {
	srv := startTestServer(t, &echoWireServer{})
	addr := serverAddr(t, srv)

	client := NewTCPTransport()
	defer client.Close()

	time.Sleep(50 * time.Millisecond)

	// Send twice — second should reuse connection.
	for i := 0; i < 3; i++ {
		env, _ := NewProbeRequestEnvelope("ops-test")
		resp, err := client.Send(context.Background(), addr, env)
		if err != nil {
			t.Fatalf("Send %d: %v", i, err)
		}
		if resp == nil {
			t.Fatalf("Send %d: nil response", i)
		}
	}
}

func TestTCPTransportConnectionRefused(t *testing.T) {
	client := NewTCPTransport()
	defer client.Close()

	addr := NodeAddress{Scheme: "tcp", Host: "127.0.0.1", Port: 1}
	env, _ := NewHeartbeatEnvelope("ops-test", "node-a", 1)

	_, err := client.Send(context.Background(), addr, env)
	if err == nil {
		t.Fatal("expected error for connection refused")
	}
}

func TestTCPWireServerCloseStopsServe(t *testing.T) {
	srv, err := NewTCPWireServer("127.0.0.1:0", &echoWireServer{})
	if err != nil {
		t.Fatalf("NewTCPWireServer: %v", err)
	}

	done := make(chan struct{})
	go func() {
		srv.Serve()
		close(done)
	}()

	// Close should cause Serve to return.
	time.Sleep(50 * time.Millisecond)
	srv.Close()

	select {
	case <-done:
		// Serve returned.
	case <-time.After(2 * time.Second):
		t.Fatal("Serve did not return after Close")
	}
}

// TestTCPTransportPushNoResponse verifies that push msg_types
// (heartbeat, surface.invalidate) don't wait for a response.
func TestTCPTransportPushNoResponse(t *testing.T) {
	srv := startTestServer(t, &echoWireServer{})
	addr := serverAddr(t, srv)

	client := NewTCPTransport()
	defer client.Close()

	time.Sleep(50 * time.Millisecond)

	env, _ := NewHeartbeatEnvelope("ops-test", "node-a", 1)
	resp, err := client.Send(context.Background(), addr, env)
	if err != nil {
		t.Fatalf("Send: %v", err)
	}
	// Heartbeat is a push type; response should be nil.
	if resp != nil {
		t.Errorf("expected nil response for heartbeat push, got %+v", resp)
	}
}
