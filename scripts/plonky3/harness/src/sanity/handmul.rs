//! Hand-written GF(2^128) multiply, the same 6-PMULL schoolbook binius64's aarch64 path
//! describes, written directly on `uint64x2_t` with no wrapper types.
//!
//! Its only purpose is to separate *algorithm* from *codegen*: if this runs far faster
//! than `binius_field`'s `B128 * B128`, the gap is in how the library's abstraction
//! lowers, not in the number of carryless multiplies the field costs.

#![cfg(target_arch = "aarch64")]

use std::arch::aarch64::*;

/// x^128 + x^7 + x^2 + x + 1
const POLY: u64 = 0x87;

#[inline(always)]
unsafe fn pmull_00(a: uint64x2_t, b: uint64x2_t) -> uint64x2_t {
	unsafe { std::mem::transmute(vmull_p64(vgetq_lane_u64(a, 0), vgetq_lane_u64(b, 0))) }
}
#[inline(always)]
unsafe fn pmull_11(a: uint64x2_t, b: uint64x2_t) -> uint64x2_t {
	unsafe { std::mem::transmute(vmull_p64(vgetq_lane_u64(a, 1), vgetq_lane_u64(b, 1))) }
}
#[inline(always)]
unsafe fn pmull_01(a: uint64x2_t, b: uint64x2_t) -> uint64x2_t {
	unsafe { std::mem::transmute(vmull_p64(vgetq_lane_u64(a, 1), vgetq_lane_u64(b, 0))) }
}
#[inline(always)]
unsafe fn pmull_10(a: uint64x2_t, b: uint64x2_t) -> uint64x2_t {
	unsafe { std::mem::transmute(vmull_p64(vgetq_lane_u64(a, 0), vgetq_lane_u64(b, 1))) }
}

#[inline(always)]
unsafe fn xor(a: uint64x2_t, b: uint64x2_t) -> uint64x2_t {
	unsafe { vreinterpretq_u64_u8(veorq_u8(vreinterpretq_u8_u64(a), vreinterpretq_u8_u64(b))) }
}

#[inline(always)]
unsafe fn move_64_to_hi(a: uint64x2_t) -> uint64x2_t {
	unsafe {
		let zero = vdupq_n_u8(0);
		vreinterpretq_u64_u8(vextq_u8::<8>(zero, vreinterpretq_u8_u64(a)))
	}
}

/// `t0 + x^64 * t1`, one PMULL.
#[inline(always)]
unsafe fn reduce_step(t0: uint64x2_t, t1: uint64x2_t) -> uint64x2_t {
	unsafe {
		let poly = vsetq_lane_u64(0, vdupq_n_u64(POLY), 1);
		let mut r = xor(t0, move_64_to_hi(t1));
		r = xor(r, pmull_01(t1, poly));
		r
	}
}

/// Full GF(2^128) multiply: 4 schoolbook PMULL + 2 reduction PMULL.
#[inline(always)]
pub unsafe fn gf128_mul(a: uint64x2_t, b: uint64x2_t) -> uint64x2_t {
	unsafe {
		let lo = pmull_00(a, b);
		let hi = pmull_11(a, b);
		let mid = xor(pmull_01(a, b), pmull_10(a, b));
		let t1 = reduce_step(mid, hi);
		reduce_step(lo, t1)
	}
}

pub fn splat(v: u128) -> uint64x2_t {
	unsafe { std::mem::transmute(v) }
}
pub fn unsplat(v: uint64x2_t) -> u128 {
	unsafe { std::mem::transmute(v) }
}
