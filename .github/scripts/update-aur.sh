#!/bin/bash
set -e
# Install dependencies required by makepkg
pacman -Syu --noconfirm pacman-contrib sudo git
useradd -m builder
chown -R builder:builder /workspace

cd /workspace/contrib/arch

prepare_pkg() {
	local dir=$1
	local src_pkgbuild=$2

	mkdir -p "$dir"
	cp "$src_pkgbuild" "$dir/PKGBUILD"
	chown -R builder:builder "$dir"
	(
		cd "$dir"
		sudo -u builder updpkgsums
		sudo -u builder makepkg --printsrcinfo >.SRCINFO
	)
}

# Run both preparations in parallel
prepare_pkg pkg-src PKGBUILD &
PID1=$!

prepare_pkg pkg-bin PKGBUILD-bin &
PID2=$!

# Wait for both processes to finish. With set -e, if any fails, the script will exit.
wait $PID1
wait $PID2
