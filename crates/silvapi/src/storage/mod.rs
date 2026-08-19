//! Persistence, with a backend per platform behind one API.
//!
//! Native builds use SQLite on disk. The web has no filesystem and no bundled
//! SQLite, so it uses `localStorage`, which is synchronous and therefore a drop-in
//! for the same blocking signatures.
//!
//! Both backends expose exactly the functions below, so callers never branch.

#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(not(target_family = "wasm"))]
pub use native::*;

#[cfg(target_family = "wasm")]
mod web;
#[cfg(target_family = "wasm")]
pub use web::*;
