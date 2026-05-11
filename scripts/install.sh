#!/usr/bin/env bash
# =============================================================================
# Xavier2 Installer — Linux/macOS one-liner
# =============================================================================
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/iberi22/xavier/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/iberi22/xavier/main/install.sh | bash -s -- --version v0.6.0
#   curl -fsSL https://raw.githubusercontent.com/iberi22/xavier/main/install.sh | bash -s -- --dir ~/xavier2
# =============================================================================
set -euo pipefail

# ── Configuration ──────────────────────────────────────────────
REPO="iberi22/xavier"
DEFAULT_VERSION="latest"
INSTALL_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/xavier2"
DATA_DIR="${HOME}/.local/share/xavier2"
SERVICE_NAME="xavier2"
GITHUB_API="https://api.github.com/repos/${REPO}"
RAW_URL="https://raw.githubusercontent.com/${REPO}/main"

# ── Colors ─────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# ── Helpers ────────────────────────────────────────────────────
info()    { echo -e "${CYAN}ℹ${NC} $*"; }
success() { echo -e "${GREEN}✓${NC} $*"; }
warn()    { echo -e "${YELLOW}⚠${NC} $*"; }
error()   { echo -e "${RED}✗${NC} $*"; }
header()  { echo -e "\n${BOLD}${CYAN}█${NC}${BOLD} $*${NC}"; }

banner() {
    echo -e "${CYAN}${BOLD}"
    echo "  ██╗  ██╗ █████╗ ██╗   ██╗██╗███████╗██████╗ "
    echo "  ╚██╗██╔╝██╔══██╗██║   ██║██║██╔════╝██╔══██╗"
    echo "   ╚███╔╝ ███████║██║   ██║██║█████╗  ██████╔╝"
    echo "   ██╔██╗ ██╔══██║╚██╗ ██╔╝██║██╔══╝  ██╔══██╗"
    echo "  ██╔╝ ██╗██║  ██║ ╚████╔╝ ██║███████╗██║  ██║"
    echo "  ╚═╝  ╚═╝╚═╝  ╚═╝  ╚═══╝  ╚═╝╚══════╝╚═╝  ╚═╝"
    echo -e "${NC}"
    echo "  Cognitive Memory Runtime for AI Agents"
    echo ""
}

# ── Argument parsing ───────────────────────────────────────────
VERSION="$DEFAULT_VERSION"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --version|-v)
            VERSION="$2"; shift 2 ;;
        --dir|-d)
            INSTALL_DIR="$2"; shift 2 ;;
        --config-dir)
            CONFIG_DIR="$2"; shift 2 ;;
        --data-dir)
            DATA_DIR="$2"; shift 2 ;;
        --no-service)
            NO_SERVICE=true; shift ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --version, -v VERSION   Install specific version (default: latest)"
            echo "  --dir, -d DIR           Install directory (default: ~/.local/bin)"
            echo "  --config-dir DIR        Config directory (default: ~/.config/xavier2)"
            echo "  --data-dir DIR          Data directory (default: ~/.local/share/xavier2)"
            echo "  --no-service            Skip systemd service setup"
            echo "  --help, -h              Show this help"
            exit 0 ;;
        *)
            error "Unknown option: $1"
            exit 1 ;;
    esac
done

# ── Platform detection ─────────────────────────────────────────
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)  PLATFORM="linux" ;;
    Darwin) PLATFORM="darwin" ;;
    *)
        error "Unsupported OS: $OS. Xavier2 currently supports Linux and macOS."
        exit 1 ;;
esac

case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64)
        warn "ARM64 support is experimental."
        ARCH="aarch64" ;;
    *)
        error "Unsupported architecture: $ARCH"
        exit 1 ;;
esac

# Binary archive name pattern from GitHub Releases
if [ "$PLATFORM" = "linux" ]; then
    TARGET_TRIPLE="${ARCH}-unknown-linux-gnu"
    BINARY_EXT=""
else
    TARGET_TRIPLE="${ARCH}-apple-darwin"
    BINARY_EXT=""
fi

# ── Main ───────────────────────────────────────────────────────
banner

