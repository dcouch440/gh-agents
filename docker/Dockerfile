# =============================================================================
# nexor Docker Build
# Multi-stage build for minimal runtime image
# =============================================================================

# =============================================================================
# Stage 1: Builder
# =============================================================================
FROM rust:1.75-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "// placeholder" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN touch src/main.rs && cargo build --release

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 nexor

# Create data directory
RUN mkdir -p /data/.nexor && chown -R nexor:nexor /data

# Copy binary from builder
COPY --from=builder /app/target/release/nexor /usr/local/bin/nexor

# Set ownership
RUN chown nexor:nexor /usr/local/bin/nexor && chmod +x /usr/local/bin/nexor

# Switch to non-root user
USER nexor

# Set working directory
WORKDIR /project

# Environment variables
ENV RUST_LOG=info
ENV NEXOR_DATA_DIR=/data/.nexor

# Default command shows help
ENTRYPOINT ["nexor"]
CMD ["--help"]
