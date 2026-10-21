#!/usr/bin/env python3
"""Static lint for GitHub Actions workflow files.

Catches two failure modes that both silently produced broken releases:

1. A step that writes one output name while a consumer reads another
   (e.g. the step writes ``version=...`` but the consumer reads
   ``steps.version.outputs.tag``). The consumer then expands to the empty
   string and the step silently degrades into a no-op.

2. A multi-line ``pattern:`` block scalar passed to ``download-artifact``,
   whose ``pattern`` input is a SINGLE glob, not a list. The literal
   two-line string matches nothing, so the download returns 0 artifacts
   and the release publishes with no assets.

Both are checked statically, with no network access, so they can run in
CI as a fast pre-flight gate.

Usage:
    python3 program/scripts/check_workflows.py [workflow_dir]

Exit codes:
    0 -- all workflows pass
    1 -- at least one workflow has a lint error
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - dependency guard
    sys.stderr.write("check_workflows.py requires PyYAML (pip install pyyaml)\n")
    raise SystemExit(2)

OUTPUT_ECHO_RE = re.compile(r'echo\s+"?([A-Za-z0-9_]+)=')
OUTPUT_REF_RE = re.compile(r"steps\.(\w+)\.outputs\.(\w+)")
MULTILINE_PATTERN_RE = re.compile(
    r"^\s*pattern:\s*[|>][-+]?\s*$",
    re.MULTILINE,
)


def check_step_outputs(job_name: str, job: dict) -> list[str]:
    """Every ``steps.<id>.outputs.<name>`` must name an output the step declares."""
    problems: list[str] = []
    steps = job.get("steps") or []

    declared: dict[str, set[str]] = {}
    for step in steps:
        step_id = step.get("id")
        if not step_id:
            continue
        run = step.get("run") or ""
        declared[step_id] = set(OUTPUT_ECHO_RE.findall(run))

    # Re-serialise so we see every consumer, including ones in `with:`,
    # `env:`, and nested job-level expressions.
    serialised = yaml.dump(job)
    for step_id, output_name in OUTPUT_REF_RE.findall(serialised):
        if step_id not in declared:
            problems.append(
                f"{job_name}: references steps.{step_id}.outputs.{output_name} "
                f"but no step has id '{step_id}'"
            )
        elif output_name not in declared[step_id]:
            have = ", ".join(sorted(declared[step_id])) or "(none)"
            problems.append(
                f"{job_name}: reads steps.{step_id}.outputs.{output_name} "
                f"but that step only declares: {have}"
            )
    return problems


def check_download_patterns(job_name: str, job: dict) -> list[str]:
    """``download-artifact``'s ``pattern`` input is a single glob, not a list."""
    problems: list[str] = []
    for step in job.get("steps") or []:
        uses = step.get("uses") or ""
        if "download-artifact" not in uses:
            continue
        with_block = step.get("with") or {}
        pattern = with_block.get("pattern")
        if isinstance(pattern, str) and "\n" in pattern.strip():
            problems.append(
                f"{job_name}: step '{step.get('name') or uses}' passes a "
                f"multi-line pattern to download-artifact; 'pattern' is a "
                f"SINGLE glob and a multi-line value matches nothing "
                f"(got {pattern!r})"
            )
    return problems


def check_workflow(path: Path) -> list[str]:
    """Run every check against one workflow file."""
    try:
        workflow = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        return [f"{path.name}: invalid YAML: {exc}"]
    if not isinstance(workflow, dict):
        return [f"{path.name}: workflow is not a mapping"]

    problems: list[str] = []
    for job_name, job in (workflow.get("jobs") or {}).items():
        if not isinstance(job, dict):
            continue
        problems.extend(check_step_outputs(job_name, job))
        problems.extend(check_download_patterns(job_name, job))

    # Raw-text check for a multi-line pattern, which YAML parsing alone
    # cannot distinguish from a legitimate single-line value with a
    # trailing newline.
    text = path.read_text(encoding="utf-8")
    if MULTILINE_PATTERN_RE.search(text):
        for match in re.finditer(MULTILINE_PATTERN_RE.pattern, text):
            problems.append(
                f"{path.name}: multi-line 'pattern:' block scalar at line "
                f"{text[: match.start()].count(chr(10)) + 1}; download-artifact "
                f"expects a single glob"
            )
    return problems


def main(argv: list[str]) -> int:
    if len(argv) > 1:
        root = Path(argv[1])
    else:
        root = Path(__file__).resolve().parents[2] / ".github" / "workflows"

    if root.is_file():
        workflows = [root]
    elif root.is_dir():
        workflows = sorted(
            p for p in root.iterdir() if p.suffix in (".yml", ".yaml")
        )
    else:
        sys.stderr.write(f"check_workflows.py: no such path: {root}\n")
        return 2

    if not workflows:
        sys.stderr.write(f"check_workflows.py: no workflow files under {root}\n")
        return 2

    failures = 0
    for workflow_path in workflows:
        problems = check_workflow(workflow_path)
        if problems:
            failures += len(problems)
            print(f"FAIL {workflow_path.name}")
            for problem in problems:
                print(f"  - {problem}")
        else:
            print(f"OK   {workflow_path.name}")

    if failures:
        print(f"\n{failures} workflow lint problem(s) found.")
        return 1
    print(f"\nAll {len(workflows)} workflow file(s) passed lint.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
