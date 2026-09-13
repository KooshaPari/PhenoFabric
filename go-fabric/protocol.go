package fabric

import "encoding/json"

// ---------------------------------------------------------------------------
// Request message types
// ---------------------------------------------------------------------------

// HealthCheckRequest is the JSON message sent for a health check.
type HealthCheckRequest struct {
	Type string `json:"type"`
}

// NewHealthCheckRequest builds a health_check request message.
func NewHealthCheckRequest() HealthCheckRequest {
	return HealthCheckRequest{Type: "health_check"}
}

// TopologyRequest is the JSON message sent for a topology query.
type TopologyRequest struct {
	Type string `json:"type"`
}

// NewTopologyRequest builds a topology_request message.
func NewTopologyRequest() TopologyRequest {
	return TopologyRequest{Type: "topology_request"}
}

// ProbeRequest is the JSON message sent for a probe query.
type ProbeRequest struct {
	Type string `json:"type"`
}

// NewProbeRequest builds a probe_request message.
func NewProbeRequest() ProbeRequest {
	return ProbeRequest{Type: "probe_request"}
}

// RoutesRequest is the JSON message sent for a routes query.
type RoutesRequest struct {
	Type string `json:"type"`
}

// NewRoutesRequest builds a routes_request message.
func NewRoutesRequest() RoutesRequest {
	return RoutesRequest{Type: "routes_request"}
}

// CapabilitiesRequest is the JSON message sent for a capabilities query.
type CapabilitiesRequest struct {
	Type string `json:"type"`
}

// NewCapabilitiesRequest builds a capabilities_request message.
func NewCapabilitiesRequest() CapabilitiesRequest {
	return CapabilitiesRequest{Type: "capabilities_request"}
}

// HeartbeatRequest is the JSON message sent for a heartbeat.
type HeartbeatRequest struct {
	Type string `json:"type"`
}

// NewHeartbeatRequest builds a heartbeat message.
func NewHeartbeatRequest() HeartbeatRequest {
	return HeartbeatRequest{Type: "heartbeat"}
}

// ---------------------------------------------------------------------------
// Response message types
// ---------------------------------------------------------------------------

// HealthResponse is the daemon's health check response.
// Mirrors crates/fabric-daemon/src/health.rs::HealthResponse.
type HealthResponse struct {
	Status         string `json:"status"`
	UptimeS        uint64 `json:"uptime_s"`
	TopologyEpoch  uint64 `json:"topology_epoch"`
	ActiveLeases   int    `json:"active_leases"`
	ActivePlans    int    `json:"active_plans"`
}

// TopologyResponse is the daemon's topology snapshot response.
// Mirrors crates/fabric-gui/src/app.rs::TopologyResponse.
type TopologyResponse struct {
	Type          string      `json:"type"`
	Status        string      `json:"status"`
	TopologyEpoch uint64      `json:"topology_epoch"`
	TopologyName  string      `json:"topology_name"`
	NodeCount     int         `json:"node_count"`
	EdgeCount     int         `json:"edge_count"`
	Nodes         []TopoNode  `json:"nodes"`
	Edges         []TopoEdge  `json:"edges"`
	ActiveLeases  int         `json:"active_leases"`
	ActivePlans   int         `json:"active_plans"`
}

// TopoNode describes a single node in the topology.
type TopoNode struct {
	ID       string   `json:"id"`
	Label    string   `json:"label"`
	Locality string   `json:"locality"`
	CapCount int      `json:"cap_count"`
	Tags     []string `json:"tags"`
}

// TopoEdge describes a single edge in the topology.
type TopoEdge struct {
	ID       string `json:"id"`
	From     string `json:"from"`
	To       string `json:"to"`
	Locality string `json:"locality"`
}

// RoutesResponse is the daemon's routes snapshot response.
type RoutesResponse struct {
	Type   string     `json:"type"`
	Routes []RouteInfo `json:"routes"`
}

