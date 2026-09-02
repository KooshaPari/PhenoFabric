# Release Acceptance Gates

## Gate A — Correctness

- all P0 functional requirements have passing trace/evidence;
- graph transactions preserve old route on prepare failure;
- no stale lease/input/object authority accepted;
- workspace round-trip and rollback pass.

## Gate B — Reference user scenarios

- local host/VM seat and Looking Glass path;
- MacBook as third display and couch seat;
- game under agent/compiler load;
- Ableton under admitted background load;
- agent-created realm/surface without focus theft;
- WAN reconnect and OOB recovery.

## Gate C — Performance and quality

- mixed-load latency distributions satisfy declared profile;
- hard audio route has no xruns in soak/fault envelope;
- HDR/color transform is explicit and tested;
- no avoidable codec/copy stage on same-host route;
- scheduler improves or correctly declines movement.

## Gate D — Security and operations

- threat/red-team gate accepted;
- signed installers/helpers and credential storage;
- upgrade/downgrade/rollback from clean machines;
- logs/evidence redacted and exportable;
- important devices retain break-glass/OOB route.

## Gate E — Product honesty

Known limits and experimental matrices are displayed. Unsupported secure surfaces, remote DSP, arbitrary process migration or atomic routing are never implied by adjacent successful demos.
