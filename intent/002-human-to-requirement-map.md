# Human Intent to Requirement Map

| Intent source | Human clause / meaning | Formalization |
|---|---|---|
| INT-P001 | “1-5 bench test PCs… main PC with multi gpu… VMs… other PCs around the world” | PF-FR-002, 012, 030, 060, 084; specs 001–004 |
| INT-P001 | laptop as a “3rd” monitor | PF-FR-031, 040–046; spec 006 |
| INT-P001 | agents create N VMs/items for trivial access | PF-FR-037, 085; spec 008 |
| INT-P001 | Windows app drawn as a real macOS window | PF-FR-032–036; spec 006 |
| INT-P001 | multiple overlapping applications and relative mouse | PF-FR-023, 033–034; specs 002, 006 |
| INT-P001 | actual process migration considered by routing/intent | PF-FR-068; ADR-0010, ADR-0011 |
| INT-P001 | both-Ctrl input swap plus similar output swap | PF-FR-021–025, 030; spec 002 |
| INT-P001 | no predetermined host | PF-FR-001–005, 036; ADR-0002 |
| INT-P001 | user profile, VM, device, RDP point | Domain model: Principal/Device/Realm/Session/Seat |
| INT-P001 | desk/couch movement every ~30 minutes | PF-FR-005, 024, 030; examples/desk-to-couch.md |
| INT-P001 | Wayland/X11/Windows/macOS clipboard, files, Apple-like continuity | PF-FR-083, 088; spec 007 |
| INT-P002 | realtime and scaled performance are massive concerns | PF-FR-070–075; PF-NFR-001–025; spec 005 |
| INT-P002 | hosts and clients also run intensive background tasks | PF-FR-071–073; spec 005 |
| INT-P002 | gaming and Ableton Live 12 | PF-FR-042, 050–056, 070–075; examples/gaming.md and ableton.md |
| INT-P002 | HDR/audio/video/color quality | PF-FR-040–056; PF-NFR-030–044 |
| INT-P002 | C27HG70 and M1 Pro MacBook | reference test topology; verification compatibility matrix |
| INT-P003 | packaged all-in-one, non-monolithic beneath UI/API | PF-FR-001, 004; ADR-0002 |
| INT-P003 | PipeWire/JACK node-program view generalized | PF-FR-010–017, 050–056; ADR-0003 |
| INT-P004 | shared-memory/PCI layers for same device | PF-FR-012–013, 044; ADR-0004, ADR-0009 |
| INT-P004 | LAN/WAN/all stages maximally optimized | PF-FR-012–017, 041, 062; spec 004 |
| INT-P005 | merge all instances into one distributed plane | PF-FR-060–069; spec 003 |
| INT-P005 | syscall granule decision route | PF-FR-065–066; spec 003; ADR-0005 |
| INT-P005 | intention, constraints, compute/space requirements | PF-FR-061–064; placement cost model |
| INT-P005 | best/available hardware runs work behind scenes | PF-FR-062–067, 070–075 |
| INT-P005 | Citrix-like app visible anywhere | PF-FR-030–037; spec 006 |
| INT-P006 | atomic capability but normally coarser due to sharding cost | PF-FR-065–066; ADR-0005 |
| INT-P007 | integration with ShareCLI and other repos | PF-FR-086; spec 010; ecosystem/ |
| INT-P008 | PRD/HLD/LLD/spec/work/plan/research package | This documentation tree |
| INT-P008 | AgilePlus spec/ADR/WBS/PERT/DAG format | specs/, adr/, work/ |
| INT-P008 | SOTA and differentiators | sota/ |
| INT-P008 | exact prompts plus LLM synthesis | intent/ |
