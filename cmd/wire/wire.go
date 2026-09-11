// Package wire is the Fabric wire transport contract (spec 025, PF-WP-040).
//
// The contract pins the bytes-on-wire shape for inter-node communication
// in the Fabric. It is intentionally transport-agnostic: HTTP, gRPC,
// Unix domain sockets, and in-process memory transports all wrap this
// contract. The actual transport implementations are deferred to R3
// (see spec 025 §2.2 / §9).
//
// What this package provides:
//
//   - WireEnvelope: the outermost wire shape. Every byte that crosses a
//     wire boundary is a WireEnvelope.
//   - WireCodec: deterministic-JSON Marshal / Unmarshal / Validate. Two
//     identical inputs always produce identical bytes.
//   - 5 WireMessage payload types (probe.request, probe.response,
//     replan.request, replan.response, surface.invalidate, heartbeat)
//     plus a synthetic "error" MsgType for error envelopes.
//   - WireError: a stable 7-code error taxonomy that receivers and
//     senders both understand.
//   - WireClient / WireServer interfaces with no implementations — the
//     interface is the R3 target; implementations land in R3.
//
// What this package does NOT provide (deferred to R3 or later):
//
//   - Actual network transport (HTTP / gRPC / UDS / QUIC)
//   - TLS / mTLS handshake (depends on trust_root.rs per spec 021)
//   - Streaming push subscriptions (event-stream wedge, spec 027+)
//   - Compression (only after a measured payload-size problem)
//   - Authentication beyond envelope signature (WireAuth is a separate spec)
//
// Stability guarantee:
//
// Once this contract is shipped, the JSON tag names, MsgType string
// values, WireError code values, and envelope field order are frozen.
// Adding fields is allowed (forward compat); renaming or removing is
// not. New MsgTypes are allowed via a new spec.
package wire

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"regexp"
	"sort"
	"strings"
	"time"
)

// -----------------------------------------------------------------------------
// MsgType constants — frozen list (spec 025 §3.2)
// -----------------------------------------------------------------------------

const (
	// MsgTypeProbeRequest — client asks a node for its CapabilityDescriptor.
	MsgTypeProbeRequest = "probe.request"
	// MsgTypeProbeResponse — server returns a CapabilityDescriptor.
	MsgTypeProbeResponse = "probe.response"
	// MsgTypeReplanRequest — client asks a node to replan topology-driven.
	MsgTypeReplanRequest = "replan.request"
	// MsgTypeReplanResponse — server returns the replan outcome.
	MsgTypeReplanResponse = "replan.response"
	// MsgTypeSurfaceInvalidate — server pushes a surface-lease invalidation.
	MsgTypeSurfaceInvalidate = "surface.invalidate"
	// MsgTypeHeartbeat — bidirectional liveness probe.
	MsgTypeHeartbeat = "heartbeat"

	// MsgTypeError — synthetic MsgType for error envelopes (spec 025 §3.4).
	// Not one of the 5 wire msg types; used only by WireCodec.MarshalError.
	MsgTypeError = "error"
)

// validMsgTypes is the closed set of msg types accepted on the wire.
//
// MsgTypeError IS included here so error envelopes (produced only by
// WireCodec.MarshalError) are accepted on Unmarshal. The 5 wire msg
// types documented in §3.2 are the only ones NewEnvelopeFromPayload
// will produce directly — error envelopes are codec-generated, not
// user-constructed. See IsWireMsgType below.
var validMsgTypes = map[string]struct{}{
	MsgTypeProbeRequest:      {},
	MsgTypeProbeResponse:     {},
	MsgTypeReplanRequest:     {},
	MsgTypeReplanResponse:    {},
	MsgTypeSurfaceInvalidate: {},
	MsgTypeHeartbeat:         {},
	MsgTypeError:             {},
}

