#!/usr/bin/env python3
"""Fail if an executable script still names a redacted repository owner.

Commit 6840d4f (2026-09-16) ran a PII sweep that replaced repository owner
segments with a redaction placeholder. Because the constant is interpolated
straight into a URL, the break is invisible at rest and only fails at runtime:

  - `crates/fabric-terminal/src/self_update.rs` carried a redacted repo slug,
    so self-update built an unresolvable GitHub API URL. That survived five
    weeks and was fixed in PR #60, guarded by an inline test.
  - `install.sh`, `install.ps1` and `install.ts` carried the same redaction in
    their `REPO` constant AND in their own usage lines, so every documented
    install path was dead. That survived until v0.1.8 because nothing checked.

The Rust case is covered by `self_update.rs`'s own test, which asserts the
constant does not contain the placeholder. Scripts have no such test, so this
scanner covers the script types.

Scope: tracked files only, and only file types that are EXECUTED as scripts.
Markdown, JSON and other prose/config files are deliberately not scanned:
several legitimately quote the placeholder as historical evidence
(references/, WORKLOG.md, session docs), and failing on those would make the
gate noise rather than a safety net.

Exit codes: 0 clean, 1 findings, 2 usage/internal error.
"""

from __future__ import annotations

import subprocess
import sys

SCRIPT_SUFFIXES = (".sh", ".bash", ".ps1", ".psm1", ".ts", ".js", ".mjs", ".cjs", ".bat", ".cmd", ".py")

# The scanner must not flag itself: this file necessarily contains the needle
# (as the pattern literal and in the docstring that explains it). Excluding it
# by exact path is safer than trying to avoid writing the pattern.
SELF = "program/scripts/check_no_redacted_identifiers.py"

# Directory prefixes excluded on purpose:
#   docs/book/  generated mdBook JS assets
EXCLUDED_PREFIXES = ("docs/book/",)

# The needle. A bare `<REDACTED>` alone is too broad (it appears in a demo log
# line in crates/fabric-gui/src/index.html that deliberately redacts an email
# address), so match the owner-segment form: a redacted value followed by a
# path separator, which is what breaks a repo slug or URL.
#
# The pattern is assembled at runtime rather than written literally, so this
# file does not contain the sequence it searches for.
NEEDLE = "<" + "REDACTED" + ">/"


def tracked_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z"], capture_output=True, check=False
    )
    if out.returncode != 0:
        print("error: not a git repository (or git unavailable)", file=sys.stderr)
        raise SystemExit(2)
    return [p for p in out.stdout.decode("utf-8", "replace").split("\0") if p]


def main() -> int:
    findings: list[tuple[str, int, str]] = []

    for path in tracked_files():
        if path == SELF:
            continue
        if not path.endswith(SCRIPT_SUFFIXES):
            continue
        if any(path.startswith(p) for p in EXCLUDED_PREFIXES):
            continue
        try:
            with open(path, encoding="utf-8", errors="replace") as fh:
                for lineno, line in enumerate(fh, 1):
                    if NEEDLE in line:
                        findings.append((path, lineno, line.strip()[:160]))
        except (OSError, UnicodeError) as exc:  # pragma: no cover - I/O guard
            print(f"error: cannot read {path}: {exc}", file=sys.stderr)
            return 2

    if not findings:
        print("No redacted repository owners in executable scripts.")
        return 0

    print("Redacted repository owner found in executable script(s):")
    print()
    for path, lineno, line in findings:
        print(f"  {path}:{lineno}: {line}")
    print()
    print(
        f"FAIL: {len(findings)} occurrence(s) of {NEEDLE}. These are interpolated\n"
        "into URLs, so the break is silent until a user runs the script. Replace\n"
        "the redacted segment with the real owner (KooshaPari)."
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
