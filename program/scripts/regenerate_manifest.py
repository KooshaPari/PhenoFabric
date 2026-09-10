#!/usr/bin/env python3
"""Regenerate MANIFEST.sha256 with SHA-256 for every file in the repo tree."""
import os
import json
import hashlib
import sys
from pathlib import Path

EXCLUDE_DIRS = {".git", ".archive", "node_modules", "__pycache__", "target"}
EXCLUDE_FILES = {"MANIFEST.sha256"}


def main() -> int:
    root = Path(".").resolve()
    manifest = {}

    # Use os.walk with early skip to avoid descending into excluded dirs
    for dirpath, dirnames, filenames in os.walk(root):
        rel_dir = os.path.relpath(dirpath, root)

        # Prune excluded dirs in-place so os.walk doesn't descend
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]

        if rel_dir == ".":
            cur_files = [f for f in filenames if f not in EXCLUDE_FILES]
        else:
            cur_files = filenames
        for fn in cur_files:
            full = os.path.join(dirpath, fn)
            # Skip symlinks (pointing in or out of repo) and non-regular files
            if os.path.islink(full):
                continue
            if not os.path.isfile(full):
                continue
            try:
                with open(full, "rb") as fh:
                    h = hashlib.sha256(fh.read()).hexdigest()
            except OSError as e:
                print(f"  ERROR reading {full}: {e}", file=sys.stderr)
                return 1
            rel = os.path.relpath(full, root)
            parts = rel.split(os.sep)
            if any(p in EXCLUDE_DIRS for p in parts):
                continue
            manifest[rel] = h

    # Atomic write via temp file
    tmp = root / ".MANIFEST.sha256.tmp"
    with open(tmp, "w") as fh:
        json.dump(manifest, fh, indent=2, sort_keys=True)
        fh.write("\n")
    os.replace(tmp, root / "MANIFEST.sha256")

    # Cross-check counts
    print(f"Generated manifest with {len(manifest)} entries.")

    # Compare to existing manifest if present
    manifest_path = root / "MANIFEST.sha256"
    # Note: at this point MANIFEST.sha256 has been replaced, so compare to backup
    return 0


if __name__ == "__main__":
    sys.exit(main())
