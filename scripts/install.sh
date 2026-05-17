#!/usr/bin/env bash
set -euo pipefail

REPO="chamuka-inc/chamuka-plane"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}" in
    Linux)  os="unknown-linux-gnu" ;;
    Darwin) os="apple-darwin" ;;
    *)      echo "Unsupported OS: ${os}" >&2; exit 1 ;;
  esac

  case "${arch}" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)             echo "Unsupported architecture: ${arch}" >&2; exit 1 ;;
  esac

  echo "${arch}-${os}"
}

get_latest_version() {
  local version
  version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
  if [ -z "${version}" ]; then
    echo "ERROR: Could not determine latest version. Check network or pass version as argument." >&2
    exit 1
  fi
  echo "${version}"
}

verify_checksum() {
  local file="$1" checksum_file="$2"
  if command -v sha256sum &>/dev/null; then
    (cd "$(dirname "${file}")" && sha256sum -c "${checksum_file}" >/dev/null 2>&1)
  else
    (cd "$(dirname "${file}")" && shasum -a 256 -c "${checksum_file}" >/dev/null 2>&1)
  fi
}

main() {
  local version="${1:-}"
  local platform
  platform="$(detect_platform)"

  if [ -z "${version}" ]; then
    echo "Fetching latest release..."
    version="$(get_latest_version)"
  fi

  local archive="chamuka-plane-${version}-${platform}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/${version}/${archive}"

  echo "Downloading chamuka-plane ${version} for ${platform}..."
  local tmpdir
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "${tmpdir}"' EXIT

  curl -fsSL "${url}" -o "${tmpdir}/${archive}"
  curl -fsSL "${url}.sha256" -o "${tmpdir}/${archive}.sha256"

  echo "Verifying checksum..."
  if ! verify_checksum "${tmpdir}/${archive}" "${tmpdir}/${archive}.sha256"; then
    echo "ERROR: Checksum verification failed." >&2
    exit 1
  fi

  tar xzf "${tmpdir}/${archive}" -C "${tmpdir}"

  echo "Installing to ${INSTALL_DIR}..."
  local extracted_dir="${tmpdir}/chamuka-plane-${version}-${platform}"
  if [ -w "${INSTALL_DIR}" ]; then
    cp "${extracted_dir}/plane" "${INSTALL_DIR}/"
    cp "${extracted_dir}/sbctl" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/plane" "${INSTALL_DIR}/sbctl"
  else
    sudo cp "${extracted_dir}/plane" "${INSTALL_DIR}/"
    sudo cp "${extracted_dir}/sbctl" "${INSTALL_DIR}/"
    sudo chmod +x "${INSTALL_DIR}/plane" "${INSTALL_DIR}/sbctl"
  fi

  echo "Installed to ${INSTALL_DIR}:"
  echo "  plane  ${version}"
  echo "  sbctl  ${version}"
}

main "$@"
