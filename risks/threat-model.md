# Threat Model

## Assets

- human keyboard/mouse/controller/microphone authority;
- screen/window/audio content;
- files, clipboard, credentials and recordings;
- VM/host/OOB control;
- compute budgets and agent permissions;
- object authority and durable artifacts;
- route/workspace policy and evidence integrity.

## Trust boundaries

```text
human ↔ UI
UI ↔ coordinator
coordinator ↔ privileged local helpers
endpoint ↔ endpoint over LAN/WAN/relay
host ↔ VM/realm
agent/thegent ↔ realm/provider
Fabric ↔ AgilePlus/Tracera/ledgers
software plane ↔ OOB KVM
```

## Adversaries

- compromised remote or bench endpoint;
- malicious/buggy adapter or plugin;
- network MITM/relay operator;
- untrusted agent or prompt-injected tool;
- local unprivileged process attempting capture/injection;
- stolen device credentials;
- malicious file/clipboard payload;
- supply-chain compromise;
- accidental operator action and stale coordinator after partition.

## Security properties

1. No input/view/mic/file/USB/compute/focus authority without explicit capability.
2. A stale token cannot act after a higher fencing epoch.
3. Privileged helpers expose narrow typed operations and no general shell.
4. Enrollment authenticates both peers and confirms human-visible identity.
5. Data-plane encryption/authentication is bound to route/identity, not only network location.
6. Secure/protected platform boundaries are respected, not bypassed.
7. Agents request attention and scoped resources; human focus is separate.
8. OOB network/credentials are isolated from ordinary user/data planes.
9. Logs/evidence minimize content and preserve integrity/provenance.
10. Local break-glass control survives coordinator/peer failure.

## Abuse cases

- invisible remote keylogger route;
- agent creates full-screen fake login surface and steals input;
- clipboard HTML/file-list parser exploit;
- recursive surface route amplifies capture/traffic;
- false low-latency capability forces sensitive route through malicious relay;
- stale coordinator sends keystrokes to old target;
- malicious cache replica returns tampered model/build artifact;
- OOB API reboots or mounts media without visible grant.
