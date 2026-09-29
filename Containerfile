# ==========================================
# Build Stage
# ==========================================
FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/lotus_connect

# Install dependencies needed for compilation
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy full workspace source
COPY . .

# Build release binary
RUN cargo build --release --bin lotus_connect_system

# ==========================================
# Runtime Stage
# ==========================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled binary from builder
COPY --from=builder /usr/src/lotus_connect/target/release/lotus_connect_system /app/lotus_connect_system

# Copy migrations folder for runtime auto-migrations
COPY --from=builder /usr/src/lotus_connect/migrations /app/migrations

# Create upload directory
RUN mkdir -p /app/uploads

EXPOSE 8080

CMD ["/app/lotus_connect_system"]
