.PHONY: build

build:
	cargo build --release

js:
	corepack prepare pnpm@latest --activate
	pnpm update --interactive --latest
