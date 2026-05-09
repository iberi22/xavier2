#!/bin/bash
# scripts/hooks/install-hooks.sh
# Git hooks installer for repository guardrails.
#
# This script installs the pre-commit hook that enforces atomic commits,
# release guardrails, and documentation discipline.
#
# Usage:
#   ./scripts/hooks/install-hooks.sh
#   ./scripts/hooks/install-hooks.sh --uninstall
#   ./scripts/hooks/install-hooks.sh --check

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

to_unix_path() {
    local input="$1"
    if [[ "$input" =~ ^([A-Za-z]):/(.*)$ ]]; then
        local drive="${BASH_REMATCH[1],,}"
        local remainder="${BASH_REMATCH[2]}"
        echo "/mnt/${drive}/${remainder}"
    else
        echo "$input"
    fi
}

resolve_hooks_dir() {
    if [ -d "$REPO_ROOT/.git/hooks" ]; then
        echo "$REPO_ROOT/.git/hooks"
        return
    fi

    if [ -f "$REPO_ROOT/.git" ]; then
        local gitdir
        gitdir=$(sed -n 's/^gitdir:[[:space:]]*//p' "$REPO_ROOT/.git" | head -1)
        gitdir=$(to_unix_path "$gitdir")
        if [ -n "$gitdir" ] && [ -d "$gitdir" ]; then
            local common_git_dir
            common_git_dir="$(cd "$gitdir/../.." && pwd)"
            echo "$common_git_dir/hooks"
            return
        fi
    fi

    echo ""
}

GIT_HOOKS_DIR="$(resolve_hooks_dir)"

# Parse arguments
ACTION="install"
for arg in "$@"; do
    case $arg in
        --uninstall|-u)
            ACTION="uninstall"
            ;;
        --check|-c)
            ACTION="check"
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --install, -i    Install git hooks (default)"
            echo "  --uninstall, -u  Remove installed hooks"
            echo "  --check, -c      Check if hooks are installed"
            echo "  --help, -h       Show this help message"
            exit 0
            ;;
    esac
done

check_hooks() {
    if [ -f "$GIT_HOOKS_DIR/pre-commit" ]; then
        if grep -q "git-core-protocol" "$GIT_HOOKS_DIR/pre-commit" 2>/dev/null; then
            echo -e "${GREEN}✓ Repository guardrail pre-commit hook is installed${NC}"
            return 0
        else
            echo -e "${YELLOW}⚠️  A different pre-commit hook is installed${NC}"
            return 1
        fi
    else
        echo -e "${YELLOW}○ Pre-commit hook is not installed${NC}"
        return 1
    fi
}

install_hooks() {
    echo -e "${CYAN}Installing repository guardrail hooks...${NC}"

    # Check if this is a git repository, including worktrees where .git is a file.
    if [ ! -d "$REPO_ROOT/.git" ] && [ ! -f "$REPO_ROOT/.git" ]; then
        echo -e "${RED}❌ Error: Not a git repository${NC}"
        exit 1
    fi

    if [ -z "$GIT_HOOKS_DIR" ]; then
        echo -e "${RED}❌ Error: Could not resolve git hooks directory${NC}"
        exit 1
    fi

    # Create hooks directory if it doesn't exist
    mkdir -p "$GIT_HOOKS_DIR"

    # Check for existing pre-commit hook
    if [ -f "$GIT_HOOKS_DIR/pre-commit" ]; then
        if grep -q "git-core-protocol" "$GIT_HOOKS_DIR/pre-commit" 2>/dev/null; then
            echo -e "${YELLOW}⚠️  Hook already installed, updating...${NC}"
        else
            # Backup existing hook
            cp "$GIT_HOOKS_DIR/pre-commit" "$GIT_HOOKS_DIR/pre-commit.backup.$(date +%s)"
            echo -e "${YELLOW}⚠️  Existing pre-commit hook backed up${NC}"
        fi
    fi

    # Create wrapper script that calls our hook
    cat > "$GIT_HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Repository guardrail pre-commit hook (git-core-protocol)
# This hook validates atomic commits and release guardrails.
# Bypass with: git commit --no-verify

# Get repository root
REPO_ROOT="$(git rev-parse --show-toplevel)"
ATOMIZE_HOOK="$REPO_ROOT/scripts/hooks/pre-commit"

# Run the hook script if it exists
if [ -f "$ATOMIZE_HOOK" ] && [ -x "$ATOMIZE_HOOK" ]; then
    exec "$ATOMIZE_HOOK"
elif [ -f "$ATOMIZE_HOOK" ]; then
    exec bash "$ATOMIZE_HOOK"
else
    # Hook script not found, skip validation
    echo "Note: scripts/hooks/pre-commit not found, skipping atomicity check"
    exit 0
fi
EOF

    chmod +x "$GIT_HOOKS_DIR/pre-commit" 2>/dev/null || true
    chmod +x "$SCRIPT_DIR/pre-commit" 2>/dev/null || true

    echo -e "${GREEN}✅ Repository guardrail hooks installed successfully${NC}"
    echo ""
    echo -e "${CYAN}Configuration:${NC}"
    echo "  • Edit .git-atomize.yml to tune commit-size rules"
    echo "  • Public surface changes must ship with docs updates"
    echo "  • Insecure defaults, placeholders, and repo artifacts are blocked"
    echo "  • Bypass with: git commit --no-verify"
}

uninstall_hooks() {
    echo -e "${CYAN}Uninstalling repository guardrail hooks...${NC}"

    if [ -f "$GIT_HOOKS_DIR/pre-commit" ]; then
        if grep -q "git-core-protocol" "$GIT_HOOKS_DIR/pre-commit" 2>/dev/null; then
            rm "$GIT_HOOKS_DIR/pre-commit"
            echo -e "${GREEN}✓ Pre-commit hook removed${NC}"

            # Restore backup if exists
            LATEST_BACKUP=$(ls -t "$GIT_HOOKS_DIR/pre-commit.backup."* 2>/dev/null | head -1)
            if [ -n "$LATEST_BACKUP" ]; then
                mv "$LATEST_BACKUP" "$GIT_HOOKS_DIR/pre-commit"
                echo -e "${GREEN}✓ Previous hook restored from backup${NC}"
            fi
        else
            echo -e "${YELLOW}⚠️  The installed hook is not from Git-Core Protocol${NC}"
            echo -e "   Remove manually if desired: rm $GIT_HOOKS_DIR/pre-commit"
        fi
    else
        echo -e "${YELLOW}○ No pre-commit hook installed${NC}"
    fi

    echo -e "${GREEN}✅ Repository guardrail hooks uninstalled${NC}"
}

# Execute based on action
case "$ACTION" in
    install)
        install_hooks
        ;;
    uninstall)
        uninstall_hooks
        ;;
    check)
        check_hooks
        ;;
esac
