# Contributing to Phenotype Fabric

## Scope of this project

Fabric is a distributed interactive operating environment and runtime fabric.
It owns physical execution substrate — capability inventory, topology, locality,
route plans, route leases, surface/presentation isolation, and real-time quality
of service.

It explicitly **does not own**: governed intent (AgilePlus), agent dispatch
(thegent), economic allocation (AGSLAG), evidence graph (Tracera), session
history (SessionLedger), or process supervision (ShareCLI).

If your change extends Fabric into one of those domains, you need an ADR
before writing code.

## Development Setup

### Prerequisites

- **Rust** 1.81+ (via [rustup](https://rustup.rs/))
- **cargo-deny** (optional, for dependency auditing): `cargo install cargo-deny`
- **mdBook** (optional, for docs): `cargo install mdbook`

### Getting Started

```bash
# Clone the repository
git clone https://github.com/phenotype-dev/phenotype-fabric.git
cd phenotype-fabric

# Build the workspace
cargo build

# Run the test suite
cargo test --workspace

# Run the CLI
cargo run -p fabric-cli -- --help
```

## Code Style

- **Rust edition**: 2021
- **Formatting**: `cargo fmt`
- **Linting**: `cargo clippy --workspace -- -D warnings`
- **No unsafe code**: All crates enforce `unsafe_code = "forbid"`

## Running Tests

```bash
# Full workspace tests
cargo test --workspace

# Specific crate tests
cargo test -p fabric-graph

# With output
cargo test --workspace -- --nocapture

# Benchmarks
cargo bench --workspace
```

## Before Writing Code

1. **Traceability**: every behavioral change must trace to an
   `intent/`, `specs/NNN-*/`, `FUNCTIONAL_REQUIREMENTS.md`,
   `NON_FUNCTIONAL_REQUIREMENTS.md`, or `SYSTEM_REQUIREMENTS.md` entry.
   If the requirement is missing, propose it first. See
   `verification/requirements-traceability.json`.

2. **ADR**: an ADR is required before changing any of the following:
   - product authority or repository boundaries
   - control/data-plane protocol or format
   - platform-specific privileged components
   - consistency, security, or real-time semantics
   - vendor lock-in decisions
   - rejection of a plausible alternative
   - meaning of an existing public claim

   See `GOVERNANCE.md#adr-rule` and `adr/INDEX.md`.

3. **Security review trigger**: changes requiring kernel/driver installation,
   input capture/injection, screen/audio capture, virtual display/HID creation,
   cross-realm shared memory, remote wake/KVM-over-IP, agent-created realms,
   or arbitrary syscall interposition require a security review.
   See `GOVERNANCE.md#security-review-triggers`.

4. **Research rule**: any claim that is not grounded in an accepted ADR or
   an existing benchmark must go through the research program.
   See `GOVERNANCE.md#research-rule` and `research/research-program.md`.

## Writing Spec Changes

Documents are append-only in history but mutable in the working tree.
Superseded documents must remain linked with:
- superseding document ID
- date
- reason
- affected requirement and work-package IDs
- migration action

See `GOVERNANCE.md#supersession`.

When editing any document:
1. Update the document's status/lifecycle field if behavior changed.
2. Update `work/tasks.json` task state if a task is now done.
3. Update `verification/requirements-traceability.json` if a claim changed.
4. Update `work/wbs.md` if a work-package boundary moved.
5. Run spec validation (see below) before committing.

## Writing Code

1. Every adapter must advertise a versioned capability descriptor and
   implement the conformance rules in `SPECIFICATION.md#conformance`.
2. Every route must bind to a topology epoch; stale epochs invalidate plans.
3. RT threads must never perform DNS, disk I/O, dynamic allocation, or
   policy evaluation. See `LLD.md#rt-execution`.
4. Copy count and domain transitions must be measured per route.
   See `verification/latency-methodology.md`.
5. Privileged code must be narrowly scoped to the minimum required capability.
   A platform-supported mechanism must be shown insufficient before a
   privileged helper is written.

## Evidence Rule

No feature may be described as "working," "zero-copy," "hard real-time,"
"HDR-preserving," "transparent," "atomic," or "seamless" based only on
architecture intent. Each such claim requires:
- exact topology and software versions
- measurement boundary
- workload and concurrent-load description
- p50, p95, p99, and worst observed result
- failure and degradation behavior
- reproducible scripts/configuration
- raw evidence artifact
- requirement IDs covered
- source/clock synchronization method where relevant

See `GOVERNANCE.md#evidence-rule`.

## Definition of Done

A work package is done only when its acceptance criteria, compatibility
matrix, benchmark/fault tests, security controls, documentation, and
rollback path are all evidenced. "Code exists" is not completion.

See `GOVERNANCE.md#definition-of-done` and `verification/acceptance-gates.md`.

## Pull Request Process

1. **Fork** the repository and create a feature branch from `main`
2. **Make your changes** with clear, descriptive commits
3. **Ensure all checks pass**:
   - `cargo fmt --check`
   - `cargo clippy --workspace -- -D warnings`
   - `cargo test --workspace`
4. **Update documentation** if your change affects public APIs or architecture
5. **Open a PR** against `main` with a clear title and description

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: add new surface type`
- `fix: resolve topology compiler edge case`
- `docs: update ADR for wire protocol`
- `test: add integration tests for workspace FSM`
- `refactor: simplify route compilation`

## Spec Validation (run before every commit)

```bash
# All JSON schemas well-formed
for s in architecture/schemas/*.json; do
  python3 -c "import json; json.load(open('$s'))" || exit 1
done

# event.proto syntactically valid
protoc --proto_path=architecture/schemas \
  --descriptor_set_out=/dev/null architecture/schemas/event.proto

# OpenAPI well-formed
python3 -c "import yaml; yaml.safe_load(open('architecture/openapi.yaml'))"
```

## Prompt Preservation

The exact prompts under `intent/` are immutable source records. When
adding new intent, add a new numbered entry rather than rewriting history.
Preserve failed experiments and rejected alternatives — they inform future
decisions.

## License

By contributing, you agree that your contributions will be licensed under the same license as the project: MIT OR Apache-2.0.
