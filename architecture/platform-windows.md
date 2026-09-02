# Windows Platform Design

## Endpoint service

A Windows service owns device/realm registration and launches user-session helpers only for authorized interactive sessions. Session 0 is not treated as a graphical session.

## Display and capture

- IddCx virtual-display driver for stable dynamic display identities;
- Windows Graphics Capture for window/display capture;
- Desktop Duplication as fallback where appropriate;
- DXGI/D3D shared-resource paths for same-OS process integration;
- hardware codec selection with session/engine pressure inventory.

Virtual displays are created before route commit and removed only after dependent surfaces drain.

## Input

Prototype:

- Raw Input for source device identity;
- SendInput for ordinary injection;
- gamepad APIs/virtual controller where safe.

Mature:

- Virtual HID Framework driver for keyboard/mouse/controller fidelity;
- explicit secure-desktop boundary;
- pressed-state reconciliation and fencing.

## Audio

- ASIO for professional device/app paths where installed;
- WASAPI for system endpoints;
- virtual audio endpoint only if required and safely signed;
- clock/format and hidden-buffer discovery;
- ETW/DPC/ISR pressure measurement for RT diagnosis.

## Workload execution

- Job Objects and process groups;
- CPU Sets/priority classes;
- ETW and performance counters;
- WSL/VM/container adapters through NVMS/ShareCLI where applicable;
- checkpoint/rematerialization preferred over arbitrary process migration.

## Application presentation

- RAIL/RemoteApp when edition/infrastructure permits;
- pixel proxy source agent otherwise;
- owned HWND graph, modality, DPI awareness, IME, clipboard, file promises;
- parking virtual display to keep source rendering active.

## Security

- service and user helper are separate;
- signed drivers;
- DPAPI/TPM-backed identity;
- UAC/secure desktop never spoofed through normal proxy;
- uninstall restores native device stack.