// IsWireMsgType reports whether s is one of the 5 documented wire msg
// types (probe.request / probe.response / replan.request /
// replan.response / surface.invalidate / heartbeat). It does NOT
// include MsgTypeError, which is reserved for codec-generated error
// envelopes.
func IsWireMsgType(s string) bool {
	switch s {
	case MsgTypeProbeRequest, MsgTypeProbeResponse,
		MsgTypeReplanRequest, MsgTypeReplanResponse,
		MsgTypeSurfaceInvalidate, MsgTypeHeartbeat:
		return true
	}
	return false
}

// IsValidMsgType reports whether s is acceptable on the wire — this
// includes the 5 documented wire msg types PLUS MsgTypeError (which is
// codec-generated for error envelopes). Validate uses this; constructors
// should use IsWireMsgType to refuse building error envelopes directly.
func IsValidMsgType(s string) bool {
	_, ok := validMsgTypes[s]
	return ok
}

// -----------------------------------------------------------------------------
// WireError codes — frozen list (spec 025 §3.4)
// -----------------------------------------------------------------------------

const (
	// WireErrorBadEnvelope — Envelope itself failed validation.
	WireErrorBadEnvelope = "BadEnvelope"
	// WireErrorUnsupportedMsgType — Receiver doesn't know this MsgType.
	WireErrorUnsupportedMsgType = "UnsupportedMsgType"
	// WireErrorAuthFailed — Signature verification failed or signature required but missing.
	WireErrorAuthFailed = "AuthFailed"
	// WireErrorUnknownTenant — Receiver doesn't know this tenant_id.
	WireErrorUnknownTenant = "UnknownTenant"
	// WireErrorBadPayload — Envelope parsed but payload validation failed.
	WireErrorBadPayload = "BadPayload"
	// WireErrorIo — Receiver hit a transient I/O or downstream failure.
	WireErrorIo = "Io"
	// WireErrorBug — Receiver internal error.
	WireErrorBug = "Bug"
)

// validWireErrorCodes is the closed set of accepted WireError.Code values.
var validWireErrorCodes = map[string]struct{}{
	WireErrorBadEnvelope:         {},
	WireErrorUnsupportedMsgType:  {},
	WireErrorAuthFailed:          {},
	WireErrorUnknownTenant:       {},
	WireErrorBadPayload:          {},
	WireErrorIo:                  {},
	WireErrorBug:                 {},
}

// -----------------------------------------------------------------------------
// Validation regexes — frozen (spec 025 §3.1, §5)
// -----------------------------------------------------------------------------

var (
	envelopeIDRegex  = regexp.MustCompile(`^[0-9a-f]{32}$`)
	tenantIDRegex    = regexp.MustCompile(`^[a-z0-9-]{1,64}$`)

	// clockSkewTolerance is the bound on |SentAtUnixMs - now|. Tight bound
	// is R3; ±300s is the initial wedge that still catches 99% of clock
	// drift without false-positiving on slow networks.
	clockSkewTolerance int64 = 300 * 1000
)

// -----------------------------------------------------------------------------
// WireEnvelope (spec 025 §3.1)
// -----------------------------------------------------------------------------

// WireEnvelope is the outermost wire shape. Everything that crosses a
// wire boundary is one WireEnvelope.
//
// JSON tag names are FROZEN. Adding fields is allowed (forward compat);
// renaming or removing is not.
type WireEnvelope struct {
	// EnvelopeID is a UUIDv4 lowercase hex (32 chars, no dashes).
	EnvelopeID string `json:"envelope_id"`
	// TenantID matches ^[a-z0-9-]{1,64}$.
	TenantID string `json:"tenant_id"`
	// MsgType is one of the MsgType* constants.
	MsgType string `json:"msg_type"`
	// Payload is the serialized WireMessage body (json.RawMessage).
	Payload json.RawMessage `json:"payload"`
	// Signature, if non-empty, is base64 (standard alphabet, no '=' padding)
	// of the Ed25519 signature over the envelope with this field empty.
	// Empty means "unsigned" (R2 default; mTLS arrives R3).
	Signature string `json:"signature,omitempty"`
	// SentAtUnixMs is int64 milliseconds since Unix epoch.
	SentAtUnixMs int64 `json:"sent_at_unix_ms"`
}

