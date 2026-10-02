#!/bin/bash
# Build silvapi for WebAssembly target
RUSTC_BOOTSTRAP=1 cargo build --target wasm32-unknown-unknown -p silvapi "$@"
