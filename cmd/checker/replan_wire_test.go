package main

// replan_wire_test.go — tests for the R3 wire-based replan path.

import (
	"context"
	"encoding/json"
	"net"
	"testing"

	"github.com/phenotype/fabric/cmd/wire"
)

// --- daemon test server ---

// testDaemonHandler is a WireServer that simulates a fabric-daemon
// for testing the wire-based replan path.
type testDaemonHandler struct {
	// outcome to return in replan response.
	outcome string
	// reason to include in no_replacement response.
	reason string
}

func (tdh *testDaemonHandler) Handle(_ context.Context, env wire.WireEnvelope) (*wire.WireEnvelope, error) {
	if env.MsgType != wire.MsgTypeReplanRequest {
		return nil, nil
	}

	var rr wire.WireReplanRequest
	if err := wire.DecodePayload(env, wire.MsgTypeReplanRequest, &rr); err != nil {
		return nil, err
	}

	payload := wire.WireReplanResponse{
		RequestID: rr.RequestID,
		Outcome:   tdh.outcome,
		NewPlan:   json.RawMessage(`{"id":"new-001","bindings":[]}`),
		Reason:    tdh.reason,
	}

	resp, err := wire.NewEnvelopeFromPayload(wire.MsgTypeReplanResponse, env.TenantID, payload)
	if err != nil {
		return nil, err
	}
	return &resp, nil
}

// startTestDaemon starts a test TCP daemon and returns its address.
func startTestDaemon(t *testing.T, outcome string, reason string) string {
	t.Helper()
	handler := &testDaemonHandler{outcome: outcome, reason: reason}
	srv, err := wire.NewTCPWireServer("127.0.0.1:0", handler)
	if err != nil {
		t.Fatalf("NewTCPWireServer: %v", err)
	}
	go srv.Serve()
	t.Cleanup(func() { srv.Close() })

	addr := srv.Addr().(*net.TCPAddr)
	return net.JoinHostPort("127.0.0.1", itoa(addr.Port))
}

// --- wireReplan integration tests ---

