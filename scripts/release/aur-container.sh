#!/usr/bin/env bash
# The part of scripts/release/aur.sh that runs inside archlinux:base-devel,
# with the repository read-only at /workspace and the output at /out. makepkg
# refuses to run as root, so the packages are built by an unprivileged user
# with no sudo: root installs their dependencies first, and keeps what gets
# published out of reach of the build scripts and tests.
set -euo pipefail

# The output goes back to the owner of its directory, the caller's user.
trap 'chown -R --reference=/out /out' EXIT

pacman -Syu --noconfirm --needed pacman-contrib sudo >/dev/null
useradd --create-home builder

work=/home/builder/work

# Copy a PKGBUILD into its own directory, point it at local sources in local
# mode, then fill in its checksums and write its .SRCINFO.
prepare() {
	local name=$1 pkgbuild=$2 dir="${work}/$1"
	mkdir -p "${dir}"
	cp "/workspace/contrib/arch/${pkgbuild}" "${dir}/PKGBUILD"
	if [[ "${MODE}" == local ]]; then
		# The local sources, under the names the PKGBUILD gives them. The last
		# assignment wins when makepkg sources the file.
		cp /out/sources/"${name}"/* "${dir}/"
		local names=("${dir}"/*.tar.gz "${dir}"/*.service)
		printf '\nsource=(' >>"${dir}/PKGBUILD"
		local file
		for file in "${names[@]}"; do
			if [[ -e "${file}" ]]; then
				printf ' "%s"' "${file##*/}" >>"${dir}/PKGBUILD"
			fi
		done
		printf ' )\n' >>"${dir}/PKGBUILD"
	fi
	chown -R builder:builder "${work}"
	(
		cd "${dir}"
		sudo -u builder updpkgsums
		sudo -u builder makepkg --printsrcinfo | sudo -u builder tee .SRCINFO >/dev/null
	)
}

# Install a package's dependencies as root, build it, install it, start the
# installed binary, then remove it: the two packages conflict.
install_and_start() {
	local name=$1
	(
		cd "${work}/${name}"
		local -a deps
		mapfile -t deps < <(sed -n 's/^\t\(make\|check\)\{0,1\}depends = //p' .SRCINFO)
		pacman -S --needed --noconfirm "${deps[@]}"
		sudo -u builder makepkg --noconfirm
		pacman -U --noconfirm ./*.pkg.tar.zst
	)
	sudo -u builder /workspace/scripts/release/start.sh /usr/bin/cranky
	pacman -R --noconfirm "${name}"
}

mkdir -p "${work}"
prepare cranky PKGBUILD
prepare cranky-bin PKGBUILD-bin

# What gets published is each prepared PKGBUILD and its .SRCINFO, copied before
# any build runs; /out is not writable by the build user.
if [[ "${MODE}" == release ]]; then
	for pair in cranky:pkg-src cranky-bin:pkg-bin; do
		mkdir -p "/out/${pair#*:}"
		cp "${work}/${pair%%:*}/PKGBUILD" "${work}/${pair%%:*}/.SRCINFO" "/out/${pair#*:}/"
	done
fi

install_and_start cranky
install_and_start cranky-bin
