# ──────────────────────────────────────────────────────────────────────
# Multi-stage Dockerfile for Phenotype Fabric daemon
# ──────────────────────────────────────────────────────────────────────
# Stage 1: Build fabric-daemon and the CLI (the CLI's [[bin]] is `fabric`)
# Stage 2: Minimal runtime image
# ──────────────────────────────────────────────────────────────────────

# ── Stage 1: Builder ─────────────────────────────────────────────────
FROM rust:1.99-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependency builds: copy manifests first, create dummy sources, build deps.
# All workspace members must be copied here, otherwise `cargo build --release`
# in the cache step fails with "failed to load manifest for workspace member
# crates/<name>". This list must be kept in sync with [workspace] members in
# the top-level Cargo.toml.
COPY Cargo.toml Cargo.lock ./
COPY crates/fabric-daemon/Cargo.toml crates/fabric-daemon/Cargo.toml
COPY crates/fabric-cli/Cargo.toml crates/fabric-cli/Cargo.toml
COPY crates/fabric-graph/Cargo.toml crates/fabric-graph/Cargo.toml
COPY crates/fabric-graph-cli/Cargo.toml crates/fabric-graph-cli/Cargo.toml
COPY crates/fabric-capability/Cargo.toml crates/fabric-capability/Cargo.toml
COPY crates/fabric-capability-ffi/Cargo.toml crates/fabric-capability-ffi/Cargo.toml
COPY crates/fabric-persist/Cargo.toml crates/fabric-persist/Cargo.toml
COPY crates/fabric-checker/Cargo.toml crates/fabric-checker/Cargo.toml
COPY crates/fabric-workspace/Cargo.toml crates/fabric-workspace/Cargo.toml
COPY crates/fabric-frame-transport/Cargo.toml crates/fabric-frame-transport/Cargo.toml
COPY crates/phenotype-nvms-adapter/Cargo.toml crates/phenotype-nvms-adapter/Cargo.toml
COPY crates/phenotype-manifest/Cargo.toml crates/phenotype-manifest/Cargo.toml
COPY crates/fabric-tray/Cargo.toml crates/fabric-tray/Cargo.toml
COPY crates/fabric-gui/Cargo.toml crates/fabric-gui/Cargo.toml
COPY crates/fabric-web/Cargo.toml crates/fabric-web/Cargo.toml
COPY crates/fabric-tui/Cargo.toml crates/fabric-tui/Cargo.toml
COPY crates/fabric-surface-mojo/Cargo.toml crates/fabric-surface-mojo/Cargo.toml
COPY crates/fabric-orchestrator/Cargo.toml crates/fabric-orchestrator/Cargo.toml
COPY crates/fabric-terminal/Cargo.toml crates/fabric-terminal/Cargo.toml
COPY crates/fabric-capture/Cargo.toml crates/fabric-capture/Cargo.toml
COPY crates/fabric-integration-tests/Cargo.toml crates/fabric-integration-tests/Cargo.toml
COPY crates/fabric-research-ledger/Cargo.toml crates/fabric-research-ledger/Cargo.toml
COPY examples/full-demo/Cargo.toml examples/full-demo/Cargo.toml

# Create stub lib.rs / main.rs for each workspace member so cargo can resolve deps.
# List must mirror the COPY list above. Crates that declare [[bench]] entries in
# their Cargo.toml need benches/ stubs too, otherwise `cargo build --release
# --locked` in the cache step fails with "can't find `<name>` bench at
# `benches/<name>.rs`" (see round 14 session notes for the 4 affected crates:
# fabric-frame-transport, fabric-persist, fabric-checker, fabric-graph). The
# stub bench body is `fn main() {}` because all four crates use `harness =
# false` in their [[bench]] declaration.
RUN mkdir -p crates/fabric-daemon/src crates/fabric-cli/src crates/fabric-graph/src \
             crates/fabric-graph-cli/src crates/fabric-capability/src \
             crates/fabric-capability-ffi/src crates/fabric-persist/src \
             crates/fabric-checker/src crates/fabric-workspace/src \
             crates/fabric-frame-transport/src crates/phenotype-nvms-adapter/src \
             crates/phenotype-manifest/src crates/fabric-tray/src \
             crates/fabric-gui/src crates/fabric-web/src crates/fabric-tui/src \
             crates/fabric-surface-mojo/src crates/fabric-orchestrator/src \
             crates/fabric-terminal/src crates/fabric-capture/src \
             crates/fabric-integration-tests/src crates/fabric-research-ledger/src \
             crates/fabric-frame-transport/benches crates/fabric-persist/benches \
             crates/fabric-checker/benches crates/fabric-graph/benches \
             examples/full-demo/src && \
    for crate in fabric-graph fabric-graph-cli fabric-capability fabric-capability-ffi \
                 fabric-persist fabric-checker fabric-workspace \
                 fabric-frame-transport phenotype-nvms-adapter phenotype-manifest \
                 fabric-tray fabric-gui fabric-web fabric-tui fabric-surface-mojo \
                 fabric-orchestrator fabric-terminal fabric-capture \
                 fabric-integration-tests fabric-research-ledger; do \
        echo "pub fn _stub() {}" > "crates/${crate}/src/lib.rs"; \
    done && \
    for bench in transport persist checker compiler; do \
        case "$bench" in \
            transport)     crate=fabric-frame-transport ;; \
            persist)       crate=fabric-persist ;; \
            checker)       crate=fabric-checker ;; \
            compiler)      crate=fabric-graph ;; \
        esac && \
        echo "fn main() {}" > "crates/${crate}/benches/${bench}.rs"; \
    done && \
    mkdir -p examples/full-demo/src && \
    echo "pub fn _stub() {}" > examples/full-demo/src/lib.rs && \
    echo "fn main() {}" > crates/fabric-daemon/src/main.rs && \
    echo "fn main() {}" > crates/fabric-cli/src/main.rs

# Pre-build dependencies (cached unless Cargo.toml changes).
RUN cargo build --release --locked 2>/dev/null || cargo build --release

# Now copy real source and rebuild only the targets we need.
COPY crates/ crates/
COPY examples/ examples/

RUN cargo build --release --bin fabric-daemon --bin fabric

# ── Stage 2: Runtime ─────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Create unrooted daemon user.
RUN groupadd --gid 1000 fabric && \
    useradd  --uid 1000 --gid fabric --shell /bin/false --create-home fabric

# State directory for SQLite database and workspace data.
RUN mkdir -p /var/lib/fabric && chown fabric:fabric /var/lib/fabric

# Copy binaries from builder.
# The CLI binary is named `fabric` (the fabric-cli crate's [[bin]] target);
# there is no `fabric-cli` binary. See .github/workflows/release.yml.
COPY --from=builder /build/target/release/fabric-daemon /usr/local/bin/fabric-daemon
COPY --from=builder /build/target/release/fabric        /usr/local/bin/fabric

# Expose wire-server port.
EXPOSE 9400

# Health check: send a health_check message over the wire protocol.
# `fabric-daemon health --connect` speaks the TCP wire protocol on port 9400.
# Note: the CLI (`fabric`) has no `health` subcommand and uses `--daemon`,
# not `--connect`; the daemon binary is the correct health probe.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD /usr/local/bin/fabric-daemon health --connect 127.0.0.1:9400 || exit 1

USER fabric

ENTRYPOINT ["/usr/local/bin/fabric-daemon"]
CMD ["start"]