func TestWireReplanReplacedReturnsAdmit(t *testing.T) {
	daemonAddr := startTestDaemon(t, wire.WireReplanOutcomeReplaced, "")
	topoPath := writeTestJSON(t, t.TempDir(), "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, t.TempDir(), "intent.json", `{"name":"test-app"}`)
	oldPlanPath := writeTestJSON(t, t.TempDir(), "old.json", `{"id":"00000000-0000-0000-0000-000000000000"}`)

	h := host(8, 8*1024*1024*1024)
	report, err := wireReplan(daemonAddr, "ops-test", topoPath, intentPath, oldPlanPath, nil, h)
	if err != nil {
		t.Fatalf("wireReplan error: %v", err)
	}
	if report.Decision != DecisionAdmit {
		t.Fatalf("decision = %s, want Admit (%+v)", report.Decision, report.Findings)
	}
	if len(report.Findings) != 1 || report.Findings[0].Code != ReasonReplanReplaced {
		t.Fatalf("unexpected findings: %+v", report.Findings)
	}
}

func TestWireReplanNoReplacementReturnsReject(t *testing.T) {
	daemonAddr := startTestDaemon(t, wire.WireReplanOutcomeNoReplacement, "AllCandidatesFailed")
	topoPath := writeTestJSON(t, t.TempDir(), "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, t.TempDir(), "intent.json", `{"name":"test-app"}`)
	oldPlanPath := writeTestJSON(t, t.TempDir(), "old.json", `{"id":"00000000-0000-0000-0000-000000000000"}`)

	h := host(8, 8*1024*1024*1024)
	report, err := wireReplan(daemonAddr, "ops-test", topoPath, intentPath, oldPlanPath, nil, h)
	if err != nil {
		t.Fatalf("wireReplan error: %v", err)
	}
	if report.Decision != DecisionReject {
		t.Fatalf("decision = %s, want Reject", report.Decision)
	}
	if report.Findings[0].Code != ReasonReplanNoReplacement {
		t.Fatalf("code = %s, want REPLAN_NO_REPLACEMENT", report.Findings[0].Code)
	}
}

func TestWireReplanWithBlacklist(t *testing.T) {
	daemonAddr := startTestDaemon(t, wire.WireReplanOutcomeReplaced, "")
	topoPath := writeTestJSON(t, t.TempDir(), "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, t.TempDir(), "intent.json", `{"name":"test-app"}`)
	oldPlanPath := writeTestJSON(t, t.TempDir(), "old.json", `{"id":"00000000-0000-0000-0000-000000000000"}`)

	blacklist := map[string]struct{}{"node-failed-1": {}, "node-failed-2": {}}
	h := host(8, 8*1024*1024*1024)
	report, err := wireReplan(daemonAddr, "ops-test", topoPath, intentPath, oldPlanPath, blacklist, h)
	if err != nil {
		t.Fatalf("wireReplan error: %v", err)
	}
	if report.Decision != DecisionAdmit {
		t.Fatalf("decision = %s, want Admit (%+v)", report.Decision, report.Findings)
	}
}

func TestWireReplanConnectionRefused(t *testing.T) {
	topoPath := writeTestJSON(t, t.TempDir(), "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, t.TempDir(), "intent.json", `{"name":"test-app"}`)
	oldPlanPath := writeTestJSON(t, t.TempDir(), "old.json", `{"id":"00000000-0000-0000-0000-000000000000"}`)

	h := host(8, 8*1024*1024*1024)
	_, err := wireReplan("127.0.0.1:1", "ops-test", topoPath, intentPath, oldPlanPath, nil, h)
	if err == nil {
		t.Fatal("expected error for connection refused")
	}
}

func TestWireReplanMissingFile(t *testing.T) {
	daemonAddr := startTestDaemon(t, wire.WireReplanOutcomeReplaced, "")

	h := host(8, 8*1024*1024*1024)
	_, err := wireReplan(daemonAddr, "ops-test", "/nonexistent/topo.json", "/nonexistent/intent.json", "/nonexistent/old.json", nil, h)
	if err == nil {
		t.Fatal("expected error for missing files")
	}
}

// --- wireReportFromResponse unit tests ---

func TestWireReportFromResponseReplaced(t *testing.T) {
	rr := &wire.WireReplanResponse{
		Outcome: wire.WireReplanOutcomeReplaced,
	}
	r := wireReportFromResponse("node-x", rr)
	if r.Decision != DecisionAdmit {
		t.Fatalf("decision = %s, want Admit", r.Decision)
	}
	if r.Findings[0].Code != ReasonReplanReplaced {
		t.Fatalf("code = %s, want REPLANNED", r.Findings[0].Code)
	}
}

func TestWireReportFromResponseNoReplacement(t *testing.T) {
	rr := &wire.WireReplanResponse{
		Outcome: wire.WireReplanOutcomeNoReplacement,
		Reason:  "AllCandidatesFailed",
	}
	r := wireReportFromResponse("node-x", rr)
	if r.Decision != DecisionReject {
		t.Fatalf("decision = %s, want Reject", r.Decision)
	}
	if r.Findings[0].Code != ReasonReplanNoReplacement {
		t.Fatalf("code = %s, want REPLAN_NO_REPLACEMENT", r.Findings[0].Code)
	}
}

func TestWireReportFromResponseError(t *testing.T) {
	rr := &wire.WireReplanResponse{
		Outcome: wire.WireReplanOutcomeError,
		Code:    "CompileFailed",
		Message: "no route found",
	}
	r := wireReportFromResponse("node-x", rr)
	if r.Decision != DecisionReject {
		t.Fatalf("decision = %s, want Reject", r.Decision)
	}
	if r.Findings[0].Code != ReasonReplanError {
		t.Fatalf("code = %s, want REPLAN_ERROR", r.Findings[0].Code)
	}
}

func TestWireReportFromResponseUnknown(t *testing.T) {
	rr := &wire.WireReplanResponse{
		Outcome: "something_weird",
	}
	r := wireReportFromResponse("node-x", rr)
	if r.Decision != DecisionReject {
		t.Fatalf("decision = %s, want Reject", r.Decision)
	}
	if r.Findings[0].Code != ReasonReplanUnknown {
		t.Fatalf("code = %s, want REPLAN_UNKNOWN_STATUS", r.Findings[0].Code)
	}
}

// --- parseAddr unit tests ---

func TestParseAddr(t *testing.T) {
	cases := []struct {
		in     string
		host   string
		port   int
		wantOk bool
	}{
		{"127.0.0.1:9090", "127.0.0.1", 9090, true},
		{"localhost:8080", "localhost", 8080, true},
		{"127.0.0.1:1", "127.0.0.1", 1, true},
		{"127.0.0.1:65535", "127.0.0.1", 65535, true},
		{"127.0.0.1", "", 0, false},
		{"127.0.0.1:0", "", 0, false},
		{"127.0.0.1:99999", "", 0, false},
		{"localhost:8080", "localhost", 8080, true},
	}
	for _, c := range cases {
		host, port, err := parseAddr(c.in)
		if (err == nil) != c.wantOk {
			t.Errorf("parseAddr(%q) ok=%v, want %v (err=%v)", c.in, err == nil, c.wantOk, err)
			continue
		}
		if err == nil && (host != c.host || port != c.port) {
			t.Errorf("parseAddr(%q) = (%q, %d), want (%q, %d)", c.in, host, port, c.host, c.port)
		}
	}
}

// --- E2E integration test ---

func TestE2E_WireReplanPipeline(t *testing.T) {
	// Full pipeline:
	// 1. Checker probes host → insufficient memory → Reject
	// 2. Wire replan to daemon → replaced → Admit
	dir := t.TempDir()

	descPath := writeTestDescriptor(t, dir, "node-wire-1", 8, 4*1024*1024*1024, false)
	manifestPath := writeTestManifest(t, dir, "wire-app", 4, "64Gi")

	host := mustLoadDescriptor(t, descPath)
	manifest := mustLoadManifest(t, manifestPath)

	// Step 1: Resource check → Reject
	report := check(host, manifest, nil)
	if report.Decision != DecisionReject {
		t.Fatalf("Step 1: expected Reject, got %s", report.Decision)
	}

	// Step 2: Start daemon and wire replan
	daemonAddr := startTestDaemon(t, wire.WireReplanOutcomeReplaced, "")
	topoPath := writeTestJSON(t, dir, "topo.json", `{"meta":{"name":"t","epoch":0},"nodes":[]}`)
	intentPath := writeTestJSON(t, dir, "intent.json", `{"name":"wire-app"}`)
	oldPlanPath := writeTestJSON(t, dir, "old.json", `{"id":"00000000-0000-0000-0000-000000000000"}`)

	blacklist := map[string]struct{}{"node-wire-1": {}}
	report, err := wireReplan(daemonAddr, "ops-test", topoPath, intentPath, oldPlanPath, blacklist, host)
	if err != nil {
		t.Fatalf("Step 2: wireReplan error: %v", err)
	}
	if report.Decision != DecisionAdmit {
		t.Fatalf("Step 2: expected Admit, got %s (%+v)", report.Decision, report.Findings)
	}
	if report.Findings[0].Code != ReasonReplanReplaced {
		t.Fatalf("Step 2: code = %s, want REPLANNED", report.Findings[0].Code)
	}
}
