# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in Phenotype Fabric, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please email: **security@phenotype.dev** (or the repository maintainer directly).

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Any suggested fix (if available)

### Response timeline

| Action | Target |
| --- | --- |
| Acknowledgement | 48 hours |
| Initial assessment | 5 business days |
| Fix or mitigation | 30 days for critical/high severity |

We will work with you to understand and resolve the issue promptly. We appreciate your help in keeping Phenotype Fabric secure.

## Security Architecture

The system can observe screens/audio/input, inject control, create realms, move data and access preboot KVMs. Treat compromise as workstation/credential compromise.

- Threat model: [`risks/threat-model.md`](risks/threat-model.md)
- Security architecture: [`architecture/security.md`](architecture/security.md)
- Security verification: [`verification/security-test-plan.md`](verification/security-test-plan.md)
- Incident response: [`operations/incident-response.md`](operations/incident-response.md)
- Pairing/privilege ADR: [`adr/0019-mutual-pairing-privileged-isolation.md`](adr/0019-mutual-pairing-privileged-isolation.md)

Security issues must not be placed in public logs/evidence bundles.
