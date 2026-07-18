# Multi-stage Docker build for PyReverseETL
# Supports: AMD64 (servers), ARM64 (Mac Studio, newer servers)

# ============================================================
# Stage 1: Builder (compiles Rust + Python)
# ============================================================
FROM --platform=$BUILDPLATFORM rust:latest AS builder

# Install build dependencies for both platforms
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    python3-dev \
    python3-pip \
    maturin \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY python ./python

# Build optimized Rust core
RUN cargo build --release --target-dir /build/target

# Build Python extension
WORKDIR /build/python
RUN pip install maturin wheel
RUN maturin build --release --sdist

# ============================================================
# Stage 2: Runtime (minimal image with just the binary)
# ============================================================
FROM --platform=$TARGETPLATFORM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    python3 \
    python3-pip \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /build/target/release/pyreverseetl /usr/local/bin/

# Copy Python extension
COPY --from=builder /build/python/target/wheels /tmp/wheels

# Install Python extension
RUN pip install --no-cache-dir /tmp/wheels/*.whl && \
    rm -rf /tmp/wheels

# Create non-root user
RUN useradd -m -u 1000 pyreverseetl
USER pyreverseetl

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# API port (configurable)
EXPOSE 8080

# Volumes for persistent data
VOLUME ["/app/data", "/app/config"]

# Default entry point
ENTRYPOINT ["pyreverseetl"]
CMD ["server", "--port", "8080"]

# Build arguments for optimization
ARG BUILDKIT_INLINE_CACHE=1
