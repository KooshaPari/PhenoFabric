package fabric

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"strings"
	"sync"
	"testing"
	"time"
)

// ---------------------------------------------------------------------------
// Mock daemon server helpers
// ---------------------------------------------------------------------------

// mockDaemon is a minimal TCP server that responds to wire protocol messages.
type mockDaemon struct {
	listener net.Listener
	handler  func(line string) string
	wg       sync.WaitGroup
}

// startMockDaemon creates and starts a mock daemon on a random port.
// The handler function receives each request line and returns the response line.
func startMockDaemon(t *testing.T, handler func(string) string) *mockDaemon {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	md := &mockDaemon{
		listener: ln,
		handler:  handler,
	}
	md.wg.Add(1)
	go md.serve()
	return md
}

func (md *mockDaemon) serve() {
	defer md.wg.Done()
	for {
		conn, err := md.listener.Accept()
		if err != nil {
			return // listener closed
		}
		go md.handleConn(conn)
	}
}

func (md *mockDaemon) handleConn(conn net.Conn) {
	defer conn.Close()
	reader := bufio.NewReader(conn)
	for {
		line, err := reader.ReadBytes('\n')
		if err != nil {
			return
		}
		line = []byte(strings.TrimSpace(string(line)))
		if len(line) == 0 {
			continue
		}
		resp := md.handler(string(line))
		if resp != "" {
			_, _ = conn.Write(append([]byte(resp), '\n'))
		}
	}
}

func (md *mockDaemon) Addr() string {
	return md.listener.Addr().String()
}

func (md *mockDaemon) Close() {
	md.listener.Close()
	md.wg.Wait()
}

// ---------------------------------------------------------------------------
// Test responses
// ---------------------------------------------------------------------------

var (
	healthResp = `{"status":"healthy","uptime_s":3600,"topology_epoch":42,"active_leases":7,"active_plans":3}`
	topologyResp = `{"type":"probe_response","status":"ok","topology_epoch":42,"topology_name":"test-topo","node_count":2,"edge_count":1,"nodes":[{"id":"n1","label":"Node One","locality":"L5","cap_count":3,"tags":["gpu"]},{"id":"n2","label":"Node Two","locality":"L3","cap_count":1,"tags":[]}],"edges":[{"id":"e1","from":"n1","to":"n2","locality":"L2"}],"active_leases":7,"active_plans":3}`
	routesResp = `{"type":"routes_response","routes":[{"intent_id":"abc123","steps":3,"topology_epoch":42,"estimated_latency_us":1500,"tags":["fast"]}]}`
	capsResp = `{"type":"capabilities_response","capabilities":[{"node_name":"n1","descriptor_id":"cap-1","trust":"Trusted"}]}`
	heartbeatResp = `{"type":"heartbeat_ack","status":"ok"}`
	errorResp = `{"error":"server_busy","message":"max connections reached"}`
)

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

func TestHealthCheck(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return `{"error":"bad_request","message":"invalid JSON"}`
		}
		if msg["type"] == "health_check" {
			return healthResp
		}
		return `{"error":"unsupported","message":"unknown type"}`
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.HealthCheck(ctx)
	if err != nil {
		t.Fatalf("HealthCheck: %v", err)
	}
	if resp.Status != "healthy" {
		t.Errorf("Status = %q, want %q", resp.Status, "healthy")
	}
	if resp.UptimeS != 3600 {
		t.Errorf("UptimeS = %d, want 3600", resp.UptimeS)
	}
	if resp.TopologyEpoch != 42 {
		t.Errorf("TopologyEpoch = %d, want 42", resp.TopologyEpoch)
	}
	if resp.ActiveLeases != 7 {
		t.Errorf("ActiveLeases = %d, want 7", resp.ActiveLeases)
	}
	if resp.ActivePlans != 3 {
		t.Errorf("ActivePlans = %d, want 3", resp.ActivePlans)
	}
}

