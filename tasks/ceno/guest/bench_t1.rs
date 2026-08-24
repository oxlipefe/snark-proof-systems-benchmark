//! zk-prover-bench · Ceno · T1 — the INT8 matrix-multiply ladder of `bench/TASKS.md`.
//!
//! `C = A[M x K] . B[K x N]`, signed INT8 operands, INT32 accumulator, output **not**
//! requantised. One RISC-V guest program serves every rung: the shape arrives as a hint, so
//! the whole ladder shares one ELF and therefore one proving/verifying key. That is a
//! deliberate choice and it is declared in EXPRESSION.md — it removes per-rung keygen as a
//! confounder, at the cost of loop bounds the compiler cannot constant-fold.
//!
//! # What a zkVM is actually proving here
//!
//! Nothing in this file is a constraint. It is ordinary RISC-V, and Ceno proves *the
//! execution of these instructions*, not the arithmetic identity they compute. The
//! denominator of `bytes/MAC` is still the MAC count `bench/TASKS.md` fixes, but the
//! numerator is paid per *cycle*, and there are several cycles per MAC. Every comparison
//! against a circuit-based system in RESULTS.md carries that sentence with it.
//!
//! # Hint layout
//!
//! Three records, read in order via `ceno_rt::read_slice()`:
//!
//! | # | bytes | content |
//! |---|---|---|
//! | 0 | 12 | `M`, `K`, `N` as little-endian `u32` |
//! | 1 | `M*K` | `A`, row-major, one `i8` per byte |
//! | 2 | `K*N` | `B`, row-major, one `i8` per byte |
//!
//! # Output
//!
//! `C` is materialised in full as `M*N` little-endian `i32` on the heap — the task's output
//! is the INT32 matrix, so the memory writes that produce it are part of the measured work
//! — and then bound to the proof's public values as a Keccak-256 digest via
//! `ceno_rt::commit`. `commit` hashes in *software*; its cost is isolated by the
//! `BENCH_NO_COMMIT` build (see `bench_t1_nocommit.rs`) and reported separately.

extern crate ceno_rt;

extern crate alloc;
use alloc::vec;

fn main() {
    let header = ceno_rt::read_slice();
    assert!(header.len() == 12, "T1 header must be three u32 words");
    let m = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let k = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let n = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;

    let a = ceno_rt::read_slice();
    let b = ceno_rt::read_slice();
    assert!(a.len() == m * k, "A has the wrong length for the declared shape");
    assert!(b.len() == k * n, "B has the wrong length for the declared shape");

    let mut out = vec![0u8; m * n * 4];

    for i in 0..m {
        let a_row = &a[i * k..i * k + k];
        for j in 0..n {
            let mut acc: i32 = 0;
            for kk in 0..k {
                // `as i8 as i32` is a sign-extending byte load: the hint region carries the
                // INT8 operand one per byte, and RISC-V `lb` sign-extends.
                acc += (a_row[kk] as i8 as i32) * (b[kk * n + j] as i8 as i32);
            }
            let at = (i * n + j) * 4;
            out[at..at + 4].copy_from_slice(&acc.to_le_bytes());
        }
    }

    ceno_rt::commit(&out);
}
