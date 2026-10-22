#!/usr/bin/env python3
"""Drift test: the release matrix and the required-asset list must agree.

The release workflow declares the platforms it builds in two places that
have no compile-time link between them:

  1. the ``build`` job's matrix ``include:`` list, which decides what is
     actually compiled and uploaded as ``cli-<platform>`` artifacts;
  2. the ``Assert required release assets are present`` step, which is
     the fail-closed gate that refuses to publish a release missing any
     of them.

If the two disagree, the failure mode is silent in one direction and
confusing in the other.

  * Matrix has a platform the gate does NOT require. The platform is
    built and uploaded, but if its upload silently produced nothing the
    release would still publish -- the gate never looked for it. This is
    exactly how v0.1.2 shipped: the archives existed but were missing
    the ``fabric`` binary, and the asset-existence gate was happy.

  * Gate requires a platform the matrix does NOT build. The release
    never publishes it, so the release fails at the gate -- which is the
    correct outcome, but it happens at TAG time, after the commit was
    merged, rather than at PR time.

Neither divergence is caught by the existing lint, which only checks
step-output wiring and download-artifact patterns. This test compares
the two lists directly.

It also asserts the platform MANDATE: Linux, macOS and Windows are all
mandatory, and the three CLI platforms that a user is most likely to be
on must each be present exactly once.

A fourth list is checked: the release BODY's platform table. The body
tells users which archives exist. If it omits a platform the release
actually publishes, the published documentation is wrong -- and if it
lists a platform the release does not publish, the body promises an
asset that is not there. Either way it is a false claim on the front
page of the release.

Every list is read FAIL-CLOSED. If a step this test looks for cannot be
found (because it was renamed or deleted), that is a failure, not a
silently-empty list that skips the comparison. A drift test that
quietly stops checking is worse than no drift test, because it reports
PASS.

Usage:
    python3 program/scripts/test_release_platforms.py

Exit codes:
    0 -- the lists agree and the mandatory platforms are covered
    1 -- the lists drifted, or a mandatory platform is missing
    2 -- the workflow could not be read or parsed
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - dependency guard
    sys.stderr.write("test_release_platforms.py requires PyYAML (pip install pyyaml)\n")
    raise SystemExit(2)

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "release.yml"

# The job whose matrix decides which CLI archives are built.
BUILD_JOB = "build"

# The step that refuses to publish a release missing an asset.
REQUIRED_ASSETS_STEP = "Assert required release assets are present"

# The post-publish step that re-downloads and checks each published CLI
# archive exists on the release page.
VERIFY_PRESENT_STEP_FRAGMENT = "CLI archives present"

# The post-publish step that checks each published archive actually
# contains the expected binaries (a present-but-empty archive passes
# the existence check).
VERIFY_CONTENTS_STEP_FRAGMENT = "expected binaries"

# The step that renders the release body.
RELEASE_BODY_STEP = "Create release body"

# CLI archive names look like:
#   phenotype-fabric-${RELEASE_TAG}-<platform>.tar.gz
#   phenotype-fabric-${RELEASE_TAG}-<platform>.zip
# The `${RELEASE_TAG}` placeholder is literal in the workflow, so the
# platform is the text between the last two dashes and the extension.
# The platform character class is deliberately permissive: a platform
# name that contained an upper-case letter would silently fall out of
# this match and read as "the gate names no archives", which is a
# confusing failure for a legitimate future rename.
CLI_ASSET_RE = re.compile(
    r"phenotype-fabric-\$\{RELEASE_TAG\}-(?P<platform>[A-Za-z0-9_-]+)"
    r"\.(?:tar\.gz|zip)"
)

# The release body renders archive names with the literal placeholder
# `TAG` rather than `${RELEASE_TAG}`, because it is substituted with
# `sed` after the heredoc is written.
CLI_BODY_ASSET_RE = re.compile(
    r"phenotype-fabric-TAG-(?P<platform>[A-Za-z0-9_-]+)\.(?:tar\.gz|zip)"
)

# The platforms that must exist. Linux, macOS and Windows are the
# mandatory operating systems; the CPU coverage under each is what
# makes the claim honest.
MANDATORY_PLATFORMS = {
    "linux-x86_64",
    "linux-aarch64",
    "macos-x86_64",
    "macos-aarch64",
    "windows-x86_64",
}

# Every operating system family that must be represented.
MANDATORY_FAMILIES = {"linux", "macos", "windows"}


def load_workflow() -> dict:
    """Parse the release workflow, or exit 2."""
    if not WORKFLOW.is_file():
        sys.stderr.write(f"test_release_platforms.py: not found: {WORKFLOW}\n")
        raise SystemExit(2)
    try:
        return yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        sys.stderr.write(f"test_release_platforms.py: invalid YAML: {exc}\n")
        raise SystemExit(2)


def matrix_platforms(workflow: dict) -> list[str]:
    """Platforms the build matrix compiles and uploads."""
    job = (workflow.get("jobs") or {}).get(BUILD_JOB) or {}
    include = (((job.get("strategy") or {}).get("matrix")) or {}).get("include") or []
    platforms = []
    for entry in include:
        if not isinstance(entry, dict):
            continue
        platform = entry.get("platform")
        if platform:
            platforms.append(platform)
    return platforms


def required_cli_platforms(workflow: dict) -> list[str]:
    """CLI platforms the required-asset gate insists on."""
    job = (workflow.get("jobs") or {}).get("github-release") or {}
    for step in job.get("steps") or []:
        if step.get("name") != REQUIRED_ASSETS_STEP:
            continue
        found = CLI_ASSET_RE.findall(step.get("run") or "")
        if found:
            return found
    return []


def verify_platform_platforms(workflow: dict) -> list[str]:
    """CLI platforms the post-publish verifier insists on."""
    job = (workflow.get("jobs") or {}).get("verify") or {}
    found: list[str] = []
    for step in job.get("steps") or []:
        run = step.get("run") or ""
        if VERIFY_PRESENT_STEP_FRAGMENT not in (step.get("name") or ""):
            continue
        for match in re.finditer(r"for platform in ([^\n;]+); do", run):
            found.extend(match.group(1).split())
    return found


def verify_content_platforms(workflow: dict) -> list[str]:
    """CLI platforms whose archive CONTENTS are checked post-publish.

    The existence check and the contents check are separate steps with
    separate platform lists, and they have already drifted once: the
    contents step checks only the four tar.gz platforms and handles
    Windows separately with a hardcoded archive name. A platform added
    to the matrix but forgotten here would publish an archive whose
    contents were never verified.
    """
    job = (workflow.get("jobs") or {}).get("verify") or {}
    found: list[str] = []
    for step in job.get("steps") or []:
        run = step.get("run") or ""
        if VERIFY_CONTENTS_STEP_FRAGMENT not in (step.get("name") or ""):
            continue
        for match in re.finditer(r"for platform in ([^\n;]+); do", run):
            found.extend(match.group(1).split())
    return found


def release_body_platforms(workflow: dict) -> list[str]:
    """CLI platforms the published release body advertises."""
    job = (workflow.get("jobs") or {}).get("github-release") or {}
    for step in job.get("steps") or []:
        if step.get("name") != RELEASE_BODY_STEP:
            continue
        return CLI_BODY_ASSET_RE.findall(step.get("run") or "")
    return []


def main() -> int:
    workflow = load_workflow()

    built = matrix_platforms(workflow)
    required = required_cli_platforms(workflow)
    verified = verify_platform_platforms(workflow)
    contents = verify_content_platforms(workflow)
    body = release_body_platforms(workflow)

    failures: list[str] = []

    # ── Every list must be READABLE ───────────────────────────────────
    # An empty list here means the step was renamed, deleted, or its
    # shape changed. That must be a FAILURE, not a skipped comparison:
    # a drift test that stops reading a list reports PASS while
    # checking nothing, which is the exact failure mode this whole
    # exercise exists to prevent.
    if not built:
        failures.append(
            f"the '{BUILD_JOB}' job matrix declares no platforms; the matrix "
            f"could not be read (expected a 'platform' key on each include entry)"
        )
    if not required:
        failures.append(
            f"step '{REQUIRED_ASSETS_STEP}' names no CLI archives; the "
            f"required-asset gate could not be read"
        )
    if not verified:
        failures.append(
            f"no post-publish step whose name contains "
            f"'{VERIFY_PRESENT_STEP_FRAGMENT}' could be read; the "
            f"post-publish archive-existence check could not be read"
        )
    if not contents:
        failures.append(
            f"no post-publish step whose name contains "
            f"'{VERIFY_CONTENTS_STEP_FRAGMENT}' could be read; the "
            f"post-publish archive-contents check could not be read"
        )
    if not body:
        failures.append(
            f"step '{RELEASE_BODY_STEP}' advertises no CLI archives; the "
            f"release body platform table could not be read"
        )

    built_set = set(built)
    required_set = set(required)

    # ── The lists must describe the same set ──────────────────────────
    def compare(label: str, left: list[str], right: list[str], left_name: str,
                right_name: str, left_consequence: str, right_consequence: str) -> None:
        """Report platforms present in one list but not the other."""
        left_set, right_set = set(left), set(right)
        only_left = sorted(left_set - right_set)
        only_right = sorted(right_set - left_set)
        if only_left:
            failures.append(
                f"{label}: {left_name} has platform(s) {right_name} does not: "
                f"{', '.join(only_left)}. {left_consequence}"
            )
        if only_right:
            failures.append(
                f"{label}: {right_name} has platform(s) {left_name} does not: "
                f"{', '.join(only_right)}. {right_consequence}"
            )

    compare(
        "build matrix vs required-asset gate",
        built, required,
        "the build matrix", "the required-asset gate",
        "If such an upload silently produced nothing, the release would "
        "still publish without it.",
        "The release would fail at tag time instead of at PR time.",
    )

    if verified:
        compare(
            "build matrix vs post-publish existence check",
            built, verified,
            "the build matrix", "the post-publish verifier",
            "An asset could be absent from the release page and still pass.",
            "The verifier would check for an archive the release never "
            "publishes, and fail on a healthy release.",
        )

    if contents:
        compare(
            "build matrix vs post-publish contents check",
            built, contents,
            "the build matrix", "the post-publish contents checker",
            "Its archive contents are never verified after publication, so "
            "an empty or truncated archive would pass.",
            "The checker inspects an archive the release never publishes.",
        )

    if body:
        compare(
            "build matrix vs release body table",
            built, body,
            "the build matrix", "the published release body",
            "The published release body does not mention it, so the "
            "release's own documentation is wrong about what ships.",
            "The published release body advertises an archive the "
            "release does not contain.",
        )

    # ── No duplicates ─────────────────────────────────────────────────
    for label, platforms in (
        ("build matrix", built),
        ("required-asset gate", required),
        ("post-publish existence check", verified),
        ("post-publish contents check", contents),
        ("release body table", body),
    ):
        dupes = sorted({p for p in platforms if platforms.count(p) > 1})
        if dupes:
            failures.append(f"the {label} lists platform(s) twice: {', '.join(dupes)}")

    # ── The mandatory platform mandate ────────────────────────────────
    missing = sorted(MANDATORY_PLATFORMS - built_set)
    if missing:
        failures.append(
            f"mandatory platform(s) absent from the build matrix: {', '.join(missing)}"
        )

    families = {p.split("-", 1)[0] for p in built_set}
    missing_families = sorted(MANDATORY_FAMILIES - families)
    if missing_families:
        failures.append(
            f"mandatory operating system(s) absent from the build matrix: "
            f"{', '.join(missing_families)}"
        )

    # ── The post-publish verifier must cover the same platforms ───────
    if verified:
        verified_set = set(verified)
        unverified = sorted(built_set - verified_set)
        if unverified:
            failures.append(
                "the post-publish verifier does not check platform(s) the "
                f"build matrix publishes: {', '.join(unverified)}. An asset "
                "could be absent from the release page and still pass."
            )

    # ── Report what was read ──────────────────────────────────────────
    for title, platforms in (
        ("build matrix platforms", built),
        ("required-asset gate platforms", required),
        ("post-publish existence-check platforms", verified),
        ("post-publish contents-check platforms", contents),
        ("release body table platforms", body),
    ):
        print(f"{title}:")
        for platform in platforms:
            print(f"  {platform}")

    if failures:
        print("\nFAIL: release platform lists drifted:")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    print("\nPASS: the build matrix, the required-asset gate, the post-publish "
          "existence check, the post-publish contents check, and the release "
          "body table all list the same platforms, and every mandatory "
          "platform is built.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
