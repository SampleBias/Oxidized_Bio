#!/bin/bash
# =============================================================================
# Oxidized Bio - Container Entrypoint Script
# =============================================================================
# This script runs when the container starts and handles:
#   1. Environment validation
#   2. SSH server setup (if enabled)
#   3. Database connection waiting
#   4. Redis connection checking
#   5. Directory permissions
#   6. Starting the main application
# =============================================================================

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo ""
echo "=========================================="
echo "  Oxidized Bio Container Starting"
echo "=========================================="
echo "  Version: ${APP_VERSION:-0.1.0}"
echo "  Environment: ${APP_ENV:-production}"
echo "  Timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo "=========================================="
echo ""

# =============================================================================
# Generate Container ID
# =============================================================================
if [ -z "$CONTAINER_ID" ]; then
    export CONTAINER_ID=$(cat /proc/sys/kernel/random/uuid 2>/dev/null | cut -d'-' -f1 || echo "unknown")
fi
log_info "Container ID: $CONTAINER_ID"

# =============================================================================
# Validate Required Environment Variables
# =============================================================================
log_info "Validating environment..."

MISSING_VARS=""

if [ -z "$BIOAGENTS_SECRET" ]; then
    MISSING_VARS="$MISSING_VARS BIOAGENTS_SECRET"
fi

if [ -n "$MISSING_VARS" ]; then
    log_warn "Missing recommended environment variables:$MISSING_VARS"
    log_warn "Some features may not work correctly."
fi

# =============================================================================
# Setup SSH Server (if enabled)
# =============================================================================
if [ "${ENABLE_SSH:-true}" = "true" ]; then
    log_info "Setting up SSH server..."
    if [ -x /app/setup-ssh.sh ]; then
        /app/setup-ssh.sh
        log_success "SSH server configured"
    else
        log_warn "SSH setup script not found or not executable"
    fi
else
    log_info "SSH server disabled (ENABLE_SSH=false)"
fi

# =============================================================================
# Wait for Database Connection
# =============================================================================
if [ -n "$DATABASE_URL" ]; then
    log_info "Checking database connection..."
    
    # Extract host and port from DATABASE_URL
    # Format: postgresql://user:pass@host:port/dbname
    DB_HOST=$(echo "$DATABASE_URL" | sed -n 's|.*@\([^:/]*\).*|\1|p')
    DB_PORT=$(echo "$DATABASE_URL" | sed -n 's|.*:\([0-9]*\)/.*|\1|p')
    DB_PORT=${DB_PORT:-5432}
    
    if [ -n "$DB_HOST" ]; then
        max_attempts=30
        attempt=0
        
        while ! nc -z "$DB_HOST" "$DB_PORT" 2>/dev/null; do
            attempt=$((attempt + 1))
            if [ $attempt -ge $max_attempts ]; then
                log_warn "Database connection timeout after ${max_attempts} attempts"
                log_warn "Continuing anyway - application will retry..."
                break
            fi
            log_info "Waiting for database at ${DB_HOST}:${DB_PORT}... (attempt $attempt/$max_attempts)"
            sleep 2
        done
        
        if [ $attempt -lt $max_attempts ]; then
            log_success "Database is reachable at ${DB_HOST}:${DB_PORT}"
        fi
    else
        log_warn "Could not parse database host from DATABASE_URL"
    fi
else
    log_warn "DATABASE_URL not set - running without database"
fi

# =============================================================================
# Check Redis Connection
# =============================================================================
if [ -n "$REDIS_URL" ]; then
    log_info "Checking Redis connection..."
    
    # Extract host and port from REDIS_URL
    # Format: redis://host:port or redis://user:pass@host:port
    REDIS_HOST=$(echo "$REDIS_URL" | sed -n 's|redis://\([^:@/]*\).*|\1|p')
    if [ -z "$REDIS_HOST" ]; then
        REDIS_HOST=$(echo "$REDIS_URL" | sed -n 's|redis://.*@\([^:]*\).*|\1|p')
    fi
    REDIS_PORT=$(echo "$REDIS_URL" | sed -n 's|.*:\([0-9]*\)$|\1|p')
    REDIS_PORT=${REDIS_PORT:-6379}
    
    if [ -n "$REDIS_HOST" ] && nc -z "$REDIS_HOST" "$REDIS_PORT" 2>/dev/null; then
        log_success "Redis is reachable at ${REDIS_HOST}:${REDIS_PORT}"
    else
        log_warn "Redis not available at ${REDIS_HOST:-unknown}:${REDIS_PORT}"
        log_warn "Job queue features may not work"
    fi
else
    log_info "REDIS_URL not set - job queue disabled"
fi

# =============================================================================
# Create Required Directories
# =============================================================================
log_info "Setting up directories..."

mkdir -p /app/data /app/logs /app/workspace /app/docs 2>/dev/null || true
chown -R oxidized:oxidized /app/data /app/logs /app/workspace /app/docs 2>/dev/null || true

# Ensure log directory is writable
touch /app/logs/.write-test 2>/dev/null && rm /app/logs/.write-test 2>/dev/null
if [ $? -eq 0 ]; then
    log_success "Directories configured"
else
    log_warn "Log directory may not be writable"
fi

# =============================================================================
# Print Configuration Summary
# =============================================================================
echo ""
echo "=========================================="
echo "  Configuration Summary"
echo "=========================================="
echo "  HTTP API Port: ${PORT:-3000}"
echo "  SSH Enabled: ${ENABLE_SSH:-true}"
echo "  RFC Enabled: ${RFC_ENABLED:-true}"
echo "  Auth Mode: ${AUTH_MODE:-none}"
echo "  Database: ${DATABASE_URL:+configured}"
echo "  Redis: ${REDIS_URL:+configured}"
echo "  Log Level: ${RUST_LOG:-info}"
echo "=========================================="
echo ""

# =============================================================================
# Start Main Application
# =============================================================================
log_info "Starting services..."
echo ""

# Execute the main command (typically supervisord)
exec "$@"