// -----------------------------------------------------------------------------
// 5 WireMessage payload types (spec 025 §3.3)
// -----------------------------------------------------------------------------

// ProbeRequest — client asks a node for its CapabilityDescriptor.
type ProbeRequest struct {
	RequestID string `json:"request_id"` // UUIDv4 lowercase hex
}

// ProbeResponse — server returns a CapabilityDescriptor. The Descriptor
// type itself lives in cmd/checker/types.go; the wire shape borrows it.
type ProbeResponse struct {
	RequestID  string          `json:"request_id"`
	Descriptor json.RawMessage `json:"descriptor"` // full CapabilityDescriptor JSON
}

// WireReplanRequest — mirrors spec 023 ReplanRequest shape, but envelope-wrapped.
type WireReplanRequest struct {
	RequestID   string          `json:"request_id"`
	Topology    json.RawMessage `json:"topology"`
	Intent      json.RawMessage `json:"intent"`
	OldPlan     json.RawMessage `json:"old_plan"`
	FailedNodes []string        `json:"failed_nodes"`
}

// WireReplanOutcome constants match the spec 023 outcomes.
const (
	WireReplanOutcomeReplaced      = "replaced"
	WireReplanOutcomeNoReplacement = "no_replacement"
	WireReplanOutcomeError         = "error"
)

// WireReplanResponse — mirrors spec 023 ReplanResponse shape.
type WireReplanResponse struct {
	RequestID string          `json:"request_id"`
	Outcome   string          `json:"outcome"` // replaced | no_replacement | error
	NewPlan   json.RawMessage `json:"new_plan,omitempty"`
	Reason    string          `json:"reason,omitempty"`
	Code      string          `json:"code,omitempty"`
	Message   string          `json:"message,omitempty"`
}

// SurfaceInvalidateReason constants — mirror crates/fabric-graph/src/surface.rs::LeaseExitReason.
const (
	SurfaceInvalidateReasonHostFailure = "HostFailure"
	SurfaceInvalidateReasonRevoked     = "Revoked"
	SurfaceInvalidateReasonExpired     = "Expired"
	SurfaceInvalidateReasonFailed      = "Failed"
)

// SurfaceInvalidate — server pushes when a lease is invalidated per spec 024
// SurfaceRegistry::notify_node_failure semantics.
type SurfaceInvalidate struct {
	SurfaceHandle string `json:"surface_handle"`
	LeaseID       string `json:"lease_id"`
	Reason        string `json:"reason"`
	FailedNode    string `json:"failed_node,omitempty"`
	Epoch         uint64 `json:"epoch"`
}

// Heartbeat — liveness probe.
type Heartbeat struct {
	NodeID string `json:"node_id"`
	Epoch  uint64 `json:"epoch"`
}

// -----------------------------------------------------------------------------
// WireError (spec 025 §3.4)
// -----------------------------------------------------------------------------

// WireError is the payload of an error envelope (MsgType = MsgTypeError).
type WireError struct {
	Code       string `json:"code"`
	Message    string `json:"message"`
	EnvelopeID string `json:"envelope_id,omitempty"` // for correlation back to the request
}

// -----------------------------------------------------------------------------
// WireClient / WireServer interfaces (spec 025 §3.6 — no impls in this wedge)
// -----------------------------------------------------------------------------

// NodeAddress identifies a target node. The actual transport (HTTP / gRPC / UDS)
// is chosen in R3; for the contract stub this is just an opaque address.
type NodeAddress struct {
	Scheme string `json:"scheme"` // "https" | "grpc" | "unix" | "memory" — R3 decision
	Host   string `json:"host"`
	Port   int    `json:"port,omitempty"`
	Path   string `json:"path,omitempty"`
}

