# shellcheck shell=bash
# Settings shared by the release scripts, which .github/workflows/release.yml
# and release-check.yml run, and `just release-check` runs locally.

# The image the release binary is built in. Its glibc (2.36) is the oldest one
# the binary runs on. The digest moves only in a pull request of its own.
build_image() {
	echo "debian:bookworm@sha256:268fb24c39bca92d326dd83ea3abebe4a3f7751c804ffef23bb6d857dd343ff8"
}

# Everything the scripts write lives here, under the ignored target directory.
readonly OUT_DIR="target/release-artifact"

# The workspace version: the first `version` key in Cargo.toml is the one in
# [workspace.package].
workspace_version() {
	sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1
}

# The tarball the GitHub release carries and PKGBUILD-bin downloads.
tarball_path() {
	echo "${OUT_DIR}/cranky-v$(workspace_version)-x86_64.tar.gz"
}
