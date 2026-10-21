# Session: Docker image runtime fix -- release gate audit

Part of the `2026-10-21-docker-image-runtime-fix` session.
Read `00_SESSION_OVERVIEW.md` first for the goal and the background.

## Release-workflow audit: the Docker image was not a release gate

Auditing `release.yml` before tagging found a gap that the twice-green
CI could not catch, because CI never exercises the release workflow's
dependency graph.

### What v0.1.4 proved

The `v0.1.4` release is the reproduction:

| Fact | Evidence |
|---|---|
| `Build Docker image` FAILED | run for tag `v0.1.4`, job `Build Docker image` |
| `Create GitHub Release` SUCCEEDED | same run, job `Create GitHub Release` |
| All ten release assets were produced | the release has 10 assets |
| No container image was published | `ghcr.io` package API returns 404 |
| The release body promised an image anyway | body text: "The Docker image is also tagged with this identity as `build-ef672c7354d0` for cross-reference." |

So a user reading the release page was told a Docker image existed, at a
specific tag, and it did not. The required-asset assertion is
fail-closed, so the ten assets were genuinely complete -- the failure was
that the image was outside the assertion entirely.

Root cause: `github-release` had `needs: [build, build-gui, identity]`.
`docker-build` was a dependency of the `docker` push job only, so a red
Docker build could not stop the release.

### The fix

Three changes, all in `.github/workflows/release.yml`:

1. **`github-release` now needs `docker-build`.** A failed Docker build
   can no longer produce a GitHub Release. The `docker-build` job
   already builds the image and asserts that both binaries load and run
   inside it, so it is a real gate rather than a best-effort side
   effect.

2. **The `docker` push job now runs BEFORE `github-release`** and
   exports a `pushed` output. The release body's Docker sentence is
   generated from that output, so it claims the image only when the image
   really reached the registry. When the push did not happen the body
   says the image "was built and verified, but was not published to the
   container registry" -- which is true, and matches the v0.1.4 failure
   mode.

   The `docker` job's best-effort behaviour is now implemented **per
   step** rather than with a job-level `continue-on-error`. A job-level
   `continue-on-error` makes the job conclude "success" while its
   outcome is "failure", which leaves the downstream `pushed` output
   dependent on output-propagation semantics that are easy to get wrong.
   With per-step `continue-on-error` the job always succeeds and the
   final step unconditionally records a definitive `pushed=true|false`.

   The success test reads `steps.push.outcome`, which is the step result
   *before* `continue-on-error` is applied, so a push that did not reach
   the registry still reports `pushed=false`.

3. **The `verify` job's Docker pull is now fail-closed.** It depends on
   `docker` so it can read `pushed`. When `pushed=true` the image must
   be pullable, or the release fails. When `pushed=false` the pull is
   skipped with an explicit warning rather than silently tolerated.
   This distinguishes an expected credential issue from an image defect.

### What was deliberately NOT changed

- The registry push stays best-effort. A missing `GITHUB_TOKEN` in a
  fork or a personal repository is an expected credential issue and must
  not fail a release whose binaries and assets are correct.
- The `production` environment has **no protection rules**
  (`protection_rules: []`, `deployment_branch_policy: null`), so it
  will not silently block a release waiting for an approval that can
  never arrive.
- The two `printf '%s\n' "$listing" | grep -qE ...` checks in the
  `verify` job were re-examined and left unchanged. Their step does not
  enable `pipefail`, and the SIGPIPE false-negative only occurs under
  `pipefail`. Verified empirically: under `bash -e -o pipefail` the same
  construct reports "NO MATCH" against a 50,000-entry listing; under
  `bash -e` (the GitHub Actions default shell) it correctly reports
  "MATCH". Adding `pipefail` to that step would have introduced the bug.

### Body-generation logic, tested locally

The conditional body was exercised against all three input states
(`pushed=true`, `pushed=false`, and an empty `pushed`):

| `pushed` | Body sentence |
|---|---|
| `true` | "The Docker image is also tagged with this identity as `build-abc123def456` for cross-reference." |
| `false` | "The Docker image for this release was built and verified, but was not published to the container registry." |
| empty | same as `false` |

No `PLACEHOLDER` token survives in any case.

Note: the body step uses `sed -i` without a suffix argument, which is
GNU sed syntax. `ubuntu-latest` runs GNU sed, so this is correct there;
macOS BSD sed would need `-i ''`. The step is CI-only and always runs on
`ubuntu-latest`.

