# Vendor and Upstream Dependency Register

| Dependency | Value | Material risk | Exit strategy |
|---|---|---|---|
| Looking Glass/KVMFR | same-host VFIO fast path | kernel/QEMU/driver compatibility or maintenance | SPICE/noVNC/encoded fallback; bounded fork/upstream contribution |
| Sunshine/Moonlight | open encoded media ecosystem | protocol/client/platform changes | adapter contract; alternate host/client or custom bounded transport |
| Parsec | convenient WAN access | API/service/pricing/terms | optional only; self-hosted WAN path remains |
| Microsoft RDP/RAIL | Windows semantic app/session delivery | edition/session/platform constraints | pixel proxy or full desktop |
| Apple macOS APIs | Mac sink/input/audio/native proxy | TCC/API/entitlement changes | supported API subset/full desktop; no bypass |
| Windows driver platform | virtual displays/HID/deep capture | signing and update maintenance | existing certified adapters and reduced feature set |
| NVIDIA codecs/CUDA/GPUDirect | reference GPU and accelerated paths | vendor/driver/topology coupling | capability interfaces and AMD/Intel/CPU alternatives |
| OOB appliance vendors | preboot/crash access | firmware/security/API abandonment | standards-like HDMI/USB and replaceable appliance layer |
| Overlay/relay provider | WAN reachability | outage/privacy/lock-in | self-hosted/direct alternatives and local-first coordinator |

No optional commercial service is allowed to become necessary for same-host or LAN operation.
