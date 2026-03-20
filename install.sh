#!/usr/bin/env bash
set -euo pipefail

REPO="TonyMilton/zip-tight"
INSTALL_DIR="${HOME}/.local/bin"
BINARY="ziptight"

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux)  TARGET_OS="unknown-linux-gnu" ;;
    Darwin) TARGET_OS="apple-darwin" ;;
    *)
        echo "Error: Unsupported operating system: ${OS}" >&2
        exit 1
        ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64)         TARGET_ARCH="x86_64" ;;
    aarch64|arm64)  TARGET_ARCH="aarch64" ;;
    *)
        echo "Error: Unsupported architecture: ${ARCH}" >&2
        exit 1
        ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"

# Get latest release tag
echo "Fetching latest release..."
TAG="$(curl -sI "https://github.com/${REPO}/releases/latest" | grep -i '^location:' | sed 's|.*/tag/||' | tr -d '\r\n')"

if [ -z "${TAG}" ]; then
    echo "Error: Could not determine latest release" >&2
    exit 1
fi

echo "Latest release: ${TAG}"

# Download and extract
ASSET="${BINARY}-${TAG}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

echo "Downloading ${ASSET}..."

TMPDIR="$(mktemp -d)"
trap 'rm -rf "${TMPDIR}"' EXIT

if ! curl -sL --fail "${URL}" -o "${TMPDIR}/${ASSET}"; then
    echo "Error: Failed to download ${URL}" >&2
    echo "No prebuilt binary available for ${TARGET}" >&2
    exit 1
fi

tar xzf "${TMPDIR}/${ASSET}" -C "${TMPDIR}"

# Install binary
mkdir -p "${INSTALL_DIR}"
cp "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

echo "Installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

# Add to PATH if needed
add_to_path() {
    local rc_file="$1"
    local export_line='export PATH="${HOME}/.local/bin:${PATH}"'

    if [ -f "${rc_file}" ] && grep -qF '.local/bin' "${rc_file}"; then
        return
    fi

    if [ -f "${rc_file}" ]; then
        echo "" >> "${rc_file}"
        echo "# Added by ziptight installer" >> "${rc_file}"
        echo "${export_line}" >> "${rc_file}"
        echo "Updated ${rc_file}"
    fi
}

if ! echo "${PATH}" | grep -q "${INSTALL_DIR}"; then
    case "$(basename "${SHELL}")" in
        bash) add_to_path "${HOME}/.bashrc" ;;
        zsh)  add_to_path "${HOME}/.zshrc" ;;
        *)
            add_to_path "${HOME}/.bashrc"
            add_to_path "${HOME}/.zshrc"
            ;;
    esac
fi

echo ""
echo "Done! To get started:"
case "$(basename "${SHELL}")" in
    zsh)  echo "  1. Run: source ~/.zshrc" ;;
    *)    echo "  1. Run: source ~/.bashrc" ;;
esac
echo "  2. Run: ${BINARY} --help"
