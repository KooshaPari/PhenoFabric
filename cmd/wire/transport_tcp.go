// Package wire — TCP transport for WireClient/WireServer.
//
// TCPTransport implements WireClient over TCP using the spec 025
// WireEnvelope format (JSON-over-TCP, one envelope per line).
//
// Wire format on the TCP stream:
//   - Each message is a single line of compact JSON (a WireEnvelope)
//   - Lines are delimited by '\n'
//   - The server reads one line, calls Handle, writes one line response
//   - For push msg_types (surface.invalidate, heartbeat) the server
//     returns nil and no response line is written
//
// This matches the Rust daemon's wire_server line-delimited JSON protocol
// but wraps payloads in spec 025 WireEnvelope envelopes.
package wire

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"net"
	"sync"
	"time"
)

// TCPTransport implements WireClient over TCP.
type TCPTransport struct {
	mu          sync.Mutex
	connections map[string]net.Conn
	codec       WireCodec
}

// NewTCPTransport creates a new TCPTransport.
func NewTCPTransport() *TCPTransport {
	return &TCPTransport{
		connections: make(map[string]net.Conn),
	}
}

// Send implements WireClient. It connects to the target node's TCP
// address (if not already connected), marshals the envelope as a
// single line of compact JSON, writes it, reads the response line,
// and unmarshals the response envelope.
//
// Returns nil response for push msg_types (surface.invalidate, heartbeat)
// since those are fire-and-forget.
func (tt *TCPTransport) Send(ctx context.Context, target NodeAddress, env WireEnvelope) (*WireEnvelope, error) {
	tt.mu.Lock()
	defer tt.mu.Unlock()

	addr := target.String()

	// Get or create connection.
	conn, ok := tt.connections[addr]
	if !ok {
		var err error
		dialer := net.Dialer{Timeout: 5 * time.Second}
		conn, err = dialer.DialContext(ctx, "tcp", net.JoinHostPort(target.Host, fmt.Sprintf("%d", target.Port)))
		if err != nil {
			return nil, fmt.Errorf("wire tcp: dial %s: %w", addr, err)
		}
		tt.connections[addr] = conn
	}

	// Marshal the envelope.
	data, err := tt.codec.Marshal(env)
	if err != nil {
		return nil, fmt.Errorf("wire tcp: marshal: %w", err)
	}

	// Write envelope + newline.
	line := append(data, '\n')
	if _, err := conn.Write(line); err != nil {
		// Connection might be stale; remove and retry once.
		conn.Close()
		delete(tt.connections, addr)

		dialer := net.Dialer{Timeout: 5 * time.Second}
		conn, err = dialer.DialContext(ctx, "tcp", net.JoinHostPort(target.Host, fmt.Sprintf("%d", target.Port)))
		if err != nil {
			return nil, fmt.Errorf("wire tcp: reconnect %s: %w", addr, err)
		}
		tt.connections[addr] = conn

		if _, err := conn.Write(line); err != nil {
			return nil, fmt.Errorf("wire tcp: write %s: %w", addr, err)
		}
	}

	// For push msg_types, don't wait for response.
	if isPushMsgType(env.MsgType) {
		return nil, nil
	}

	// Read response line with deadline.
	if err := conn.SetReadDeadline(time.Now().Add(30 * time.Second)); err != nil {
		return nil, fmt.Errorf("wire tcp: set deadline: %w", err)
	}

	reader := bufio.NewReader(conn)
	respLine, err := reader.ReadBytes('\n')
	if err != nil {
		return nil, fmt.Errorf("wire tcp: read response from %s: %w", addr, err)
	}

	// Trim trailing newline.
	if len(respLine) > 0 && respLine[len(respLine)-1] == '\n' {
		respLine = respLine[:len(respLine)-1]
	}

	// Unmarshal response envelope.
	resp, err := tt.codec.Unmarshal(respLine)
	if err != nil {
		return nil, fmt.Errorf("wire tcp: unmarshal response from %s: %w", addr, err)
	}

	return &resp, nil
}

