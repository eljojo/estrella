# ── Stage 1: Build the Preact/Vite frontend ──────────────────────────────────
FROM node:25-trixie-slim AS frontend
WORKDIR /app

# Cache npm deps before copying the full source tree
COPY frontend/package.json frontend/package-lock.json ./frontend/
RUN cd frontend && npm ci

# Vite components import fixture JSON files via a relative path that resolves
# to <project-root>/src/fixtures/ — copy them alongside the frontend source.
COPY src/fixtures/ ./src/fixtures/
COPY frontend/ ./frontend/

RUN cd frontend && npm run build


# ── Stage 2: Build the Rust binary ───────────────────────────────────────────
FROM rust:1-trixie AS builder
WORKDIR /app

# Build-time system libraries.
# HEIF/HEIC support is disabled: libheif-sys 5.2.0 requires libheif ≥ 1.21,
# but no standard distro ships that yet (Trixie has 1.19; the Nix build
# compiles 1.21.2 from source). Use --no-default-features to skip it.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

# ── Dependency caching layer ──────────────────────────────────────────────────
# Copy manifests + build script only; stub out the crate roots with minimal
# valid Rust so that `cargo build` fetches and compiles every third-party crate.
# This layer is only invalidated when Cargo.lock changes.
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src frontend/dist && \
    printf 'fn main() {}\n' > src/main.rs && \
    touch src/lib.rs
# Compile all dependencies (ignore the final link of the stubs — we don't care)
RUN cargo build --release --no-default-features || true

# ── Real build ────────────────────────────────────────────────────────────────
# Replace stubs with the real source tree and the pre-built frontend assets.
# Frontend assets are embedded into the binary at compile time via include_dir!
COPY src/ ./src/
COPY --from=frontend /app/frontend/dist ./frontend/dist/

# Touch crate roots so cargo detects they changed and recompiles project code.
# Third-party .rlib files from the caching layer above are reused unchanged.
RUN touch src/main.rs src/lib.rs

RUN cargo build --release --no-default-features

# ── Stage 3: Minimal runtime image ───────────────────────────────────────────
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/estrella /usr/local/bin/estrella

EXPOSE 8080

# ENTRYPOINT + CMD split lets docker-compose (or `docker run`) override just
# the flags without having to repeat `estrella serve`.
ENTRYPOINT ["estrella", "serve"]
CMD ["--listen", "0.0.0.0:8080", "--device", "/dev/rfcomm0"]
