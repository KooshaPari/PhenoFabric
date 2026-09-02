# Scenario: Agent-Created Game E2E Realm

## Request

```yaml
realm_request:
  owner: principal://agent/build-12
  purpose: game-e2e
  ttl: 2h
  resources:
    gpu: { class: graphics, memory_min_gib: 8 }
    cpu_cores_min: 6
    memory_min_gib: 16
  policy:
    service_class: background
    may_preempt: false
    may_reduce_foreground_game_slo: false
  publish:
    - desktop
    - application: game-under-test
    - report: e2e-results
  attention:
    request_only: true
```

## Execution

1. thegent submits the authorized request with AgilePlus WorkPackage/evidence IDs.
2. Fabric chooses a local VM, bench PC or remote node based on GPU availability, game assets and foreground externality.
3. Assets/toolchain are prefetched to the chosen realm.
4. The game runs and publishes a low-priority surface; vision/telemetry can execute elsewhere.
5. The agent may request attention when a human decision is needed.
6. The realm expires, captures artifacts, emits evidence references and enters grace cleanup.

## User experience

The new realm appears in the surface palette but never takes the C27HG70, keyboard or audio. The user may pin it to the MacBook, portal the game window or take over its desktop.
