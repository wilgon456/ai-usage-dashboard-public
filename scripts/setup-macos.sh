#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

log() {
  printf '\n[setup-macos] %s\n' "$1"
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

brew_install_if_missing() {
  local formula="$1"
  if brew list "$formula" >/dev/null 2>&1; then
    return 0
  fi

  brew install "$formula"
}

brew_install_cask_if_missing() {
  local formula="$1"
  if brew list --cask "$formula" >/dev/null 2>&1; then
    return 0
  fi

  brew install --cask "$formula"
}

ensure_brew_shellenv() {
  if [[ -x /opt/homebrew/bin/brew ]]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
    return 0
  fi

  if [[ -x /usr/local/bin/brew ]]; then
    eval "$(/usr/local/bin/brew shellenv)"
    return 0
  fi

  return 1
}

if ! xcode-select -p >/dev/null 2>&1; then
  log "Xcode Command Line Tools are required for Tauri builds."
  xcode-select --install || true
  log "Finish the Xcode Command Line Tools installation, then rerun this script."
  exit 1
fi

if ! has_cmd brew; then
  log "Installing Homebrew"
  NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
fi

ensure_brew_shellenv || {
  log "Homebrew was installed but shellenv could not be loaded."
  exit 1
}

if ! has_cmd node || ! has_cmd npm; then
  log "Installing Node.js"
  brew install node
fi

if ! has_cmd rustup; then
  log "Installing Rust via rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"
fi

if ! has_cmd cargo || ! has_cmd rustc; then
  log "Rust installation did not finish cleanly."
  exit 1
fi

log "Selecting the stable Rust toolchain"
rustup default stable

log "Installing provider CLIs"
brew_install_if_missing gh
brew_install_cask_if_missing claude-code
if ! has_cmd codex; then
  npm install -g @openai/codex
fi

log "Installing npm workspace dependencies"
npm install

log "Running doctor"
npm run doctor

log "Running smoke checks"
npm run smoke

log "Setup complete. Next command: npm run dev:tauri"
