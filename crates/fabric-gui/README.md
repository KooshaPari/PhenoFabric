# Fabric GUI

Native desktop GUI for Phenotype Fabric — topology visualization with surface streaming.

## Architecture

Built with [eframe/egui](https://github.com/emilk/egui) (Rust). Communicates with the Fabric daemon
on port 9400 via a JSON wire protocol. Falls back to SQLite when the daemon is unavailable.

### Panels

| Panel | Description |
|-------|-------------|
| **Dashboard** | System overview, daemon status, resource summary |
| **Topology** | Interactive graph visualization of nodes, edges, and surfaces |
| **Routes** | Route table with real-time updates from the daemon |
| **Leases** | Lease management, TTL tracking, renewal controls |

## Build from Source

### Prerequisites

- Rust 1.75+
- `cargo`
- macOS 11+ / Linux (X11 or Wayland) / Windows 10+

### Quick Build

```bash
cargo build --release -p fabric-gui
# Binary at: target/release/fabric-gui
```

## Bundle

### macOS (.app)

```bash
# 1. Generate the app icon
pip3 install Pillow
python3 crates/fabric-gui/bundle/macos/gen-icon.py

# 2. Create Fabric.icns
bash crates/fabric-gui/bundle/macos/gen-all-icons.sh

# 3. Build the .app bundle
bash crates/fabric-gui/bundle/macos/build-app.sh

# Output: bundle/macos/Fabric.app
open bundle/macos/Fabric.app
```

Or use the Makefile:

```bash
make bundle-macos
```

### Linux

```bash
make bundle-linux
# Staged in dist/linux/
```

Installs a `.desktop` file for XDG integration.

### Windows

```bash
make bundle-windows
# Staged in dist/windows/
```

Includes a DPI-aware manifest for proper HiDPI scaling.

## Install (any platform)

```bash
make install
# Installs to /usr/local/bin/fabric-gui
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl+1` | Switch to Dashboard |
| `Cmd/Ctrl+2` | Switch to Topology |
| `Cmd/Ctrl+3` | Switch to Routes |
| `Cmd/Ctrl+4` | Switch to Leases |
| `Cmd/Ctrl+R` | Refresh current panel |
| `Cmd/Ctrl+,` | Open settings |
| `Cmd/Ctrl+Q` | Quit |
| `F5` | Force reconnect to daemon |
| `F11` | Toggle fullscreen |
| `Esc` | Close dialog / deselect |

## Configuration

On first launch, Fabric GUI creates a config directory:

- **macOS:** `~/Library/Application Support/com.phenotype.fabric/`
- **Linux:** `~/.config/phenotype-fabric/`
- **Windows:** `%APPDATA%/phenotype-fabric/`

Configuration file: `config.json`

## Daemon Connection

The GUI expects the Fabric daemon listening on `localhost:9400`.
If the daemon is unavailable, it falls back to local SQLite storage.

## License

MIT OR Apache-2.0
