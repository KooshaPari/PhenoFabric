# ──────────────────────────────────────────────────────────────────────
# Multi-stage Dockerfile for Phenotype Fabric daemon
# ──────────────────────────────────────────────────────────────────────
# Stage 1: Build fabric-daemon and the CLI (the CLI's [[bin]] is `fabric`)
# Stage 2: Minimal runtime image
# ──────────────────────────────────────────────────────────────────────

# ── Stage 1: Builder ─────────────────────────────────────────────────
FROM rust:1.99-slim AS builder

# cmake is REQUIRED, not optional.
#
# `fabric-daemon` verifies JWTs with `jsonwebtoken`, which is wired to the
# `aws_lc_rs` crypto backend. That backend is a binding to AWS-LC, whose
# FIPS-capable core is C code built by `aws-lc-sys` -- and `aws-lc-sys`
# drives that build through CMake. `rust:1.99-slim` does not ship CMake,
# so without this package the container build dies in the dependency
# build with "cmake executable not found".
#
# The backend switch was made for a security reason, not a build reason:
# the previous `rust_crypto` backend pulled `crypto-bigint 0.5.5`
# transitively (jsonwebtoken -> p256/p384 -> elliptic-curve ->
# crypto-bigint), which RUSTSEC-2026-0387 flags for secret-dependent
# branches in constant-time operations. AWS-LC does not depend on
# crypto-bigint at all.
#
# `nasm` is only needed to assemble the x86-64 optimized code paths on
# Linux; AWS-LC falls back to a portable C implementation without it, so
# it is not installed here. `perl` and `clang` are not required either --
# the `cc` crate invokes the system C compiler, and the base image's
# gcc is sufficient.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
        cmake \
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
#
# This is also the daemon's WORKDIR (see below), so `state.db` is created
# here by default.
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

# WORKDIR must be a directory the unprivileged `fabric` user can write,
# because the daemon opens its SQLite database at a RELATIVE path
# ("state.db") resolved against the process working directory.
#
# Without this directive the working directory is `/`, which the
# `fabric` user cannot write, and every container died at startup with:
#
#   ERROR fabric_daemon: failed to create coordinator: database error:
#   database error: unable to open database file: state.db
#
# The image built fine, the binaries ran fine, and `docker run` still
# failed -- so a build-time or `--version` check cannot catch this. Only
# actually starting the daemon can. See the release workflow's
# "daemon starts and serves" assertions.
WORKDIR /var/lib/fabric

# Health check: send a health_check message over the wire protocol.
# `fabric-daemon health --connect` speaks the TCP wire protocol on port 9400.
# Note: the CLI (`fabric`) has no `health` subcommand and uses `--daemon`,
# not `--connect`; the daemon binary is the correct health probe.
#
# The probe is a one-shot request/response: it writes the health request,
# reads exactly one response line, and exits. It does not stream, because
# the daemon keeps the connection open for further messages -- a probe
# that waited for EOF would hang forever and every health check would
# time out. The probe itself enforces a 5s read timeout, so a daemon
# that stops answering fails the check instead of hanging it.
#
# The 15s timeout here is the outer backstop for the whole check (process
# start plus the round trip), and the 15s start period gives the daemon
# time to open its database and bind the port before the first probe is
# treated as meaningful.
HEALTHCHECK --interval=30s --timeout=15s --start-period=15s --retries=3 \
    CMD /usr/local/bin/fabric-daemon health --connect 127.0.0.1:9400 || exit 1

USER fabric

# The daemon binds the loopback interface by default (127.0.0.1:9400),
# which is correct for a local daemon but useless in a container: the
# port is unreachable from outside the container, so `docker run -p 9400:9400`
# publishes a port nothing is listening on and a CLI in a second
# container fails with "status check failed ... Broken pipe".
#
# The default here is overridden for the container by the CMD below, which
# listens on 0.0.0.0. Binding all interfaces inside the container is
# safe: the container's network namespace still isolates the daemon from
# the host, and the port is only reachable where the operator publishes
# it with `-p`.
ENTRYPOINT ["/usr/local/bin/fabric-daemon"]
CMD ["start", "--listen", "0.0.0.0:9400", "--db", "/var/lib/fabric/state.db"]
