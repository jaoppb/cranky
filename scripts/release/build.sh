#!/usr/bin/env bash
# Build the release tarball: the binary, built in the image common.sh pins
# with the toolchain rust-toolchain.toml names and Cargo.lock as it is, plus
# README.md and LICENSE. CI pins the same toolchain and lockfile through the
# flake; the devShell's own glibc is too new for a binary meant to run
# elsewhere.
set -euo pipefail

cd "$(dirname "$0")/../.."
# shellcheck source=scripts/release/common.sh
source scripts/release/common.sh

# rust-version is the floor the AUR package states to a user's own Rust; it
# names the toolchain this build and CI use.
channel="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
floor="$(sed -n 's/^rust-version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
if [[ "${channel}" != "${floor}" && "${channel}" != "${floor}".* ]]; then
	echo "error: rust-version ${floor:-(none)} in Cargo.toml does not name the toolchain ${channel} of rust-toolchain.toml" >&2
	exit 1
fi

mkdir -p "${OUT_DIR}"
# The source is read-only; the build writes only to the output directory, and
# gives what it wrote the owner of that directory, which is the caller's user
# whether docker is rootful (CI) or rootless. The toolchain and crate downloads stay
# there between local runs.
docker run --rm \
	--volume "${PWD}:/src:ro" \
	--volume "${PWD}/${OUT_DIR}:/out" \
	--workdir /src \
	--env RUSTUP_HOME=/out/rustup \
	--env CARGO_HOME=/out/cargo \
	--env CARGO_TARGET_DIR=/out/target \
	"$(build_image)" \
	bash -euo pipefail -c '
		trap "chown -R --reference=/out /out" EXIT
		apt-get update -qq
		DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
			binutils ca-certificates curl gcc libc6-dev pkg-config libxkbcommon-dev >/dev/null
		if [[ ! -x "${CARGO_HOME}/bin/rustup" ]]; then
			curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs |
				sh -s -- -y --quiet --no-modify-path --default-toolchain none --profile minimal
		fi
		export PATH="${CARGO_HOME}/bin:${PATH}"
		# The toolchain rust-toolchain.toml names, with its components.
		rustup toolchain install
		rustc --version
		cargo build --release --locked
		# [profile.release] strips the binary users install (#82).
		# Captured first, so a readelf failure stops the build instead of passing the check.
		sections="$(readelf --section-headers --wide /out/target/release/cranky)"
		if grep --quiet --extended-regexp "\.symtab|\.debug_" <<<"${sections}"; then
			echo "error: the release binary still has a symbol table or debug info" >&2
			exit 1
		fi
	'

staging="${OUT_DIR}/staging"
rm -rf "${staging}"
mkdir -p "${staging}"
cp "${OUT_DIR}/target/release/cranky" README.md LICENSE "${staging}/"
tarball="$(tarball_path)"
tar -czf "${tarball}" -C "${staging}" cranky README.md LICENSE
echo "${tarball}"
