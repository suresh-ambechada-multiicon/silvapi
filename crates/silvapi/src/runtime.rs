//! Shared async runtime for non-blocking network I/O.
//!
//! Two very different backends behind one API:
//!
//! - **Native**: one shared multi-threaded tokio runtime handles every in-flight
//!   request, rather than an OS thread plus a throwaway runtime per request.
//!   Aborting drops the task, which really does stop the network I/O.
//! - **Web**: there is one thread, so tasks run on the browser's event loop via
//!   `spawn_local`. Aborting stops the future from being polled again; the
//!   underlying `fetch` is not cancelled at the network level, so a cancelled
//!   request may still finish in the background — its result is simply dropped.
//!
//! Results are bridged back to the gpui UI thread over channels either way.

use std::future::Future;

#[cfg(not(target_family = "wasm"))]
mod platform {
    use super::Future;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

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

    /// Cancels an in-flight task. Cheap to clone and store.
    #[derive(Clone)]
    pub struct AbortHandle(tokio::task::AbortHandle);

    impl AbortHandle {
        pub fn abort(&self) {
            self.0.abort();
        }
    }

    pub struct Task<T>(tokio::task::JoinHandle<T>);

    impl<T> Task<T> {
        pub fn abort_handle(&self) -> AbortHandle {
            AbortHandle(self.0.abort_handle())
        }
    }

    pub fn spawn<F>(future: F) -> Task<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        Task(runtime().spawn(future))
    }
}

#[cfg(target_family = "wasm")]
mod platform {
    use super::Future;
    use futures::future::{AbortHandle as FuturesAbortHandle, Abortable};
    use std::marker::PhantomData;

    /// Cancels an in-flight task. Cheap to clone and store.
    #[derive(Clone)]
    pub struct AbortHandle(FuturesAbortHandle);

    impl AbortHandle {
        pub fn abort(&self) {
            self.0.abort();
        }
    }

    pub struct Task<T> {
        abort: AbortHandle,
        _output: PhantomData<T>,
    }

    impl<T> Task<T> {
        pub fn abort_handle(&self) -> AbortHandle {
            self.abort.clone()
        }
    }

    /// No `Send` bound: on the web everything runs on one thread, and the
    /// browser-backed futures reqwest produces there are not `Send` anyway.
    pub fn spawn<F>(future: F) -> Task<F::Output>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        let (handle, registration) = FuturesAbortHandle::new_pair();
        wasm_bindgen_futures::spawn_local(async move {
            // The task's output is delivered over a channel by the caller, so
            // both completion and cancellation are discarded here.
            let _ = Abortable::new(future, registration).await;
        });
        Task {
            abort: AbortHandle(handle),
            _output: PhantomData,
        }
    }
}

pub use platform::{AbortHandle, Task, spawn};
