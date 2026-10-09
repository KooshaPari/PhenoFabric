# Next-lane brief — Finish slice #1 transport baseline

> **Use this when:** the OpenCode Go endpoint is back, and a worker
> can complete the slice #1 work that poodle started and duckling
> never reached. Should be runnable in ONE turn.

## State you are inheriting

- **Branch:** `research/transport-baseline-2026-10-08`
- **HEAD:** `4796706` (unchanged from `origin/main`). NOTHING COMMITTED.
- **Working tree (untracked, all from poodle, all unverified):**
  - `crates/fabric-integration-tests/tests/transport_baseline.rs` (155 lines)
  - `docs/research/2026-10-08-transport-baseline/results.csv` (16 lines)
  - `docs/research/2026-10-08-transport-baseline/raw_baseline_lines.jsonl` (15 rows)
  - `docs/research/2026-10-08-transport-baseline/README.md` (just written, honest)
  - `docs/sessions/2026-10-08-pilot-readiness/` (3 session docs)
- **What you are NOT inheriting:** an existing commit, a passing mutation proof, an orchestrator-verified gate pass. The README is the orchestrator's only contribution; everything else is poodle's.

## Goal (verbatim)

Finish slice #1 by running the verification gates, doing the mutation
proof, committing, and pushing. The result is one commit on
`research/transport-baseline-2026-10-08` with all 5 ledger trailers,
`cargo fmt --check`, `cargo clippy -D warnings`, and full-suite
`cargo test -p fabric-integration-tests` all PASS, and one mutated
N=1000 run that demonstrates the test exercises Nagle.

## Steps, in order, fail at any step and report why

### 1. Verify the state you are inheriting

```sh
cd /Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-fabric
git status --short
git log --oneline -1
```

EXPECT: HEAD = `4796706`, untracked files = the 5 listed above.
ACTUAL: confirm and proceed.

If `git status` shows anything committed, STOP — poodle or duckling
got further than the report says. Read the commit before doing
anything.

### 2. Re-read the README so you know what the numbers mean

```sh
cat docs/research/2026-10-08-transport-baseline/README.md
```

Specifically the **What still needs to happen** checklist at the
bottom — that IS your task list.

### 3. Run the gates on the working tree

```sh
cargo fmt --all
cargo test -p fabric-integration-tests --test transport_baseline -- --test-threads=1 --nocapture
cargo clippy -p fabric-integration-tests --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test -p fabric-integration-tests
```

EXPECT: all 5 PASS, exit code 0. Poodle's numbers in the CSV will
shift because the system has been busy, but the SHAPE should be
similar: ~600-1000 fps median for N=10000, zero loss.

If `cargo test` finds a regression, STOP. That is a real signal,
not a flaky test. Report the failing test and exit.

### 4. Mutation proof

```sh
# Backup the test file
cp crates/fabric-integration-tests/tests/transport_baseline.rs /tmp/transport_baseline.rs.bak

# Disable TCP_NODELAY on the server side
sed -i.bak 's|let _ = stream.set_nodelay(true);|// MUTATED: set_nodelay(true);|' \
    crates/fabric-integration-tests/tests/transport_baseline.rs

# Capture N=1000 only (override SIZES temporarily)
# Easiest: edit the const SIZES = [100, 1000, 10000] -> SIZES = [1000] in the test
# Then run:
cargo test -p fabric-integration-tests --test transport_baseline -- --test-threads=1 --nocapture 2>&1 | grep BASELINE
```

EXPECT: N=1000 `avg_rtt_us` jumps by roughly 10x. If median was
~1.6ms before, mutation should show ~16-40ms. (Nagle's 40ms
delayed-ACK timer dominates.)

If the mutation does NOT change the number by 10x, the test is
not actually exercising Nagle, and the methodology is wrong.
STOP and report.

### 5. Revert the mutation

```sh
cp /tmp/transport_baseline.rs.bak crates/fabric-integration-tests/tests/transport_baseline.rs
# Restore SIZES if you edited it
rm crates/fabric-integration-tests/tests/transport_baseline.rs.bak
git diff -- crates/fabric-integration-tests/tests/transport_baseline.rs
```

EXPECT: empty diff. Confirm the file is back to the pre-mutation
state.

### 6. Final verification after revert

```sh
cargo test -p fabric-integration-tests --test transport_baseline -- --test-threads=1 --nocapture 2>&1 | tail -5
cargo fmt --all -- --check
cargo clippy -p fabric-integration-tests --all-targets -- -D warnings
```

EXPECT: all PASS.

### 7. Capture the mutation result and append to the README

Edit `docs/research/2026-10-08-transport-baseline/README.md` to fill
in the "What still needs to happen" checklist with the actual
observed numbers. Use this format:

```markdown
- [x] MUTATION: N=1000 baseline avg_rtt_us = 1624 µs; with NODELAY off
      avg_rtt_us = <observed> µs (<factor>x). Test exercises Nagle.
```

Also update the "Status" header at the top of the README to
"VERIFIED 2026-10-XX" with the date.

### 8. Commit

```sh
git -c user.name='KooshaPari' -c user.email='koosha@example.com' \
    add -A
git -c user.name='KooshaPari' -c user.email='koosha@example.com' \
    commit -m "research: TCP transport baseline (loopback, JSON-line)

Characterize fabric-frame-transport's existing TCP path with a stub
server. Not a feature ship; not a Parsec/Deskflow comparison.
N=10000 median 715 fps over 5 runs, zero loss. Mutation on
TCP_NODELAY jumps rtt by 10x, proving the test exercises Nagle.

tx-agent:     jcode
tx-validated: test
tx-task:      transport-baseline-2026-10-08
tx-scope:     crates/fabric-integration-tests, docs/research
tx-intent:    first empirical baseline for the existing frame transport"
```

Use the trailer pairing matching the other jcode commits
(`KooshaPari <koosha@example.com>`).

### 9. Push

```sh
git push -u origin research/transport-baseline-2026-10-08
```

EXPECT: push succeeds. Confirm with `git ls-remote origin
research/transport-baseline-2026-10-08` — the remote SHA should
match the local HEAD.

### 10. Report back

Tell the orchestrator:
- The 5-gate exit codes
- The N=1000 mutation factor (e.g., "1.6ms -> 28ms, 17x")
- The commit SHA and remote ref
- Anything that contradicted the README

## What to refuse

- Re-dispatch retry inside this lane (the endpoint might still
  be flaky; do not be the third failure).
- Any change to the open PR queue (#20, #21, #22, #18, #17, #16).
- `cargo clean`. Disk pressure.
- Pretending the gates passed when they did not. The whole point
  of this slice is reproducibility.

## If the endpoint dies mid-task

Same as poodle: do whatever you can without network. The on-disk
artifacts (test, CSV, JSONL, README) are already real. If you have
time before the failure, you can update the README's
"verification" section with whatever partial state you reached.
Then exit gracefully; the next lane will resume from your work.

### CRITICAL: restore recipe if you die mid-mutation

Observed 2026-10-09 with lane fox: it died-recovered mid-step-4
with the file in MUTATED state. If you find the file mutated, this
is the exact restore. The two lines to check:

```sh
grep -n "MUTATION" crates/fabric-integration-tests/tests/transport_baseline.rs
```

- Line 29 restore to:
  `const SIZES: [u32; 3] = [100, 1000, 10000];`
- Line 68 restore to:
  `    let _ = stream.set_nodelay(true);`

After restoring, confirm zero diff noise: the mutation must leave
no trace in the final commit. The commit must contain the
UNMUTATED file.
