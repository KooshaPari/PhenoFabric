# Branch Cleanup Audit

**Date**: 2026-09-14
**Repo**: phenotype-fabric (KooshaPari/PhenoFabric)
**Local main**: `b2564f6` (fix: convert phenotype-manifest path dep to git dep)

## Task 1: Sync Local Main

```
git pull origin main
Updating 1282092..b2564f6
Fast-forward
 Cargo.lock                               | 1 +
 crates/phenotype-nvms-adapter/Cargo.toml | 2 +-
 2 files changed, 2 insertions(+), 1 deletion(-)
```

Verified: local HEAD is `b2564f6`.

---

## Task 2: Unmerged Remote Branch Audit

### Branch 1: `fix/path-dep-nanovms`

| Field | Value |
|---|---|
| **PR** | #3 (MERGED 2026-09-14) |
| **Commit** | `60b1124` (same content as `b2564f6`) |
| **Status** | Already merged into main |
| **Recommendation** | **DELETE** (remote branch) |

The commit is on main. Branch is stale.

---

### Branch 2: `dependabot/cargo/jsonwebtoken-10.3.0`

| Field | Value |
|---|---|
| **PR** | #6 (OPEN) |
| **Current on main** | `jsonwebtoken = "9"` (9.3.1) |
| **Proposed** | `jsonwebtoken = "10"` (10.3.0) |
| **Files changed** | `crates/fabric-daemon/Cargo.toml`, `Cargo.lock` |
| **Breaking changes?** | Major version bump (9 -> 10). Swaps `ring` dependency for `signature` crate. API may have breaking changes. |
| **Recommendation** | **REVIEW and potentially MERGE** after verifying `fabric-daemon` code compiles with v10. Dep update is valid but requires a build check. |

Key changes in jsonwebtoken 10:
- Removed `ring` dependency, now uses `signature` crate
- Added `getrandom` dependency
- Semver major bump = possible API breakage

---

### Branch 3: `dependabot/cargo/opentelemetry_sdk-0.32.1`

| Field | Value |
|---|---|
| **PR** | #5 (OPEN) |
| **Current on main** | `opentelemetry_sdk = "0.27"` (0.27.1) |
| **Proposed** | `opentelemetry_sdk = "0.32"` (0.32.1) |
| **Files changed** | `crates/fabric-daemon/Cargo.toml`, `Cargo.lock` |
| **Problem** | **Partial upgrade creates version duplication.** Only bumps `opentelemetry_sdk` but not `opentelemetry` (still "0.27"), `opentelemetry-otlp` (still "0.27"), or `tracing-opentelemetry` (still "0.28"). |
| **Cargo.lock impact** | Pulls in BOTH `opentelemetry 0.27.1` + `0.32.0` and `opentelemetry_sdk 0.27.1` + `0.32.1` simultaneously. |
| **Recommendation** | **CLOSE without merge.** This is a bad partial upgrade. The OpenTelemetry crates must be upgraded together as a coordinated set. To upgrade, all four crates need version bumps: `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`, and `tracing-opentelemetry`. |

---

### Branch 4: `dependabot/github_actions/github-actions-1c3183c272`

| Field | Value |
|---|---|
| **PR** | #4 (OPEN). Previous PR #2 (same branch) was CLOSED without merge. |
| **Changes** | CI action version bumps across 3 workflow files |
| **Specific bumps** | `actions/checkout` v4 -> v7, `actions/cache` v4 -> v6, `actions/setup-go` v5 -> v7 |
| **Files changed** | `.github/workflows/ci.yml`, `.github/workflows/deny.yml`, `.github/workflows/e2e.yml` |
| **Risk** | Low - standard action version bumps |
| **Recommendation** | **MERGE** - straightforward CI maintenance. Should pass CI before merge. |

---

## Summary

| Branch | Has Open PR? | Verdict | Action |
|--------|-------------|---------|--------|
| `fix/path-dep-nanovms` | No (PR #3 merged) | Stale, already merged | DELETE |
| `dependabot/cargo/jsonwebtoken-10.3.0` | Yes (PR #6) | Valid but major version bump | REVIEW + build check |
| `dependabot/cargo/opentelemetry_sdk-0.32.1` | Yes (PR #5) | Bad partial upgrade, version duplication | CLOSE |
| `dependabot/github_actions/github-actions-1c3183c272` | Yes (PR #4) | Low-risk CI maintenance | MERGE |

### Dependency Upgrade Note

If upgrading the OpenTelemetry stack in the future, all crates must be bumped together:
```
opentelemetry = "0.27"      # -> latest compatible
opentelemetry_sdk = "0.27"   # -> latest compatible
opentelemetry-otlp = "0.27"  # -> latest compatible
tracing-opentelemetry = "0.28" # -> latest compatible
```
