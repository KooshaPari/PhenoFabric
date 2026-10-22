#!/usr/bin/env python3
"""Behavioural regression test for the release checksum gates.

The static lint in ``check_workflows.py`` cannot catch a checksum gate
that exits 0 on broken input. This test does, by extracting the real
``run:`` blocks out of ``.github/workflows/release.yml`` and executing
them against deliberately broken fixtures.

Three holes were found this way and are the reason this file exists:

F1. An ORPHAN SIDECAR -- a ``.sha256`` file whose subject was never
    published -- passed both gates with exit 0. The reverse-coverage
    loops enumerate only the seven archive/installer extensions, so a
    ``.sha256`` was never in their find set, and the manifest walks
    never see a sidecar either. This matters in production because the
    post-publish download glob is ``phenotype-fabric-*.sha256``: a stale
    sidecar for a removed installer gets published, and a user running
    ``sha256sum -c`` on it is pointed at a file that does not exist.

F2. A MALFORMED MANIFEST LINE -- a single-field line such as
    ``deadbeef`` -- was silently skipped by the old
    ``[ -n "$name" ] || continue`` guard. The gate exited 0 and the
    success message counted the malformed line as an archive.

F3. DUPLICATE MANIFEST LINES were accepted and inflated the artifact
    count in the success message.

The gates are extracted verbatim from the workflow, so this test cannot
drift from what actually runs in CI. If the workflow changes, the
extraction changes with it and the assertions below are re-evaluated
against the new text.

Usage:
    python3 program/scripts/test_release_checksums.py

Exit codes:
    0 -- every case behaved as required
    1 -- at least one case regressed
    2 -- the workflow could not be read or parsed
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - dependency guard
    sys.stderr.write("test_release_checksums.py requires PyYAML (pip install pyyaml)\n")
    raise SystemExit(2)

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "release.yml"

# The two gates that decide whether a release may ship. Both must
# fail closed on every broken fixture.
RELEASE_GATE = "Verify every archive has a matching checksum"
PUBLISHED_GATE = "Verify published checksums match the downloaded assets"

# The extensions the workflow treats as downloadable artifacts. The
# fixtures must use exactly these, because the gates' reverse-coverage
# `find` expression enumerates them.


def extract_gates() -> dict[str, str]:
    """Pull the two checksum gates out of the workflow, verbatim."""
    if not WORKFLOW.is_file():
        sys.stderr.write(f"workflow not found: {WORKFLOW}\n")
        raise SystemExit(2)
    workflow = yaml.safe_load(WORKFLOW.read_text())
    gates: dict[str, str] = {}
    for job in (workflow.get("jobs") or {}).values():
        for step in job.get("steps") or []:
            name = step.get("name")
            if name in (RELEASE_GATE, PUBLISHED_GATE):
                script = step.get("run") or ""
                if not script.strip():
                    sys.stderr.write(f"gate has no run block: {name}\n")
                    raise SystemExit(2)
                gates[name] = script
    missing = {RELEASE_GATE, PUBLISHED_GATE} - set(gates)
    if missing:
        sys.stderr.write(f"gate(s) not found in workflow: {sorted(missing)}\n")
        raise SystemExit(2)
    return gates


def sha256(path: Path) -> str:
    """Return the hex SHA-256 of a file, using the system tool."""
    out = subprocess.run(
        ["sha256sum", str(path)], capture_output=True, text=True, check=True
    )
    return out.stdout.split()[0]


def gate_workdir(script: str) -> str:
    """Return the directory a gate script cd's into.

    The two gates work in different directories -- the release job
    verifies ``artifacts/`` in the runner workspace, the post-publish
    job verifies ``release-assets/`` downloaded from the release page.
    The directory is read out of the script itself rather than
    hardcoded, so the test keeps working if either is renamed.
    """
    for line in script.splitlines():
        stripped = line.strip()
        if stripped.startswith("cd "):
            return stripped[3:].strip()
    raise AssertionError("gate script does not cd into a directory")


def build_fixture(root: Path, artifacts: list[str], workdir: str) -> None:
    """Create artifacts, their sidecars, and a combined SHA256SUMS.

    The manifest is written the way ``sha256sum`` writes it, so the
    fixtures match the shape the workflow's own generate step produces.
    That generate step runs ``find .``, so the recorded names carry the
    ``./`` prefix and the gates' reverse-coverage comparison sees the
    same shape on both sides.
    """
    artifacts_dir = root / workdir
    artifacts_dir.mkdir(parents=True, exist_ok=True)
    manifest_lines: list[str] = []
    for name in artifacts:
        target = artifacts_dir / name
        target.write_text(f"fabricated bytes for {name}\n")
        digest = sha256(target)
        (artifacts_dir / f"{name}.sha256").write_text(f"{digest}  {name}\n")
        manifest_lines.append(f"{digest}  ./{name}")
    (artifacts_dir / "SHA256SUMS").write_text("\n".join(manifest_lines) + "\n")


def run_gate(script: str, root: Path) -> tuple[int, str]:
    """Execute a gate script the way GitHub would.

    GitHub runs ``shell: bash`` steps as ``bash -eo pipefail`` and
    steps with no explicit shell as ``bash -e``. The gate scripts
    ``cd`` into their own directory, so they are run from the parent
    of that directory.
    """
    script_path = root / "gate.sh"
    script_path.write_text(script)
    proc = subprocess.run(
        ["bash", "-eo", "pipefail", str(script_path)],
        cwd=root,
        capture_output=True,
        text=True,
        stdin=subprocess.DEVNULL,
    )
    return proc.returncode, proc.stdout + proc.stderr


def artifact_set() -> list[str]:
    """Two artifacts with different extensions, covering a CLI archive
    and a GUI installer."""
    return [
        "phenotype-fabric-v0.1.7-linux-x86_64.tar.gz",
        "phenotype-fabric-gui-v0.1.7-windows-x64.msi",
    ]


def case_positive(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """A well-formed release passes and reports the true count."""
    build_fixture(root, artifact_set(), workdir)
    rc, output = run_gate(script, root)
    expected = f"All {len(artifact_set())}"
    return rc, f"rc={rc} (want 0) count_msg_ok={expected in output}"


def case_orphan_sidecar(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """F1: a sidecar with no published subject must fail."""
    build_fixture(root, artifact_set(), workdir)
    orphan = root / workdir / "phenotype-fabric-gui-windows-x64.msi.sha256"
    orphan.write_text(f"{'0' * 64}  phenotype-fabric-gui-windows-x64.msi\n")
    rc, output = run_gate(script, root)
    detail = "no matching file" in output.lower()
    return rc, f"rc={rc} (want 1) orphan_reported={detail}"


def case_malformed_manifest_line(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """F2: a single-field manifest line must fail."""
    build_fixture(root, artifact_set(), workdir)
    manifest = root / workdir / "SHA256SUMS"
    manifest.write_text(manifest.read_text() + "deadbeef\n")
    rc, output = run_gate(script, root)
    detail = "malformed" in output.lower()
    return rc, f"rc={rc} (want 1) malformed_reported={detail}"


def case_bad_digest_format(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """A non-hex digest must fail rather than be compared as a string."""
    build_fixture(root, artifact_set(), workdir)
    manifest = root / workdir / "SHA256SUMS"
    lines = manifest.read_text().splitlines()
    lines[0] = "not-a-hex-digest  ./" + artifact_set()[0]
    manifest.write_text("\n".join(lines) + "\n")
    rc, output = run_gate(script, root)
    detail = "malformed" in output.lower() or "mismatch" in output.lower()
    return rc, f"rc={rc} (want 1) bad_digest_reported={detail}"


def case_duplicate_manifest_lines(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """F3: a duplicated manifest entry must fail."""
    build_fixture(root, artifact_set(), workdir)
    manifest = root / workdir / "SHA256SUMS"
    manifest.write_text(manifest.read_text() + manifest.read_text())
    rc, output = run_gate(script, root)
    detail = "more than once" in output.lower() or "duplicate" in output.lower()
    return rc, f"rc={rc} (want 1) duplicate_reported={detail}"


def case_corrupt_artifact(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """A corrupt artifact must fail."""
    build_fixture(root, artifact_set(), workdir)
    target = root / workdir / artifact_set()[0]
    target.write_text("tampered bytes\n")
    rc, output = run_gate(script, root)
    detail = "mismatch" in output.lower()
    return rc, f"rc={rc} (want 1) mismatch_reported={detail}"


def case_artifact_without_sidecar(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """An artifact with no sidecar must fail."""
    build_fixture(root, artifact_set(), workdir)
    (root / workdir / f"{artifact_set()[0]}.sha256").unlink()
    rc, output = run_gate(script, root)
    detail = "missing" in output.lower()
    return rc, f"rc={rc} (want 1) missing_reported={detail}"


def case_empty_manifest(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """An empty manifest must fail."""
    build_fixture(root, artifact_set(), workdir)
    (root / workdir / "SHA256SUMS").write_text("")
    rc, output = run_gate(script, root)
    return rc, f"rc={rc} (want 1)"


def case_artifact_missing_from_manifest(root: Path, script: str, workdir: str) -> tuple[int, str]:
    """An artifact published without a manifest entry must fail."""
    build_fixture(root, artifact_set(), workdir)
    extra = root / workdir / "phenotype-fabric-gui-linux-x64.deb"
    extra.write_text("installer bytes\n")
    rc, output = run_gate(script, root)
    detail = "not covered" in output.lower()
    return rc, f"rc={rc} (want 1) uncovered_reported={detail}"


CASES = [
    ("positive", case_positive, 0),
    ("orphan sidecar (F1)", case_orphan_sidecar, 1),
    ("malformed manifest line (F2)", case_malformed_manifest_line, 1),
    ("non-hex digest", case_bad_digest_format, 1),
    ("duplicate manifest lines (F3)", case_duplicate_manifest_lines, 1),
    ("corrupt artifact", case_corrupt_artifact, 1),
    ("artifact without sidecar", case_artifact_without_sidecar, 1),
    ("artifact missing from manifest", case_artifact_missing_from_manifest, 1),
    ("empty manifest", case_empty_manifest, 1),
]


def main() -> int:
    gates = extract_gates()
    failures = 0
    total = 0

    for gate_name, script in gates.items():
        workdir = gate_workdir(script)
        print(f"\n=== gate: {gate_name} (workdir: {workdir}/) ===")
        for case_name, case_fn, expected_rc in CASES:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                try:
                    rc, detail = case_fn(root, script, workdir)
                except Exception as exc:  # noqa: BLE001 - report, do not crash
                    rc, detail = -1, f"harness error: {exc}"
            total += 1
            ok = rc == expected_rc
            if not ok:
                failures += 1
            marker = "ok  " if ok else "FAIL"
            print(f"  [{marker}] {case_name}: {detail}")

    print(f"\n{total - failures}/{total} cases passed")
    if failures:
        print(f"FAIL: {failures} case(s) regressed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
