# Packaging and Installation Architecture

## User promise

The user installs one product per device. Internally, installation may contain a UI, unprivileged endpoint daemon, optional privileged helpers, adapter bundles and platform drivers. Component boundaries remain visible for security and diagnostics but not as unrelated products.

## Component manifest

```text
phenotype-fabric-ui             user shell / graph / workspace
phenotype-fabric-agent          unprivileged endpoint and local control
phenotype-fabric-coordinator    local-first workspace/lease authority
phenotype-fabric-helper-input   narrow privileged input service
phenotype-fabric-helper-media   platform capture/virtual-display helper as needed
phenotype-fabric-adapters/*     Looking Glass, RDP, Sunshine, Xpra, OOB, storage
phenotype-fabric-diagnostics    probe, trace, evidence and recovery bundle
```

## Platform distribution

| Platform | Primary package | Privileged setup | UI |
|---|---|---|---|
| Linux | native distro packages plus portable archive for preview | udev/group/systemd rules; optional KVM/VFIO and portal integration | native/web-capable shell; tray/status item |
| Windows | signed MSIX/MSI or equivalent bootstrapper | signed VHF/virtual-display/helper only when feature enabled | WinUI/native shell or shared frontend host |
| macOS | notarized signed app/pkg | TCC-guided Screen Recording, Accessibility, microphone; system extension only if justified | AppKit/Swift shell with native proxy windows |

## Install flow

1. Explain requested capabilities before asking for privilege.
2. Install unprivileged core first.
3. Probe platform and show optional feature bundles.
4. Enroll device using confirmed mutual pairing.
5. Install only required helpers/adapters.
6. Run clean local self-test and emergency-return test.
7. Offer workspace templates and optional peer discovery.

No cloud account is required for local operation. Hosted relay/discovery may be optional and independently disableable.