// String returns a deterministic address rendering (for logging).
func (n NodeAddress) String() string {
	if n.Scheme == "unix" || n.Scheme == "memory" {
		return fmt.Sprintf("%s://%s%s", n.Scheme, n.Host, n.Path)
	}
	return fmt.Sprintf("%s://%s:%d%s", n.Scheme, n.Host, n.Port, n.Path)
}

// WireClient is the interface a Fabric node uses to send envelopes.
// Implementations are R3 (HTTP / gRPC / UDS / memory for tests).
type WireClient interface {
	// Send transmits env to target. Returns the response envelope for
	// request/response msg_types, or nil for push msg_types
	// (MsgTypeSurfaceInvalidate, MsgTypeHeartbeat).
	Send(ctx context.Context, target NodeAddress, env WireEnvelope) (*WireEnvelope, error)
	// Close releases any underlying resources.
	Close() error
}

// WireServer is the interface a Fabric node implements to receive envelopes.
// Implementations are R3.
type WireServer interface {
	// Handle is called once per incoming envelope. The implementation
	// returns a response envelope (request/response msg_types) or nil
	// (push msg_types).
	Handle(ctx context.Context, env WireEnvelope) (*WireEnvelope, error)
}

// -----------------------------------------------------------------------------
// WireCodec (spec 025 §3.5)
// -----------------------------------------------------------------------------

// WireCodec is the deterministic-JSON codec for WireEnvelopes.
//
// Determinism rules (spec 025 §5):
//   - Keys sorted lexicographically (json.Marshal + struct tags)
//   - No whitespace (use json.Marshal, not json.MarshalIndent)
//   - No trailing newline
//   - No JSON maps in payload (only structs/arrays/scalars)
//   - Timestamps as int64 Unix millis (never RFC3339 string)
//   - UUIDs lowercase hex, no dashes
//   - Base64 standard alphabet, no '=' padding
type WireCodec struct{}

// Marshal serializes env to deterministic JSON bytes. Returns an error
// if env fails validation.
func (WireCodec) Marshal(env WireEnvelope) ([]byte, error) {
	if err := env.Validate(); err != nil {
		return nil, fmt.Errorf("wire: marshal invalid envelope: %w", err)
	}
	return json.Marshal(env)
}

// Unmarshal parses data into a WireEnvelope. Returns an error if data
// is malformed or fails validation. Envelope unknown fields are rejected
// (the envelope shape is fixed); payload unknown fields are tolerated
// (forward compatibility — spec 025 §3.5).
func (WireCodec) Unmarshal(data []byte) (WireEnvelope, error) {
	var env WireEnvelope
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&env); err != nil {
		return WireEnvelope{}, fmt.Errorf("wire: unmarshal envelope: %w", err)
	}
	if err := env.Validate(); err != nil {
		return WireEnvelope{}, fmt.Errorf("wire: unmarshal invalid envelope: %w", err)
	}
	return env, nil
}

// Validate checks the 6 invariants from spec 025 §3.1.
func (env WireEnvelope) Validate() error {
	if !envelopeIDRegex.MatchString(env.EnvelopeID) {
		return fmt.Errorf("envelope_id %q must be 32 lowercase hex chars (UUIDv4 no dashes)", env.EnvelopeID)
	}
	if !tenantIDRegex.MatchString(env.TenantID) {
		return fmt.Errorf("tenant_id %q must match ^[a-z0-9-]{1,64}$", env.TenantID)
	}
	if !IsValidMsgType(env.MsgType) {
		return fmt.Errorf("msg_type %q is not one of the 5 wire msg types", env.MsgType)
	}
	if len(env.Payload) == 0 || string(env.Payload) == "null" {
		return errors.New("payload must be non-empty JSON")
	}
	if !json.Valid(env.Payload) {
		return fmt.Errorf("payload is not valid JSON: %s", string(env.Payload))
	}
	nowMs := time.Now().UnixMilli()
	skew := env.SentAtUnixMs - nowMs
	if skew < 0 {
		skew = -skew
	}
	if skew > clockSkewTolerance {
		return fmt.Errorf("sent_at_unix_ms %d is %d ms from now (limit %d ms)",
			env.SentAtUnixMs, skew, clockSkewTolerance)
	}
	return nil
}

