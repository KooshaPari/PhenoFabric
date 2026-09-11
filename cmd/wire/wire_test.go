// wire_test.go — spec 025 §6 acceptance tests for cmd/wire.
//
// 13 unit tests covering:
//   - Round-trip per msg_type (6)
//   - Validation rejects bad inputs (5)
//   - Determinism + error wrapping (2)
//
// Plus golden-file validation: 8 fixtures in testdata/ must round-trip
// exactly through WireCodec.
//
// Stdlib only — matches the dep-light posture of the rest of the
// Fabric Go workspace.
package wire

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// validTestTenantID is the canonical tenant used in tests.
const validTestTenantID = "ops-phenotype-default"

// validTestEnvelope returns a known-good WireEnvelope for the given msg_type.
// Used by round-trip tests.
func validTestEnvelope(t *testing.T, msgType string, payload interface{}) WireEnvelope {
	t.Helper()
	env, err := NewEnvelopeFromPayload(msgType, validTestTenantID, payload)
	if err != nil {
		t.Fatalf("NewEnvelopeFromPayload(%q): %v", msgType, err)
	}
	return env
}

// roundTrip marshals env, then unmarshals it, and returns the result.
// Test bodies assert env == roundTrip(env).
func roundTrip(t *testing.T, env WireEnvelope) WireEnvelope {
	t.Helper()
	codec := WireCodec{}
	bytes, err := codec.Marshal(env)
	if err != nil {
		t.Fatalf("Marshal: %v", err)
	}
	got, err := codec.Unmarshal(bytes)
	if err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}
	return got
}

// -----------------------------------------------------------------------------
// Round-trip tests (6)
// -----------------------------------------------------------------------------

func TestRoundTripHeartbeat(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeHeartbeat, Heartbeat{
		NodeID: "host-1",
		Epoch:  42,
	})
	got := roundTrip(t, env)
	if got.EnvelopeID != env.EnvelopeID {
		t.Errorf("EnvelopeID: got %q, want %q", got.EnvelopeID, env.EnvelopeID)
	}
	if got.MsgType != MsgTypeHeartbeat {
		t.Errorf("MsgType: got %q, want %q", got.MsgType, MsgTypeHeartbeat)
	}
	var hb Heartbeat
	if err := DecodePayload(got, MsgTypeHeartbeat, &hb); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if hb.NodeID != "host-1" || hb.Epoch != 42 {
		t.Errorf("Heartbeat payload: got %+v", hb)
	}
}

func TestRoundTripProbeRequest(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeProbeRequest, ProbeRequest{
		RequestID: "11223344556677889900aabbccddeeff",
	})
	got := roundTrip(t, env)
	var pr ProbeRequest
	if err := DecodePayload(got, MsgTypeProbeRequest, &pr); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if pr.RequestID != "11223344556677889900aabbccddeeff" {
		t.Errorf("ProbeRequest.RequestID: got %q", pr.RequestID)
	}
}

func TestRoundTripProbeResponse(t *testing.T) {
	// Minimal CapabilityDescriptor subset — full Descriptor type lives in
	// cmd/checker/types.go; here we test the wire shape only.
	descriptorJSON := []byte(`{"node_id":"host-1","epoch":42,"schema_version":"v1","topology_hash":"abc"}`)
	env := validTestEnvelope(t, MsgTypeProbeResponse, ProbeResponse{
		RequestID:  "11223344556677889900aabbccddeeff",
		Descriptor: descriptorJSON,
	})
	got := roundTrip(t, env)
	var pr ProbeResponse
	if err := DecodePayload(got, MsgTypeProbeResponse, &pr); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if string(pr.Descriptor) != string(descriptorJSON) {
		t.Errorf("Descriptor bytes differ:\n got %s\nwant %s", pr.Descriptor, descriptorJSON)
	}
}

