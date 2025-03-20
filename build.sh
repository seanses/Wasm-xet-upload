RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --dev --out-dir www/pkg --target no-modules