// Close closes all open TCP connections.
func (tt *TCPTransport) Close() error {
	tt.mu.Lock()
	defer tt.mu.Unlock()
	for addr, conn := range tt.connections {
		conn.Close()
		delete(tt.connections, addr)
	}
	return nil
}

// isPushMsgType returns true for msg_types that are fire-and-forget
// (no response expected from the server).
func isPushMsgType(msgType string) bool {
	switch msgType {
	case MsgTypeSurfaceInvalidate, MsgTypeHeartbeat:
		return true
	}
	return false
}

// Compile-time check: TCPTransport implements WireClient.
var _ WireClient = (*TCPTransport)(nil)

// ---------------------------------------------------------------------------
// TCP WireServer (listens on a TCP port, dispatches to WireServer)
// ---------------------------------------------------------------------------

// TCPWireServer wraps a WireServer and exposes it over TCP.
// Each incoming connection is handled in a goroutine. Lines are read,
// decoded as WireEnvelopes, passed to the inner WireServer's Handle
// method, and the response (if any) is written back as a single line.
type TCPWireServer struct {
	listener net.Listener
	handler  WireServer
	codec    WireCodec
	quit     chan struct{}
	wg       sync.WaitGroup
}

// NewTCPWireServer creates a TCPWireServer listening on addr.
// addr should be "host:port" (e.g. "0.0.0.0:9090").
func NewTCPWireServer(addr string, handler WireServer) (*TCPWireServer, error) {
	ln, err := net.Listen("tcp", addr)
	if err != nil {
		return nil, fmt.Errorf("wire tcp server: listen %s: %w", addr, err)
	}
	return &TCPWireServer{
		listener: ln,
		handler:  handler,
		quit:     make(chan struct{}),
	}, nil
}

// Addr returns the listener's address (useful when binding to :0).
func (s *TCPWireServer) Addr() net.Addr {
	return s.listener.Addr()
}

// Serve starts accepting connections. It blocks until Close is called.
func (s *TCPWireServer) Serve() {
	for {
		conn, err := s.listener.Accept()
		if err != nil {
			select {
			case <-s.quit:
				return
			default:
				continue
			}
		}
		s.wg.Add(1)
		go s.handleConn(conn)
	}
}

// Close stops the server and waits for all connections to drain.
func (s *TCPWireServer) Close() error {
	close(s.quit)
	s.listener.Close()
	s.wg.Wait()
	return nil
}

func (s *TCPWireServer) handleConn(conn net.Conn) {
	defer s.wg.Done()
	defer conn.Close()

	reader := bufio.NewReader(conn)
	for {
		line, err := reader.ReadBytes('\n')
		if err != nil {
			return // connection closed or error
		}

		// Trim trailing newline.
		if len(line) > 0 && line[len(line)-1] == '\n' {
			line = line[:len(line)-1]
		}

		// Decode envelope.
		env, err := s.codec.Unmarshal(line)
		if err != nil {
			// Send error envelope back.
			errEnv, _ := s.codec.MarshalError(WireError{
				Code:    WireErrorBadEnvelope,
				Message: err.Error(),
			}, WireEnvelope{
				EnvelopeID: newUUIDv4(),
				TenantID:   "ops-phenotype-default",
				MsgType:    MsgTypeError,
			})
			respBytes, _ := s.codec.Marshal(errEnv)
			conn.Write(append(respBytes, '\n'))
			continue
		}

		// Dispatch to handler.
		resp, err := s.handler.Handle(context.Background(), env)
		if err != nil {
			errEnv, _ := s.codec.MarshalError(WireError{
				Code:    WireErrorBug,
				Message: err.Error(),
			}, env)
			respBytes, _ := s.codec.Marshal(errEnv)
			conn.Write(append(respBytes, '\n'))
			continue
		}

		// Write response (nil for push types).
		if resp != nil {
			respBytes, _ := s.codec.Marshal(*resp)
			conn.Write(append(respBytes, '\n'))
		}
	}
}

// Compile-time check: *TCPWireServer doesn't implement WireServer
// directly (it wraps one), but the pattern is: TCPWireServer accepts
// connections and delegates to an inner WireServer.
var _ = (*TCPWireServer)(nil)

