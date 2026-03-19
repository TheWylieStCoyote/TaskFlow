#!/usr/bin/env bash
# install.sh — build and install TaskFlow
set -euo pipefail

BINARY_NAME="taskflow"
REQUIRED_RUST_MINOR="87"  # 1.87

# ── Flags ──────────────────────────────────────────────────────────────────────
SYSTEM_INSTALL=false
INSTALL_COMPLETIONS=""   # "" = auto-detect, "yes" = force, "no" = skip
UNINSTALL=false

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Build and install TaskFlow.

Options:
  --system           Install to /usr/local/bin (requires sudo)
  --completions      Install shell completions (auto-detects shell)
  --no-completions   Skip shell completion install
  --uninstall        Remove binary and completions from install location
  --help             Show this help

Default (no flags): install binary to ~/.local/bin
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --system)          SYSTEM_INSTALL=true ;;
        --completions)     INSTALL_COMPLETIONS="yes" ;;
        --no-completions)  INSTALL_COMPLETIONS="no" ;;
        --uninstall)       UNINSTALL=true ;;
        --help|-h)         usage; exit 0 ;;
        *) echo "Unknown option: $1"; usage; exit 1 ;;
    esac
    shift
done

# ── Paths ──────────────────────────────────────────────────────────────────────
if $SYSTEM_INSTALL; then
    BIN_DIR="/usr/local/bin"
    BASH_COMP_DIR="/etc/bash_completion.d"
    ZSH_COMP_DIR="/usr/local/share/zsh/site-functions"
    FISH_COMP_DIR="/usr/share/fish/completions"
    INSTALL_CMD="sudo"
else
    BIN_DIR="${HOME}/.local/bin"
    BASH_COMP_DIR="${HOME}/.local/share/bash-completion/completions"
    ZSH_COMP_DIR="${HOME}/.zfunc"
    FISH_COMP_DIR="${HOME}/.config/fish/completions"
    INSTALL_CMD=""
fi

BINARY_PATH="${BIN_DIR}/${BINARY_NAME}"

# ── Helpers ────────────────────────────────────────────────────────────────────
info()    { echo "  [info] $*"; }
success() { echo "  [ok]   $*"; }
warn()    { echo "  [warn] $*" >&2; }
error()   { echo "  [err]  $*" >&2; exit 1; }

run_install() {
    if [[ -n "$INSTALL_CMD" ]]; then
        sudo "$@"
    else
        "$@"
    fi
}

# ── Uninstall ──────────────────────────────────────────────────────────────────
if $UNINSTALL; then
    echo "Uninstalling TaskFlow..."
    removed=0

    for path in \
        "${BINARY_PATH}" \
        "${BASH_COMP_DIR}/${BINARY_NAME}" \
        "${ZSH_COMP_DIR}/_${BINARY_NAME}" \
        "${FISH_COMP_DIR}/${BINARY_NAME}.fish"
    do
        if [[ -e "$path" ]]; then
            run_install rm -f "$path"
            success "Removed $path"
            removed=$((removed + 1))
        fi
    done

    if [[ $removed -eq 0 ]]; then
        info "Nothing to remove — TaskFlow does not appear to be installed."
    else
        success "Uninstall complete."
    fi
    exit 0
fi

# ── Prerequisites ──────────────────────────────────────────────────────────────
echo "Checking prerequisites..."

if ! command -v cargo &>/dev/null; then
    error "cargo not found. Install Rust from https://rustup.rs and try again."
fi

if ! command -v rustc &>/dev/null; then
    error "rustc not found. Install Rust from https://rustup.rs and try again."
fi

RUST_VERSION_FULL=$(rustc --version)
# Extract "1.XX" from e.g. "rustc 1.87.0 (xxxxxx 2025-05-22)"
RUST_MINOR=$(echo "$RUST_VERSION_FULL" | grep -oP '(?<=1\.)\d+' | head -1)

if [[ -z "$RUST_MINOR" ]]; then
    warn "Could not parse Rust version from: $RUST_VERSION_FULL"
    warn "Continuing anyway — install may fail if Rust is too old."
