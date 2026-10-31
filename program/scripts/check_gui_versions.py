#!/usr/bin/env python3
"""Verify every GUI version literal matches the workspace version.

Exits 0 if all GUI version literals agree with [workspace.package].version
in the root Cargo.toml, 1 otherwise.
"""
from __future__ import annotations
import re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).parent.parent.parent

def workspace_version() -> str:
    text = (ROOT / "Cargo.toml").read_text()
    in_wp = False
    for line in text.splitlines():
        if line.strip() == "[workspace.package]":
            in_wp = True; continue
        if in_wp and line.startswith("["):
            in_wp = False
        if in_wp:
            m = re.match(r'\s*version\s*=\s*"([^"]+)"', line)
            if m:
                return m.group(1)
    raise SystemExit("workspace version not found")

WS = workspace_version()

errors = []

def check_file(path, pattern, pred, optional=False):
    full = ROOT / path
    if not full.exists():
        if optional:
            return
        errors.append(f"{path}: file not found")
        return
    text = full.read_text()
    matches = list(re.finditer(pattern, text, re.M))
    if not matches:
        errors.append(f"{path}: pattern not found ({pattern})")
        return
    for m in matches:
        val = m.group(1)
        if not pred(val):
            errors.append(f"{path}: found {val!r}, expected match")
            break

# Tracked source files
check_file("crates/fabric-gui/Cargo.toml",
           r'^version\s*=\s*"([^"]+)"', lambda v: v == WS)
check_file("crates/fabric-gui/package.json",
           r'"version":\s*"([^"]+)"', lambda v: v == WS)
check_file("crates/fabric-gui/bundle/windows/fabric-gui.manifest",
           r'assemblyIdentity\s*\n\s*version="([^"]+)"', lambda v: v == WS + ".0")
check_file("crates/fabric-gui/src-tauri/Cargo.toml",
           r'^version\s*=\s*"([^"]+)"', lambda v: v == WS)
check_file("crates/fabric-gui/src-tauri/tauri.conf.json",
           r'"version":\s*"([^"]+)"', lambda v: v == WS)

html = (ROOT / "crates/fabric-gui/src/index.html").read_text()
about = re.search(r'Version</span>\s*\n\s*<span[^>]*>([^<]+)</span>', html)
sidebar = re.search(r'id="sb-version">([^<]+)</span>', html)
if not about:
    errors.append("index.html about version span not found")
elif about.group(1).strip() != WS:
    errors.append(f"index.html about: {about.group(1).strip()!r} != {WS!r}")
if not sidebar:
    errors.append("index.html sidebar version span not found")
elif sidebar.group(1).strip().lstrip('v') != WS:
    errors.append(f"index.html sidebar: {sidebar.group(1).strip()!r} != v{WS!r}")

# Generated macOS bundle artifact (may be absent in CI checkout)
check_file("crates/fabric-gui/bundle/macos/Fabric.app/Contents/Info.plist",
           r'CFBundleShortVersionString</key>\s*\n\s*<string>([^<]+)</string>',
           lambda v: v == WS, optional=True)
check_file("crates/fabric-gui/bundle/macos/Fabric.app/Contents/Info.plist",
           r'CFBundleVersion</key>\s*\n\s*<string>([^<]+)</string>',
           lambda v: v == WS, optional=True)

# build-app.sh must derive VERSION from workspace Cargo.toml, never hardcode
sh = ROOT / "crates/fabric-gui/bundle/macos/build-app.sh"
script_text = sh.read_text()
if 'VERSION="0.1.0"' in script_text:
    errors.append('build-app.sh still hardcodes VERSION="0.1.0"')
out = subprocess.run(["awk", """
/^[[]workspace[.]package[]]/ { inwp=1; next }
/^[[]/ { inwp=0 }
inwp && /^version[[:space:]]*=/ { gsub(/.*=[[:space:]]*"/, ""); gsub(/".*/, ""); print; exit }
""", str(ROOT/"Cargo.toml")], capture_output=True, text=True)
if out.stdout.strip() != WS:
    errors.append(f"build-app.sh extraction yields {out.stdout.strip()!r}, expected {WS!r}")

if errors:
    print("GUI version drift:")
    for e in errors:
        print("  - " + e)
    sys.exit(1)
print(f"All GUI version literals match workspace version {WS}.")
