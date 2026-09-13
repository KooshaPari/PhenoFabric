# Changelog

All notable changes to Phenotype Fabric will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-09-13

### Added

- **Core crates**: fabric-graph (topology compiler), fabric-capability (descriptor model + probe),
  fabric-checker (constraint checker), fabric-persist (SQLite storage), fabric-workspace (seat lease FSM)
- **Transport**: frame-transport (encode/decode, multihop routing), daemon wire server (TCP JSON protocol)
- **GPU surface**: fabric-surface-mojo (MLIR/FFI bridge for GPU compute)
- **GUI surfaces**:
  - fabric-tray (macOS system tray with tao)
  - fabric-tui (ratatui terminal dashboard, 4 tabs)
  - fabric-gui (egui native desktop, 4 panels, dark/light mode)
  - fabric-web (Leptos + WebRTC data channels)
  - macOS .app bundle with icon, Info.plist, ad-hoc codesign
- **CLI**: fabric-cli (11 commands: topology, route, workspace, surface, check, probe, status, etc.)
  - graph-cli (dedicated graph compiler CLI)
- **Orchestration**: fabric-orchestrator (end-to-end pipeline with CLI)
- **Integration**: fabric-integration-tests (10 cross-crate tests), nvms-adapter
- **Examples**: full-demo with 6-node topology, Rust binary, shell script
- **Daemon**: fabric-daemon with wire server, config, hot-reload
- **DevOps**: Dockerfile, docker-compose, systemd unit, GitHub Actions CI/CD
- **Testing**: 464 tests, fuzz targets, proptest properties, criterion benchmarks
- **Documentation**: 35+ ADRs, mdBook site, architecture docs, Go adapter guide
- **Mojo GPU**: fabric-surface-mojo crate with MLIR bridge
