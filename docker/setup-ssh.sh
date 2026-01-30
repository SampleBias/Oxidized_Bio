#!/bin/bash
# =============================================================================
# Oxidized Bio - SSH Server Setup Script
# =============================================================================
# Configures the OpenSSH server for RFC remote access.
#
# Features:
#   - Generates host keys if not present
#   - Sets up user authentication (password or key-based)
#   - Configures secure SSH settings
#   - Persists keys across container restarts (via mounted volume)
#
# Environment Variables:
#   SSH_PASSWORD - Password for the 'oxidized' user
#   SSH_AUTHORIZED_KEYS - Public keys for key-based auth (optional)
# =============================================================================

set -e

log_info() {
    echo "[SSH] $1"
}

log_success() {
    echo "[SSH] ✓ $1"
}

log_error() {
    echo "[SSH] ✗ $1" >&2
}

# =============================================================================
# Create SSH Directory for User
# =============================================================================
log_info "Creating SSH directory for user 'oxidized'..."
mkdir -p /home/oxidized/.ssh
chmod 700 /home/oxidized/.ssh

# =============================================================================
# Generate Host Keys (if not present)
# =============================================================================
# Host keys are stored in /etc/ssh/keys (mounted volume) for persistence
KEY_DIR="/etc/ssh/keys"
mkdir -p "$KEY_DIR"

# RSA Host Key
if [ ! -f "$KEY_DIR/ssh_host_rsa_key" ]; then
    log_info "Generating RSA host key..."
    ssh-keygen -t rsa -b 4096 -f "$KEY_DIR/ssh_host_rsa_key" -N '' -q
    log_success "RSA host key generated"
else
    log_info "Using existing RSA host key"
fi

# ECDSA Host Key
if [ ! -f "$KEY_DIR/ssh_host_ecdsa_key" ]; then
    log_info "Generating ECDSA host key..."
    ssh-keygen -t ecdsa -b 521 -f "$KEY_DIR/ssh_host_ecdsa_key" -N '' -q
    log_success "ECDSA host key generated"
else
    log_info "Using existing ECDSA host key"
fi

# ED25519 Host Key
if [ ! -f "$KEY_DIR/ssh_host_ed25519_key" ]; then
    log_info "Generating ED25519 host key..."
    ssh-keygen -t ed25519 -f "$KEY_DIR/ssh_host_ed25519_key" -N '' -q
    log_success "ED25519 host key generated"
else
    log_info "Using existing ED25519 host key"
fi

# Create symlinks to standard locations
ln -sf "$KEY_DIR/ssh_host_rsa_key" /etc/ssh/ssh_host_rsa_key
ln -sf "$KEY_DIR/ssh_host_rsa_key.pub" /etc/ssh/ssh_host_rsa_key.pub
ln -sf "$KEY_DIR/ssh_host_ecdsa_key" /etc/ssh/ssh_host_ecdsa_key
ln -sf "$KEY_DIR/ssh_host_ecdsa_key.pub" /etc/ssh/ssh_host_ecdsa_key.pub
ln -sf "$KEY_DIR/ssh_host_ed25519_key" /etc/ssh/ssh_host_ed25519_key
ln -sf "$KEY_DIR/ssh_host_ed25519_key.pub" /etc/ssh/ssh_host_ed25519_key.pub

# Set correct permissions on host keys
chmod 600 "$KEY_DIR"/ssh_host_*_key
chmod 644 "$KEY_DIR"/ssh_host_*_key.pub

# =============================================================================
# Configure User Password
# =============================================================================
if [ -n "$SSH_PASSWORD" ]; then
    log_info "Setting password for user 'oxidized'..."
    echo "oxidized:$SSH_PASSWORD" | chpasswd
    log_success "Password configured"
else
    # Generate a random password if not provided
    RANDOM_PASS=$(head -c 32 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 16)
    echo "oxidized:$RANDOM_PASS" | chpasswd
    log_info "Generated random password for user 'oxidized'"
    log_info "Password: $RANDOM_PASS"
    log_info "Set SSH_PASSWORD environment variable to use a specific password"
fi

# =============================================================================
# Configure Authorized Keys (for key-based auth)
# =============================================================================
if [ -n "$SSH_AUTHORIZED_KEYS" ]; then
    log_info "Setting up authorized keys..."
    echo "$SSH_AUTHORIZED_KEYS" > /home/oxidized/.ssh/authorized_keys
    chmod 600 /home/oxidized/.ssh/authorized_keys
    log_success "Authorized keys configured"
fi

# =============================================================================
# Set Ownership
# =============================================================================
chown -R oxidized:oxidized /home/oxidized/.ssh

# =============================================================================
# Verify Configuration
# =============================================================================
log_info "Verifying SSH configuration..."

# Test sshd config
if sshd -t 2>/dev/null; then
    log_success "SSH configuration is valid"
else
    log_error "SSH configuration has errors!"
    sshd -t
    exit 1
fi

# =============================================================================
# Print SSH Info
# =============================================================================
echo ""
echo "=========================================="
echo "  SSH Server Configuration"
echo "=========================================="
echo "  User: oxidized"
echo "  Port: 22 (container) -> mapped to host"
echo "  Password Auth: enabled"
echo "  Key Auth: ${SSH_AUTHORIZED_KEYS:+enabled}${SSH_AUTHORIZED_KEYS:-disabled}"
echo ""
echo "  Connect with: ssh oxidized@<host> -p <port>"
echo "=========================================="
echo ""

log_success "SSH server setup complete"
