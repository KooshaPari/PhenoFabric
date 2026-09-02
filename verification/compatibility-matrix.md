# Compatibility Matrix and Support Policy

| Area | Reference baseline | Dimensions | Support gate |
|---|---|---|---|
| Host OS | Linux host; Windows and macOS endpoint agents | kernel/compositor; Windows edition/build; macOS version/TCC | Capability probe and clean install; unsupported feature disabled rather than guessed |
| Hypervisor | QEMU/KVM/libvirt VFIO | QEMU/libvirt/OVMF machine versions, IOMMU groups | LG/SPICE/input/audio/files recovery test |
| GPU | RTX 3090 Ti; optional GTX 1080 Ti; M1 Pro decode/presentation | driver, codec, external memory, copy/encode/decode engines | Measured capability and mixed-load pressure |
| Displays | Samsung C27HG70 and M1 Pro panel | resolution/scale/refresh/HDR/VRR/ICC | Probe plus reference pattern/frame test |
| Audio | Ableton Live 12 Suite and available interface | ASIO/WASAPI/CoreAudio/PipeWire/JACK, sample rates/buffers | xrun/round-trip/clock soak |
| Input | keyboard, mouse, controller, trackpad, microphone/MIDI as available | evdev/uinput/libei, Raw Input/VHF, CGEvent/IOHID | state torture, raw-relative and secure-boundary tests |
| Network | wired LAN plus controlled WAN | 1/2.5/10GbE as available, Wi-Fi, overlay, NAT/relay | impairment and roaming suite |
| OOB | PiKVM/JetKVM-class or hardware matrix | capture mode, HID, virtual media, ATX, API/security | BIOS/no-boot recovery |

## Levels

- `validated`: full acceptance on exact fingerprint.
- `compatible`: same capability class with passing probes and reduced suite.
- `experimental`: explicit opt-in; no automatic route.
- `unsupported`: capability is hidden or fallback used.

Support follows fingerprints and probes, not broad claims such as “NVIDIA” or “macOS compatible.”