// ---------------------------------------------------------------------------
// Wire-envelope-based JSON-RPC bridge for daemon wire_server protocol
// ---------------------------------------------------------------------------

// DaemonBridge bridges the spec 025 WireEnvelope protocol to the
// daemon's simple JSON protocol. This is used when the Go side
// talks to the Rust daemon over TCP using spec 025 envelopes, and
// the daemon responds in its own format.
//
// The bridge converts:
//   - WireEnvelope{MsgType: "replan.request"} → simple JSON {"type":"compile_request",...}
//   - Daemon simple JSON response → WireEnvelope{MsgType: "replan.response"}
type DaemonBridge struct {
	// Topology JSON to include in requests.
	Topology json.RawMessage
	// Intent JSON to include in requests.
	Intent json.RawMessage
}

// ReplanViaWire sends a replan request to a daemon over a WireClient
// using the spec 025 envelope format.
//
// The function:
//  1. Builds a WireReplanRequest with the given topology/intent/oldPlan/failedNodes
//  2. Wraps it in a WireEnvelope with MsgType = MsgTypeReplanRequest
//  3. Sends via the provided WireClient
//  4. Decodes the response WireReplanResponse
func ReplanViaWire(
	ctx context.Context,
	client WireClient,
	target NodeAddress,
	tenantID string,
	topology json.RawMessage,
	intent json.RawMessage,
	oldPlan json.RawMessage,
	failedNodes []string,
) (*WireReplanResponse, error) {
	req := WireReplanRequest{
		RequestID:   newUUIDv4(),
		Topology:    topology,
		Intent:      intent,
		OldPlan:     oldPlan,
		FailedNodes: failedNodes,
	}

	env, err := NewEnvelopeFromPayload(MsgTypeReplanRequest, tenantID, req)
	if err != nil {
		return nil, fmt.Errorf("wire: build replan envelope: %w", err)
	}

	resp, err := client.Send(ctx, target, env)
	if err != nil {
		return nil, fmt.Errorf("wire: send replan: %w", err)
	}
	if resp == nil {
		return nil, fmt.Errorf("wire: replan returned nil response")
	}

	// Check for error response.
	if resp.MsgType == MsgTypeError {
		var werr WireError
		if err := DecodePayload(*resp, MsgTypeError, &werr); err != nil {
			return nil, fmt.Errorf("wire: decode error response: %w", err)
		}
		return nil, fmt.Errorf("wire: daemon error %s: %s", werr.Code, werr.Message)
	}

	var rr WireReplanResponse
	if err := DecodePayload(*resp, MsgTypeReplanResponse, &rr); err != nil {
		return nil, fmt.Errorf("wire: decode replan response: %w", err)
	}
	return &rr, nil
}

// ProbeViaWire sends a probe.request to a daemon and returns the
// descriptor from the probe.response.
func ProbeViaWire(
	ctx context.Context,
	client WireClient,
	target NodeAddress,
	tenantID string,
) (json.RawMessage, error) {
	env, err := NewProbeRequestEnvelope(tenantID)
	if err != nil {
		return nil, fmt.Errorf("wire: build probe envelope: %w", err)
	}

	resp, err := client.Send(ctx, target, env)
	if err != nil {
		return nil, fmt.Errorf("wire: send probe: %w", err)
	}
	if resp == nil {
		return nil, fmt.Errorf("wire: probe returned nil response")
	}

	if resp.MsgType == MsgTypeError {
		var werr WireError
		if err := DecodePayload(*resp, MsgTypeError, &werr); err != nil {
			return nil, fmt.Errorf("wire: decode error response: %w", err)
		}
		return nil, fmt.Errorf("wire: daemon error %s: %s", werr.Code, werr.Message)
	}

	var pr ProbeResponse
	if err := DecodePayload(*resp, MsgTypeProbeResponse, &pr); err != nil {
		return nil, fmt.Errorf("wire: decode probe response: %w", err)
	}
	return pr.Descriptor, nil
}
