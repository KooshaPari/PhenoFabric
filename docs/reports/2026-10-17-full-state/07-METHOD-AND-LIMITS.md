# 07 — Method and Limits

**Purpose:** so the owner can calibrate how much to trust each number in this set.

---

## How this audit was performed

Seven independent read-only lanes, each with a self-contained brief and a report obligation:

| Lane | Task | Scope |
|---|---|---|
| `quokka` | stub / fake / laziness inventory | production code, 52,353 LOC |
| `ocelot` | usability journey tracing | 5 end-to-end journeys |
| `marten` | test and CI honesty | 22 crates, 5 workflows |
| `gecko` | goal status + ADR survey + effort | A1/A2/A3, 35 ADRs |
| `lynx` | documentation trust | 33 claims across 25 rows |
| `tapir` | A1 live-run reconnaissance | WorkOS auth path |
| `marmot` | A3 transport selection forensics | route compiler |

Every lane was read-only: no edits, no commits, no pushes. Lanes were forbidden from
touching `crates/fabric-daemon/src/wire/`, which was under concurrent development.

**Parent-side re-verification.** Two of the most consequential claims were independently
re-verified by the coordinating session rather than accepted from a lane:

| Claim | Method | Result |
|---|---|---|
| `_ => "tcp_transport"` catch-all exists | direct `grep` | **confirmed** at `multihop/mod.rs:211` |
| `RouteExecutor` absent | workspace-wide search | **confirmed absent** |
| `fabric-gui` workspace membership | direct read of `Cargo.toml` | **is a member (line 17)**; only `src-tauri` excluded |

That third check **corrected an earlier claim made during this ownership session** and is
recorded in `00-EXECUTIVE-SUMMARY.md` §7.

---

## What was NOT verified

This is the most important section in the set.

| Not verified | Why | Consequence |
|---|---|---|
| Test suites were not run by lanes | deliberate, to avoid build contention with concurrent work in `wire/` | test counts come from the parent's own verified runs on `c9eb1a3`, not from lane execution |
| No GUI was launched | no human click available | the GUI is **UNKNOWN**, not "broken". 2,608 lines of HTML exist; whether they render live state is unproven |
| No `wire/protocol.rs` enumeration by the A2 lane | file was in the forbidden active area | the claim "no federation arm exists in protocol.rs" rests on the module's own `#[allow(dead_code)]` self-admission, not a direct read |
| No hardware transports tested | no RDMA NIC, no VFIO-passthrough GPU on this host | PCIe / RDMA / IVSHMEM effort is **UNKNOWN**, not estimated |
| 28 of 35 ADRs characterized by summary, not deep-read | time-boxed | recommendation: deep-read ADRs 0030, 0031, 0033, 0034 before A2/A3 work |
| No performance measurement taken | nothing to measure — no executor exists | any future benchmark must run on the deployment hardware; transport deltas are platform-dependent |

---

## How to read the effort ranges

Effort is given in **focused engineering sessions**. One session is one uninterrupted block
on one concern.

No wall-clock hours and no calendar dates are given, for two reasons:

1. The project forbids fabricated durations.
2. A calendar estimate would require a staffing assumption that is not the auditor's to make.

To convert: multiply sessions by the responsible engineer's real sessions-per-week, then add
roughly 30% for review and iteration. Ranges assume one engineer with no other concurrent
work on this repository.

---

## Confidence by finding class

| Finding class | Confidence | Basis |
|---|---|---|
| BLOCKER stubs (3) | **High** | each read at file:line, each has a quoted body |
| `RouteExecutor` absent | **High** | search-verified twice |
| Stub density (0.034%) | **High** | LOC denominator computed explicitly |
| Zero-test crates (4) | **High** | per-crate test count table |
| `--lib` skips ~83 tests | **Medium** | estimated from main-only module inventory, not a differential run |
| "73% of integration tests are in-process" | **Medium** | classification of 27 files, judgement applied per file |
| Docs 55% wrong/stale | **Medium** | 33-claim sample; the sample is not the whole corpus |
| Effort ranges | **Low–Medium** | engineering judgement; hardware-gated parts explicitly UNKNOWN |
| A1 requires a human | **High** | no headless/device-code path found after directed search |
| GUI functional status | **UNKNOWN** | never launched |

---

## Known biases in this audit

1. **The lanes were told to be blunt.** That produces a report that reads more negative than
   the code deserves. The correct calibration is in `00-EXECUTIVE-SUMMARY.md` §3: the code is
   well engineered, the verification net is thin in user-facing places, and the wiring is
   unfinished.
2. **Read-only audits cannot prove absence perfectly.** "No executor exists" is a strong
   search result, not a proof. It is reported with high confidence because two independent
   searches agreed and the federation module self-admits the same gap.
3. **Effort estimates cluster around the auditor's mental model of the codebase**, having read
   it. Inflate for unfamiliarity if a different engineer executes.

---

## Reproducing this audit

```bash
# Stub inventory
rg -n 'todo!|unimplemented!|panic!\("not implemented"\)|expect\("TODO' crates/*/src/

# Per-crate test counts
for c in crates/*/; do printf '%-40s %s\n' "$c" \
  "$(rg -c '#\[(test|tokio::test|rstest)\]' "$c/src" 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')"; done

# Executor absence
rg -n 'execute_route|run_route|apply_route|run_plan|RouteExecutor' crates/*/src/

# The catch-all
rg -n 'tcp_transport' crates/fabric-graph/src/multihop/mod.rs

# Zero-test crates
for c in crates/*/; do n=$(rg -c '#\[test\]' "$c" 2>/dev/null | awk -F: '{s+=$2} END{print s+0}'); \
  [ "${n:-0}" -eq 0 ] && echo "ZERO TESTS: $c"; done
```

---

## What would change this report's conclusions

| If this were true | Then |
|---|---|
| A `RouteExecutor` exists under a name this search missed | A3 is far closer than stated; re-run the search first |
| The GUI renders correct live state when launched | A1's code gap is much smaller |
| L3/L4/L8 are documented elsewhere as intentional TCP fallback | the catch-all is a design decision, not a defect — but the silence remains one |
| Test lanes ran the suite and it disagreed with 751/63 | the quality section needs revision |
| The deployment host has RDMA and a passthrough GPU | A3's upper bound stops being UNKNOWN |
