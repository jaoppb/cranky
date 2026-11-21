#!/usr/bin/env bash
# Write a new release version into every file that carries one: the workspace
# version in Cargo.toml, Cargo.lock, and the Arch packaging in contrib/arch.
# It only edits files. The release itself is a pull request with this change,
# then the tag vX.Y.Z pushed on main, which starts .github/workflows/release.yml.
# `version_files_agree` in tests/architecture.rs checks the same files.
set -euo pipefail

if [[ $# -ne 1 ]]; then
	echo "usage: $0 X.Y.Z" >&2
	exit 2
fi
new="$1"

cd "$(dirname "$0")/.."

if [[ ! "${new}" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
	echo "error: '${new}' is not a version of the form X.Y.Z" >&2
	exit 1
fi

# The first `version` key in Cargo.toml is the one in [workspace.package].
current="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
if [[ -z "${current}" ]]; then
	echo "error: no workspace version found in Cargo.toml" >&2
	exit 1
fi

highest="$(printf '%s\n%s\n' "${current}" "${new}" | sort --version-sort | tail -n 1)"
if [[ "${new}" == "${current}" || "${highest}" != "${new}" ]]; then
	echo "error: ${new} is not greater than the current version ${current}" >&2
	exit 1
fi

# From here on the files are being rewritten, and a second run would refuse the
# version it finds half-written.
trap 'echo "error: the bump stopped part-way; undo it with: git restore Cargo.toml Cargo.lock contrib/arch" >&2' ERR

sed -i "0,/^version = .*/s//version = \"${new}\"/" Cargo.toml

for pkgbuild in contrib/arch/PKGBUILD contrib/arch/PKGBUILD-bin; do
	sed -i -e "s/^pkgver=.*/pkgver=${new}/" -e "s/^pkgrel=.*/pkgrel=1/" "${pkgbuild}"
done

# The git package computes its version from the tags; the literal is what it
# shows before a build, and its fallback when the clone has no tag.
sed -i \
	-e "s/^pkgver=.*/pkgver=${new}.r0.g0000000/" \
	-e "s/^pkgrel=.*/pkgrel=1/" \
	-e "s/printf \"[0-9.]*\.r%s\.%s\"/printf \"${new}.r%s.%s\"/" \
	contrib/arch/PKGBUILD-git

# Only the workspace's own entries change in Cargo.lock.
cargo update --workspace --offline --quiet

# sed changes nothing, silently, when a line it expects is gone.
missing=0
expect() {
	if ! grep --quiet --fixed-strings --line-regexp -- "$2" "$1"; then
		echo "error: $1 does not carry ${new}; expected the line: $2" >&2
		missing=1
	fi
}
expect Cargo.toml "version = \"${new}\""
expect contrib/arch/PKGBUILD "pkgver=${new}"
expect contrib/arch/PKGBUILD-bin "pkgver=${new}"
expect contrib/arch/PKGBUILD-git "pkgver=${new}.r0.g0000000"
if ! grep --quiet --fixed-strings -- "printf \"${new}.r%s.%s\"" contrib/arch/PKGBUILD-git; then
	echo "error: contrib/arch/PKGBUILD-git does not carry ${new} in its pkgver() fallback" >&2
	missing=1
fi
if [[ "$(grep --count --fixed-strings --line-regexp -- "version = \"${new}\"" Cargo.lock)" -lt 3 ]]; then
	echo "error: Cargo.lock does not carry ${new} for the three workspace crates" >&2
	missing=1
fi
if [[ "${missing}" -ne 0 ]]; then
	false
fi

echo "Bumped ${current} -> ${new}. Review with 'git diff', then open a pull request."
echo "After it merges, push the tag v${new} on main to release."
