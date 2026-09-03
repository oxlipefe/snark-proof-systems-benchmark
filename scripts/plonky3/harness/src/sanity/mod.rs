//! Build-integrity checks that must pass before any measured number is accepted.
//!
//! # Why this exists, and why the criterion is a ratio
//!
//! Rounds 1-3 of experiment E-001 measured binius64 from a harness workspace whose
//! `[profile.release]` was missing `lto = "thin"`. Because the harness is a separate
//! workspace, the prover under measurement was built with cross-crate inlining off, its
//! `GF(2^128)` multiply ran **27.6x slower** than the library ships, and the campaign's
//! conclusion inverted. Nothing in the timing output revealed it: the phase shares still
//! summed to one. What revealed it was dividing the measured kernel rate by the throughput of
//! the hardware primitive the kernel is built from. That division is this module.
//!
//! Plonky3 has a second failure of the same class, and it is a `cfg` rather than a profile:
//! `p3-binary-field`'s carryless-multiply backend is compiled only under
//! `target_feature = "aes"` (`binary-field/src/clmul/mod.rs:14-31`), and without it every
//! `GF(2^128)` multiply falls back to a bit-serial loop that is roughly two orders of
//! magnitude slower. A cell built without `-C target-cpu=native` on a target that does not
//! carry `aes` in its baseline would produce plausible, ruinous numbers. The same ratio
//! catches it.
//!
//! # What the binary reference is, and what it is NOT
//!
//! The hand-written kernel below is the **6-PMULL schoolbook GHASH multiply** — the algorithm
//! binius64's aarch64 path uses, kept byte-identical to E-001's so the two campaigns stay
//! comparable. **Plonky3's multiply is a different algorithm.** It stores elements in the
//! Wiedemann tower basis, so each multiply is: map both operands into the polynomial basis
//! (`clmul/basis.rs`, byte-at-a-time table lookups), four `clmul_64x64`, a two-round shift/XOR
//! reduction, and a map back. The 6-PMULL rate is therefore a **ceiling this multiply is not
//! expected to reach**, and the ratio it produces is a representation tax plus a codegen
//! check, not a codegen check alone. The gate's floor is set to catch the two order-of-
//! magnitude failures above, not to police the tax; the tax itself is reported as a number.

pub mod handmul;

use std::time::Instant;

use p3_binary_field::BinaryField128;
use p3_field::{Field, PrimeCharacteristicRing};
use p3_koala_bear::KoalaBear;

/// Independent accumulator chains, enough to cover the multiplier's latency, so the figure is
/// throughput and not latency.
const LANES: usize = 16;

/// Repetitions; the best is reported, as in E-001 and E-006.
const REPS: usize = 5;

/// A build is rejected below this fraction of the hand-written 6-PMULL kernel's rate.
///
/// It is deliberately far below 1: see the module doc — Plonky3's multiply pays a basis change
/// the reference does not. The two failures this exists to catch are 20x-100x, so a floor here
/// separates them from the tax with two orders of margin either side.
pub const MIN_B128_RATIO_VS_HAND: f64 = 0.02;

/// A build is rejected above this many raw 32-bit integer multiplies per KoalaBear multiply.
///
/// A Montgomery multiply in a 31-bit prime field costs two 32-bit multiplies and a few adds,
/// so a healthy build lands near 2-6 once the loop's own overhead is counted. Twenty-four is
/// four times the loosest healthy value: it catches a multiply that has become a function call
/// or a division, not one that is a cycle slower than optimal.
pub const MAX_U32_MULS_PER_KB_MUL: f64 = 24.0;

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

/// Throughput of `p3-binary-field`'s own `BinaryField128 * BinaryField128`, in Mmul/s.
pub fn p3_b128_mul_rate(iters: u64) -> f64 {
    let secs = best(|| {
        let mut acc = [BinaryField128::GENERATOR; LANES];
        for (i, slot) in acc.iter_mut().enumerate() {
            *slot += BinaryField128::from_u8(i as u8 + 1);
        }
        let step = BinaryField128::GENERATOR;
        for _ in 0..iters {
            for slot in &mut acc {
                *slot *= step;
            }
        }
        std::hint::black_box(&acc);
    });
    iters as f64 * LANES as f64 / secs / 1e6
}

/// Throughput of `p3-koala-bear`'s `KoalaBear * KoalaBear`, in Mmul/s.
pub fn p3_koalabear_mul_rate(iters: u64) -> f64 {
    let secs = best(|| {
        let mut acc = [KoalaBear::GENERATOR; LANES];
        for (i, slot) in acc.iter_mut().enumerate() {
            *slot += KoalaBear::from_u8(i as u8 + 1);
        }
        let step = KoalaBear::GENERATOR;
        for _ in 0..iters {
            for slot in &mut acc {
                *slot *= step;
            }
        }
        std::hint::black_box(&acc);
    });
    iters as f64 * LANES as f64 / secs / 1e6
}

/// Raw 32-bit integer-multiply throughput of the machine, in Mops/s. The ceiling a Montgomery
/// multiply is built from.
pub fn u32_mul_rate(iters: u64) -> f64 {
    let secs = best(|| {
        let mut acc = [0u32; LANES];
        for (i, slot) in acc.iter_mut().enumerate() {
            *slot = 0x9e37_79b9u32.wrapping_mul(i as u32 + 1) | 1;
        }
        let k: u32 = 0xcafe_f00d;
        for _ in 0..iters {
            for slot in &mut acc {
                *slot = slot.wrapping_mul(k) ^ (*slot >> 17);
            }
        }
        std::hint::black_box(&acc);
    });
    iters as f64 * LANES as f64 / secs / 1e6
}

/// Throughput of the 6-PMULL GHASH multiply written directly on the intrinsics, in Mmul/s.
///
/// Byte-identical to the kernel E-001 and E-006 used, which is what makes the binary row of
/// this gate comparable with `systems/binius64/BUILD.md` §2.
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

/// Raw carryless-multiply throughput of the machine, in Mops/s.
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
