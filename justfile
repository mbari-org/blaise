# A convenient default for development: test and format
default: test format

# default + clippy; good to run before committing changes
all: default clippy

# List recipes
list:
    @just --list --unsorted

# cargo check
check:
    cargo check

# Run tests
test:
    cargo test

# Run tests with --nocapture
test-nocapture:
    cargo test -- --nocapture

# Run program with basic example
run *args='-p data -o data/out':
    cargo run -- {{args}}

# Run program in release mode
rrun *args:
    cargo run --release -- {{args}}

# Package source code
tgz:
    #!/usr/bin/env bash
    HASH=$(git rev-parse --short HEAD)
    tar zcf blaise-src-${HASH}.tgz --exclude-from .gitignore .

# Format source code
format:
    cargo fmt

# Run clippy
clippy:
    cargo clippy --all-targets -- -D warnings

# Build
build *args='--release':
    cargo build {{ args }}

# Install
install: build
    cargo install --path .

# (cargo install --locked cargo-outdated)
# Show outdated dependencies
outdated:
    cargo outdated --root-deps-only

# (cargo install --locked cargo-udeps)
# Find unused dependencies
udeps:
    cargo +nightly udeps

# cargo update
update:
    cargo update