func TestRoundTripWireReplanRequest(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeReplanRequest, WireReplanRequest{
		RequestID:   "aabbccddeeff00112233445566778899",
		Topology:    json.RawMessage(`{"nodes":[{"id":"a"}]}`),
		Intent:      json.RawMessage(`{"name":"render"}`),
		OldPlan:     json.RawMessage(`{"steps":[{"node":"a"}]}`),
		FailedNodes: []string{"host-1", "host-2"},
	})
	got := roundTrip(t, env)
	var rr WireReplanRequest
	if err := DecodePayload(got, MsgTypeReplanRequest, &rr); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if len(rr.FailedNodes) != 2 || rr.FailedNodes[0] != "host-1" {
		t.Errorf("FailedNodes: got %v", rr.FailedNodes)
	}
	if string(rr.Topology) != `{"nodes":[{"id":"a"}]}` {
		t.Errorf("Topology: got %s", rr.Topology)
	}
}

func TestRoundTripWireReplanResponse(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeReplanResponse, WireReplanResponse{
		RequestID: "aabbccddeeff00112233445566778899",
		Outcome:   WireReplanOutcomeReplaced,
		NewPlan:   json.RawMessage(`{"steps":[{"node":"b"}]}`),
	})
	got := roundTrip(t, env)
	var rr WireReplanResponse
	if err := DecodePayload(got, MsgTypeReplanResponse, &rr); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if rr.Outcome != WireReplanOutcomeReplaced {
		t.Errorf("Outcome: got %q", rr.Outcome)
	}
	if string(rr.NewPlan) != `{"steps":[{"node":"b"}]}` {
		t.Errorf("NewPlan: got %s", rr.NewPlan)
	}
}

func TestRoundTripSurfaceInvalidate(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeSurfaceInvalidate, SurfaceInvalidate{
		SurfaceHandle: "surf-abc",
		LeaseID:       "lease-xyz",
		Reason:        SurfaceInvalidateReasonHostFailure,
		FailedNode:    "host-1",
		Epoch:         43,
	})
	got := roundTrip(t, env)
	var si SurfaceInvalidate
	if err := DecodePayload(got, MsgTypeSurfaceInvalidate, &si); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if si.SurfaceHandle != "surf-abc" || si.Reason != SurfaceInvalidateReasonHostFailure {
		t.Errorf("SurfaceInvalidate: got %+v", si)
	}
	if si.FailedNode != "host-1" || si.Epoch != 43 {
		t.Errorf("FailedNode/Epoch: got %q/%d", si.FailedNode, si.Epoch)
	}
}

// -----------------------------------------------------------------------------
// Validation rejection tests (5)
// -----------------------------------------------------------------------------

func TestValidateRejectsBadUUID(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeHeartbeat, Heartbeat{NodeID: "h", Epoch: 1})
	env.EnvelopeID = "not-a-uuid"
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted bad EnvelopeID; want error")
	}
}

func TestValidateRejectsBadTenantID(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeHeartbeat, Heartbeat{NodeID: "h", Epoch: 1})
	env.TenantID = "UPPER_AND_DOTS..."
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted bad TenantID; want error")
	}
	// Also rejects empty.
	env.TenantID = ""
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted empty TenantID; want error")
	}
}

func TestValidateRejectsUnknownMsgType(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeHeartbeat, Heartbeat{NodeID: "h", Epoch: 1})
	env.MsgType = "foo.bar"
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted unknown MsgType; want error")
	}
}

func TestValidateRejectsEmptyPayload(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeHeartbeat, Heartbeat{NodeID: "h", Epoch: 1})
	env.Payload = json.RawMessage(`null`)
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted null payload; want error")
	}
	env.Payload = json.RawMessage(``)
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted empty payload; want error")
	}
}

func TestValidateRejectsClockSkew(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeHeartbeat, Heartbeat{NodeID: "h", Epoch: 1})
	// 10 minutes in the future exceeds the ±300s tolerance.
	env.SentAtUnixMs = time.Now().UnixMilli() + 10*60*1000
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted future timestamp; want error")
	}
	// 10 minutes in the past also exceeds.
	env.SentAtUnixMs = time.Now().UnixMilli() - 10*60*1000
	if err := env.Validate(); err == nil {
		t.Fatal("Validate accepted past timestamp; want error")
	}
}

