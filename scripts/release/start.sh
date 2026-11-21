#!/usr/bin/env bash
# Start a cranky binary with no compositor: it must log its first line, which
# proves the loader, glibc and its libraries resolve, and then fail to
# connect. The `nix` job of ci.yml runs it on the flake package, and the
# release scripts on the release binary and the installed AUR packages.
set -euo pipefail

if [[ $# -ne 1 ]]; then
	echo "usage: $0 BINARY" >&2
	exit 2
fi

XDG_RUNTIME_DIR="$(mktemp -d)"
export XDG_RUNTIME_DIR
output="$(timeout 30 "$1" 2>&1 || true)"
echo "${output}"
grep --quiet "Starting Cranky" <<<"${output}"
