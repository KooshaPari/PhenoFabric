# Protocol Suite

## Design

No single payload protocol is mandatory. A shared envelope, identity, time, capability, and route model allows specialized transports.

## Control RPC

Recommended baseline: protobuf/gRPC semantics over local IPC and QUIC/TLS.

Core services:

```text
RegistryService
WorkspaceService
GraphService
RouteService
PlacementService
ObjectService
RealmService
EvidenceService
```

MCP may expose selected agent-friendly operations, but MCP is not the internal RT or bulk transport.

## Event envelope

```protobuf
message EventEnvelope {
  string event_id = 1;
  string event_type = 2;
  string schema_version = 3;
  string producer_id = 4;
  string principal_id = 5;
  string correlation_id = 6;
  string causation_id = 7;
  int64 observed_unix_nanos = 8;
  uint64 topology_epoch = 9;
  bytes payload = 10;
  bytes signature = 11;
}
```

## Local IPC

- Unix domain sockets/SCM_RIGHTS on Linux/macOS;
- named pipes/ALPC-compatible service boundary on Windows;
- shared-memory rings for hot paths;
- eventfd/futex/OS event primitives;
- handles/fds passed rather than copied where safe.

## Peer transport

A QUIC-family transport is a strong baseline because it provides:

- authenticated encrypted connection;
- streams and datagrams;
- independent flow control;
- connection/path migration;
- NAT traversal ecosystem.

It is not mandatory for RDMA, shared memory, RAIL/RDP, or third-party backends.

## Media framing

Media messages carry:

- route and stream IDs;
- sequence and source timestamp;
- format epoch;
- frame/buffer boundaries;
- dependency/recovery metadata;
- optional FEC group;
- deadline/expiry.

Late media is discardable; control state is reliable.

## Compatibility

Protocol negotiation chooses schema major/minor and capabilities. Unknown fields are retained/ignored according to schema rules. Privileged-helper protocols fail closed on unsupported versions.
