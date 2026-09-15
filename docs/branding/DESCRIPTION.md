# terminal-fabric

## One-liner (60 chars max)

Multi-device terminal mirroring across machines.

## Short description (160 chars)

Mirror terminals across devices. See the same panes, share focus, resume work -- tmux across machines for the Phenotype ecosystem.

## Full description

Terminal Fabric is a multi-device terminal mirroring system built for the Phenotype ecosystem. It lets a laptop, desktop, or remote workstation observe and safely control the same workspace without fighting over cursor input.

Think of it as tmux across machines. Terminal Fabric coordinates terminal state across devices by mirroring windows, tabs, and panes onto another machine, mapping an N window x M pane layout into a deterministic local pane map, and enforcing focus ownership so only one device writes to a pane at a time.

The system captures pane and focus state so Phenotype resume-all workflows can restart work exactly where you left off. Whether you are switching from a laptop to a desktop, or picking up a session on a remote workstation, your terminal context travels with you.

Terminal Fabric is intentionally small and modular. It talks to the transport layer through a socket/API boundary and keeps all orchestration logic in testable modules that run without a live server.

## Key features

- **Multi-device mirroring** -- observe the same terminal session from any connected device
- **Focus ownership** -- only one device writes to a pane at a time, preventing conflicts
- **Pane layout sync** -- deterministic mapping of source to target panes across different screen sizes
- **Session snapshots** -- capture pane and focus state for resume-all workflows
- **WebSocket viewer** -- browser-based terminal viewer via the web server component
- **Cross-platform capture** -- Windows agent using UIA and console APIs
- **SSH sync** -- CLI-based terminal synchronization over SSH
- **Lightweight transport** -- newline-delimited JSON-RPC over Unix sockets or TCP
