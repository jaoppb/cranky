# Recipes run only inside the flake devShell (`nix develop`, or direnv with `.envrc`), so a local
# run and CI use the same tools, at the versions flake.lock pins. just evaluates every assignment
# before it runs a recipe, so this guard covers all of them, new ones included; `just --list`
# evaluates nothing and works anywhere. flake.nix sets CRANKY_DEVSHELL.
_devshell := if env("CRANKY_DEVSHELL", "") == "" { error("run just inside the flake devShell: 'nix develop -c just <recipe>', or enter it with 'nix develop' or 'direnv allow'") } else { "" }

[private]
default:
    @just --list --unsorted

# Run cargo build in debug mode
[group('build')]
build:
    cargo build

# Run cargo build in release mode
[group('build')]
release:
    cargo build --release --locked

# Run cargo clean to clean the build target
[group('build')]
clean:
    cargo clean

# Run Cranky in debug mode
[group('run')]
run:
    cargo run

# Run Cranky in release mode
[group('run')]
run-release:
    cargo run --release

# Run the full gate a pull request must pass: formatting, Clippy, architecture, LOC limits, tests, dependencies and workflows
[group('quality')]
check: fmt-check lint arch-check loc-check test deps-check workflows-check

# Format all code with rustfmt
[group('quality')]
fmt:
    cargo fmt --all

# Check code formatting with rustfmt
[group('quality')]
fmt-check:
    cargo fmt --all -- --check

# Run Clippy, denying warnings
[group('quality')]
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Apply Clippy's automatic fixes, including to dirty and staged files
[group('quality')]
lint-fix:
    cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets --all-features

# Run architecture lint checks
[group('quality')]
arch-check:
    cargo test --test architecture

# Check the 200 production LOC per file limit
[group('quality')]
loc-check:
    ./scripts/lint-loc.sh

# Check dependencies against deny.toml: advisories, licences, bans and sources (requires cargo-deny and the network)
[group('quality')]
deps-check:
    cargo deny check

# Lint the GitHub workflows and their shell scripts (requires actionlint and shellcheck)
[group('quality')]
workflows-check:
    @command -v shellcheck >/dev/null || { echo "shellcheck not found: actionlint would skip the shell checks" >&2; exit 1; }
    actionlint

# Run every test: unit, architecture and integration (needs dbus-daemon, sway and grim)
[group('test')]
test:
    cargo test --workspace

# Run the unit tests and the architecture gate, which need no running service
[group('test')]
test-unit:
    cargo test --workspace --lib --bins --test architecture

# Run the D-Bus and Wayland integration tests alone, optionally only the cases matching a name
[group('test')]
test-integration *args:
    cargo test --test integration -- {{args}}

# Run every test and generate a coverage report (requires cargo-llvm-cov and genhtml)
[group('test')]
coverage:
    cargo llvm-cov --workspace --lcov --output-path lcov.info
    genhtml lcov.info --output-directory coverage_report

# Check that the tests cover at least 80% of lines (requires cargo-llvm-cov)
[group('test')]
coverage-check:
    cargo llvm-cov --workspace --fail-under-lines 80

# Run performance benchmarks with criterion
[group('profile')]
bench *args:
    cargo bench {{args}}

# Check that the benchmarks compile, without running them
[group('profile')]
bench-check:
    cargo bench --workspace --no-run

# Record performance profile with samply
[group('profile')]
record-profile *args:
    samply record --save-only -o scripts/profile.json.gz ./target/release/cranky {{args}}

# Analyze performance profile (JSON.GZ)
[group('profile')]
analyze-profile *args:
    python3 scripts/analyze_profile.py {{args}}

# Audit the GitHub issues against tools.toml: type, area, milestone, parent, Project membership and pending label migrations; --no-membership turns off the membership rule only, the board is still read (needs gh authentication, with the read:project scope while the board is private)
[group('repo')]
issues-check *args:
    tools issues check {{args}}

# Check that the GitHub labels, milestones, merge settings and Project title match tools.toml, or only the parts named, e.g. --labels (needs gh authentication with push access to see the settings and, while the board is private, the read:project scope for the title)
[group('repo')]
repo-check *args:
    tools repo check {{args}}

# Print the Status moves the Project cards need and the frozen cards, writing nothing; exits 1 when there are any (needs gh authentication, with the read:project scope while the board is private)
[group('repo')]
project-check *args:
    tools project check {{args}}

# Write the planned Status moves to the Project (needs gh authentication with the project scope)
[group('repo')]
project-sync:
    tools project sync

# Install git hooks with Lefthook
[group('setup')]
setup:
    lefthook install

# Update flake.lock, the pin of every tool in the devShell: all inputs, or only the ones named (flake.nix holds the jaoppb/tools tag)
[group('setup')]
flake-update *inputs:
    nix flake update {{inputs}}

# Write a new version X.Y.Z into Cargo.toml, Cargo.lock and the Arch packaging; the release is that change merged, then the tag vX.Y.Z pushed on main
[group('release')]
bump-version version:
    ./scripts/bump-version.sh {{version}}

# Build the release tarball outside Nix, start it on Arch and Debian, and build, install and start both AUR packages, as release-check.yml does (requires docker)
[group('release')]
release-check:
    ./scripts/release/build.sh
    ./scripts/release/smoke.sh
    ./scripts/release/aur.sh local
