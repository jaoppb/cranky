#!/usr/bin/env bash
# Prove the two AUR packages in a clean Arch Linux container: fill in their
# checksums, write .SRCINFO, build them with makepkg (check() included),
# install each one and start the installed binary.
#
#   local    the sources are the tarball scripts/release/build.sh made and a
#            `git archive` of the working tree's tracked files, so a pull
#            request is checked before any tag exists. Nothing is kept.
#   release  the sources are the published ones of the tag. The PKGBUILD and
#            .SRCINFO of each package are kept in OUT_DIR/aur/pkg-src and
#            OUT_DIR/aur/pkg-bin for publishing, only once both installed.
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
mkdir -p "${out}"
if [[ "${mode}" == local ]]; then
	version="$(workspace_version)"
	mkdir -p "${out}/sources/cranky" "${out}/sources/cranky-bin"
	# HEAD with any uncommitted change to a tracked file, as a commit object
	# that no ref or stash entry points at.
	tree="$(git stash create)"
	git archive --format=tar.gz --prefix="cranky-${version}/" --output="${out}/sources/cranky/cranky-${version}.tar.gz" "${tree:-HEAD}"
	cp "$(tarball_path)" "${out}/sources/cranky-bin/cranky-bin-${version}.tar.gz"
	cp contrib/systemd/cranky.service contrib/systemd/cranky-uwsm.service "${out}/sources/cranky-bin/"
fi

docker run --rm \
	--volume "${PWD}:/workspace:ro" \
	--volume "${PWD}/${out}:/out" \
	--env MODE="${mode}" \
	archlinux:base-devel \
	bash /workspace/scripts/release/aur-container.sh
