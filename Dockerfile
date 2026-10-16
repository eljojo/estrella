FROM node:25-trixie-slim AS frontend
WORKDIR /app

COPY frontend/package.json frontend/package-lock.json ./frontend/
RUN cd frontend && npm ci

# Vite imports fixtures via a path relative to the project root
COPY src/fixtures/ ./src/fixtures/
COPY frontend/ ./frontend/
RUN cd frontend && npm run build


FROM rust:1-trixie AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Dependency caching layer — invalidated only when Cargo.lock changes.
# Built with --no-default-features: libheif-sys requires libheif ≥ 1.21,
# which no standard distro ships yet.
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src frontend/dist && \
    printf 'fn main() {}\n' > src/main.rs && \
    touch src/lib.rs
RUN cargo build --release --no-default-features || true

COPY src/ ./src/
COPY --from=frontend /app/frontend/dist ./frontend/dist/
RUN touch src/main.rs src/lib.rs
RUN cargo build --release --no-default-features


FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/estrella /usr/local/bin/estrella

EXPOSE 8080

ENTRYPOINT ["estrella", "serve"]
CMD ["--listen", "0.0.0.0:8080", "--device", "/dev/rfcomm0"]
