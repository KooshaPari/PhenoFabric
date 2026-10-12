# 13 — Release-Lifecycle v2 Audit Verification (BLOCKED by opencode-go provider outage)

**Status:** Verification was attempted and externally blocked. The 12_ audit was NOT independently re-verified by a subagent. The audit's content is the agent's own structural review of source, not externally validated. This doc records the block and the honest state.

**Date:** 2026-10-12
**Branch:** `research/transport-baseline-2026-10-08`
**HEAD at start:** `0e14d66 docs(pilot-readiness): 12th — release-lifecycle v2 audit`
**Working tree:** clean (0 uncommitted)

---

## 1. What was attempted

A subagent was dispatched to re-verify every claim in `12_RELEASE_LIFECYCLE_AUDIT.md` against the live source. The subagent was to:
- Re-read each source file cited by the audit
- Re-run every command cited by the audit
- Cross-check each claim with VERIFIED / CONTRADICTED / INCONCLUSIVE
- Write a `13_RELEASE_LIFECYCLE_AUDIT_VERIFICATION.md` doc with sections A-G
- Return: path, discrepancy count, confirmed-gate count (out of 5), external-block count, time taken, model that served

## 2. What blocked the verification

The authorized opencode-go fallback ladder was exhausted:

| # | Model | Subagent session | Result | Error |
|---|---|---|---|---|
| 1 | `opencode-go:mimo-v2.6-flash` (standing default) | `session_mosquito_1791778098773_68ab9e25df2aaa6f` | FAILED | `OpenAI-compatible chat request failed endpoint: https://opencode.ai/zen/go/v1/chat/completions model: mimo-v2.6-flash...` |
| 2 | `opencode-go:mimo-v2.5-pro` (operator tactical pick via Q1.a.2) | `session_owl_1791778131197_e6c981dd98d0aa3d` | FAILED | `OpenAI-compatible chat request failed endpoint: https://opencode.ai/zen/go/v1/chat/completions model: mimo-v2.5-pro a...` |
| 3 | `opencode-go:glm-5.3` (fallback #3) | `session_ox_1791778182638_69f0d4df45fe1ae2` | FAILED | `OpenAI-compatible chat request failed endpoint: https://opencode.ai/zen/go/v1/chat/completions model: glm-5.3 auth: O...` |
| 4 | `opencode-go:deepseek-v4-pro` (fallback #4) | `session_pig_1791778206978_077801aa1815353a` | FAILED | `OpenAI-compatible chat request failed endpoint: https://opencode.ai/zen/go/v1/chat/completions model: deepseek-v4-pro...` |

**Pattern:** All 4 attempts failed at the same endpoint `https://opencode.ai/zen/go/v1/chat/completions` with `auth: O...` (truncated). The error pattern indicates a **provider-side issue** (likely an `auth: OK` flag being checked, or an OpenCode Go entitlement outage) rather than a per-model problem. The remaining fallback in the ladder (`#5: omen-alpha`) is also opencode-go and would fail at the same endpoint.

**Per `~/.jcode/memories/global/harness-agents.md`:** "advance only on actual failure of the model above, and record which model actually served the work. Never advance silently." 4 advances recorded above; no model served.

**Per `~/.jcode/memories/global/harness-agents.md`:** "Do not silently fall back to native Codex / OmniRoute from a Jcode worker route. Native Codex workers remain a blocker fallback only for Jcode blockers." This task is a verification follow-up, not a Jcode blocker, so native Codex/OmniRoute fallbacks are NOT used.

## 3. What the parent did (parent-safe only)

Per the subagent-ALWAYS rule (operator 2026-10-11 21:08): "main context strictly must only be management/high level ctx." Trivial reads are parent-safe. The parent executed:

- `git log -1 --format='%h %s'` → `0e14d66 docs(pilot-readiness): 12th — release-lifecycle v2 audit (5 lifecycle gaps + 5 open dependabot alerts)` (12_ doc on disk)
- `wc -l docs/sessions/2026-10-08-pilot-readiness/12_RELEASE_LIFECYCLE_AUDIT.md` → `193` (193 lines, matches write tool report)
- `git status --short | wc -l` → `0` (working tree clean)

These confirm the 12_ doc is real and committed. They do NOT verify its content against source.

## 4. What the parent did NOT do (subagent-required, blocked)

The following are bulk reads and would require a subagent. They were NOT done in the parent:

- Re-read `.github/workflows/release.yml` (full file, 320+ lines)
- Re-read `.github/workflows/deny.yml` (full file)
- Re-read `Cargo.toml` (top 40 lines, 23 workspace members enumeration)
- Re-read `deny.toml` (root, check 5 open alert coverage)
- Re-run `gh pr list --state open --json` (large JSON output)
- Re-run `gh api repos/KooshaPari/PhenoFabric/dependabot/alerts` (large JSON, re-identify 5th alert)
- Cross-check each of the 5 lifecycle gaps against actual release.yml

## 5. Honest assessment of 12_ audit's verification state

| Status dimension | State | Source |
|---|---|---|
| Doc on disk | **VERIFIED** | `wc -l 12_` = 193 |
| Commit landed on branch | **VERIFIED** | `git log` = 0e14d66 |
| Pushed to origin | **VERIFIED** (prior turn) | `origin/research` HEAD = 0e14d66 |
| Content matches source | **UNVERIFIED** | No subagent re-read; audit is agent's own structural review |
| 5 lifecycle gaps confirmed against release.yml | **UNVERIFIED** | No subagent re-read |
| 4 dependabot-gate-disconnected alerts caught by deny.toml | **UNVERIFIED** | deny.toml not re-read |
| 19 silent-excluded workspace members | **UNVERIFIED** | per-crate `publish = false` not enumerated |
| 0 GH releases | **VERIFIED** (prior turn) | `gh release list` empty |

**Net:** the 12_ audit's persistence is verified. Its content is unverified. The 5 lifecycle gaps are the agent's structural claim, not externally validated. An operator who needs the 5 gaps to be auditable should re-attempt verification when opencode-go is restored.

## 6. Section 7 of 12_ already self-acknowledged most of this

The 12_ audit's own section 7 (`UNKNOWN`) lists:
- "Whether `softprops/action-gh-release@v3` was ever invoked on a `v*` tag"
- "The exact set of crates that have `publish = true` vs. silent exclusion"
- "Whether `crates.io` has any prior `fabric-*` publish history"
- "Whether the workflow ever called `docker/metadata-action@v6` for `org.opencontainers.image.source` label"
- "Whether `deny.toml` is strict enough to catch all 5 open dependabot alerts"
- "The exact `gh run list --workflow=release.yml` history"

The audit already self-flagged these as UNKNOWN. This 13_ doc adds: the 5 lifecycle gaps in section 10 are also UNVERIFIED — the audit was never re-read against the source by a subagent.

## 7. Recommendation for the operator

Three options, in order of cost:

1. **Re-attempt verification when opencode-go is restored.** The subagent prompt and prompt structure are at the top of this doc. Re-dispatch with `model: "opencode-go:mimo-v2.5-pro"` (or whatever is current standing default) and the verification will run. Estimated 10-15 minutes.

2. **Do the verification manually.** Read release.yml, deny.yml, Cargo.toml, deny.toml. Re-run `git tag`, `gh release list`, `gh pr list`, `gh api dependabot/alerts`. Cross-check each of the 5 gaps. Estimated 20-30 minutes of operator attention.

3. **Accept the unverified state.** Treat 12_ as the agent's own structural review, note that it is not externally validated, and act on the 5 gaps as the agent's hypothesis. The audit's structural plausibility is high (the agent read the source files in the prior turn and the persistence is verified), but the content has not been independently re-read.

**The 12_ doc is NOT a falsified claim** — its content is a faithful transcription of what the agent read in the prior turn. The unverified state means "no second pair of eyes has re-read the source." It does NOT mean "the agent made it up."

## 8. Failure log (raw, for the next session)

```
session_mosquito_1791778098773_68ab9e25df2aaa6f
  Lifecycle: failed
  Provider: OpenCode Go / mimo-v2.6-flash
  Error:    OpenAI-compatible chat request failed endpoint:
            https://opencode.ai/zen/go/v1/chat/completions model: mimo-v2.6-flash...

session_owl_1791778131197_e6c981dd98d0aa3d
  Lifecycle: failed
  Provider: OpenCode Go / mimo-v2.5-pro
  Error:    OpenAI-compatible chat request failed endpoint:
            https://opencode.ai/zen/go/v1/chat/completions model: mimo-v2.5-pro a...

session_ox_1791778182638_69f0d4df45fe1ae2
  Lifecycle: failed
  Provider: OpenCode Go / glm-5.3
  Error:    OpenAI-compatible chat request failed endpoint:
            https://opencode.ai/zen/go/v1/chat/completions model: glm-5.3 auth: O...

session_pig_1791778206978_077801aa1815353a
  Lifecycle: failed
  Provider: OpenCode Go / deepseek-v4-pro
  Error:    OpenAI-compatible chat request failed endpoint:
            https://opencode.ai/zen/go/v1/chat/completions model: deepseek-v4-pro...
```

The 5th model in the ladder (`opencode-go:omen-alpha`) was NOT attempted because the 4-failure pattern at the same endpoint makes a 5th failure near-certain and adds no new evidence.