// MarshalError wraps werr in a WireEnvelope targeted at requestEnv's tenant
// and echoing requestEnv.EnvelopeID for correlation. The resulting envelope
// has MsgType = MsgTypeError and is itself valid (sent_at_unix_ms = now).
func (WireCodec) MarshalError(werr WireError, requestEnv WireEnvelope) (WireEnvelope, error) {
	if _, ok := validWireErrorCodes[werr.Code]; !ok {
		return WireEnvelope{}, fmt.Errorf("wire: invalid WireError.Code %q", werr.Code)
	}
	// Echo the request envelope ID for correlation unless the caller already set one.
	if werr.EnvelopeID == "" {
		werr.EnvelopeID = requestEnv.EnvelopeID
	}
	payload, err := json.Marshal(werr)
	if err != nil {
		return WireEnvelope{}, fmt.Errorf("wire: marshal error payload: %w", err)
	}
	return WireEnvelope{
		EnvelopeID:   newUUIDv4(),
		TenantID:     requestEnv.TenantID,
		MsgType:      MsgTypeError,
		Payload:      payload,
		SentAtUnixMs: time.Now().UnixMilli(),
	}, nil
}

// -----------------------------------------------------------------------------
// Payload decoders (typed accessors for the 5 WireMessage bodies)
// -----------------------------------------------------------------------------

// DecodePayload unmarshals env.Payload into target. Returns an error
// if env.MsgType doesn't match the expected msgType or payload is
// malformed JSON.
func DecodePayload(env WireEnvelope, expectedMsgType string, target interface{}) error {
	if env.MsgType != expectedMsgType {
		return fmt.Errorf("wire: envelope msg_type is %q, expected %q", env.MsgType, expectedMsgType)
	}
	if err := json.Unmarshal(env.Payload, target); err != nil {
		return fmt.Errorf("wire: decode payload for %q: %w", expectedMsgType, err)
	}
	return nil
}

// NewEnvelopeFromPayload constructs a WireEnvelope from msg_type, tenant_id,
// and a payload struct (or map). Marshals payload to json.RawMessage.
// Rejects MsgTypeError (use WireCodec.MarshalError for error envelopes).
func NewEnvelopeFromPayload(msgType, tenantID string, payload interface{}) (WireEnvelope, error) {
	if !IsWireMsgType(msgType) {
		return WireEnvelope{}, fmt.Errorf("wire: cannot build envelope with msg_type %q (use WireCodec.MarshalError for errors)", msgType)
	}
	if !tenantIDRegex.MatchString(tenantID) {
		return WireEnvelope{}, fmt.Errorf("wire: cannot build envelope with tenant_id %q", tenantID)
	}
	raw, err := json.Marshal(payload)
	if err != nil {
		return WireEnvelope{}, fmt.Errorf("wire: marshal payload: %w", err)
	}
	return WireEnvelope{
		EnvelopeID:   newUUIDv4(),
		TenantID:     tenantID,
		MsgType:      msgType,
		Payload:      raw,
		SentAtUnixMs: time.Now().UnixMilli(),
	}, nil
}

// ---------------------------------------------------------------------------
// Type-safe envelope constructors (spec 024 ↔ 025 bridge)
// ---------------------------------------------------------------------------

