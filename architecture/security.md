# Security Architecture

## Assets

- human input and credentials;
- screen/window/audio/mic/camera contents;
- files and clipboard;
- VM/process control;
- GPU/CPU/storage capacity;
- OOB power/BIOS access;
- device identity keys;
- workspace and route policy;
- audit/evidence records.

## Trust zones

- local device;
- personal LAN;
- management/OOB LAN;
- authenticated overlay;
- WAN peer;
- blind relay;
- agent realm;
- untrusted application surface.

Network location alone grants nothing.

## Enrollment

1. endpoint generates device key in platform secret store;
2. coordinator and endpoint perform ephemeral key agreement;
3. user confirms code/QR/device display;
4. coordinator issues revocable device certificate;
5. capabilities remain separate from identity.

No reusable secret is returned over plain HTTP.

## Capabilities

A token binds:

- principal;
- source and sink;
- action;
- purpose;
- constraints;
- expiry;
- topology epoch;
- audit ID.

Actions include view, inject input, microphone, audio output, clipboard read/write, file send/mount, USB, create realm, place compute, migrate, and steal/take focus.

## Privileged-helper design

- one helper per privilege class where practical;
- no network listener;
- authenticated local IPC;
- strict message schema and size limits;
- device/path allowlists;
- no shell command interpolation;
- seccomp/sandbox/job restrictions;
- independent update/version compatibility;
- audit of privileged actions.

## Agent boundaries

Agent UI/surfaces are labeled and non-stealing. Agent-created realms receive TTL, resource quota, and minimal grants. Agent cannot inherit the user's microphone/clipboard/file permissions.

## OOB

KVM appliances are isolated on a management VLAN/overlay ACL. Credentials are unique. No direct public forwarding. Firmware/version inventory is maintained.

## Evidence privacy

Default evidence contains timing, topology, decisions, errors, and content hashes—not frame/audio/key/file payloads. Payload capture is explicit, scoped, encrypted, and expiring.

## Threats

Detailed STRIDE/abuse analysis is in `../risks/threat-model.md`. Release blocking threats include stale-token input, helper privilege escalation, route confused deputy, malicious agent surface, clipboard exfiltration, relay impersonation, driver persistence, and OOB takeover.