# Resolve version
if [ "$VERSION" = "latest" ]; then
    info "Fetching latest version..."
    VERSION=$(curl -fsSL "${GITHUB_API}/releases/latest" 2>/dev/null | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        error "Could not determine latest version. Try specifying one with --version"
        exit 1
    fi
fi
success "Version: v${VERSION}"

# Create directories
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR"

# Download URL
TAG="v${VERSION}"
ARCHIVE="xavier-v${VERSION}-${TARGET_TRIPLE}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"

header "Downloading Xavier2 v${VERSION}..."
info "URL: $DOWNLOAD_URL"

TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

if ! curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$TMP_DIR/$ARCHIVE"; then
    error "Download failed. Check that version v${VERSION} exists and has a release for ${TARGET_TRIPLE}."
    error "Available releases: https://github.com/${REPO}/releases"
    exit 1
fi
success "Downloaded"

header "Extracting..."
tar xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"
EXTRACT_DIR="$TMP_DIR/xavier-v${VERSION}-${TARGET_TRIPLE}"

if [ ! -d "$EXTRACT_DIR" ]; then
    # Try alternative directory name pattern
    EXTRACT_DIR=$(find "$TMP_DIR" -maxdepth 1 -type d -name "xavier*" | head -1)
fi

if [ ! -d "$EXTRACT_DIR" ]; then
    error "Could not find extracted directory"
    ls -la "$TMP_DIR"
    exit 1
fi

# Install binaries
header "Installing..."
for BIN in xavier xavier-installer; do
    SRC="$EXTRACT_DIR/$BIN"
    if [ -f "$SRC" ]; then
        cp "$SRC" "$INSTALL_DIR/$BIN"
        chmod +x "$INSTALL_DIR/$BIN"
        success "Installed $INSTALL_DIR/$BIN"
    else
        warn "$BIN not found in archive (optional)"
    fi
done

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qF "$INSTALL_DIR"; then
    warn ""
    warn "  ⚠  $INSTALL_DIR is not in your PATH"
    warn ""
    warn "  Add this to your shell config (~/.bashrc, ~/.zshrc, etc.):"
    warn "    export PATH=\"$INSTALL_DIR:\$PATH\""
    warn ""
    SHELL_CONFIG=""
    case "$SHELL" in
        */bash) SHELL_CONFIG="$HOME/.bashrc" ;;
        */zsh)  SHELL_CONFIG="$HOME/.zshrc" ;;
        */fish) SHELL_CONFIG="$HOME/.config/fish/config.fish" ;;
    esac
    if [ -n "$SHELL_CONFIG" ]; then
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$SHELL_CONFIG"
        success "Added to $SHELL_CONFIG"
    fi
fi

# ── systemd service (Linux only) ────────────────────────────────
if [ "$OS" = "Linux" ] && [ "${NO_SERVICE:-false}" != "true" ]; then
    header "Setting up systemd service..."
    
    SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
    USER_SERVICE_DIR="${HOME}/.config/systemd/user"
    USER_SERVICE_FILE="${USER_SERVICE_DIR}/${SERVICE_NAME}.service"
    
    # Try system-wide first, fall back to user service
    if [ -w "/etc/systemd/system" ] || [ "$(id -u)" = "0" ]; then
        USE_SYSTEM=true
        SERVICE_PATH="$SERVICE_FILE"
    else
        USE_SYSTEM=false
        mkdir -p "$USER_SERVICE_DIR"
        SERVICE_PATH="$USER_SERVICE_FILE"
    fi
    
    cat > "$SERVICE_PATH" << SERVICE_EOF
[Unit]
Description=Xavier2 Cognitive Memory Runtime
Documentation=https://github.com/iberi22/xavier
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/xavier serve
Restart=on-failure
RestartSec=5
Environment="XAVIER_CONFIG_PATH=${CONFIG_DIR}/xavier2.config.json"
Environment="XAVIER_DATA_DIR=${DATA_DIR}"

# Security hardening
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=${DATA_DIR} ${CONFIG_DIR}
ReadOnlyPaths=${INSTALL_DIR}

# Resource limits
LimitNOFILE=4096
MemoryMax=512M

[Install]
WantedBy=multi-user.target
SERVICE_EOF
    
    if [ "$USE_SYSTEM" = true ]; then
        systemctl daemon-reload
        success "Systemd service created: $SERVICE_PATH"
        echo ""
        info "Start Xavier2:"
        echo "  sudo systemctl enable --now ${SERVICE_NAME}"
        echo "  sudo systemctl status ${SERVICE_NAME}"
    else
        systemctl --user daemon-reload 2>/dev/null || true
        success "User systemd service created: $SERVICE_PATH"
        echo ""
        info "Start Xavier2:"
        echo "  systemctl --user enable --now ${SERVICE_NAME}"
        echo "  systemctl --user status ${SERVICE_NAME}"
    fi
fi

# ── Configuration ──────────────────────────────────────────────
header "Configuration"

if command -v xavier-installer >/dev/null 2>&1; then
    echo ""
    info "Run the interactive setup wizard to configure Xavier2:"
    echo "  ${BOLD}xavier-installer${NC}"
    echo ""
else
    info "No installer binary found. You can configure manually in ${CONFIG_DIR}/xavier2.config.json"
fi

# ── Done ───────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}${BOLD}╔══════════════════════════════════════════╗${NC}"
echo -e "${GREEN}${BOLD}║        Xavier2 Installation Complete!    ║${NC}"
echo -e "${GREEN}${BOLD}╚══════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${BOLD}Binary:${NC}  ${INSTALL_DIR}/xavier"
echo -e "  ${BOLD}Config:${NC}  ${CONFIG_DIR}/xavier2.config.json"
echo -e "  ${BOLD}Data:${NC}    ${DATA_DIR}"
echo ""
echo -e "  ${BOLD}Quick start:${NC}"
echo "    xavier-installer       # Run setup wizard"
echo "    xavier serve           # Start memory server"
echo "    xavier tui             # Launch dashboard"
echo "    xavier save -k episodic \"text\"  # Save memory"
echo ""
echo -e "  ${BOLD}Docs:${NC} https://github.com/iberi22/xavier"
echo ""