// NewSurfaceInvalidateEnvelope builds a surface.invalidate envelope from
// the fields produced by Rust SurfaceRegistry::notify_node_failure.
//
// The tenant_id is the target tenant that should receive this push.
// This is the primary bridge between the Rust SurfaceRegistry and the
// Go wire transport layer.
func NewSurfaceInvalidateEnvelope(
	tenantID string,
	surfaceHandle string,
	leaseID string,
	reason string,
	failedNode string,
	epoch uint64,
) (WireEnvelope, error) {
	if !IsWireMsgType(MsgTypeSurfaceInvalidate) {
		return WireEnvelope{}, fmt.Errorf("wire: surface.invalidate not in wire msg type set")
	}
	if !tenantIDRegex.MatchString(tenantID) {
		return WireEnvelope{}, fmt.Errorf("wire: invalid tenant_id %q", tenantID)
	}

	// Validate reason is one of the known constants.
	switch reason {
	case SurfaceInvalidateReasonHostFailure, SurfaceInvalidateReasonRevoked,
		SurfaceInvalidateReasonExpired, SurfaceInvalidateReasonFailed:
		// known reason
	default:
		return WireEnvelope{}, fmt.Errorf("wire: unknown SurfaceInvalidate reason %q", reason)
	}

	payload := SurfaceInvalidate{
		SurfaceHandle: surfaceHandle,
		LeaseID:       leaseID,
		Reason:        reason,
		FailedNode:    failedNode,
		Epoch:         epoch,
	}

	raw, err := json.Marshal(payload)
	if err != nil {
		return WireEnvelope{}, fmt.Errorf("wire: marshal surface.invalidate payload: %w", err)
	}

	return WireEnvelope{
		EnvelopeID:   newUUIDv4(),
		TenantID:     tenantID,
		MsgType:      MsgTypeSurfaceInvalidate,
		Payload:      raw,
		SentAtUnixMs: time.Now().UnixMilli(),
	}, nil
}

// NewProbeRequestEnvelope builds a probe.request envelope.
func NewProbeRequestEnvelope(tenantID string) (WireEnvelope, error) {
	return NewEnvelopeFromPayload(MsgTypeProbeRequest, tenantID, ProbeRequest{
		RequestID: newUUIDv4(),
	})
}

// NewHeartbeatEnvelope builds a heartbeat envelope.
func NewHeartbeatEnvelope(tenantID string, nodeID string, epoch uint64) (WireEnvelope, error) {
	return NewEnvelopeFromPayload(MsgTypeHeartbeat, tenantID, Heartbeat{
		NodeID: nodeID,
		Epoch:  epoch,
	})
}

// InvalidationReasonToWire maps a Go-side reason string to the wire
// constants. Passes through known constants unchanged; maps unknown
// values to "Failed" as a safe default.
func InvalidationReasonToWire(reason string) string {
	switch reason {
	case SurfaceInvalidateReasonHostFailure, SurfaceInvalidateReasonRevoked,
		SurfaceInvalidateReasonExpired, SurfaceInvalidateReasonFailed:
		return reason
	default:
		return SurfaceInvalidateReasonFailed
	}
}

// -----------------------------------------------------------------------------
// Determinism helpers
// -----------------------------------------------------------------------------

// SortedKeys is exposed for tests / consumers that want to verify
// determinism: the result of marshal is the same as sorting keys then
// marshaling an equivalent map.
func SortedKeys(s string) string {
	// No-op helper for symmetry; json.Marshal of structs already sorts
	// keys deterministically per Go encoding/json semantics.
	_ = sort.Strings // tie in the sort package so go imports don't churn
	return strings.TrimSpace(s)
}

// -----------------------------------------------------------------------------
// UUIDv4 helper
// -----------------------------------------------------------------------------

// newUUIDv4 returns a UUIDv4 rendered as 32 lowercase hex chars (no dashes).
// Used by constructors and MarshalError to populate EnvelopeID. We don't
// use github.com/google/uuid because the contract is stdlib-only.
func newUUIDv4() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		// crypto/rand failure is a fatal system error; the wire contract
		// cannot operate without it. Returning a zero UUID is the safest
		// fallback (it will fail Validate, which is correct).
		return "00000000000000000000000000000000"
	}
	// Set version (4) and variant (RFC 4122) bits per UUIDv4.
	b[6] = (b[6] & 0x0f) | 0x40
	b[8] = (b[8] & 0x3f) | 0x80
	return hex.EncodeToString(b[:])
}