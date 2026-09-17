# WOL (Wake-on-LAN) Setup

**Date:** 2026-09-17
**Status:** Complete

## Overview

Wake-on-LAN scripts for remotely powering on the Fabric machine from the network.

## Target Machine

| Property | Value |
|----------|-------|
| MAC | `00:81:2a:ee:d4:9b` |
| IP | `192.168.1.62` |
| Platform | macOS Apple Silicon (M4) |
| BIOS WOL | Enabled |
| pmset womp | 1 (enabled) |

## Scripts

### `~/bin/wake-fabric-machine.sh`

Sends a WOL magic packet to the Fabric machine.

**Usage:**
```bash
wake-fabric-machine.sh          # Send packet only
wake-fabric-machine.sh --wait   # Send packet and wait for machine to respond
```

**Implementation:**
- Primary: Uses `wakeonlan` (installed via `brew install wakeonlan`)
- Fallback: Python 3 socket to construct and broadcast the magic packet (6 bytes `0xFF` + 16 copies of MAC)
- Broadcasts to `192.168.1.255` (port 9)
- Checks `pmset -g` womp status before sending

### `~/bin/wake-monitor.sh`

Pings the Fabric machine at intervals until it responds online.

**Usage:**
```bash
wake-monitor.sh                          # Default: 30s interval, 300s timeout
wake-monitor.sh 10 120                   # 10s interval, 120s timeout
```

## Installation

`wakeonlan` was installed via Homebrew:
```bash
brew install wakeonlan
```

## Prerequisites

- WOL must be enabled in macOS: `sudo pmset -a womp 1` (already set)
- Both machines must be on the same LAN/subnet (`192.168.1.0/24`)
- Network switch/router must forward broadcast frames

## Typical Workflow

```bash
# 1. Send the magic packet
wake-fabric-machine.sh

# 2. Monitor until online
wake-monitor.sh 30 120

# 3. Or do both in one step
wake-fabric-machine.sh --wait
```

## Troubleshooting

- **Machine doesn't wake:** Verify BIOS WOL is enabled, check `pmset -g | grep womp`
- **Packet sent but no response:** Ensure both machines are on same subnet/VLAN
- **Python fallback used:** Install `wakeonlan` via `brew install wakeonlan`
- **Network issues:** Some managed switches block broadcast; check switch config