func TestTopology(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return ""
		}
		if msg["type"] == "topology_request" {
			return topologyResp
		}
		return ""
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.Topology(ctx)
	if err != nil {
		t.Fatalf("Topology: %v", err)
	}
	if resp.NodeCount != 2 {
		t.Errorf("NodeCount = %d, want 2", resp.NodeCount)
	}
	if resp.EdgeCount != 1 {
		t.Errorf("EdgeCount = %d, want 1", resp.EdgeCount)
	}
	if resp.TopologyEpoch != 42 {
		t.Errorf("TopologyEpoch = %d, want 42", resp.TopologyEpoch)
	}
	if resp.TopologyName != "test-topo" {
		t.Errorf("TopologyName = %q, want %q", resp.TopologyName, "test-topo")
	}
	if len(resp.Nodes) != 2 {
		t.Errorf("len(Nodes) = %d, want 2", len(resp.Nodes))
	}
	if resp.Nodes[0].ID != "n1" {
		t.Errorf("Nodes[0].ID = %q, want %q", resp.Nodes[0].ID, "n1")
	}
	if resp.Nodes[0].Label != "Node One" {
		t.Errorf("Nodes[0].Label = %q, want %q", resp.Nodes[0].Label, "Node One")
	}
	if len(resp.Edges) != 1 {
		t.Errorf("len(Edges) = %d, want 1", len(resp.Edges))
	}
	if resp.Edges[0].From != "n1" {
		t.Errorf("Edges[0].From = %q, want %q", resp.Edges[0].From, "n1")
	}
	if resp.Edges[0].To != "n2" {
		t.Errorf("Edges[0].To = %q, want %q", resp.Edges[0].To, "n2")
	}
}

func TestRoutes(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return ""
		}
		if msg["type"] == "routes_request" {
			return routesResp
		}
		return ""
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.Routes(ctx)
	if err != nil {
		t.Fatalf("Routes: %v", err)
	}
	if len(resp.Routes) != 1 {
		t.Fatalf("len(Routes) = %d, want 1", len(resp.Routes))
	}
	r := resp.Routes[0]
	if r.IntentID != "abc123" {
		t.Errorf("Routes[0].IntentID = %q, want %q", r.IntentID, "abc123")
	}
	if r.Steps != 3 {
		t.Errorf("Routes[0].Steps = %d, want 3", r.Steps)
	}
	if r.EstimatedLatencyUs != 1500 {
		t.Errorf("Routes[0].EstimatedLatencyUs = %d, want 1500", r.EstimatedLatencyUs)
	}
}

func TestCapabilities(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return ""
		}
		if msg["type"] == "capabilities_request" {
			return capsResp
		}
		return ""
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.Capabilities(ctx)
	if err != nil {
		t.Fatalf("Capabilities: %v", err)
	}
	if len(resp.Capabilities) != 1 {
		t.Fatalf("len(Capabilities) = %d, want 1", len(resp.Capabilities))
	}
	c := resp.Capabilities[0]
	if c.NodeName != "n1" {
		t.Errorf("Capabilities[0].NodeName = %q, want %q", c.NodeName, "n1")
	}
	if c.DescriptorID != "cap-1" {
		t.Errorf("Capabilities[0].DescriptorID = %q, want %q", c.DescriptorID, "cap-1")
	}
}

func TestProbe(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return ""
		}
		if msg["type"] == "probe_request" {
			return topologyResp
		}
		return ""
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	raw, err := client.Probe(ctx)
	if err != nil {
		t.Fatalf("Probe: %v", err)
	}
	if len(raw) == 0 {
		t.Fatal("Probe returned empty response")
	}
	// Verify it's valid JSON.
	var parsed map[string]interface{}
	if err := json.Unmarshal(raw, &parsed); err != nil {
		t.Fatalf("Probe response is not valid JSON: %v", err)
	}
	if parsed["type"] != "probe_response" {
		t.Errorf("Probe type = %q, want %q", parsed["type"], "probe_response")
	}
}

func TestHeartbeat(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return ""
		}
		if msg["type"] == "heartbeat" {
			return heartbeatResp
		}
		return ""
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.Heartbeat(ctx)
	if err != nil {
		t.Fatalf("Heartbeat: %v", err)
	}
	if resp.Status != "ok" {
		t.Errorf("Status = %q, want %q", resp.Status, "ok")
	}
}

func TestDaemonError(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		return errorResp
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.HealthCheck(ctx)
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	dErr, ok := err.(*DaemonError)
	if !ok {
		t.Fatalf("expected *DaemonError, got %T: %v", err, err)
	}
	if dErr.ErrorType != "server_busy" {
		t.Errorf("ErrorType = %q, want %q", dErr.ErrorType, "server_busy")
	}
}

