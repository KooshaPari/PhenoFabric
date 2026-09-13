// Package fabric provides a Go client library for the Phenotype Fabric
// wire protocol (spec 025, PF-WP-040).
//
// The Fabric daemon communicates over TCP using line-delimited JSON messages.
// Each request is a single JSON line terminated by '\n'; the daemon responds
// with a single JSON line. This package wraps that protocol in a reusable
// Go client with connection pooling, automatic reconnection, and typed
// response models.
//
// # Message Types
//
// The daemon supports these request types:
//
//	health_check    → HealthResponse
//	topology_request / probe_request → TopologyResponse
//	routes_request  → RoutesResponse
//	capabilities_request → CapabilitiesResponse
//	heartbeat       → HeartbeatResponse
//
// # Usage
//
//	client := fabric.New("127.0.0.1:9400")
//	defer client.Close()
//
//	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
//	defer cancel()
//
//	health, err := client.HealthCheck(ctx)
//	if err != nil {
//		log.Fatal(err)
//	}
//	fmt.Printf("Daemon healthy: %v, uptime: %ds\n", health.Status, health.UptimeS)
//
//	topo, err := client.Topology(ctx)
//	if err != nil {
//		log.Fatal(err)
//	}
//	fmt.Printf("Topology: %d nodes, %d edges, epoch %d\n",
//		topo.NodeCount, topo.EdgeCount, topo.TopologyEpoch)
//
// # Connection Pooling
//
// The client maintains a pool of TCP connections (default: 1 min, 10 max).
// Connections are reused across requests. If a connection drops, the client
// automatically reconnects on the next request.
//
// # Timeouts
//
// Each request respects the provided context deadline. If no deadline is
// set, the client uses a configurable default timeout (default: 10s).
package fabric
