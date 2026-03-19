# Multi-stage build for tabula-kb node
# Stage 1: Build
FROM rust:1.85-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release -p node 2>&1 && \
    strip target/release/node

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/node /usr/local/bin/tabula-node

# Data directory
VOLUME /data

# P2P port
EXPOSE 9000
# HTTP API port
EXPOSE 8080

ENV RUST_LOG=info

ENTRYPOINT ["tabula-node"]
CMD ["--data-dir", "/data", "--port", "9000", "--api-port", "8080"]
