#!/usr/bin/env bash
# The part of scripts/release/aur.sh that runs inside archlinux:base-devel,
# with the repository read-only at /workspace and the output at /out. makepkg
# refuses to run as root, so the packages are built by an unprivileged user
# allowed to install their dependencies.
set -euo pipefail

# The output goes back to the owner of its directory, the caller's user.
trap 'chown -R --reference=/out /out' EXIT

pacman -Syu --noconfirm --needed pacman-contrib sudo >/dev/null
useradd --create-home builder
echo 'builder ALL=(ALL) NOPASSWD: ALL' >/etc/sudoers.d/builder

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

# Build and install a package, start the installed binary, then remove it:
# the two packages conflict.
install_and_start() {
	local name=$1
	(
		cd "${work}/${name}"
		sudo -u builder makepkg --syncdeps --noconfirm
		pacman -U --noconfirm ./*.pkg.tar.zst
	)
	sudo -u builder /workspace/scripts/release/start.sh /usr/bin/cranky
	pacman -R --noconfirm "${name}"
}

mkdir -p "${work}"
prepare cranky PKGBUILD
prepare cranky-bin PKGBUILD-bin

install_and_start cranky
install_and_start cranky-bin

# What gets published is each prepared PKGBUILD and its .SRCINFO, which the
# builds above did not change.
if [[ "${MODE}" == release ]]; then
	for pair in cranky:pkg-src cranky-bin:pkg-bin; do
		mkdir -p "/out/${pair#*:}"
		cp "${work}/${pair%%:*}/PKGBUILD" "${work}/${pair%%:*}/.SRCINFO" "/out/${pair#*:}/"
	done
fi