func TestConnectionRetry(t *testing.T) {
	// Start daemon that closes first connection, then accepts second.
	var attempts int32
	daemon := startMockDaemon(t, func(line string) string {
		attempts++
		if attempts <= 1 {
			// Close the connection on first request to simulate connection drop.
			return "" // empty response triggers retry
		}
		return healthResp
	})
	defer daemon.Close()

	client := New(daemon.Addr(), WithTimeout(5*time.Second))
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// The first call may fail due to empty response, but subsequent calls
	// should work because the client retries with a new connection.
	var resp *HealthResponse
	var err error
	for i := 0; i < 3; i++ {
		resp, err = client.HealthCheck(ctx)
		if err == nil {
			break
		}
		time.Sleep(100 * time.Millisecond)
	}
	if err != nil {
		t.Fatalf("HealthCheck after retry: %v", err)
	}
	if resp.Status != "healthy" {
		t.Errorf("Status = %q, want %q", resp.Status, "healthy")
	}
}

func TestTimeout(t *testing.T) {
	// Start a daemon that never responds (hangs on read).
	daemon := startMockDaemon(t, func(line string) string {
		// Simulate slow response by sleeping.
		time.Sleep(5 * time.Second)
		return healthResp
	})
	defer daemon.Close()

	client := New(daemon.Addr(), WithTimeout(500*time.Millisecond))
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer cancel()

	_, err := client.HealthCheck(ctx)
	if err == nil {
		t.Fatal("expected timeout error, got nil")
	}
}

func TestClosedClient(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		return healthResp
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	_, err := client.HealthCheck(ctx)
	if err == nil {
		t.Fatal("expected error on closed client, got nil")
	}
}

func TestConnectionRefused(t *testing.T) {
	// Use a port that nothing is listening on.
	client := New("127.0.0.1:1")
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	_, err := client.HealthCheck(ctx)
	if err == nil {
		t.Fatal("expected connection error, got nil")
	}
}

func TestMultipleRequests(t *testing.T) {
	var requestCount int
	daemon := startMockDaemon(t, func(line string) string {
		requestCount++
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return ""
		}
		switch msg["type"] {
		case "health_check":
			return healthResp
		case "topology_request":
			return topologyResp
		case "routes_request":
			return routesResp
		default:
			return `{"error":"unknown","message":"unknown type"}`
		}
	})
	defer daemon.Close()

	client := New(daemon.Addr())
	defer client.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Send multiple different requests.
	health, err := client.HealthCheck(ctx)
	if err != nil {
		t.Fatalf("HealthCheck: %v", err)
	}
	topo, err := client.Topology(ctx)
	if err != nil {
		t.Fatalf("Topology: %v", err)
	}
	routes, err := client.Routes(ctx)
	if err != nil {
		t.Fatalf("Routes: %v", err)
	}

	// Verify all responses came back correctly.
	if health.Status != "healthy" {
		t.Errorf("health status = %q", health.Status)
	}
	if topo.NodeCount != 2 {
		t.Errorf("topo node_count = %d", topo.NodeCount)
	}
	if len(routes.Routes) != 1 {
		t.Errorf("routes count = %d", len(routes.Routes))
	}
	if requestCount != 3 {
		t.Errorf("request count = %d, want 3", requestCount)
	}
}

func TestConcurrentRequests(t *testing.T) {
	daemon := startMockDaemon(t, func(line string) string {
		var msg map[string]string
		if err := json.Unmarshal([]byte(line), &msg); err != nil {
			return ""
		}
		// Simulate some processing time.
		time.Sleep(10 * time.Millisecond)
		return healthResp
	})
	defer daemon.Close()

	client := New(daemon.Addr(), WithPoolConfig(ConnPoolConfig{
		MinIdle: 5,
		MaxIdle: 10,
		Timeout: 5 * time.Second,
	}))
	defer client.Close()

	const numGoroutines = 20
	var wg sync.WaitGroup
	errors := make(chan error, numGoroutines)

	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
			defer cancel()

			resp, err := client.HealthCheck(ctx)
			if err != nil {
				errors <- fmt.Errorf("goroutine: %w", err)
				return
			}
			if resp.Status != "healthy" {
				errors <- fmt.Errorf("status = %q", resp.Status)
			}
		}()
	}

	wg.Wait()
	close(errors)

	for err := range errors {
		t.Errorf("concurrent request: %v", err)
	}
}

