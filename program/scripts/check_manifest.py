#!/usr/bin/env python3
"""
check_manifest.py

Validates that MANIFEST.sha256 contains an entry for every git-TRACKED
file in the repo, and that the recorded SHA-256 matches the file's
current content.

The file list comes from `git ls-files`, not a filesystem walk. CI runs
this against a clean checkout, where untracked and git-ignored paths
(such as `.env`, build outputs, and generated `docs/book/**`) simply do
not exist. Walking the filesystem here made the check disagree with CI:
locally it demanded manifest entries for ignored files that CI can never
have, and a filesystem-derived manifest could never match CI at all.

Exits 0 if the tracked tree matches the manifest, non-zero otherwise.
"""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

EXCLUDE_FILES = {"MANIFEST.sha256"}


def tracked_files(root: Path) -> set[str]:
    """Return repo-relative paths of all git-tracked files."""
    out = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        capture_output=True,
        check=True,
    ).stdout
    return {p for p in out.decode("utf-8").split("\0") if p}


def load_manifest(manifest_file: Path) -> dict[str, str]:
    """Parse MANIFEST.sha256 (JSON map, or sha256sum-style lines)."""
    content = manifest_file.read_text()
    try:
        data = json.loads(content)
    except json.JSONDecodeError:
        data = None

    if isinstance(data, dict):
        return {k: str(v) for k, v in data.items()}

    # sha256sum format: "<hash>  <filename>"
    entries: dict[str, str] = {}
    for line in content.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split(None, 1)
        if len(parts) == 2:
            entries[parts[1].lstrip("*").strip()] = parts[0]
    return entries


def main() -> int:
    root = Path(__file__).parent.parent.parent
    manifest_file = root / "MANIFEST.sha256"

    if not manifest_file.exists():
        print(f"WARNING: {manifest_file} does not exist; skipping manifest check")
        return 0

    try:
        manifest = load_manifest(manifest_file)
    except Exception as e:
        print(f"ERROR: failed to parse {manifest_file}: {e}")
        return 1

    try:
        actual_files = tracked_files(root) - EXCLUDE_FILES
    except Exception as e:
        print(f"ERROR: git ls-files failed: {e}")
        return 1

    manifest_files = set(manifest)

    missing_from_manifest = actual_files - manifest_files
    missing_from_tree = manifest_files - actual_files

    errors = False
    if missing_from_manifest:
        print("Tracked files not in MANIFEST:")
        for f in sorted(missing_from_manifest)[:50]:
            print(f"  {f}")
        if len(missing_from_manifest) > 50:
            print(f"  ... and {len(missing_from_manifest) - 50} more")
        errors = True

    if missing_from_tree:
        print("MANIFEST entries with no corresponding tracked file:")
        for f in sorted(missing_from_tree)[:50]:
            print(f"  {f}")
        if len(missing_from_tree) > 50:
            print(f"  ... and {len(missing_from_tree) - 50} more")
        errors = True

    if errors:
        print(
            f"\nManifest mismatch: {len(missing_from_manifest)} added, "
            f"{len(missing_from_tree)} removed"
        )
        return 1

    # Content check: every recorded digest must still match the file.
    mismatched = []
    for rel, digest in sorted(manifest.items()):
        path = root / rel
        if not path.is_file() or path.is_symlink():
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != digest:
            mismatched.append(rel)

    if mismatched:
        print("Files whose content does not match the manifest digest:")
        for f in mismatched[:50]:
            print(f"  {f}")
        if len(mismatched) > 50:
            print(f"  ... and {len(mismatched) - 50} more")
        print(f"\nContent mismatch: {len(mismatched)} files changed since manifest generation")
        return 1

    print(f"Manifest matches tracked tree ({len(actual_files)} files, all digests verified).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