// RouteInfo describes a single active route plan.
type RouteInfo struct {
	IntentID          string `json:"intent_id"`
	Steps             int    `json:"steps"`
	TopologyEpoch     uint64 `json:"topology_epoch"`
	EstimatedLatencyUs int64 `json:"estimated_latency_us"`
	Tags              []string `json:"tags"`
}

// CapabilitiesResponse is the daemon's capabilities snapshot response.
type CapabilitiesResponse struct {
	Type         string           `json:"type"`
	Capabilities []CapabilityInfo `json:"capabilities"`
}

// CapabilityInfo describes a single capability descriptor across all nodes.
type CapabilityInfo struct {
	NodeName     string `json:"node_name"`
	DescriptorID string `json:"descriptor_id"`
	Trust        string `json:"trust"`
}

// HeartbeatResponse is the daemon's heartbeat acknowledgment.
type HeartbeatResponse struct {
	Type   string `json:"type"`
	Status string `json:"status"`
}

// GenericResponse is a catch-all for unrecognized or raw daemon responses.
type GenericResponse struct {
	Type   string          `json:"type"`
	Status string          `json:"status"`
	Raw    json.RawMessage `json:"-"`
}

// ---------------------------------------------------------------------------
// Wire envelope types (spec 025 — for spec-025-compliant nodes)
// ---------------------------------------------------------------------------

// WireEnvelope is the outermost wire shape per spec 025.
// Every byte crossing a wire boundary is a WireEnvelope.
type WireEnvelope struct {
	EnvelopeID   string          `json:"envelope_id"`
	TenantID     string          `json:"tenant_id"`
	MsgType      string          `json:"msg_type"`
	Payload      json.RawMessage `json:"payload"`
	Signature    string          `json:"signature,omitempty"`
	SentAtUnixMs int64           `json:"sent_at_unix_ms"`
}

// MsgType constants — frozen list per spec 025.
const (
	MsgTypeProbeRequest      = "probe.request"
	MsgTypeProbeResponse     = "probe.response"
	MsgTypeReplanRequest     = "replan.request"
	MsgTypeReplanResponse    = "replan.response"
	MsgTypeSurfaceInvalidate = "surface.invalidate"
	MsgTypeHeartbeat         = "heartbeat"
	MsgTypeError             = "error"
)

// WireProbeRequest is the probe.request payload.
type WireProbeRequest struct {
	RequestID string `json:"request_id"`
}

// WireProbeResponse is the probe.response payload.
type WireProbeResponse struct {
	RequestID  string          `json:"request_id"`
	Descriptor json.RawMessage `json:"descriptor"`
}

// WireReplanRequest is the replan.request payload.
type WireReplanRequest struct {
	RequestID   string          `json:"request_id"`
	Topology    json.RawMessage `json:"topology"`
	Intent      json.RawMessage `json:"intent"`
	OldPlan     json.RawMessage `json:"old_plan"`
	FailedNodes []string        `json:"failed_nodes"`
}

// WireReplanResponse is the replan.response payload.
type WireReplanResponse struct {
	RequestID string `json:"request_id"`
	Outcome   string `json:"outcome"`
	NewPlan   json.RawMessage `json:"new_plan,omitempty"`
	Reason    string `json:"reason,omitempty"`
	Code      string `json:"code,omitempty"`
	Message   string `json:"message,omitempty"`
}

// WireReplanOutcome constants.
const (
	WireReplanOutcomeReplaced      = "replaced"
	WireReplanOutcomeNoReplacement = "no_replacement"
	WireReplanOutcomeError         = "error"
)

// SurfaceInvalidate is a server-pushed surface lease invalidation.
type SurfaceInvalidate struct {
	SurfaceHandle string `json:"surface_handle"`
	LeaseID       string `json:"lease_id"`
	Reason        string `json:"reason"`
	FailedNode    string `json:"failed_node,omitempty"`
	Epoch         uint64 `json:"epoch"`
}

// WireHeartbeat is the heartbeat payload.
type WireHeartbeat struct {
	NodeID string `json:"node_id"`
	Epoch  uint64 `json:"epoch"`
}
