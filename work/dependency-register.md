# External Dependency and Assumption Register

| ID | Dependency / assumption | Affected work | Failure response |
|---|---|---|---|
| DEP-001 | Looking Glass/KVMFR remains compatible with target Linux/QEMU/NVIDIA stack | 040 | Pin known version; retain SPICE and encoded fallback; contribute upstream where bounded. |
| DEP-002 | macOS permits required capture, presentation and input under TCC | 060, 140 | Use user-approved APIs; fall back to full desktop; do not bypass secure input. |
| DEP-003 | Windows virtual display/capture helper can be signed and maintained | 060, 070, 140 | Use existing virtual display/RemoteApp path or physical display adapter; keep helper minimal. |
| DEP-004 | M1 Pro hardware/software path decodes selected 10-bit codecs at target mode | 070, 090 | Benchmark; choose HEVC/Main10 or lower mode; do not assume AV1 hardware support. |
| DEP-005 | C27HG70 HDR metadata/OS/driver chain behaves consistently | 090 | Preserve per-route calibration and expose SDR fallback. |
| DEP-006 | Ableton/ASIO path can be isolated from host contention | 080, 120 | Admission control; pin/relocate background work; reject route when deadline cannot be guaranteed. |
| DEP-007 | Cross-platform global hotkeys and input capture remain permitted | 030, 050, 060 | Per-platform permissions and local emergency hardware path. |
| DEP-008 | Adapter projects expose stable automation surfaces | 040, 070, 140, 150 | Wrap process/config surfaces; fork only when maintenance cost is justified. |
| DEP-009 | Network topology has sufficient wired bandwidth/latency for selected quality | 070, 080, 090 | Route compiler lowers quality or keeps execution/presentation local. |
| DEP-010 | Atomic interposition produces net benefit for some workloads | 190 | A negative result leaves capability unshipped; explicit TaskSpec remains product path. |