func TestConnPool(t *testing.T) {
	pool := NewConnPool("127.0.0.1:1", ConnPoolConfig{
		MinIdle: 0,
		MaxIdle: 5,
		Timeout: 1 * time.Second,
	})
	defer pool.Close()

	// Pool starts with 0 connections (minIdle=0 and can't connect).
	if pool.Len() != 0 {
		t.Errorf("pool.Len() = %d, want 0", pool.Len())
	}
}

func TestProtocolTypes(t *testing.T) {
	// Test that protocol types serialize/deserialize correctly.
	health := NewHealthCheckRequest()
	data, err := json.Marshal(health)
	if err != nil {
		t.Fatalf("marshal HealthCheckRequest: %v", err)
	}
	if string(data) != `{"type":"health_check"}` {
		t.Errorf("HealthCheckRequest marshal = %q", string(data))
	}

	topo := NewTopologyRequest()
	data, err = json.Marshal(topo)
	if err != nil {
		t.Fatalf("marshal TopologyRequest: %v", err)
	}
	if string(data) != `{"type":"topology_request"}` {
		t.Errorf("TopologyRequest marshal = %q", string(data))
	}

	hb := NewHeartbeatRequest()
	data, err = json.Marshal(hb)
	if err != nil {
		t.Fatalf("marshal HeartbeatRequest: %v", err)
	}
	if string(data) != `{"type":"heartbeat"}` {
		t.Errorf("HeartbeatRequest marshal = %q", string(data))
	}

	// Test response deserialization.
	var hr HealthResponse
	if err := json.Unmarshal([]byte(healthResp), &hr); err != nil {
		t.Fatalf("unmarshal HealthResponse: %v", err)
	}
	if hr.Status != "healthy" {
		t.Errorf("HealthResponse.Status = %q", hr.Status)
	}

	var tr TopologyResponse
	if err := json.Unmarshal([]byte(topologyResp), &tr); err != nil {
		t.Fatalf("unmarshal TopologyResponse: %v", err)
	}
	if tr.NodeCount != 2 {
		t.Errorf("TopologyResponse.NodeCount = %d", tr.NodeCount)
	}
}

func TestWireEnvelopeTypes(t *testing.T) {
	env := WireEnvelope{
		EnvelopeID:   "aabbccdd11223344aabbccdd11223344",
		TenantID:     "ops-phenotype-default",
		MsgType:      MsgTypeProbeRequest,
		Payload:      json.RawMessage(`{"request_id":"test"}`),
		SentAtUnixMs: time.Now().UnixMilli(),
	}
	data, err := json.Marshal(env)
	if err != nil {
		t.Fatalf("marshal WireEnvelope: %v", err)
	}

	var decoded WireEnvelope
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("unmarshal WireEnvelope: %v", err)
	}
	if decoded.EnvelopeID != env.EnvelopeID {
		t.Errorf("EnvelopeID = %q", decoded.EnvelopeID)
	}
	if decoded.MsgType != MsgTypeProbeRequest {
		t.Errorf("MsgType = %q", decoded.MsgType)
	}
}

func TestErrorTypes(t *testing.T) {
	e := &Error{Code: "TestCode", Message: "test message"}
	if e.Error() != "fabric: TestCode: test message" {
		t.Errorf("Error.Error() = %q", e.Error())
	}

	de := &DaemonError{ErrorType: "server_busy", Message: "full"}
	if de.Error() != "fabric daemon error: server_busy: full" {
		t.Errorf("DaemonError.Error() = %q", de.Error())
	}

	// Test Is matching.
	e2 := &Error{Code: "TestCode", Message: "different"}
	if !e.Is(e2) {
		t.Error("Error.Is should match by code")
	}

	e3 := &Error{Code: "OtherCode", Message: "test message"}
	if e.Is(e3) {
		t.Error("Error.Is should not match different code")
	}
}

func TestParseDaemonError(t *testing.T) {
	raw := json.RawMessage(`{"error":"bad_envelope","message":"invalid JSON"}`)
	dErr, ok := ParseDaemonError(raw)
	if !ok {
		t.Fatal("DaemonError returned false for error response")
	}
	if dErr.ErrorType != "bad_envelope" {
		t.Errorf("ErrorType = %q", dErr.ErrorType)
	}
	if dErr.Message != "invalid JSON" {
		t.Errorf("Message = %q", dErr.Message)
	}

	// Non-error response.
	raw2 := json.RawMessage(`{"status":"ok"}`)
	_, ok = ParseDaemonError(raw2)
	if ok {
		t.Error("DaemonError returned true for non-error response")
	}
}
