
all: contracts/wormhole/target/wasm32-unknown-unknown/release/wormhole.wasm contracts/portal/target/wasm32-unknown-unknown/release/portal.wasm

contracts/wormhole/target/wasm32-unknown-unknown/release/wormhole.wasm: contracts/wormhole/src/*.rs contracts/wormhole/Cargo.toml
	(cd contracts/wormhole; cargo build --target wasm32-unknown-unknown --release)

contracts/portal/target/wasm32-unknown-unknown/release/portal.wasm: contracts/portal/src/*.rs  contracts/portal/Cargo.toml
	(cd contracts/portal; cargo build --target wasm32-unknown-unknown --release)

test: all FORCE
	npx ts-node test/test.ts

FORCE:
