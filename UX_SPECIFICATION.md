# User Experience Specification

## Two interaction depths

### Workspace mode

For ordinary use:

- search devices, realms, applications and agent surfaces;
- activate `main-desk`, `couch`, `creator-safe`, `gaming`, `bench` or `remote`;
- cycle input, output or coupled seat focus;
- drag an app/surface to a visible display target;
- see route health, quality profile and fallback state;
- answer capability/attention/permission prompts;
- return to local control immediately.

### Graph mode

For expert use:

- inspect nested devices/realms/apps/resources;
- patch typed ports;
- inspect desired link versus compiled route;
- pin formats, nodes, transports or experiment variants;
- view clocks, buffers, memory domains, copies, codecs and costs;
- compare counterfactual plans;
- save graph fragments as workspace templates.

## Interaction guarantees

- Input focus and output focus are visually distinct.
- A coupled switch commits only when both sides are ready.
- The currently authoritative target is always visible in a persistent/quickly accessible indicator.
- Agents appear in a catalog and may notify but cannot unexpectedly take over.
- Automatic degradation shows what changed and why.
- Preview/experimental routes are labeled before activation.
- The user can pin a physical route for testing without changing the abstract workspace.
- Errors name the failed stage and offer the simplest valid fallback.

## Primary workflows

| Workflow | Maximum normal action |
|---|---|
| Desk ↔ couch | one workspace activation or coupled hotkey/palette selection |
| Cycle among prepared realms | one chord/tap |
| MacBook as third display | enable saved link/workspace; source display provisions automatically |
| Inspect agent realm | open/pin published surface; no provider-console navigation |
| Recover black VM | choose recovery surface; SPICE/OOB route is already cataloged |
| Enter creator-safe | one profile activation; background work relocates/throttles before admission |
| Understand slow route | “Why this route?” shows dominant stages, alternatives and blocked constraints |

## Accessibility and failure UX

All primary operations are keyboard accessible. Color is not the sole state cue. Screen readers receive semantic node/link/route summaries rather than only canvas coordinates. Break-glass return is not dependent on the main UI process.
