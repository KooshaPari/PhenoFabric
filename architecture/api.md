# Public API and Operator Surface

## Principles

- the GUI, CLI, SDK, and agents use the same capability model;
- APIs express intent and constraints, not backend commands;
- explanations and dry-run are first-class;
- destructive/focus/privileged actions are explicit;
- resource and route IDs are stable.

## Example CLI

```bash
pf nodes list
pf graph show --workspace main-desk
pf workspace activate couch
pf focus input next
pf focus output set pf://realm/main-pc/win-game-vm
pf surface portal pf://surface/... --sink pf://device/macbook/panel
pf route explain pf://route/...
pf run cargo-build --cpu ">=8" --protect workspace:music
pf realm create --template win-test --ttl 2h --publish desktop
pf object where sha256:...
pf diagnostics export --redact payloads
```

## API resources

- `/v1/principals`
- `/v1/devices`
- `/v1/realms`
- `/v1/sessions`
- `/v1/seats`
- `/v1/surfaces`
- `/v1/resources`
- `/v1/ports`
- `/v1/links`
- `/v1/routes`
- `/v1/objects`
- `/v1/workspaces`
- `/v1/placements`
- `/v1/evidence`

## Graph patch

```json
{
  "baseVersion": 42,
  "operations": [
    {
      "op": "connect",
      "sourcePort": "pf://surface/win-vm/app/video",
      "sinkPort": "pf://device/macbook/panel/video",
      "policy": {"latency":"realtime","hdr":"preserve"}
    }
  ],
  "dryRun": true
}
```

Dry-run returns candidates, selected plan, rejected constraints, reservations, and expected degradation.

## Placement request

```json
{
  "workId": "agileplus://wp/PF-WP-...",
  "task": {"command":["cargo","build","--release"]},
  "constraints": {
    "deadline": null,
    "trustZone":"personal",
    "protectWorkspaces":["music"],
    "requiredObjects":["sha256:..."]
  },
  "objective":"minimize_completion_time"
}
```

## Agent publication

```json
{
  "realmId":"pf://realm/agent-test-483",
  "surface":{
    "kind":"report",
    "title":"GPU regression results",
    "uri":"object://sha256:..."
  },
  "attention":"request",
  "stealFocus":false
}
```

## Explain API

Every route/placement can return:

- selected candidate;
- alternatives;
- hard constraints;
- predicted/actual cost terms;
- copies/transforms;
- resource pressure;
- confidence;
- fallback;
- governing policy/requirement/run IDs.
