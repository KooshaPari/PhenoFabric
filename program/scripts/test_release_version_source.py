#!/usr/bin/env python3
"""Regression test: the release workflow must never derive the version
from the git ref.

Every release job used to derive the release version from the git ref
in one of two ways::

    tag=${GITHUB_REF#refs/tags/}     # in a `run:` block
    ${{ github.ref_name }}           # in a step `env:` block

Both are correct ONLY for a tag push. On a ``workflow_dispatch`` run the
ref is a BRANCH, so:

  * ``${GITHUB_REF#refs/tags/}`` yields the literal string
    ``refs/heads/main`` -- which becomes the artifact filename prefix,
    the Docker image tag, and the name of the release that
    ``gh release download`` looks for.

  * ``github.ref_name`` yields ``main`` -- which becomes the filename
    the required-asset gate looks for.

Neither is a version, and both produce a release that is named after a
branch instead of a version.

This has happened twice, in two different forms, and both times the
release was REFUSED by a fail-closed gate even though every artifact
had been built correctly:

  * v0.1.7 republish, run 40149865321: ``cargo-deny`` blocked it for an
    unrelated new advisory (RUSTSEC-2026-0387).
  * v0.1.7 republish, run 40180377333: all five CLI targets, all four
    GUI targets, the Docker image and the multi-arch manifest push
    SUCCEEDED, and then the required-asset gate refused to publish
    because it looked for ``phenotype-fabric-main-linux-x86_64.tar.gz``
    while the build had produced
    ``phenotype-fabric-v0.1.7-linux-x86_64.tar.gz``.

The fix is a shared resolver (``scripts/release/resolve_version.sh``)
that every job sources, so there is exactly one expression for the
version in the whole workflow. This test enforces that.

It FAILS CLOSED. If the workflow stops containing a version-resolution
step at all, this test fails rather than reporting "no problems
found" -- a drift test that quietly stops checking is worse than no
drift test, because it reports PASS.

Usage:
    python3 program/scripts/test_release_version_source.py

Exit codes:
    0 -- every check passed
    1 -- at least one check failed
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "release.yml"
RESOLVER = REPO_ROOT / "scripts" / "release" / "resolve_version.sh"

# The resolver every job is supposed to source.
RESOLVER_SOURCE = "source scripts/release/resolve_version.sh"

# Ways the workflow can leak a BRANCH name into a release version.
#
# `github.ref_name` is the branch name on a dispatch and the tag name on
# a tag push, so it is only safe if it is gated on the ref being a tag.
# The workflow has no such gate, so any use of it is a defect.
FORBIDDEN_EXPRESSIONS = (
    "${{ github.ref_name }}",
    "${{ github.head_ref }}",
    "${{ github.ref }}",
    "${GITHUB_REF_NAME}",
    "${GITHUB_HEAD_REF}",
    "$GITHUB_REF_NAME",
    "$GITHUB_HEAD_REF",
    "${GITHUB_REF#refs/tags/",
    "${GITHUB_REF#refs/heads/",
    "$GITHUB_REF#refs/tags/",
)

# `GITHUB_REF` used to derive a version. It may still appear in prose
# (a comment explaining the hazard) but never in a `run:` or `env:`
# value that produces a version.
GITHUB_REF_RE = re.compile(r"\$\{?GITHUB_REF\b")


def fail(message: str) -> None:
    print(f"  FAIL  {message}")


def ok(message: str) -> None:
    print(f"  ok    {message}")


def strip_comments(text: str) -> str:
    """Remove YAML comments, leaving the code.

    A comment that mentions the forbidden expression is documentation of
    the hazard, not a defect. Only executable lines are checked.
    """
    out = []
    for line in text.splitlines():
        # A '#' inside a quoted string is not a comment, but this file
        # has no such case in the lines that matter. The conservative
        # approach is fine here: stripping a trailing comment can only
        # ever remove text, so it can hide a defect in a quoted string,
        # never invent one.
        stripped = line.split("#", 1)[0] if "#" in line else line
        out.append(stripped)
    return "\n".join(out)


def main() -> int:
    if not WORKFLOW.exists():
        print(f"FATAL: workflow not found: {WORKFLOW}", file=sys.stderr)
        return 1
    if not RESOLVER.exists():
        print(f"FATAL: resolver not found: {RESOLVER}", file=sys.stderr)
        return 1

    raw = WORKFLOW.read_text()
    code = strip_comments(raw)

    failures = 0

    # ── check 1: the resolver is present and used ────────────────────
    uses = code.count(RESOLVER_SOURCE)
    if uses == 0:
        fail(
            "no job sources the version resolver "
            f"({RESOLVER_SOURCE!r}); the workflow must resolve the "
            "version in exactly one place"
        )
        failures += 1
    else:
        ok(f"{uses} job(s) source the version resolver")

    # ── check 2: the resolver emits the outputs the workflow reads ───
    # Each variable must be ASSIGNED in the resolver and EXPORTED, so a
    # caller that sources it gets the value without extra plumbing.
    resolver_src = RESOLVER.read_text()
    for var in ("RELEASE_TAG", "RELEASE_VERSION", "RELEASE_DOCKER_TAG"):
        assigned = re.search(rf"^{var}=", resolver_src, re.MULTILINE) is not None
        exported = re.search(
            rf"^export\s+[A-Z_ ]*\b{var}\b", resolver_src, re.MULTILINE
        ) is not None
        if not assigned:
            fail(f"resolver does not assign {var}")
            failures += 1
        elif not exported:
            fail(f"resolver does not export {var}")
            failures += 1
        else:
            ok(f"resolver assigns and exports {var}")

    # ── check 3: no branch-derived version anywhere ──────────────────
    for expr in FORBIDDEN_EXPRESSIONS:
        if expr in code:
            line_no = next(
                (i for i, l in enumerate(code.splitlines(), 1) if expr in l), 0
            )
            fail(
                f"line ~{line_no}: {expr!r} derives a version from the git "
                "ref. On a workflow_dispatch run the ref is a BRANCH, so "
                "this yields a branch name instead of a version. Use "
                "steps.version.outputs.tag (the resolved tag) instead."
            )
            failures += 1
        else:
            ok(f"no use of {expr!r}")

    # ── check 4: GITHUB_REF is not used to derive anything executable ─
    for i, line in enumerate(code.splitlines(), 1):
        if GITHUB_REF_RE.search(line):
            fail(
                f"line ~{i}: GITHUB_REF is used in executable code: "
                f"{line.strip()!r}. It must not produce a version."
            )
            failures += 1

    # ── check 5: every consumer reads the resolved outputs ───────────
    # A job that reads `steps.version.outputs.tag` but has no
    # version-resolution step of its own would read an EMPTY string.
    for job_block in split_jobs(raw):
        job_name = job_block[0]
        body = job_block[1]
        body_code = strip_comments(body)
        if "steps.version.outputs.tag" in body_code or (
            "steps.version.outputs.version" in body_code
        ):
            if RESOLVER_SOURCE not in body_code:
                fail(
                    f"job {job_name!r} reads a version output but does not "
                    f"source the resolver ({RESOLVER_SOURCE!r}); it would "
                    "read an empty string"
                )
                failures += 1
            else:
                ok(f"job {job_name!r} resolves its own version")

    print()
    print("-" * 45)
    if failures == 0:
        print("PASS: the release workflow derives the version in one place")
        print("      and never from the git ref.")
        return 0
    print(f"FAIL: {failures} version-derivation check(s) failed.")
    return 1


def split_jobs(text: str) -> list[tuple[str, str]]:
    """Split the workflow into (job_name, job_body) pairs.

    Job keys are two-space-indented `name:` lines under `jobs:`.
    """
    jobs: list[tuple[str, str]] = []
    lines = text.splitlines()
    in_jobs = False
    current_name: str | None = None
    current: list[str] = []

    for line in lines:
        if line.rstrip() == "jobs:":
            in_jobs = True
            continue
        if not in_jobs:
            continue
        if not line.startswith("  ") or line.startswith("   "):
            if current_name is not None:
                jobs.append((current_name, "\n".join(current)))
                current_name = None
                current = []
            continue
        m = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if m:
            if current_name is not None:
                jobs.append((current_name, "\n".join(current)))
            current_name = m.group(1)
            current = []
            continue
        if current_name is not None:
            current.append(line)

    if current_name is not None:
        jobs.append((current_name, "\n".join(current)))
    return jobs


if __name__ == "__main__":
    sys.exit(main())
