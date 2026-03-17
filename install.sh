#!/bin/bash
# OMEGA CLI installer
# Usage: curl -fsSL https://omgagi.ai/install.sh | bash
#
# Environment variables:
#   OMG_INSTALL_DIR  — override install location (default: ~/.local/bin)
#   OMG_REPO         — override GitHub repo (default: omgagi/omega-cli)

set -e

REPO="${OMG_REPO:-omgagi/omega-cli}"
INSTALL_DIR="${OMG_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_NAME="omg"

# --- Platform detection -------------------------------------------------------

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "${os}" in
        Darwin) os="apple-darwin" ;;
        Linux)  os="unknown-linux-gnu" ;;
        *)
            echo "Error: Unsupported operating system: ${os}"
            echo "OMEGA CLI supports macOS and Linux."
            exit 1
            ;;
    esac

    case "${arch}" in
        arm64|aarch64) arch="aarch64" ;;
        x86_64|amd64)  arch="x86_64" ;;
        *)
            echo "Error: Unsupported architecture: ${arch}"
            echo "OMEGA CLI supports x86_64 and ARM64."
            exit 1
            ;;
    esac

    echo "${arch}-${os}"
}

# --- Dependency checks --------------------------------------------------------

check_command() {
    command -v "$1" >/dev/null 2>&1
}

ensure_deps() {
    if ! check_command curl; then
        echo "Error: curl is required but not found."
        exit 1
    fi
    if ! check_command shasum && ! check_command sha256sum; then
        echo "Error: shasum or sha256sum is required but not found."
        exit 1
    fi
}

# --- Checksum verification ----------------------------------------------------

verify_checksum() {
    local file="$1" expected="$2" actual

    if check_command shasum; then
        actual="$(shasum -a 256 "$file" | awk '{print $1}')"
    else
        actual="$(sha256sum "$file" | awk '{print $1}')"
    fi

    if [ "$actual" != "$expected" ]; then
        echo ""
        echo "Error: Checksum verification failed!"
        echo "  Expected: ${expected}"
        echo "  Got:      ${actual}"
        echo ""
        echo "The downloaded binary may be corrupted or tampered with."
        echo "Please try again or report this at https://github.com/${REPO}/issues"
        rm -f "$file"
        exit 1
    fi
}

# --- JSON parsing (portable) --------------------------------------------------

# Extract a value from JSON. Tries python3 first, then jq, then grep fallback.
json_value() {
    local json="$1" key="$2"

    if check_command python3; then
        echo "$json" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d${key})" 2>/dev/null && return
    fi
    if check_command jq; then
        echo "$json" | jq -r "${key}" 2>/dev/null && return
    fi
    # grep fallback for simple top-level string keys
    echo "$json" | grep -o "\"$(echo "$key" | tr -d "[]'\".")\"[[:space:]]*:[[:space:]]*\"[^\"]*\"" | head -1 | sed 's/.*:.*"\(.*\)"/\1/'
}

# --- Main ---------------------------------------------------------------------

main() {
    ensure_deps

    local target
    target="$(detect_platform)"

    echo ""
    echo "  OMEGA CLI Installer"
    echo "  ==================="
    echo ""
    echo "  Platform: ${target}"
    echo "  Install:  ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""

    # Fetch latest release info
    echo "  Fetching latest release..."
    local release_json
    release_json="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null)" || {
        echo "Error: Failed to fetch release info from GitHub."
        echo "Check your internet connection and try again."
        exit 1
    }

    local version
    version="$(echo "$release_json" | json_value '["tag_name"]')"
    version="${version#v}"

    if [ -z "$version" ]; then
        echo "Error: Could not determine latest version."
        echo "There may be no releases yet at https://github.com/${REPO}/releases"
        exit 1
    fi

    echo "  Latest version: v${version}"

    # Find the download URL for our platform
    local binary_asset_name="omg-${target}"
    local download_url
    download_url="$(echo "$release_json" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for asset in data.get('assets', []):
    if asset['name'] == '${binary_asset_name}':
        print(asset['browser_download_url'])
        break
" 2>/dev/null)" || true

    if [ -z "$download_url" ]; then
        # Fallback: construct URL from tag
        download_url="https://github.com/${REPO}/releases/download/v${version}/${binary_asset_name}"
    fi

    # Find checksum
    local checksum_url checksum
    checksum_url="$(echo "$release_json" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for asset in data.get('assets', []):
    if asset['name'] == '${binary_asset_name}.sha256':
        print(asset['browser_download_url'])
        break
" 2>/dev/null)" || true

    # Create install directory
    mkdir -p "${INSTALL_DIR}"

    # Download binary
    echo "  Downloading ${binary_asset_name}..."
    local tmp_file="${INSTALL_DIR}/${BINARY_NAME}.tmp"
    curl -fSL --progress-bar -o "$tmp_file" "$download_url" || {
        echo "Error: Failed to download binary from:"
        echo "  ${download_url}"
        rm -f "$tmp_file"
        exit 1
    }

    # Download and verify checksum
    if [ -n "$checksum_url" ]; then
        echo "  Verifying checksum..."
        checksum="$(curl -fsSL "$checksum_url" | awk '{print $1}')" || true
        if [ -n "$checksum" ]; then
            verify_checksum "$tmp_file" "$checksum"
            echo "  Checksum verified."
        fi
    fi

    # Install
    chmod +x "$tmp_file"
    mv "$tmp_file" "${INSTALL_DIR}/${BINARY_NAME}"

    # Verify it runs
    if "${INSTALL_DIR}/${BINARY_NAME}" version >/dev/null 2>&1; then
        echo "  Binary verified."
    else
        echo "  Warning: Binary exists but failed to run. Check your system compatibility."
    fi

    echo ""
    echo "  Installed omg v${version} to ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""

    # PATH guidance
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo "  Add to your PATH (add this to ~/.zshrc or ~/.bashrc):"
        echo ""
        echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
    fi

    echo "  Get started:"
    echo "    cd your-project"
    echo "    omg init"
    echo ""
}

main "$@"
