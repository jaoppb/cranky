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
    cargo build --release

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

# Install git hooks with Lefthook
[group('setup')]
setup:
    lefthook install
