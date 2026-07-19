# rsshogi-usi workspace tasks

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
npx := if os() == "windows" { "npx.cmd" } else { "npx" }

default:
	@just --list

# Build all workspace crates
build:
	cargo build --workspace

# Build release binary
release:
	cargo build --workspace --release

# No runnable engine binary is provided in this workspace yet
run:
	@echo "No engine binary is defined yet. Build or test the protocol crate instead."

# Run all tests
test:
	cargo test --workspace

# Legacy helper placeholders retained during bootstrap
test-bitboard-simd:
	@echo "No SIMD helper is defined in rsshogi-usi."

test-nnue-simd:
	@echo "No NNUE SIMD helper is defined in rsshogi-usi."

# Run tests for specific crate
test-support:
	@echo "No support crate is defined yet."

test-usi:
	cargo test -p rsshogi-usi

test-nnue:
	@echo "No NNUE runtime crate is part of this workspace."

test-generate-all-legal-moves:
	@echo "generate-all-legal-moves is not defined in this workspace."

search-test:
	@echo "search-test is deferred until a session or engine consumer exists."

# Lint and format
lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- \
		-W clippy::pedantic -W clippy::nursery -W clippy::cargo \
		-A clippy::module_name_repetitions \
		-A clippy::missing_panics_doc \
		-A clippy::missing_errors_doc \
		-D warnings

# (Optional) Lint with all features enabled. Nightly-only features may失敗する場合があります。
clippy-all:
	cargo clippy --workspace --all-targets --all-features -- \
		-W clippy::pedantic -W clippy::nursery -W clippy::cargo \
		-A clippy::module_name_repetitions -A clippy::missing_panics_doc \
		-A clippy::missing_errors_doc \
		-D warnings

# Build Rustdoc と mdBook（TS資産を事前ビルド）
docs:
    cargo doc --no-deps --workspace
    just docs-prepare-assets
    mdbook build docs/book

# mdBook 用のフロント資産（TypeScript）をバンドル
docs-prepare-assets:
    {{npx}} --yes esbuild \
        docs/book/src/assets/shogi-board.ts \
        --bundle \
        --format=esm \
        --target=es2018 \
        --outfile=docs/book/src/assets/shogi-board.js \
        --minify

# mdBook のみ（Rustdoc を省略）
book:
    just docs-prepare-assets
    mdbook build docs/book

# Format only
fmt:
	cargo fmt --all

# Clippy only
clippy:
	cargo clippy --workspace --all-targets --all-features -- \
		-W clippy::pedantic -W clippy::nursery -W clippy::cargo \
		-A clippy::module_name_repetitions -A clippy::missing_panics_doc \
		-A clippy::missing_errors_doc \
		-D warnings

# Check without building
check:
	cargo check --workspace

