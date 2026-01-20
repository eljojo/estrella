# Estrella Makefile
#
# All commands run through nix develop to ensure correct toolchain

SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.ONESHELL:

# Output directory for local builds
OUT_DIR := .cargo/target/release

# Default target
.PHONY: all
all: build

# Build release binary
.PHONY: build
build:
	nix develop --command cargo build --release
	@echo "Binary available at: $(OUT_DIR)/estrella"

# Build debug binary (faster compilation)
.PHONY: build-debug
build-debug:
	nix develop --command cargo build
	@echo "Binary available at: .cargo/target/debug/estrella"

# Format code
.PHONY: format
format:
	nix develop --command cargo fmt

# Check formatting without modifying
.PHONY: format-check
format-check:
	nix develop --command cargo fmt --check

# Run all tests
.PHONY: test
test:
	nix develop --command cargo test

# Run tests with output
.PHONY: test-verbose
test-verbose:
	nix develop --command cargo test -- --nocapture

# Run clippy lints
.PHONY: lint
lint:
	nix develop --command cargo clippy -- -D warnings

# Regenerate golden test files
# Use this when pattern code changes intentionally
.PHONY: golden
golden:
	@echo "Regenerating golden test files..."
	nix develop --command cargo run -- print --png tests/golden/ripple_576x500.png --height 500 --width 576 ripple
	nix develop --command cargo run -- print --png tests/golden/waves_576x500.png --height 500 --width 576 waves
	nix develop --command cargo run -- print --png tests/golden/sick_576x1920.png --height 1920 --width 576 sick
	nix develop --command cargo run -- print --png tests/golden/calibration_576x500.png --height 500 --width 576 calibration
	@echo "Golden files regenerated. Run 'make test' to verify."

# Clean build artifacts
.PHONY: clean
clean:
	nix develop --command cargo clean

# Run the CLI (usage: make run ARGS="print ripple")
.PHONY: run
run:
	nix develop --command cargo run -- $(ARGS)

# Show available patterns
.PHONY: patterns
patterns:
	nix develop --command cargo run -- print

# Generate a preview PNG (usage: make preview PATTERN=ripple)
.PHONY: preview
preview:
	nix develop --command cargo run -- print --png /tmp/$(PATTERN).png $(PATTERN)
	@echo "Preview saved to /tmp/$(PATTERN).png"

.PHONY: help
help:
	@echo "Estrella Makefile targets:"
	@echo ""
	@echo "  build        Build release binary"
	@echo "  build-debug  Build debug binary (faster)"
	@echo "  format       Format code with rustfmt"
	@echo "  format-check Check formatting without changes"
	@echo "  test         Run all tests"
	@echo "  test-verbose Run tests with output"
	@echo "  lint         Run clippy lints"
	@echo "  golden       Regenerate golden test files"
	@echo "  clean        Clean build artifacts"
	@echo "  patterns     List available patterns"
	@echo "  run          Run CLI (e.g., make run ARGS='print ripple')"
	@echo "  preview      Generate preview PNG (e.g., make preview PATTERN=ripple)"
	@echo "  help         Show this help"
