# Scenario: Desk to Couch and Back

## Initial workspace: `main-desk`

```text
Input:    main keyboard/mouse → Windows VFIO game/dev realm
Display:  realm display-0 → C27HG70
Audio:    realm/app → desk DAC/interface
Pinned:   agent build report → MacBook corner window
```

## User action

The user opens the workspace palette on the MacBook or taps the coupled-seat hotkey and chooses `couch`.

## Transaction

1. Prepare MacBook input, display and speaker endpoints.
2. Create/restore a stable virtual display in the source realm matching the MacBook sink policy.
3. Start the lowest-cost valid media path and wait for first current frame/audio readiness.
4. Synthesize releases to the desk target and revoke its input lease.
5. Commit the MacBook output and input leases under one fencing epoch.
6. Place the remote surface full-screen; restore pinned native Mac surfaces.
7. Crossfade audio and show a short route OSD.
8. Keep process execution on the main PC unless a separate compute decision justifies movement.

## Failure behavior

If the Mac route does not prepare, nothing leaves the desk. If video prepares but input does not, output may be offered as a mirror but coupled-seat commit fails. The user always retains an emergency local path.

## Return

Activating `main-desk` reverses the seat transaction. It does not restart the game, DAW, VM or agent tasks.
