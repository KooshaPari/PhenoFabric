#!/usr/bin/env python3
"""
check_manifest.py

Validates that MANIFEST.sha256 contains an entry for every file in
the repo (excluding .git, .archive, MANIFEST.sha256 itself, and
generated files).

Exits 0 if the file tree matches the manifest, non-zero otherwise.
"""
import json
import os
import sys
from pathlib import Path

def main():
    root = Path(__file__).parent.parent.parent
    manifest_file = root / "MANIFEST.sha256"

    if not manifest_file.exists():
        print(f"WARNING: {manifest_file} does not exist; skipping manifest check")
        return 0

    # Load manifest
    try:
        with open(manifest_file) as f:
            content = f.read()
        # Try parsing as JSON first, then as plain text (sha256sum -f format)
        try:
            manifest_entries = json.loads(content)
            manifest_files = set(manifest_entries.keys()) if isinstance(manifest_entries, dict) else set()
        except json.JSONDecodeError:
            # Parse sha256sum format: "<hash>  <filename>" or "<hash>  *<filename>"
            manifest_files = set()
            for line in content.splitlines():
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                parts = line.split(None, 1)
                if len(parts) == 2:
                    filename = parts[1].lstrip("*").strip()
                    if filename:
                        manifest_files.add(filename)
    except Exception as e:
        print(f"ERROR: failed to parse {manifest_file}: {e}")
        return 1

    # Build actual file list
    actual_files = set()
    exclude_patterns = [".git/", ".archive/", "node_modules/", "__pycache__/"]
    exclude_files = {"MANIFEST.sha256"}

    for f in sorted(root.rglob("*")):
        if not f.is_file():
            continue
        rel = f.relative_to(root)
        rel_str = str(rel)
        if any(p in str(f) for p in exclude_patterns):
            continue
        if rel_str in exclude_files:
            continue
        actual_files.add(rel_str)

    # Compare
    missing_from_manifest = actual_files - manifest_files
    missing_from_tree = manifest_files - actual_files

    errors = False
    if missing_from_manifest:
        print("Files in tree but not in MANIFEST:")
        for f in sorted(missing_from_manifest)[:50]:
            print(f"  {f}")
        if len(missing_from_manifest) > 50:
            print(f"  ... and {len(missing_from_manifest) - 50} more")
        errors = True

    if missing_from_tree:
        print("Files in MANIFEST but not in tree:")
        for f in sorted(missing_from_tree)[:50]:
            print(f"  {f}")
        if len(missing_from_tree) > 50:
            print(f"  ... and {len(missing_from_tree) - 50} more")
        errors = True

    if errors:
        print(f"\nManifest mismatch: {len(missing_from_manifest)} added, {len(missing_from_tree)} removed")
        return 1

    print(f"Manifest matches tree ({len(actual_files)} files).")
    return 0

if __name__ == "__main__":
    sys.exit(main())
