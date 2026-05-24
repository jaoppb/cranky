default:
    @just --list

# Run cargo check to verify code compiles
check:
    cargo check

# Run cargo build in debug mode
build:
    cargo build

# Run cargo build in release mode
release:
    cargo build --release

# Run Cranky in debug mode
run:
    cargo run

# Run Cranky in release mode
run-release:
    cargo run --release

# Run all unit tests
test:
    cargo test

# Run Clippy checks
clippy:
    cargo clippy --all-targets --all-features

# Check code formatting with rustfmt
fmt-check:
    cargo fmt --all -- --check

# Format all code with rustfmt
fmt:
    cargo fmt --all

# Run cargo clean to clean the build target
clean:
    cargo clean

# Run unit tests and generate coverage report (requires cargo-llvm-cov and genhtml)
coverage:
    cargo llvm-cov --lcov --output-path lcov.info
    genhtml lcov.info --output-directory coverage_report

# Install git hooks with Lefthook
setup-hooks:
    lefthook install
