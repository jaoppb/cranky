#!/usr/bin/env bash
# Start the binary of the release tarball in clean containers outside Nix,
# each with only libxkbcommon installed: current Arch Linux, where the AUR
# users are, and the build image, whose glibc is the oldest it supports.
set -euo pipefail

cd "$(dirname "$0")/../.."
# shellcheck source=scripts/release/common.sh
source scripts/release/common.sh

tarball="$(tarball_path)"
unpacked="${OUT_DIR}/smoke"
rm -rf "${unpacked}"
mkdir -p "${unpacked}"
tar -xzf "${tarball}" -C "${unpacked}"

run_in() {
	local image=$1 install=$2
	echo "== ${image}"
	docker run --rm \
		--volume "${PWD}/${unpacked}:/release:ro" \
		--volume "${PWD}/scripts/release:/scripts:ro" \
		"${image}" \
		bash -euo pipefail -c "${install} >/dev/null && /scripts/start.sh /release/cranky"
}

run_in archlinux:latest "pacman -Sy --noconfirm libxkbcommon"
run_in "$(build_image)" "apt-get update -qq && apt-get install -y -qq --no-install-recommends libxkbcommon0"
