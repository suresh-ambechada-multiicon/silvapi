//! Shared async runtime for non-blocking network I/O.
//!
//! `reqwest` (even its blocking client) is built on tokio + hyper, so tokio is
//! already compiled into the binary. Here we make that runtime explicit and
//! shared: one multi-threaded runtime handles every in-flight request, instead
//! of the previous model of one OS thread + a throwaway runtime per request.
//! Futures spawned here can be aborted mid-flight (real cancellation), and
//! results are bridged back to the gpui UI thread over channels.

use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

#[cfg(not(target_arch = "wasm32"))]
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("silvapi-net")
            .build()
            .expect("failed to build the shared tokio runtime")
    })
}

/// Spawn a future onto the shared network runtime. The returned `JoinHandle`
/// (or its `abort_handle()`) can be used to cancel the task mid-flight.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(future)
}

/// Spawn a future onto the shared network runtime. The returned `JoinHandle`
/// (or its `abort_handle()`) can be used to cancel the task mid-flight.
#[cfg(target_arch = "wasm32")]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    tokio::task::spawn_local(future)
}
