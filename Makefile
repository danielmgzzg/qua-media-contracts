.PHONY: gen gen-ts gen-rust validate clean install

SCHEMAS := $(shell find schemas/v1 -name '*.schema.json' -o -name '*.yaml')

install:
	cd packages/ts && npm install

gen: gen-ts gen-rust

gen-ts:
	cd packages/ts && npm run gen
	cd packages/ts && npx tsc -p tsconfig.json

gen-rust:
	# Sync bundled schema copy (committed for registry publishing).
	rsync -a --delete schemas/v1/ crates/qua-media-contracts/schemas/v1/
	cargo build --manifest-path crates/qua-media-contracts/Cargo.toml

validate:
	cd packages/ts && npm run validate

clean:
	rm -rf packages/ts/node_modules packages/ts/dist packages/ts/src/index.ts
	cargo clean --manifest-path crates/qua-media-contracts/Cargo.toml
