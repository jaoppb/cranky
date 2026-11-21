#!/usr/bin/env bash
# The parts of scripts/release/aur.sh that run inside archlinux:base-devel,
# with the repository read-only at /workspace and the local sources, empty in
# release mode, read-only at /sources. makepkg refuses to run as root, so it
# runs as an unprivileged user with no sudo.
#
#   prepare  copies each PKGBUILD, points it at the local sources in local
#            mode, fills in its checksums and writes its .SRCINFO, and leaves
#            the two files in /prepared/<package>. It builds nothing.
#   build    reads /prepared, read-only here like every other mount. As root,
#            it installs the dependencies both .SRCINFO files name; then it
#            builds each package, installs it, starts the installed binary
#            and removes it.
set -euo pipefail

phase=$1

pacman -Syu --noconfirm --needed pacman-contrib sudo >/dev/null
useradd --create-home builder
work=/home/builder/work
mkdir -p "${work}"

# A package's directory: its PKGBUILD, or the prepared one, and its local
# sources, owned by the build user.
stage() {
	local name=$1 pkgbuild=$2 dir="${work}/$1"
	mkdir -p "${dir}"
	cp "${pkgbuild}" "${dir}/PKGBUILD"
	cp -r "/sources/${name}/." "${dir}/"
	chown -R builder:builder "${dir}"
}

prepare() {
	local name=$1 dir="${work}/$1"
	stage "${name}" "/workspace/contrib/arch/$2"
	if [[ "${MODE}" == local ]]; then
		# Point the sources at the local files, under the names the PKGBUILD
		# gives them. The last assignment wins when makepkg sources the file.
		local file line='source=('
		for file in "${dir}"/*.tar.gz "${dir}"/*.service; do
			if [[ -e "${file}" ]]; then
				line+=" \"${file##*/}\""
			fi
		done
		printf '\n%s )\n' "${line}" >>"${dir}/PKGBUILD"
	fi
	(
		cd "${dir}"
		sudo -u builder updpkgsums
		sudo -u builder makepkg --printsrcinfo | sudo -u builder tee .SRCINFO >/dev/null
	)
	mkdir -p "/prepared/${name}"
	cp "${dir}/PKGBUILD" "${dir}/.SRCINFO" "/prepared/${name}/"
	# The prepared files go back to the owner of the output directory.
	chown -R --reference=/prepared "/prepared/${name}"
}

# Every dependency a .SRCINFO names, architecture-specific ones included.
dependencies_of() {
	sed -n 's/^\t\(make\|check\)\{0,1\}depends\(_x86_64\)\{0,1\} = //p' "$1"
}

# Build a package, install it, start the installed binary, then remove it:
# the two packages conflict.
install_and_start() {
	local name=$1
	stage "${name}" "/prepared/${name}/PKGBUILD"
	(
		cd "${work}/${name}"
		sudo -u builder makepkg --noconfirm
		pacman -U --noconfirm ./*.pkg.tar.zst
	)
	sudo -u builder /workspace/scripts/release/start.sh /usr/bin/cranky
	pacman -R --noconfirm "${name}"
}

case "${phase}" in
prepare)
	prepare cranky PKGBUILD
	prepare cranky-bin PKGBUILD-bin
	;;
build)
	# Read from the read-only mount before any build runs.
	mapfile -t deps < <(
		dependencies_of /prepared/cranky/.SRCINFO
		dependencies_of /prepared/cranky-bin/.SRCINFO
	)
	if ((${#deps[@]} > 0)); then
		pacman -S --needed --noconfirm -- "${deps[@]}"
	fi
	install_and_start cranky
	install_and_start cranky-bin
	;;
*)
	echo "usage: $0 prepare|build" >&2
	exit 2
	;;
esac
