# Continuation brief — slice #1, steps 4-9 only

> **Use this when:** the previous lane (slice-1-verifier) reported
> all 5 gates passed, started step 4 (mutation proof), and then
> died before committing. The expensive gate work is already done.
> The remaining work is the mutation proof, README flip, commit, push.

## State you are inheriting (verified by parent 2026-10-11 04:33)

- **Branch:** `research/transport-baseline-2026-10-08`
- **HEAD:** `74af377` (orchestrator's pre-dispatch doc commit; pushed)
- **Working tree:** CLEAN. File is in unmutated state.
  - `crates/fabric-integration-tests/tests/transport_baseline.rs`
    - line 29: `const SIZES: [u32; 3] = [100, 1000, 10000];` ✓
    - line 68: `let _ = stream.set_nodelay(true);` ✓ (uncommented)
  - No `MUTATED` or `MUTATION` markers anywhere in the file.
- **Slice #1 artifacts:** COMMITTED at 78abd22, all 4 files in the tree.
- **Gates status (from prior lane's checkpoint, 02:38:58):**
  - `cargo fmt --all` exit 0
  - `cargo test -p fabric-integration-tests --test transport_baseline -- --test-threads=1 --nocapture` 1 passed, 0 failed
  - `cargo clippy -p fabric-integration-tests --all-targets -- -D warnings` exit 0
  - `cargo fmt --all -- --check` exit 0
  - `cargo test -p fabric-integration-tests` all green

## Test type note (added 2026-10-11 23:41)

The `transport_baseline` test is a **characterization harness**, not a
correctness test. The original mutation recipe below (Step 4) assumed the
test had assertions on SIZES, WIDTH, or `set_nodelay`; it does not. The
test emits JSON rows with measured values (`avg_rtt_us`, `throughput_fps`)
but never asserts that those values fall within a range or differ from a
control condition. Mutations to constants cannot cause test failures
because there is no assertion surface to break.

Future mutation attempts must use a different strategy. Two options:
1. Add assertions to the test itself (e.g., `assert!(avg_rtt_us < 5000)`)
   so that mutations cause measurable test failures.
2. Mutate the measured path itself (e.g., inject a syscall-level delay via
   `LD_PRELOAD` or equivalent) so that the RTT measurements shift visibly.

The Step 4 mutation recipe below is retained for reference but will not
produce the expected 10x RTT jump detection because the test will not
fail regardless of what constants are changed.

## What to do (only the steps that are not done)

### Step 4 — mutation proof (the only file mutation you will make)

1. Backup the test file:
   ```sh
   cp crates/fabric-integration-tests/tests/transport_baseline.rs /tmp/transport_baseline.rs.bak
   ```
2. Apply the mutation — disable `set_nodelay` on the server side:
   ```sh
   sed -i.bak 's|let _ = stream.set_nodelay(true);|// MUTATED: set_nodelay(true);|' \
       crates/fabric-integration-tests/tests/transport_baseline.rs
   ```
3. Edit `SIZES` so only N=1000 runs (saves time):
   - Change line 29 from `const SIZES: [u32; 3] = [100, 1000, 10000];`
   - To `const SIZES: [u32; 1] = [1000];` (note: array length and type change)
   - This requires also changing the loop and indexing downstream. Simpler: keep SIZES as `[u32; 3] = [100, 1000, 10000];` and just `continue` the loop on `n != 1000` in the test body. That is one line, no type changes.
4. Run the test:
   ```sh
   cargo test -p fabric-integration-tests --test transport_baseline -- --test-threads=1 --nocapture 2>&1 | grep BASELINE
   ```
5. **EXPECT:** N=1000 `avg_rtt_us` jumps by roughly 10x. Poodle's baseline (mutated state was not run; this is the first mutation run) showed N=1000 median ~595 fps = ~1680 µs/frame RTT. Mutation should land in the 16-40 ms range (~16,800-40,000 µs/frame). If mutation factor is 10x or more: the test exercises Nagle. If less than 10x: STOP, the test is not exercising Nagle correctly, report why and exit.
6. Revert the mutation:
   ```sh
   cp /tmp/transport_baseline.rs.bak crates/fabric-integration-tests/tests/transport_baseline.rs
   rm crates/fabric-integration-tests/tests/transport_baseline.rs.bak
   ```
7. Verify zero diff:
   ```sh
   git diff -- crates/fabric-integration-tests/tests/transport_baseline.rs
   ```
   EXPECT: empty output.

### Step 5 — re-run the targeted test post-revert (sanity)

```sh
cargo test -p fabric-integration-tests --test transport_baseline -- --test-threads=1 --nocapture 2>&1 | tail -5
cargo fmt --all -- --check
```

EXPECT: PASS, 0 violations. The gates already passed once this run;
this is just confirming the revert is clean.

### Step 6 — update the README to VERIFIED

Edit `docs/research/2026-10-08-transport-baseline/README.md`:

1. Change the `> **Status:** UNVERIFIED.` line at the top to:
   `> **Status:** VERIFIED 2026-10-11. Mutation proof: N=1000
   avg_rtt_us = <baseline> µs, mutated = <observed> µs
   (<factor>x). Five cargo gates passed: fmt, targeted test,
   clippy -D warnings, fmt --check, full suite.`

2. In the "What still needs to happen" checklist, flip every
   item to `[x]`. Be specific with the observed numbers.

3. Add a short "Verification log" section at the bottom with:
   - Date and time
   - The mutation factor (e.g., "1.6ms → 28ms, 17x")
   - The 5 gate exit codes
   - The lane session id (you) and the model used

### Step 7 — commit

```sh
git -c user.name='KooshaPari' -c user.email='koosha@example.com' \
    add -A
git -c user.name='KooshaPari' -c user.email='koosha@example.com' \
    commit -m "research: TCP transport baseline (loopback, JSON-line)

Characterize fabric-frame-transport's existing TCP path with a stub
server. Not a feature ship; not a Parsec/Deskflow comparison.
N=10000 median 715-9281 fps over 5 runs (system-load dependent),
zero loss. Mutation on TCP_NODELAY jumps N=1000 rtt by <factor>x,
proving the test exercises Nagle.

tx-agent:     jcode
tx-validated: test
tx-task:      transport-baseline-2026-10-08
tx-scope:     crates/fabric-integration-tests, docs/research
tx-intent:    first empirical baseline for the existing frame transport"
```

Adjust the "N=10000 median 715-9281 fps" line to reflect whatever
the lane actually observed on this run (the prior lane's checkpoint
mentioned 9281; your run may be different — use the actual number).

### Step 8 — push

```sh
git push -u origin research/transport-baseline-2026-10-08
```

EXPECT: push succeeds. Confirm with
`git ls-remote origin research/transport-baseline-2026-10-08` —
the remote SHA should match your local HEAD.

### Step 9 — report

Tell the parent:
- The 5 gate exit codes (you only re-ran 2 of them in step 5; the
  other 3 are from the prior lane's checkpoint, which you trust)
- The N=1000 mutation factor
- The commit SHA and remote ref
- The total time elapsed
- Anything that contradicted the README

## Critical safeguards

- DO NOT run `cargo clean`. Disk pressure (currently 24G free,
  but sibling sessions have driven it to 2.8G in the past 2h).
- DO NOT touch the open PR queue (#20, #21, #22, #18, #17, #16).
- DO NOT pretend gates passed when they did not. Cite the
  prior lane's checkpoint as evidence for the gates you did
  not re-run, but only after step 5 re-confirms the test
  still passes.
- DO NOT skip the mutation revert verification. The brief
  requires zero diff after revert.
- DO NOT make any source changes other than the mutation
  (which is reverted before commit) and the README flip.

## If the endpoint dies mid-task

If you die after step 4 (mutation revert) but before commit:
leave the README in whatever state you reached (the
"VERIFIED 2026-10-11" line and a partial checklist are
better than nothing). The parent will commit the README
change as a doc-only commit if the diff is small. Report
your progress precisely.

If you die during the mutation: restore the file via the
backup, then exit. Parent will re-dispatch.
