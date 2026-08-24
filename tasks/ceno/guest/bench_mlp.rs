//! zk-prover-bench · Ceno · T2 and T3 — the 200-256-128-64-1 MLP of `bench/TASKS.md`.
//!
//! One RISC-V guest program serves both: T2 is batch 1, T3 is batch 8, and the batch size
//! arrives as a hint, so both share one ELF and therefore one proving/verifying key. T3
//! proves 8 independent inputs over the same weights in **one** proof.
//!
//! # Requantisation: none, per Amendment A1
//!
//! Accumulators carry full width from one layer to the next. The consequence for a 32-bit
//! RISC-V target is direct and is the reason this file is worth reading: only layer 1's
//! accumulator fits in `i32`. Layer 1's worst case over all INT8 operands is
//! `200 * 128 * 128 = 3_276_800`, comfortably inside `i32`. From layer 2 on the accumulator
//! needs 64 bits, and **RV32IM has no 64-bit arithmetic** — every `i64` multiply-accumulate
//! is lowered to a multi-instruction sequence over register pairs. That is not a defect of
//! Ceno; it is what "no requantisation" costs on a 32-bit machine, and it is declared here
//! and in EXPRESSION.md rather than hidden inside a cycle count.
//!
//! The instance's own bound is asserted **on the host**, in the generator, in `i128`, with
//! the factor-of-two margin `bench/TASKS.md` A1 demands — the same discipline binius64's
//! builder applies. The guest does not re-derive it, because a guest-side check would be
//! paid for in proved cycles and would not make the emitted circuit any safer.
//!
//! # Hint layout
//!
//! Three records, read in order via `ceno_rt::read_slice()`:
//!
//! | # | bytes | content |
//! |---|---|---|
//! | 0 | 4 | `BATCH` as a little-endian `u32` (1 for T2, 8 for T3) |
//! | 1 | 51 200 + 32 768 + 8 192 + 64 | the four weight matrices, each `[out][in]` row-major, one `i8` per byte |
//! | 2 | `BATCH * 200` | the inputs, row-major, one `i8` per byte |
//!
//! # Output
//!
//! `BATCH` little-endian `i64` outputs, bound to the proof's public values as a Keccak-256
//! digest via `ceno_rt::commit`.

extern crate ceno_rt;

extern crate alloc;
use alloc::{vec, vec::Vec};

/// Layer widths: input, then one entry per layer output. `bench/TASKS.md` T2.
const WIDTHS: [usize; 5] = [200, 256, 128, 64, 1];

fn main() {
    let header = ceno_rt::read_slice();
    assert!(header.len() == 4, "MLP header must be one u32 word");
    let batch = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;

    let weights_flat = ceno_rt::read_slice();
    let inputs = ceno_rt::read_slice();

    let mut weight_len = 0usize;
    for layer in 0..WIDTHS.len() - 1 {
        weight_len += WIDTHS[layer] * WIDTHS[layer + 1];
    }
    assert!(weights_flat.len() == weight_len, "weight block has the wrong length");
    assert!(inputs.len() == batch * WIDTHS[0], "input block has the wrong length");

    let mut out = vec![0u8; batch * 8];

    for s in 0..batch {
        let sample = &inputs[s * WIDTHS[0]..(s + 1) * WIDTHS[0]];

        // Layer 1 is the only one whose accumulator provably fits in i32 (worst case
        // 200 * 128 * 128 = 3_276_800), so it is computed in i32 and the result widened.
        // Doing every layer in i64 would be simpler and would cost Ceno cycles it does not
        // owe; the benchmark's fairness protocol says give the system its best honest
        // expression.
        let mut act: Vec<i64> = Vec::with_capacity(WIDTHS[1]);
        {
            let w = &weights_flat[0..WIDTHS[0] * WIDTHS[1]];
            for o in 0..WIDTHS[1] {
                let row = &w[o * WIDTHS[0]..(o + 1) * WIDTHS[0]];
                let mut acc: i32 = 0;
                for i in 0..WIDTHS[0] {
                    acc += (sample[i] as i8 as i32) * (row[i] as i8 as i32);
                }
                // ReLU after layers 1-3.
                act.push(if acc > 0 { acc as i64 } else { 0 });
            }
        }

        let mut base = WIDTHS[0] * WIDTHS[1];
        for layer in 1..WIDTHS.len() - 1 {
            let fan_in = WIDTHS[layer];
            let fan_out = WIDTHS[layer + 1];
            let is_last = layer == WIDTHS.len() - 2;
            let w = &weights_flat[base..base + fan_in * fan_out];
            let mut next: Vec<i64> = Vec::with_capacity(fan_out);
            for o in 0..fan_out {
                let row = &w[o * fan_in..(o + 1) * fan_in];
                let mut acc: i64 = 0;
                for i in 0..fan_in {
                    acc += act[i] * (row[i] as i8 as i64);
                }
                // The output layer is linear; layers 1-3 take a ReLU.
                next.push(if is_last || acc > 0 { acc } else { 0 });
            }
            act = next;
            base += fan_in * fan_out;
        }

        assert!(act.len() == 1, "the network must end in a single output");
        out[s * 8..s * 8 + 8].copy_from_slice(&act[0].to_le_bytes());
    }

    ceno_rt::commit(&out);
}
