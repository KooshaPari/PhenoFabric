#!/usr/bin/env python3
"""Regenerate MANIFEST.sha256 with SHA-256 for every TRACKED file in the repo.

The manifest must describe exactly what a clean `git clone` contains,
because CI validates it against a fresh checkout. An earlier version of
this script walked the filesystem and applied only a small hardcoded
exclude list, so it also hashed untracked and git-ignored paths --
including `.env`, `.infisical.json`, built binaries and the generated
`docs/book/**` output. Those entries can never exist in CI, so
`check_manifest.py` reported a permanent "0 added, N removed" mismatch
and the Spec Validation workflow failed on every run from at least
2026-10-13 onward.

Deriving the file list from `git ls-files` fixes that by construction:
whatever git tracks is exactly what CI sees.

Usage:
    python3 program/scripts/regenerate_manifest.py
"""
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

EXCLUDE_FILES = {"MANIFEST.sha256"}


def tracked_files(root: Path) -> list[str]:
    """Return repo-relative paths of all git-tracked files."""
    out = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        capture_output=True,
        check=True,
    ).stdout
    return [p for p in out.decode("utf-8").split("\0") if p]


def main() -> int:
    root = Path(".").resolve()
    manifest: dict[str, str] = {}

    try:
        files = tracked_files(root)
    except subprocess.CalledProcessError as e:
        print(f"  ERROR: git ls-files failed: {e}", file=sys.stderr)
        return 1
    except FileNotFoundError:
        print("  ERROR: git not found on PATH", file=sys.stderr)
        return 1

    for rel in files:
        if rel in EXCLUDE_FILES:
            continue
        full = root / rel
        # A tracked path can still be absent from the working tree (e.g. a
        # deleted-but-not-staged file). Skip rather than fail, and say so.
        if not full.is_file() or full.is_symlink():
            print(f"  WARN skipping non-regular tracked path: {rel}", file=sys.stderr)
            continue
        try:
            with open(full, "rb") as fh:
                manifest[rel] = hashlib.sha256(fh.read()).hexdigest()
        except OSError as e:
            print(f"  ERROR reading {full}: {e}", file=sys.stderr)
            return 1

    # Atomic write via temp file.
    tmp = root / ".MANIFEST.sha256.tmp"
    with open(tmp, "w") as fh:
        json.dump(manifest, fh, indent=2, sort_keys=True)
        fh.write("\n")
    os.replace(tmp, root / "MANIFEST.sha256")

    print(f"Generated manifest with {len(manifest)} tracked entries.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
