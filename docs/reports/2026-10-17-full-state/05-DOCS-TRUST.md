# 05 — Documentation Trust

**Commit:** `c9eb1a3` · **Method:** claim extraction then verification against code
**Sample:** 33 line-level claims across 25 rows, drawn from top-level docs, release notes, spec files, and the single runbook

---

## Verdict counts

| Verdict | Count | Share |
|---|---|---|
| WRONG (contradicts code incl. non-existent paths) | 13 | ~39% |
| STALE (was true, drifted) | 9 | ~27% |
| UNVERIFIABLE (not contradicted, not checked) | 6 | ~18% |
| ACCURATE | 5 | ~15% |

**WRONG + STALE = 55%** of the sampled documentary evidence layer is factually wrong or out
of date. Only 15% verified accurate.

At source-document level: **5 documents contain WRONG claims** (R0 release note, R1 release
note, `specs/025-wire-transport/spec.md`, the 024-025 wire mapping, `DEVELOPMENT_GUIDE.md`).

---

## The three most damaging inaccuracies

### 1. `docs/releases/2026-09-08-R1.md` — the R1 release evidence file

Most damaging because it is the document a stakeholder reads as the canonical "what was
delivered" record. It describes delivered capability that the code does not contain. A reader
who trusts it will believe the system is further along than it is.

### 2. `specs/025-wire-transport/spec.md` + `architecture/spec-024-025-wire-mapping.md`

These describe a wire surface whose message set and test counts do not match the code. A
developer implementing against this spec will build against a fiction.

### 3. `config/settings.toml` (as pointed to by the WorkOS runbook)

The runbook's commands lead a user to a config whose auth is enabled with an empty secret, so
the documented path fails at the first step. The config is the thing that turns a documented
procedure into a broken one.

---

## Runbook commands that fail if followed today

The single runbook in the repo is `docs/runbooks/workos-infisical-shared.md`. Following it
literally leads to `workos_not_configured`, because the shipped `config/settings.toml` has
`[auth] enabled = true` and an empty `workos_client_secret`.

This is the worst category of documentation defect: **a user follows the instructions exactly
and it does not work.**

---

## Structural doc problems

| Category | Finding |
|---|---|
| Paths cited as files that are directories | several |
| Aspirational layouts described as present | a proposed crate/file layout never implemented |
| Feature descriptions contradicting live behaviour | in-tree files describing modes the code does not have |
| Config keys silently ignored | at least one key read by nothing |
| Test counts that do not match | claimed counts differ from actual |
| LoC claims with small deltas | drift, low impact |

---

## The honest reading for the owner

The documentation is **not dishonest. It is a decision and specification layer being read as
a status layer.**

The signals are all present if you look for them:

- `README.md:6` — "Status: Research and architecture baseline; not an implementation-complete claim"
- `STATUS.md:4` — "Lifecycle state: `specified`"

Those two lines are accurate. But release notes, spec files, and the runbook speak in the
present tense about delivered capability. A reader who starts at the README is well served. A
reader who starts at the release notes is misled.

**Recommendation:** do not write new capability docs until the corresponding code lands.
Instead, add one authoritative `STATUS.md`-derived truth table (this report set can serve)
and mark release notes as *planning records*, not delivered evidence. Retracting or annotating
R0 and R1 is higher value than writing anything new.

---

## Can the docs be trusted as a description of current state?

| Document class | Trustworthy as current state? |
|---|---|
| `README.md` status lines | Yes |
| `STATUS.md` lifecycle state | Yes |
| `docs/adr/` | Yes as *decisions*, not as *implementations* — 35 ADRs, a large fraction partially or entirely unimplemented |
| Release notes R0 / R1 | **No** |
| `specs/025` + wire mapping | **No** |
| `DEVELOPMENT_GUIDE.md` | **No** |
| `docs/runbooks/workos-infisical-shared.md` | **No** — commands fail as written |

**Bottom line:** the ADRs and the README status lines are trustworthy. The release notes and
spec files are not. This report set should be treated as the current status layer until code
catches up to the documents.
