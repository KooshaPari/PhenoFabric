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

# ── Dependency caching ───────────────────────────────────────────────────────
# This used to be a two-stage scheme:
#
#   1. COPY only the workspace manifests, hand-write a stub source file for
#      every declared cargo target, and `cargo build --release` to warm the
#      dependency cache.
#   2. COPY the real sources and rebuild.
#
# The hand-written stub list drifted, and the v0.1.4 release's Docker build
# failed with:
#
#   error: can't find lib `fabric_cli` at path `/build/crates/fabric-cli/src/lib.rs`
#   error: can't find bin `fabric-tui` at path `/build/crates/fabric-tui/src/main.rs`
#   error: can't find bin `tf-web` at path `/build/crates/fabric-terminal/src/web_main.rs`
#   (7 distinct targets in total)
#
# The stub layer could not be repaired by hand and stay repaired -- a crate
# may declare several `[[bin]]`s, a `[lib]`, and `[[bench]]`es, and each new
# target breaks the build again. Worse, cargo refuses to resolve the
# workspace AT ALL when any declared target's source file is missing, so the
# stub list has to be exhaustive to work at all.
#
# The stub layer is therefore REMOVED. It never provided useful caching
# anyway: every crate the image needs is a path dependency of another
# workspace crate, so the cache would be warmed with stub crates rather than
# the real ones and the second build would recompile everything regardless.
#
# The build now copies the real sources and builds only the two binaries the
# runtime stage consumes. Because cargo builds only the requested targets and
# their dependency closure, crates that are irrelevant to the image (the
# terminal/TUI/GUI crates) are never compiled. Docker's layer cache still
# does the useful part of the job: any change to an unrelated file leaves
# the earlier layers intact.
#
# `cargo build` errors out if a declared target's source file is missing, so
# a green build proves the whole workspace resolves.
#
# `docs/` must be copied too: two crates in the image's dependency closure
# embed documentation into the binary with `include_str!`, and the
# compiler fails if the file is absent:
#
#   crates/fabric-frame-transport/src/schema.rs
#     include_str!("../../../docs/spec/wire-protocol-schema.json")
#   crates/fabric-daemon/src/secret_loader/review_response.rs
#     include_str!("../../../../docs/runbooks/workos-infisical-shared.md")
#
# The first of those is a hard build error (the crate is in the
# `fabric-daemon` dependency closure); the second is a compile error too
# even though the constant is only read on a review path. Copying the
# whole `docs/` tree is simpler and more robust than copying individual
# files, and it is the only way to avoid this class of failure recurring
# when a new `include_str!` is added.
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY examples/ examples/
COPY docs/ docs/

# Build only the binaries the runtime stage consumes. `--locked` keeps the
# build reproducible against Cargo.lock.
RUN cargo build --release --locked --bin fabric --bin fabric-daemon

# ── Stage 2: Runtime ─────────────────────────────────────────────────
# The runtime base MUST match the builder's Debian release.
#
# `rust:1.99-slim` is Debian 13 "trixie" (glibc 2.41). A runtime stage
# on `debian:bookworm-slim` is Debian 12 (glibc 2.36), and a binary
# linked against glibc 2.41 fails to start on glibc 2.36 with
# "version `GLIBC_2.38' not found". The two stages were mismatched, so the
# image could have built successfully and still been unrunnable.
#
# The OpenSSL package name also differs between the releases: trixie
# provides `libssl3t64`, bookworm provides `libssl3`. Both ship
# /usr/lib/<triplet>/libssl.so.3, which is what the binaries link.
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

# Create unrooted daemon user.
RUN groupadd --gid 1000 fabric && \
    useradd  --uid 1000 --gid fabric --shell /bin/false --create-home fabric

# State directory for SQLite database and workspace data.
RUN mkdir -p /var/lib/fabric && chown fabric:fabric /var/lib/fabric

# Copy binaries from builder.
# The CLI binary is named `fabric` (the fabric-cli crate's [[bin]] target);
# there is no `fabric-cli` binary. See .github/workflows/release.yml.
#
# These binaries link OpenSSL: `reqwest` (a fabric-daemon dependency) uses
# native-tls, which links libssl/libcrypto. The runtime stage therefore
# installs libssl3t64 -- without it the binaries fail to load with
# "error while loading shared libraries: libssl.so.3".
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
