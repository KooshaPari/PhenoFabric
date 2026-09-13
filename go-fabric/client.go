package fabric

import (
	"context"
	"encoding/json"
	"fmt"
	"time"
)

// Client is the main entry point for communicating with the Fabric daemon
// over the wire protocol.
//
// Zero value is not usable; use New to create instances.
type Client struct {
	addr    string
	timeout time.Duration
	pool    *ConnPool
	closed  bool
}

// Option configures the client.
type Option func(*Client)

// WithTimeout sets the default request timeout. If a context has a shorter
// deadline, the context deadline wins.
func WithTimeout(d time.Duration) Option {
	return func(c *Client) {
		c.timeout = d
	}
}

// WithPoolConfig sets custom connection pool parameters.
func WithPoolConfig(cfg ConnPoolConfig) Option {
	return func(c *Client) {
		if c.pool != nil {
			c.pool.Close()
		}
		c.pool = NewConnPool(c.addr, cfg)
	}
}

// New creates a new fabric client connected to addr (host:port).
func New(addr string, opts ...Option) *Client {
	c := &Client{
		addr:    addr,
		timeout: 10 * time.Second,
	}
	for _, opt := range opts {
		opt(c)
	}
	if c.pool == nil {
		c.pool = NewConnPool(addr, ConnPoolConfig{
			MinIdle: 1,
			MaxIdle: 10,
			Timeout: c.timeout,
		})
	}
	return c
}

// Close closes all connections managed by the client.
func (c *Client) Close() error {
	c.closed = true
	return c.pool.Close()
}

// HealthCheck queries the daemon's health endpoint.
//
// The daemon responds with status, uptime, topology epoch, and counts
// of active leases and plans.
func (c *Client) HealthCheck(ctx context.Context) (*HealthResponse, error) {
	raw, err := c.send(ctx, "health_check")
	if err != nil {
		return nil, err
	}
	var resp HealthResponse
	if err := json.Unmarshal(raw, &resp); err != nil {
		return nil, fmt.Errorf("fabric: unmarshal health response: %w", err)
	}
	return &resp, nil
}

// Topology queries the daemon's topology snapshot.
//
// The response includes all nodes, edges, and metadata about the
// current topology state.
func (c *Client) Topology(ctx context.Context) (*TopologyResponse, error) {
	raw, err := c.send(ctx, "topology_request")
	if err != nil {
		return nil, err
	}
	var resp TopologyResponse
	if err := json.Unmarshal(raw, &resp); err != nil {
		return nil, fmt.Errorf("fabric: unmarshal topology response: %w", err)
	}
	return &resp, nil
}

// Routes queries the daemon's active route plans.
func (c *Client) Routes(ctx context.Context) (*RoutesResponse, error) {
	raw, err := c.send(ctx, "routes_request")
	if err != nil {
		return nil, err
	}
	var resp RoutesResponse
	if err := json.Unmarshal(raw, &resp); err != nil {
		return nil, fmt.Errorf("fabric: unmarshal routes response: %w", err)
	}
	return &resp, nil
}

// Capabilities queries the daemon's capabilities snapshot across all nodes.
func (c *Client) Capabilities(ctx context.Context) (*CapabilitiesResponse, error) {
	raw, err := c.send(ctx, "capabilities_request")
	if err != nil {
		return nil, err
	}
	var resp CapabilitiesResponse
	if err := json.Unmarshal(raw, &resp); err != nil {
		return nil, fmt.Errorf("fabric: unmarshal capabilities response: %w", err)
	}
	return &resp, nil
}

// Probe sends a probe_request and returns the raw topology snapshot.
// This is equivalent to Topology() but returns the raw JSON for
// advanced use cases.
func (c *Client) Probe(ctx context.Context) (json.RawMessage, error) {
	return c.send(ctx, "probe_request")
}

// Heartbeat sends a heartbeat to the daemon and returns the acknowledgment.
func (c *Client) Heartbeat(ctx context.Context) (*HeartbeatResponse, error) {
	raw, err := c.send(ctx, "heartbeat")
	if err != nil {
		return nil, err
	}
	var resp HeartbeatResponse
	if err := json.Unmarshal(raw, &resp); err != nil {
		return nil, fmt.Errorf("fabric: unmarshal heartbeat response: %w", err)
	}
	return &resp, nil
}

// ParseDaemonError checks whether the raw daemon response is an error response
// and extracts the error if so.
func ParseDaemonError(raw json.RawMessage) (*DaemonError, bool) {
	var obj struct {
		Error   string `json:"error"`
		Message string `json:"message"`
	}
	if err := json.Unmarshal(raw, &obj); err != nil {
		return nil, false
	}
	if obj.Error == "" {
		return nil, false
	}
	return &DaemonError{
		ErrorType: obj.Error,
		Message:   obj.Message,
	}, true
}

// send is the internal method that handles the request/response cycle.
func (c *Client) send(ctx context.Context, msgType string) (json.RawMessage, error) {
	if c.closed {
		return nil, ErrClosed
	}

	// Apply context deadline to transport timeout if shorter.
	deadline, ok := ctx.Deadline()
	if ok {
		remaining := time.Until(deadline)
		if remaining <= 0 {
			return nil, ErrTimeout
		}
		c.pool.timeout = remaining
	} else {
		c.pool.timeout = c.timeout
	}

	// Check context cancellation.
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	// Use the transport to send the request.
	transport := NewTransport(c.pool)
	raw, err := transport.SendRequest(msgType)
	if err != nil {
		return nil, err
	}

	// Check for daemon error responses.
	if dErr, ok := ParseDaemonError(raw); ok {
		return nil, dErr
	}

	return raw, nil
}
