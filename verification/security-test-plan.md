# Security Verification Plan

## Surfaces

- enrollment and device identity;
- capability tokens and focus leases;
- control/data channel authentication and downgrade resistance;
- privileged Linux input/capture, Windows driver/helper and macOS permissions;
- clipboard/files/drag-drop and MIME confusion;
- screen/window capture, protected surfaces and recursive capture;
- agent realm creation and surface publication;
- route/compiler metadata poisoning;
- adapter/plugin supply chain;
- OOB KVM, management VLAN and overlay ACLs;
- logs/evidence containing secrets or captured content.

## Required attacks

- MITM and replay during first pairing;
- stolen/revoked device credentials;
- stale fencing token after partition;
- malicious endpoint advertising impossible capabilities;
- input injection outside granted seat/surface;
- clipboard exfiltration and oversized payload denial;
- route recursion and capture amplification;
- malicious adapter attempting privilege escalation;
- unsigned/downgraded helper/update;
- agent requesting broad focus/USB/file permissions;
- OOB service exposed outside management policy;
- trace/log redaction failures.

## Promotion rule

No automatic route with a critical unresolved finding. Preview routes are still prohibited from bypassing identity, capability and privileged-helper boundaries.