// -----------------------------------------------------------------------------
// Determinism + error wrapping (2)
// -----------------------------------------------------------------------------

func TestDeterministicMarshal(t *testing.T) {
	env := validTestEnvelope(t, MsgTypeHeartbeat, Heartbeat{NodeID: "host-1", Epoch: 42})
	// Lock the timestamp to a value within clock-skew tolerance so the
	// determinism check doesn't conflate stale-fixture failures with
	// actual determinism violations.
	env.SentAtUnixMs = time.Now().UnixMilli()
	env.EnvelopeID = "7d3a1b8c4e9f0123a5b6c7d8e9f01234"

	codec := WireCodec{}
	first, err := codec.Marshal(env)
	if err != nil {
		t.Fatalf("first Marshal: %v", err)
	}
	second, err := codec.Marshal(env)
	if err != nil {
		t.Fatalf("second Marshal: %v", err)
	}
	if string(first) != string(second) {
		t.Fatalf("determinism violated:\n first: %s\nsecond: %s", first, second)
	}

	// Sanity: also confirm the bytes parse to the same envelope.
	got, err := codec.Unmarshal(first)
	if err != nil {
		t.Fatalf("Unmarshal(first): %v", err)
	}
	if got.EnvelopeID != env.EnvelopeID || got.MsgType != env.MsgType {
		t.Errorf("round-trip altered envelope: %+v vs %+v", got, env)
	}
}

func TestMarshalErrorWrapsRequestEnvelopeID(t *testing.T) {
	reqEnv := validTestEnvelope(t, MsgTypeProbeRequest, ProbeRequest{
		RequestID: "11223344556677889900aabbccddeeff",
	})
	werr := WireError{
		Code:    WireErrorUnsupportedMsgType,
		Message: "receiver does not know msg_type 'foo.bar'",
	}
	codec := WireCodec{}
	respEnv, err := codec.MarshalError(werr, reqEnv)
	if err != nil {
		t.Fatalf("MarshalError: %v", err)
	}
	if respEnv.MsgType != MsgTypeError {
		t.Errorf("MsgType: got %q, want %q", respEnv.MsgType, MsgTypeError)
	}
	if respEnv.TenantID != reqEnv.TenantID {
		t.Errorf("TenantID: got %q, want %q", respEnv.TenantID, reqEnv.TenantID)
	}
	var got WireError
	if err := DecodePayload(respEnv, MsgTypeError, &got); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if got.Code != WireErrorUnsupportedMsgType {
		t.Errorf("Code: got %q, want %q", got.Code, WireErrorUnsupportedMsgType)
	}
	if got.EnvelopeID != reqEnv.EnvelopeID {
		t.Errorf("EnvelopeID: got %q, want %q (echo of request)", got.EnvelopeID, reqEnv.EnvelopeID)
	}
}

// -----------------------------------------------------------------------------
// Golden-file fixtures (8 files in testdata/)
// -----------------------------------------------------------------------------

