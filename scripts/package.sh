#!/usr/bin/env bash
#
# Builds .deb and .rpm packages for fast-diary using cargo-deb and
# cargo-generate-rpm. Run from the repository root:
#
#   ./scripts/package.sh [deb|rpm|all]
#
# Produced packages are left under target/debian/ and target/generate-rpm/.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

kind="${1:-all}"

ensure_installed() {
  local binary="$1"
  local crate="$2"

  if ! command -v "${binary}" >/dev/null 2>&1; then
    echo "Installing ${crate} (provides ${binary})..."
    cargo install "${crate}" --locked
  fi
}

build_release() {
  echo "Building release binary..."
  cargo build --release --locked
}

build_deb() {
  ensure_installed cargo-deb cargo-deb
  echo "Building .deb package..."
  cargo deb --no-build
}

build_rpm() {
  ensure_installed cargo-generate-rpm cargo-generate-rpm
  echo "Building .rpm package..."
  cargo generate-rpm
}

case "${kind}" in
  deb)
    build_release
    build_deb
    ;;
  rpm)
    build_release
    build_rpm
    ;;
  all)
    build_release
    build_deb
    build_rpm
    ;;
  *)
    echo "Usage: $0 [deb|rpm|all]" >&2
    exit 1
    ;;
esac

echo "Done."
echo "  .deb -> target/debian/"
echo "  .rpm -> target/generate-rpm/"
