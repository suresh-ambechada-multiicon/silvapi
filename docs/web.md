# Running silvapi in the browser

Status: **builds and boots, does not render yet.** The blocker is a build
configuration, not our code. Details below.

## What works

- `cargo check`/`build` for `wasm32-unknown-unknown` is green, and the native
  build is unchanged.
- The page loads, acquires a WebGPU adapter, and `gpui_web` creates its canvas
  and IME input element.
- gpui runs frames: the render tree executes every frame (visible in stack
  traces from `gpui::elements::svg`).
- Icons load, storage works against `localStorage`, requests go through `fetch`.

## What does not

Nothing paints. The console repeats:

    Uncaught Error: closure invoked recursively or after being dropped

thrown from the `requestAnimationFrame` callback, the `ResizeObserver`
callback, and DOM event listeners. There is no Rust panic.

## Why: we are building an unsupported configuration

`gpui_web`'s only upstream example, `crates/gpui_web/examples/hello_web`,
requires a **threaded** wasm build:

    # rust-toolchain.toml
    channel = "nightly"
    components = ["rust-src"]

    # .cargo/config.toml
    [target.wasm32-unknown-unknown]
    rustflags = [
      "-C", "target-feature=+atomics,+bulk-memory,+mutable-globals",
      "-C", "link-arg=--shared-memory",
      "-C", "link-arg=--max-memory=1073741824",
      "-C", "link-arg=--import-memory",
      "-C", "link-arg=--export=__wasm_init_tls",
      "-C", "link-arg=--export=__tls_size",
      "-C", "link-arg=--export=__tls_align",
      "-C", "link-arg=--export=__tls_base",
    ]
    [unstable]
    build-std = ["std,panic_abort"]

We build **single-threaded** instead, because the threaded path pulls
`wasm_thread`, which needs `feature(stdarch_wasm_atomic_wait)` and therefore a
nightly compiler. Evidence that single-threaded is not exercised upstream: it
does not compile at all at this revision — `shared_memory_supported` is gated on
the `multithreaded` feature but called unconditionally. See `vendor/gpui_web`.

Contributing factors on the single-threaded path:

- `gpui_wgpu` blocks on GPU work: `device.poll(PollType::Wait)` in the surface
  resize path, and `gpui::block_on` in `wgpu_context`/`wgpu_atlas`. A browser's
  main thread cannot block; WebGPU only progresses when control returns to the
  event loop.
- `WebPlatform::run` spawns its startup future and returns immediately, so
  `Application::run` returns and drops the `Application` rather than blocking in
  an event loop the way native platforms do.

## To finish it

1. Install a nightly toolchain with `rust-src` (rustup installs user-local, no
   root):

       curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
       rustup toolchain install nightly
       rustup component add rust-src --toolchain nightly
       rustup target add wasm32-unknown-unknown --toolchain nightly

2. Add the `rust-toolchain.toml` and `.cargo/config.toml` above.
3. Drop the `default-features = false` on `gpui_web` in
   `crates/silvapi/Cargo.toml` so the `multithreaded` feature is enabled, and
   revert `vendor/gpui_web` to plain upstream (the fix is only needed
   single-threaded).
4. Switch `start()` in `crates/silvapi/src/main.rs` back to
   `gpui_platform::application()` and `gpui_platform::web_init()`.
5. Serve with COOP/COEP so `SharedArrayBuffer` is available —
   `scripts/serve-web.sh` already sends both.

## Web-only behaviour differences

These are inherent to the platform, not temporary:

- **No SSE, no progressive response rendering.** wasm `reqwest` is fetch-backed
  with no body stream, so a response body arrives in one piece.
- **Storage is `localStorage`**, a few MB per origin, so response history is
  capped at 10 per request instead of 20.
- **No theme hot-reload** — there is no directory to watch.
- Connection failures lose their cause; `fetch` does not report why.
- Cancelling a request stops the result being delivered, but cannot abort the
  underlying `fetch`.

## Building

    ./scripts/serve-web.sh          # release
    ./scripts/serve-web.sh --debug

WebGPU is only exposed on `localhost` or HTTPS. On Linux, Chrome ships it
disabled; the page detects a missing adapter and prints the flags to use:

    google-chrome-stable --enable-unsafe-webgpu --enable-features=Vulkan http://localhost:3000
