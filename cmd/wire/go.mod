module github.com/phenotype/fabric/cmd/wire

go 1.21

// cmd/wire — Fabric wire transport contract (spec 025, PF-WP-040, R2 wedge #4).
//
// This package is the bytes-on-wire contract for inter-node communication
// in the Fabric. It ships:
//   - WireEnvelope: the outermost wire shape (envelope_id, tenant_id,
//     msg_type, payload, signature, sent_at_unix_ms)
//   - WireCodec: deterministic JSON Marshal/Unmarshal/Validate
//   - 5 WireMessage types (probe.request/response, replan.request/response,
//     surface.invalidate, heartbeat) plus a synthetic error msg_type
//   - WireError: stable 7-code error taxonomy
//   - WireClient / WireServer interfaces (no implementations — R3)
//
// Actual transport (HTTP / gRPC / UDS) is deferred to R3 per spec §2.2.
// This package has zero runtime dependencies — stdlib only.
//
// Spec:    specs/025-wire-transport/spec.md
// Plan:    specs/025-wire-transport/plan.md
// Tasks:   specs/025-wire-transport/tasks.md
// ADRs:    ADR-0028 (read-authoritative-source-first), ADR-0029 (Go is
//          canonical for wire transport), ADR-0030 (route failover model),
//          ADR-0031 (trust-root for descriptor signatures).