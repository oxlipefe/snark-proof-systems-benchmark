//! Sanity checks that must pass before any measured number is accepted.
//!
//! # Why this exists
//!
//! Rounds 1-3 of E-001 were measured with the harness workspace's `[profile.release]`
//! missing `lto = "thin"`, which binius64's own workspace sets. Because the harness is a
//! separate workspace, the prover under measurement was built with cross-crate inlining
//! off: its GF(2^128) multiply emitted eight non-inlined calls around the six PMULL the
//! algorithm actually costs, and ran **27.6x slower** than the library ships. The whole
//! campaign was rerun.
//!
//! Nothing in the phase-split output could have revealed that on its own — the shares
//! still summed to one. What reveals it is dividing the measured kernel rate by the
//! throughput of the *hardware primitive* it is built from. That division is this module.

pub mod handmul;

use std::time::Instant;

use binius_field::{BinaryField128bGhash as B128, Field, arch::OptimalPackedB128 as P};

/// Independent accumulator chains. Enough to cover the multiplier's latency; the measured
/// rate is flat from 2 lanes up, so this is a throughput figure, not a latency one.
const LANES: usize = 16;

/// Repetitions; the best is reported, as everywhere else in E-001.
const REPS: usize = 5;

/// The multiply is a schoolbook product (4 carryless multiplies) plus a reduction
/// (2 more). See `crates/field/src/arch/aarch64/arithmetic/ghash.rs` in the pinned binius64 clone.
pub const PMULL_PER_FIELD_MUL: f64 = 6.0;

/// A build is rejected below this fraction of the hand-written kernel's rate. The gap seen
/// with LTO off was 27.6x, i.e. a ratio of 0.036; a healthy build sits above 0.9. The
/// threshold is deliberately loose: it exists to catch a broken build, not to police noise.
pub const MIN_RATIO_VS_HAND: f64 = 0.5;

fn best<F: FnMut()>(mut f: F) -> f64 {
	f();
	let mut samples = Vec::with_capacity(REPS);
	for _ in 0..REPS {
		let started = Instant::now();
		f();
		samples.push(started.elapsed().as_secs_f64());
	}
	samples.sort_by(|x, y| x.partial_cmp(y).expect("timings are never NaN"));
	samples[0]
}

/// Throughput of `binius_field`'s own `B128 * B128`, in millions per second.
pub fn binius_mul_rate(iters: u64) -> f64 {
	let secs = best(|| {
		let mut acc = [P::broadcast(B128::MULTIPLICATIVE_GENERATOR); LANES];
		let step = P::broadcast(B128::MULTIPLICATIVE_GENERATOR);
		for _ in 0..iters {
			for slot in &mut acc {
				*slot *= step;
			}
		}
		std::hint::black_box(&acc);
	});
	iters as f64 * LANES as f64 / secs / 1e6
}

/// Throughput of the same field multiply written directly on the intrinsics, in millions
/// per second. This is the reference: same algorithm, same PMULL count, no abstraction.
#[cfg(target_arch = "aarch64")]
pub fn hand_mul_rate(iters: u64) -> f64 {
	let secs = best(|| {
		let mut acc = [handmul::splat(0x0123456789abcdef_fedcba9876543210); LANES];
		for (i, slot) in acc.iter_mut().enumerate() {
			*slot = handmul::splat(handmul::unsplat(*slot) ^ (i as u128 + 1));
		}
		let k = handmul::splat(0xdeadbeefcafef00d_0123456789abcdef);
		for _ in 0..iters {
			for slot in &mut acc {
				*slot = unsafe { handmul::gf128_mul(*slot, k) };
			}
		}
		std::hint::black_box(acc.map(handmul::unsplat));
	});
	iters as f64 * LANES as f64 / secs / 1e6
}

/// Raw carryless-multiply throughput of the machine, in millions per second. The ceiling
/// every field multiply is built from.
#[cfg(target_arch = "aarch64")]
pub fn pmull_rate(iters: u64) -> f64 {
	use std::arch::aarch64::vmull_p64;
	let secs = best(|| {
		let mut acc = [0u64; LANES];
		for (i, slot) in acc.iter_mut().enumerate() {
			*slot = 0x9e3779b97f4a7c15u64.wrapping_mul(i as u64 + 1) | 1;
		}
		let k: u64 = 0xdeadbeefcafef00d;
		for _ in 0..iters {
			for slot in &mut acc {
				let p: u128 = unsafe { vmull_p64(*slot, k) };
				*slot = (p as u64) ^ ((p >> 64) as u64);
			}
		}
		std::hint::black_box(&acc);
	});
	iters as f64 * LANES as f64 / secs / 1e6
}