elif [[ "$RUST_MINOR" -lt "$REQUIRED_RUST_MINOR" ]]; then
    error "Rust 1.${REQUIRED_RUST_MINOR} or later required (found $RUST_VERSION_FULL). Run: rustup update"
fi

success "Rust: $RUST_VERSION_FULL"

# ── Build ──────────────────────────────────────────────────────────────────────
echo ""
echo "Building TaskFlow (release)..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

cargo build --release

success "Build complete: target/release/${BINARY_NAME}"

# ── Install binary ─────────────────────────────────────────────────────────────
echo ""
echo "Installing binary..."

run_install mkdir -p "$BIN_DIR"
run_install cp "target/release/${BINARY_NAME}" "$BINARY_PATH"
run_install chmod 755 "$BINARY_PATH"
success "Installed: $BINARY_PATH"

# PATH warning
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    warn "${BIN_DIR} is not in \$PATH."
    warn "Add this to your shell rc file:"
    warn "  export PATH=\"${BIN_DIR}:\$PATH\""
fi

# ── Shell completions ──────────────────────────────────────────────────────────
install_bash_completion() {
    run_install mkdir -p "$BASH_COMP_DIR"
    "$BINARY_PATH" completion bash | run_install tee "${BASH_COMP_DIR}/${BINARY_NAME}" >/dev/null
    success "Bash completion: ${BASH_COMP_DIR}/${BINARY_NAME}"
    info "Reload your shell or run:"
    info "  source ${BASH_COMP_DIR}/${BINARY_NAME}"
}

install_zsh_completion() {
    run_install mkdir -p "$ZSH_COMP_DIR"
    "$BINARY_PATH" completion zsh | run_install tee "${ZSH_COMP_DIR}/_${BINARY_NAME}" >/dev/null
    success "Zsh completion: ${ZSH_COMP_DIR}/_${BINARY_NAME}"
    if ! grep -q "fpath.*${ZSH_COMP_DIR}" "${ZDOTDIR:-$HOME}/.zshrc" 2>/dev/null; then
        info "Add this to ~/.zshrc if ${ZSH_COMP_DIR} is not already in fpath:"
        info "  fpath=(${ZSH_COMP_DIR} \$fpath)"
        info "  autoload -Uz compinit && compinit"
    fi
}

install_fish_completion() {
    run_install mkdir -p "$FISH_COMP_DIR"
    "$BINARY_PATH" completion fish | run_install tee "${FISH_COMP_DIR}/${BINARY_NAME}.fish" >/dev/null
    success "Fish completion: ${FISH_COMP_DIR}/${BINARY_NAME}.fish"
    info "Fish will pick up completions automatically on next launch."
}

echo ""

if [[ "$INSTALL_COMPLETIONS" == "no" ]]; then
    info "Skipping shell completions (--no-completions)."
else
    if [[ "$INSTALL_COMPLETIONS" == "yes" ]]; then
        # Force: detect shell and install
        detect_and_install=true
    else
        # Auto-detect: only install if shell is known
        detect_and_install=true
    fi

    if $detect_and_install; then
        echo "Installing shell completions..."
        case "${SHELL:-}" in
            */bash) install_bash_completion ;;
            */zsh)  install_zsh_completion ;;
            */fish) install_fish_completion ;;
            *)
                if [[ "$INSTALL_COMPLETIONS" == "yes" ]]; then
                    warn "Could not detect shell from \$SHELL='${SHELL:-}'."
                    warn "Run one of:"
                    warn "  bash install.sh --completions  (from a bash session)"
                    warn "  zsh  install.sh --completions  (from a zsh session)"
                    warn "  fish install.sh --completions  (from a fish session)"
                else
                    info "Shell not detected; skipping completions."
                    info "Re-run with --completions to install them."
                fi
                ;;
        esac
    fi
fi

# ── Summary ────────────────────────────────────────────────────────────────────
echo ""
echo "────────────────────────────────────────"
echo " TaskFlow installed successfully"
echo "────────────────────────────────────────"
echo " Binary : $BINARY_PATH"
echo ""
echo " Run 'taskflow --help' to get started."
echo "────────────────────────────────────────"
