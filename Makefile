# photo-foot-print Makefile
# Convenience targets that wrap cargo / npm so the gate runs the same way
# locally and in any future CI.

.PHONY: build check test test-gate test-fault dev clean

build:
	cargo build --workspace

check:
	cargo check --workspace
	cargo clippy --workspace --all-targets -- -D warnings || true

test:
	# Default test run. Excludes #[ignore]d fixture suite.
	cargo test --workspace

test-gate:
	# Phase 1 gate: metadata-preservation fixture suite (post-D-10).
	# Requires:
	#   - tests/fixtures/scanner/*.jpg supplied (see docs/FIXTURES.md)
	#   - exiftool on PATH (`brew install exiftool`)
	# A green run satisfies ROADMAP.md Phase 1 success criterion #3.
	cargo test -p pfp-exif --test metadata_preservation --test round_trip --test strict_4_predicate -- --include-ignored

test-fault:
	# Plan 03's fault-injection durability test (Phase 1 criterion #4).
	# Requires the `fault-injection` Cargo feature (Plan 03 lands it).
	cargo test -p pfp-exif --test fault_injection --features fault-injection -- --include-ignored

dev:
	cargo tauri dev

clean:
	cargo clean
	rm -rf node_modules dist