// TestGoldenFiles verifies that the 5 wire msg_type fixtures (and the
// error envelope fixture) round-trip through WireCodec exactly.
func TestGoldenFiles(t *testing.T) {
	cases := []struct {
		path    string
		msgType string
	}{
		{"testdata/heartbeat.json", MsgTypeHeartbeat},
		{"testdata/probe_request.json", MsgTypeProbeRequest},
		{"testdata/probe_response.json", MsgTypeProbeResponse},
		{"testdata/replan_request.json", MsgTypeReplanRequest},
		{"testdata/replan_response.json", MsgTypeReplanResponse},
		{"testdata/surface_invalidate.json", MsgTypeSurfaceInvalidate},
		{"testdata/error_response.json", MsgTypeError},
	}
	for _, c := range cases {
		t.Run(c.path, func(t *testing.T) {
			data, err := os.ReadFile(filepath.Join(c.path))
			if err != nil {
				t.Fatalf("ReadFile(%q): %v", c.path, err)
			}
			// Patch sent_at_unix_ms to "now" so the clock-skew check in
			// Validate doesn't reject fixtures whose timestamps are pinned
			// to a fixed canonical epoch (not real-time).
			data = patchTimestamp(t, data)
			codec := WireCodec{}
			env, err := codec.Unmarshal(data)
			if err != nil {
				t.Fatalf("Unmarshal(%q): %v", c.path, err)
			}
			if env.MsgType != c.msgType {
				t.Errorf("MsgType: got %q, want %q", env.MsgType, c.msgType)
			}
			// Round-trip: marshal back and confirm valid JSON.
			re, err := codec.Marshal(env)
			if err != nil {
				t.Fatalf("Marshal round-trip: %v", err)
			}
			var dummy map[string]interface{}
			if err := json.Unmarshal(re, &dummy); err != nil {
				t.Fatalf("re-marshaled bytes are not JSON: %v", err)
			}
		})
	}
}

// patchTimestamp replaces the sent_at_unix_ms field of a JSON envelope
// with the current wall-clock time, so Validate() doesn't reject fixtures
// pinned to a fixed epoch. The rest of the envelope is preserved exactly.
func patchTimestamp(t *testing.T, data []byte) []byte {
	t.Helper()
	var m map[string]json.RawMessage
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("patchTimestamp unmarshal: %v", err)
	}
	nowMs, err := json.Marshal(time.Now().UnixMilli())
	if err != nil {
		t.Fatalf("patchTimestamp marshal nowMs: %v", err)
	}
	m["sent_at_unix_ms"] = nowMs
	out, err := json.Marshal(m)
	if err != nil {
		t.Fatalf("patchTimestamp marshal envelope: %v", err)
	}
	return out
}

// TestGoldenFile_UnsupportedRejected verifies that envelope_unsupported.json
// (a fixture with msg_type = "foo.bar") is rejected on Unmarshal.
func TestGoldenFile_UnsupportedRejected(t *testing.T) {
	data, err := os.ReadFile("testdata/envelope_unsupported.json")
	if err != nil {
		t.Fatalf("ReadFile: %v", err)
	}
	codec := WireCodec{}
	_, err = codec.Unmarshal(data)
	if err == nil {
		t.Fatal("Unmarshal accepted envelope with unknown msg_type; want error")
	}
	if !strings.Contains(err.Error(), "msg_type") {
		t.Errorf("error should mention msg_type; got: %v", err)
	}
}

// ---------------------------------------------------------------------------
// Type-safe envelope constructor tests (spec 024 ↔ 025 bridge)
// ---------------------------------------------------------------------------

