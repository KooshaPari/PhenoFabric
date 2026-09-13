package fabric

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"sync"
	"time"
)

// ConnPool manages a pool of TCP connections to the daemon.
//
// Connections are reused across requests. The pool maintains a minimum
// number of idle connections and creates new ones on demand up to the
// maximum. Dead connections are detected on read/write and automatically
// replaced.
type ConnPool struct {
	mu       sync.Mutex
	addr     string
	conns    chan net.Conn
	minIdle  int
	maxIdle  int
	timeout  time.Duration
	closed   bool
	newConn  func() (net.Conn, error)
}

// ConnPoolConfig configures the connection pool.
type ConnPoolConfig struct {
	MinIdle int
	MaxIdle int
	Timeout time.Duration
}

// DefaultConnPoolConfig returns sensible defaults.
func DefaultConnPoolConfig() ConnPoolConfig {
	return ConnPoolConfig{
		MinIdle: 1,
		MaxIdle: 10,
		Timeout: 10 * time.Second,
	}
}

// NewConnPool creates a new connection pool.
func NewConnPool(addr string, cfg ConnPoolConfig) *ConnPool {
	if cfg.MinIdle <= 0 {
		cfg.MinIdle = 1
	}
	if cfg.MaxIdle <= 0 {
		cfg.MaxIdle = 10
	}
	if cfg.Timeout <= 0 {
		cfg.Timeout = 10 * time.Second
	}

	pool := &ConnPool{
		addr:    addr,
		conns:   make(chan net.Conn, cfg.MaxIdle),
		minIdle: cfg.MinIdle,
		maxIdle: cfg.MaxIdle,
		timeout: cfg.Timeout,
		newConn: func() (net.Conn, error) {
			dialer := net.Dialer{Timeout: cfg.Timeout}
			return dialer.Dial("tcp", addr)
		},
	}

	// Pre-warm with minIdle connections.
	for i := 0; i < cfg.MinIdle; i++ {
		conn, err := pool.newConn()
		if err != nil {
			// Pool will lazily create connections on first use.
			break
		}
		pool.conns <- conn
	}

	return pool
}

// Get retrieves a connection from the pool. If no idle connection is
// available and the pool is below maxIdle, a new connection is created.
func (p *ConnPool) Get() (net.Conn, error) {
	p.mu.Lock()
	if p.closed {
		p.mu.Unlock()
		return nil, ErrClosed
	}
	p.mu.Unlock()

	// Try to get an idle connection without blocking.
	select {
	case conn := <-p.conns:
		// Verify the connection is still alive.
		if isConnAlive(conn) {
			return conn, nil
		}
		conn.Close()
		// Fall through to create a new one.
	default:
		// No idle connections available.
	}

	return p.newConn()
}

// Put returns a connection to the pool. If the pool is full or the
// connection is dead, it is closed instead.
func (p *ConnPool) Put(conn net.Conn) {
	if conn == nil {
		return
	}

	p.mu.Lock()
	if p.closed {
		p.mu.Unlock()
		conn.Close()
		return
	}
	p.mu.Unlock()

	if !isConnAlive(conn) {
		conn.Close()
		return
	}

	// Non-blocking put to the channel.
	select {
	case p.conns <- conn:
	default:
		// Pool is full, close the connection.
		conn.Close()
	}
}

// Close drains and closes all connections in the pool.
func (p *ConnPool) Close() error {
	p.mu.Lock()
	if p.closed {
		p.mu.Unlock()
		return nil
	}
	p.closed = true
	p.mu.Unlock()

	close(p.conns)
	var firstErr error
	for conn := range p.conns {
		if err := conn.Close(); err != nil && firstErr == nil {
			firstErr = err
		}
	}
	return firstErr
}

// Len returns the number of idle connections in the pool.
func (p *ConnPool) Len() int {
	return len(p.conns)
}

// isConnAlive checks if a TCP connection is still usable by performing
// a zero-byte read. This detects connections that have been closed by
// the remote end.
func isConnAlive(conn net.Conn) bool {
	// Set a very short deadline to make this non-blocking.
	_ = conn.SetReadDeadline(time.Now().Add(1 * time.Millisecond))
	buf := make([]byte, 1)
	_, err := conn.Read(buf)
	_ = conn.SetReadDeadline(time.Time{}) // Clear deadline.

	if err == nil {
		return true
	}
	// A timeout error means the connection is alive but no data is available.
	if netErr, ok := err.(net.Error); ok && netErr.Timeout() {
		return true
	}
	return false
}

// ---------------------------------------------------------------------------
// Wire transport — line-delimited JSON over TCP
// ---------------------------------------------------------------------------

// Transport handles the wire protocol encoding/decoding over TCP.
type Transport struct {
	pool *ConnPool
}

// NewTransport creates a new wire transport.
func NewTransport(pool *ConnPool) *Transport {
	return &Transport{pool: pool}
}

// SendRequest sends a JSON request message and reads the JSON response.
// The msgType is used for constructing the request line.
func (t *Transport) SendRequest(msgType string) (json.RawMessage, error) {
	return t.SendRaw(fmt.Sprintf(`{"type":"%s"}`, msgType))
}

// SendRaw sends a raw JSON line and reads the response line.
func (t *Transport) SendRaw(line string) (json.RawMessage, error) {
	conn, err := t.pool.Get()
	if err != nil {
		return nil, fmt.Errorf("transport: get connection: %w", err)
	}

	// Ensure connection is returned to pool or closed on error.
	success := false
	defer func() {
		if !success {
			conn.Close()
		}
	}()

	// Set write deadline.
	_ = conn.SetWriteDeadline(time.Now().Add(t.pool.timeout))

	// Write request line + newline.
	lineBytes := append([]byte(line), '\n')
	if _, err := conn.Write(lineBytes); err != nil {
		// Connection may be stale; try reconnecting once.
		conn.Close()
		conn2, retryErr := t.pool.newConn()
		if retryErr != nil {
			return nil, fmt.Errorf("transport: reconnect: %w", retryErr)
		}
		conn = conn2
		_ = conn.SetWriteDeadline(time.Now().Add(t.pool.timeout))
		if _, err := conn.Write(lineBytes); err != nil {
			conn.Close()
			return nil, fmt.Errorf("transport: write: %w", err)
		}
	}

	// Set read deadline.
	_ = conn.SetReadDeadline(time.Now().Add(t.pool.timeout))

	// Read response line.
	reader := bufio.NewReader(conn)
	respLine, err := reader.ReadBytes('\n')
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("transport: read: %w", err)
	}

	// Trim trailing newline.
	if len(respLine) > 0 && respLine[len(respLine)-1] == '\n' {
		respLine = respLine[:len(respLine)-1]
	}

	if len(respLine) == 0 {
		conn.Close()
		return nil, ErrEmptyResponse
	}

	// Clear deadline and return connection to pool.
	_ = conn.SetReadDeadline(time.Time{})
	t.pool.Put(conn)
	success = true

	return json.RawMessage(respLine), nil
}
