# Security Policy and Design Entry Point

The system can observe screens/audio/input, inject control, create realms, move data and access preboot KVMs. Treat compromise as workstation/credential compromise.

- Threat model: [`risks/threat-model.md`](risks/threat-model.md)
- Security architecture: [`architecture/security.md`](architecture/security.md)
- Security verification: [`verification/security-test-plan.md`](verification/security-test-plan.md)
- Incident response: [`operations/incident-response.md`](operations/incident-response.md)
- Pairing/privilege ADR: [`adr/0019-mutual-pairing-privileged-isolation.md`](adr/0019-mutual-pairing-privileged-isolation.md)

Security issues must not be placed in public logs/evidence bundles. A future repository should define a private reporting address and coordinated disclosure window before public release.
