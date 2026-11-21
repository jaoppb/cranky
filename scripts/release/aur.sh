#!/usr/bin/env bash
# Prove the two AUR packages in clean Arch Linux containers: fill in their
# checksums, write .SRCINFO, build them with makepkg (check() included),
# install each one and start the installed binary.
#
#   local    the sources are the tarball scripts/release/build.sh made and a
#            `git archive` of the working tree's tracked files, so a pull
#            request is checked before any tag exists. Nothing is kept.
#   release  the sources are the published ones of the tag. The PKGBUILD and
#            .SRCINFO of each package land in OUT_DIR/aur/pkg-src and
#            OUT_DIR/aur/pkg-bin, for publishing, only once both packages
#            built, installed and started.
#
# Two containers keep what gets published out of reach of the build: the
# first prepares the files and builds nothing; the second, which runs every
# build script and test, mounts nothing writable.
set -euo pipefail

if [[ $# -ne 1 || ( "$1" != local && "$1" != release ) ]]; then
	echo "usage: $0 local|release" >&2
	exit 2
fi
mode=$1

cd "$(dirname "$0")/../.."
# shellcheck source=scripts/release/common.sh
source scripts/release/common.sh

out="${OUT_DIR}/aur"
rm -rf "${out}"
mkdir -p "${out}/sources/cranky" "${out}/sources/cranky-bin" "${out}/prepared"
if [[ "${mode}" == local ]]; then
	version="$(workspace_version)"
	# HEAD with any uncommitted change to a tracked file, as a commit object
	# that no ref or stash entry points at.
	tree="$(git stash create)"
	git archive --format=tar.gz --prefix="cranky-${version}/" --output="${out}/sources/cranky/cranky-${version}.tar.gz" "${tree:-HEAD}"
	cp "$(tarball_path)" "${out}/sources/cranky-bin/cranky-bin-${version}.tar.gz"
	cp contrib/systemd/cranky.service contrib/systemd/cranky-uwsm.service "${out}/sources/cranky-bin/"
fi

docker run --rm \
	--volume "${PWD}:/workspace:ro" \
	--volume "${PWD}/${out}/sources:/sources:ro" \
	--volume "${PWD}/${out}/prepared:/prepared" \
	--env MODE="${mode}" \
	archlinux:base-devel \
	bash /workspace/scripts/release/aur-container.sh prepare

docker run --rm \
	--volume "${PWD}:/workspace:ro" \
	--volume "${PWD}/${out}/sources:/sources:ro" \
	--volume "${PWD}/${out}/prepared:/prepared:ro" \
	archlinux:base-devel \
	bash /workspace/scripts/release/aur-container.sh build

if [[ "${mode}" == release ]]; then
	for pair in cranky:pkg-src cranky-bin:pkg-bin; do
		mkdir -p "${out}/${pair#*:}"
		cp "${out}/prepared/${pair%%:*}/PKGBUILD" "${out}/prepared/${pair%%:*}/.SRCINFO" "${out}/${pair#*:}/"
	done
fi
