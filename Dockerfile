# =============================================================================
# Oxidized Bio - Production Dockerfile
# =============================================================================
# Multi-stage build for minimal final image size while maintaining full
# Linux environment compatibility.
#
# Build:  docker build -t oxidized-bio .
# Run:    docker run -p 3000:3000 -p 2222:22 oxidized-bio
#
# Features:
#   - Multi-stage build (small final image)
#   - SSH server for RFC remote access
#   - Supervisor for process management
#   - Full Debian Bookworm Linux environment
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build Environment
# -----------------------------------------------------------------------------
# Using rust:bookworm to match runtime glibc version (debian:bookworm-slim)
FROM rust:bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libpq-dev \
    cmake \
    clang \
    llvm \
    libleptonica-dev \
    libtesseract-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies - copy only Cargo files first
# Note: Cargo.lock is optional (use * pattern to copy if exists)
COPY Cargo.toml Cargo.loc[k] ./

# Create dummy source to build dependencies (this caches them)
RUN mkdir src && \
    echo "fn main() {println!(\"Placeholder for dependency caching\")}" > src/main.rs && \
    echo "pub fn placeholder() {}" > src/lib.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src target/release/deps/oxidized* target/release/.fingerprint/oxidized*

# Copy actual source code
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release

# Verify the binary was built
RUN test -f /app/target/release/oxidized-bio && \
    echo "Binary built successfully: $(ls -lh /app/target/release/oxidized-bio)"

# -----------------------------------------------------------------------------
# Stage 2: Runtime Environment
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Labels for container metadata
LABEL org.opencontainers.image.title="Oxidized Bio"
LABEL org.opencontainers.image.description="High-performance AI agent framework for biological research"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.authors="Oxidized Bio Contributors"
LABEL org.opencontainers.image.source="https://github.com/s4mpl3bi4s/oxidized-bio"
LABEL org.opencontainers.image.licenses="MIT"

# Install runtime dependencies and tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Core runtime libraries
    ca-certificates \
    libssl3 \
    libpq5 \
    # OCR support (Tesseract)
    libleptonica-dev \
    libtesseract5 \
    tesseract-ocr-eng \
    tesseract-ocr-osd \
    # SSH Server for RFC remote access
    openssh-server \
    # Process supervision
    supervisor \
    # Networking tools
    curl \
    wget \
    dnsutils \
    iputils-ping \
    netcat-openbsd \
    # Development tools (for RFC shell access)
    git \
    vim-tiny \
    less \
    jq \
    procps \
    # Python (for helper scripts)
    python3-minimal \
    # Timezone data
    tzdata \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create application user (non-root for security)
RUN useradd -m -s /bin/bash -u 1000 oxidized && \
    # Create required directories
    mkdir -p /var/run/sshd && \
    mkdir -p /var/log/supervisor && \
    mkdir -p /etc/ssh/keys && \
    # Allow oxidized user to read logs
    chown -R oxidized:oxidized /var/log/supervisor

# Set timezone (configurable via TZ env var)
ENV TZ=UTC
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

WORKDIR /app

# Copy built binary from builder stage
COPY --from=builder /app/target/release/oxidized-bio /app/oxidized-bio
RUN chmod +x /app/oxidized-bio

# Copy migrations and configuration templates
COPY migrations ./migrations
COPY .env.example ./.env.example

# Copy Docker support files
COPY docker/supervisord.conf /etc/supervisor/conf.d/supervisord.conf
COPY docker/sshd_config /etc/ssh/sshd_config
COPY docker/setup-ssh.sh /app/setup-ssh.sh
COPY docker/entrypoint.sh /app/entrypoint.sh

# Make scripts executable
RUN chmod +x /app/setup-ssh.sh /app/entrypoint.sh

# Create data directories with proper permissions
RUN mkdir -p /app/data /app/docs /app/workspace /app/logs && \
    chown -R oxidized:oxidized /app

# Environment variables (defaults)
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV APP_ENV=production
ENV PORT=3000
ENV HOST=0.0.0.0
ENV ENABLE_SSH=true
ENV RFC_ENABLED=true

# Expose ports
# 3000 - HTTP API
# 22   - SSH for RFC remote access
EXPOSE 3000 22

# Health check - verify API is responding
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -sf http://localhost:3000/api/health || exit 1

# Entrypoint script handles initialization
ENTRYPOINT ["/app/entrypoint.sh"]

# Default command - run supervisor (manages all processes)
CMD ["/usr/bin/supervisord", "-n", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
