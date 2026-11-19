# syntax=docker/dockerfile:1
#
# One Dockerfile, two images: `--target web` (default) and `--target worker`.
# Build:  docker build --build-arg CARGO_CHEF_VERSION=… --build-arg CARGO_AUDITABLE_VERSION=… .
# (tool versions have no default on purpose: CI passes the pinned values, Renovate bumps them).
#
# Notes
# * CSS/JS/fonts are committed under crates/ui/assets and embedded at compile time, so the build
#   needs no Tailwind CLI and no `site/` directory; `just css-check` in CI guarantees the
#   committed CSS is current (deviation from INFRASTRUCTURE §3, see docs/INFRASTRUCTURE.md).
# * Release binaries are built with cargo-auditable so Trivy can see the exact Rust dependency set
#   inside the shipped binary, not just the OS packages.
# * No secrets: nothing is passed as a build arg except tool versions, the commit SHA and the
#   reproducibility timestamp.

# renovate: datasource=github-releases depName=rust-lang/rust
ARG RUST_VERSION=1.97.0

# ---- chef: toolchain + build tools ------------------------------------------------------------
FROM rust:${RUST_VERSION}-bookworm AS chef
ARG CARGO_CHEF_VERSION
ARG CARGO_AUDITABLE_VERSION
RUN test -n "${CARGO_CHEF_VERSION}" && test -n "${CARGO_AUDITABLE_VERSION}" \
 && cargo install --locked cargo-chef --version "${CARGO_CHEF_VERSION}" \
 && cargo install --locked cargo-auditable --version "${CARGO_AUDITABLE_VERSION}"
WORKDIR /app

# ---- planner: dependency recipe (changes only when manifests/lockfile change) -------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---- builder: cached dependency layer, then the workspace binaries ------------------------------
FROM chef AS builder
ARG SOURCE_DATE_EPOCH=0
ENV SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH} \
    RUSTFLAGS="--remap-path-prefix=/app=/src --remap-path-prefix=/usr/local/cargo=/cargo"
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json
COPY . .
RUN cargo auditable build --release --locked -p web -p worker

# ---- worker image ------------------------------------------------------------------------------
FROM gcr.io/distroless/cc-debian12:nonroot AS worker
ARG VCS_REF=unknown
LABEL org.opencontainers.image.title="aao-worker" \
      org.opencontainers.image.source="https://github.com/nhattruong0204/affiliate-all-in-one" \
      org.opencontainers.image.revision="${VCS_REF}"
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/worker"]

# ---- web image (default target; keep last) -----------------------------------------------------
FROM gcr.io/distroless/cc-debian12:nonroot AS web
ARG VCS_REF=unknown
LABEL org.opencontainers.image.title="aao-web" \
      org.opencontainers.image.source="https://github.com/nhattruong0204/affiliate-all-in-one" \
      org.opencontainers.image.revision="${VCS_REF}"
COPY --from=builder /app/target/release/web /usr/local/bin/web
# The container network must reach the listeners; publishing is a deployment decision
# (never publish 9100: it serves /metrics, see docs/OBSERVABILITY.md §7).
ENV WEB_BIND=0.0.0.0:8080 \
    METRICS_BIND=0.0.0.0:9100
EXPOSE 8080
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/web"]
