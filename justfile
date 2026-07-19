# Vigo Browser — Task Runner
# Install: cargo install just
# Usage:  just <recipe>

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Default: list available recipes
default:
    @just --list

# ───── Build ─────

# Check all Rust crates (fast compile check, no codegen)
check:
    cargo check --workspace --all-targets

# Build all Rust crates in debug mode
build:
    cargo build --workspace

# Build all Rust crates in release mode
release:
    cargo build --workspace --release

# Build Zig modules (requires zig in PATH)
zig-build:
    cd zig && zig build

# Run Zig unit tests
zig-test:
    cd zig && zig build test

# Build everything (Zig libraries must exist before Rust links the app)
all: zig-build build

# ───── Test ─────

# Run all Rust tests
test:
    cargo test --workspace

# Run tests with output shown
test-verbose:
    cargo test --workspace -- --nocapture

# Run a specific crate's tests
test-crate crate:
    cargo test -p {{crate}}

# ───── Quality ─────

# Run clippy linter with strict warnings
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format all Rust code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run all quality checks (Zig build/test + Rust format/lint/test)
ci: zig-build zig-test fmt-check lint test

# ───── Run ─────

# Run the Vigo browser (debug)
run:
    cargo run -p vex-app

# Run the Vigo browser (release)
run-release:
    cargo run -p vex-app --release

# ───── Clean ─────

# Clean Rust build artifacts
clean:
    cargo clean

# Clean Zig build artifacts
zig-clean:
    cd zig && rm -rf zig-out zig-cache .zig-cache

# Clean everything
clean-all: clean zig-clean

# ───── Docs ─────

# Generate Rust documentation
doc:
    cargo doc --workspace --no-deps --open

# Generate docs without opening
doc-build:
    cargo doc --workspace --no-deps

# ───── Utilities ─────

# Show workspace dependency tree
deps:
    cargo tree --workspace --depth 1

# Count lines of code (requires tokei)
loc:
    tokei crates/ zig/ --sort code

# Audit dependencies for known vulnerabilities (requires cargo-audit)
audit:
    cargo audit
