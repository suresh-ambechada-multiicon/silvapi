#!/usr/bin/env bash
# Build the wasm bundle and serve it on http://localhost:3000
#
#   ./scripts/serve-web.sh            # release build (slow to compile, fast to run)
#   ./scripts/serve-web.sh --debug    # faster compile, much larger .wasm
#
# WebGPU is only exposed on localhost or HTTPS, so serving over the network by IP
# will load the page but fail to start the renderer.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

profile="release"
profile_flag="--release"
if [[ "${1:-}" == "--debug" ]]; then
  profile="debug"
  profile_flag=""
fi

echo "==> building silvapi for wasm32-unknown-unknown (${profile})"
cargo build --target wasm32-unknown-unknown -p silvapi ${profile_flag}

wasm="target/wasm32-unknown-unknown/${profile}/silvapi.wasm"
[[ -f "$wasm" ]] || { echo "error: $wasm not found" >&2; exit 1; }

if ! command -v wasm-bindgen >/dev/null; then
  # The CLI must match the wasm-bindgen crate version exactly or the generated
  # glue will not line up with the symbols in the module.
  version="$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/[",]/,""); print $3; exit}' Cargo.lock)"
  echo "error: wasm-bindgen CLI missing. Install the matching version:" >&2
  echo "  cargo install wasm-bindgen-cli --version ${version}" >&2
  exit 1
fi

echo "==> generating JS bindings"
wasm-bindgen "$wasm" --out-dir web --target web --no-typescript

echo "==> serving http://localhost:3000  (Ctrl+C to stop)"
cd web
exec python3 -m http.server 3000
