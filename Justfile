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

# Run the full gate a pull request must pass: formatting, Clippy, architecture, LOC limits and tests
[group('quality')]
check: fmt-check lint arch-check loc-check test

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
    cargo clippy --all-targets --all-features -- -D warnings

# Apply Clippy's automatic fixes, including to dirty and staged files
[group('quality')]
lint-fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features

# Run architecture lint checks
[group('quality')]
arch-check:
    cargo test --test architecture

# Check the 200 production LOC per file limit
[group('quality')]
loc-check:
    ./scripts/lint-loc.sh

# Run all unit tests
[group('test')]
test:
    cargo test

# Run unit tests and generate coverage report (requires cargo-llvm-cov and genhtml)
[group('test')]
coverage:
    cargo llvm-cov --lcov --output-path lcov.info
    genhtml lcov.info --output-directory coverage_report

# Run performance benchmarks with criterion
[group('profile')]
bench *args:
    cargo bench {{args}}

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