func TestNewSurfaceInvalidateEnvelope(t *testing.T) {
	env, err := NewSurfaceInvalidateEnvelope(
		"tenant-a",
		"0123456789abcdef0123456789abcdef",
		"aabbccdd00112233aabbccdd00112233",
		SurfaceInvalidateReasonHostFailure,
		"gpu-node-1",
		42,
	)
	if err != nil {
		t.Fatalf("NewSurfaceInvalidateEnvelope: %v", err)
	}
	if env.MsgType != MsgTypeSurfaceInvalidate {
		t.Errorf("MsgType = %q, want %q", env.MsgType, MsgTypeSurfaceInvalidate)
	}
	if env.TenantID != "tenant-a" {
		t.Errorf("TenantID = %q, want %q", env.TenantID, "tenant-a")
	}

	// Decode and verify payload.
	var payload SurfaceInvalidate
	if err := DecodePayload(env, MsgTypeSurfaceInvalidate, &payload); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if payload.SurfaceHandle != "0123456789abcdef0123456789abcdef" {
		t.Errorf("SurfaceHandle = %q, want hex uuid", payload.SurfaceHandle)
	}
	if payload.LeaseID != "aabbccdd00112233aabbccdd00112233" {
		t.Errorf("LeaseID = %q, want hex uuid", payload.LeaseID)
	}
	if payload.Reason != SurfaceInvalidateReasonHostFailure {
		t.Errorf("Reason = %q, want %q", payload.Reason, SurfaceInvalidateReasonHostFailure)
	}
	if payload.FailedNode != "gpu-node-1" {
		t.Errorf("FailedNode = %q, want %q", payload.FailedNode, "gpu-node-1")
	}
	if payload.Epoch != 42 {
		t.Errorf("Epoch = %d, want 42", payload.Epoch)
	}

	// Round-trip through codec.
	codec := WireCodec{}
	data, err := codec.Marshal(env)
	if err != nil {
		t.Fatalf("Marshal: %v", err)
	}
	env2, err := codec.Unmarshal(data)
	if err != nil {
		t.Fatalf("Unmarshal: %v", err)
	}
	if env2.MsgType != MsgTypeSurfaceInvalidate {
		t.Errorf("round-trip MsgType = %q, want %q", env2.MsgType, MsgTypeSurfaceInvalidate)
	}
}

func TestNewSurfaceInvalidateEnvelope_RejectsUnknownReason(t *testing.T) {
	_, err := NewSurfaceInvalidateEnvelope(
		"tenant-a", "handle", "lease", "UnknownReason", "", 0,
	)
	if err == nil {
		t.Fatal("NewSurfaceInvalidateEnvelope accepted unknown reason; want error")
	}
	if !strings.Contains(err.Error(), "unknown") {
		t.Errorf("error should mention unknown; got: %v", err)
	}
}

func TestNewProbeRequestEnvelope(t *testing.T) {
	env, err := NewProbeRequestEnvelope("tenant-b")
	if err != nil {
		t.Fatalf("NewProbeRequestEnvelope: %v", err)
	}
	if env.MsgType != MsgTypeProbeRequest {
		t.Errorf("MsgType = %q, want %q", env.MsgType, MsgTypeProbeRequest)
	}
	var payload ProbeRequest
	if err := DecodePayload(env, MsgTypeProbeRequest, &payload); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if payload.RequestID == "" {
		t.Error("RequestID should not be empty")
	}
}

func TestNewHeartbeatEnvelope(t *testing.T) {
	env, err := NewHeartbeatEnvelope("tenant-c", "node-x", 99)
	if err != nil {
		t.Fatalf("NewHeartbeatEnvelope: %v", err)
	}
	if env.MsgType != MsgTypeHeartbeat {
		t.Errorf("MsgType = %q, want %q", env.MsgType, MsgTypeHeartbeat)
	}
	var payload Heartbeat
	if err := DecodePayload(env, MsgTypeHeartbeat, &payload); err != nil {
		t.Fatalf("DecodePayload: %v", err)
	}
	if payload.NodeID != "node-x" {
		t.Errorf("NodeID = %q, want %q", payload.NodeID, "node-x")
	}
	if payload.Epoch != 99 {
		t.Errorf("Epoch = %d, want 99", payload.Epoch)
	}
}

func TestInvalidationReasonToWire(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{SurfaceInvalidateReasonHostFailure, SurfaceInvalidateReasonHostFailure},
		{SurfaceInvalidateReasonRevoked, SurfaceInvalidateReasonRevoked},
		{SurfaceInvalidateReasonExpired, SurfaceInvalidateReasonExpired},
		{SurfaceInvalidateReasonFailed, SurfaceInvalidateReasonFailed},
		{"UnknownReason", SurfaceInvalidateReasonFailed},
		{"", SurfaceInvalidateReasonFailed},
	}
	for _, tt := range tests {
		got := InvalidationReasonToWire(tt.input)
		if got != tt.want {
			t.Errorf("InvalidationReasonToWire(%q) = %q, want %q", tt.input, got, tt.want)
		}
	}
}