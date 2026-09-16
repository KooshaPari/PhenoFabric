# 2-Node Test Harness Feasibility Design

**Session:** 20260916-2node-harness-feasibility
**Date:** 2026-09-16
**Status:** Research complete, design produced

## Goal

Design a test harness that validates 2-node Fabric operation: two daemon instances
that discover each other, exchange topology, compile routes across the network, and
stream frames between nodes. This is the minimum viable step toward the dossier's
"desk-to-laptop-to-desk" vision.

## Key Finding

~80% of what 2-node needs **already exists**. The federation layer, wire protocol,
multihop compiler, and workspace FSM are all implemented and tested in-process.
The missing piece is the **harness scaffolding**: a test runner that spins up two
coordinators on different TCP ports within the same process, wires them as peers,
and exercises the full lifecycle.

## Files produced

- `01_CODEBASE_ASSESSMENT.md` — what exists vs what needs building
- `02_HARNESS_DESIGN.md` — architecture, transport, simulation strategy
- `03_WBS.md` — 10-minute task breakdown with dependencies
