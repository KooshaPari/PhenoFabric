# Release and Versioning Plan

## Channels

- `lab`: unsigned/local experimental adapters; never auto-promoted.
- `dev`: signed internal builds and schema migrations.
- `preview`: opt-in devices, bounded hardware/OS matrix, telemetry required.
- `stable`: only routes/adapters with accepted compatibility and rollback evidence.

## Compatibility

- Graph and event schemas use explicit versions and capability negotiation.
- Adapter ABI/API has a minimum/maximum compatible core range.
- Privileged helper updates are independent packages but transactionally coordinated.
- Workspace files remain forward-readable; unknown node/port types are preserved but disabled.
- A route never silently changes quality/security semantics during upgrade.

## Promotion

A route implementation is promoted independently. The product can release with a stable graph and preview pixel-proxy adapter. Status appears in the UI and policy can prohibit preview routes.
