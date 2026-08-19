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

# The threaded wasm build needs a nightly toolchain: gpui_web's multithreaded
# dispatcher uses wasm_thread, which requires feature(stdarch_wasm_atomic_wait),
# and shared memory requires rebuilding std with atomics via -Z build-std.
#
# Deliberately set per-invocation rather than in .cargo/config.toml: `build-std`
# there is not target-scoped and would force a std rebuild for native builds too.
#
# __heap_base is exported on top of upstream's list because wasm-bindgen needs it
# to inject per-thread ids, and fails with "failed to find `__heap_base`" without it.
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly}"
export CARGO_UNSTABLE_BUILD_STD="std,panic_abort"
export RUSTFLAGS="\
-C target-feature=+atomics,+bulk-memory,+mutable-globals \
-C link-arg=--shared-memory \
-C link-arg=--max-memory=1073741824 \
-C link-arg=--import-memory \
-C link-arg=--export=__wasm_init_tls \
-C link-arg=--export=__tls_size \
-C link-arg=--export=__tls_align \
-C link-arg=--export=__tls_base \
-C link-arg=--export=__heap_base"

if ! rustup toolchain list 2>/dev/null | grep -q nightly; then
  echo "error: a nightly toolchain is required for the threaded wasm build." >&2
  echo "  rustup toolchain install nightly" >&2
  echo "  rustup component add rust-src --toolchain nightly" >&2
  exit 1
fi

echo "==> building silvapi for wasm32-unknown-unknown (${profile}, threaded)"
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
# COOP/COEP are what gate SharedArrayBuffer, which gpui_web's multithreaded
# dispatcher needs. Harmless for the single-threaded build, and required the
# moment threads are enabled, so always send them.
exec python3 - <<'PY'
import functools, http.server

class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        # The bundle is rebuilt constantly during development.
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

http.server.test(HandlerClass=Handler, port=3000, bind="127.0.0.1")
PY
