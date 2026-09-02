# Recovery Playbooks

## Lost input

- use dedicated ungrabbed keyboard/button or local emergency chord;
- stop input helper; uinput endpoints release all held state;
- revoke current focus lease;
- restore local seat before debugging remote route.

## Black/invalid surface

- switch to full desktop or SPICE/noVNC recovery;
- inspect virtual display and source mode;
- disable HDR/advanced profile only as an explicit fallback;
- use OOB if guest/host display path is unavailable.

## Audio instability

- protect/restore local interface path;
- stop remote monitoring/DSP nodes;
- increase buffer only under declared profile change;
- identify clock/xrun/resource source from trace before readmission.

## Coordinator loss

- current data planes run until lease policy expires;
- warm replica acquires a higher fencing epoch;
- endpoints reject stale coordinator tokens;
- unresolved transactions abort and preserve prior route.

## Object authority loss

- freeze mutable writes;
- identify last authoritative version and valid replicas;
- promote only through policy/epoch transaction;
- verify hashes/provenance before resuming.

## No boot / OS crash

- use OOB HDMI/HID/virtual media/ATX path;
- recover firmware/boot/driver state;
- software fabric rejoins only after endpoint identity and capability probe pass.
