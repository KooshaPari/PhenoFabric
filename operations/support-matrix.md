# Support and Deprecation Policy

## Support unit

The unit is a **capability fingerprint**, not merely an OS/product name. It includes core/adapter/helper versions, OS/kernel/build, GPU/driver, display/audio modes, hypervisor and transport.

## Adapter status

- stable: full required suite on declared matrix;
- preview: opt-in with telemetry and explicit fallback;
- lab: manual experiments, no compatibility promise;
- deprecated: replacement available and migration documented;
- removed: security/maintenance incompatibility; workspace node preserved as disabled metadata.

## Deprecation

A route is not removed solely because it is old if it remains the only recovery path. Security-critical removals override this rule but require an alternate safe path or explicit unsupported notice.
