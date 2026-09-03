//! zk-prover-bench · the Plonky3 measurement harness.
//!
//! OUR code. Plonky3 is never vendored into this repository; this crate depends on a pinned
//! clone by path (see `Cargo.toml.in` and `systems/plonky3/COMMIT`) and calls only its public
//! API.
//!
//! The campaign this serves is the CROSS-FIELD one: the same task, the same machine, the same
//! campaign and — the part nobody else has — **the same codebase**, over a small prime field
//! and over a binary tower field. Read `systems/plonky3/EXPRESSION.md` before any number: the
//! two fields do not prove the same theorem, and the difference is not a detail.

pub mod fields;
pub mod matmul;
pub mod mle;
pub mod pcs;
/// A deliberate compile failure; see the module doc. Off by default.
#[cfg(feature = "probe-binary-pcs")]
pub mod probe_binary_pcs;
pub mod route;
pub mod sanity;
pub mod stats;
pub mod tasks;
