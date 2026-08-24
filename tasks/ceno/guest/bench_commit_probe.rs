//! zk-prover-bench · Ceno · the commit-cost probe. **Not a task.**
//!
//! `bench/TASKS.md` defines T1/T2/T3 as arithmetic. A zkVM guest cannot publish an output
//! the way a circuit publishes an `inout` wire: it binds the output by hashing it into the
//! proof's public values, and `ceno_rt::commit` does that Keccak-256 **in software**, as
//! ordinary RISC-V. So a share of every T1/T2/T3 cycle count is hashing, not multiplying.
//!
//! This program exists to measure that share and nothing else. It reads a length `L` and a
//! byte pattern, fills an `L`-byte heap buffer, and commits it. Run at `L = 0` and at the
//! output size of each task, the difference in cycle count is the cost `ceno_rt::commit`
//! adds to that task — measured, not estimated from the Keccak block count.
//!
//! It is never proved and never appears in a published proving figure. It is an instrument,
//! and it is in this directory so the instrument is auditable alongside what it measured.
//!
//! # Hint layout
//!
//! | # | bytes | content |
//! |---|---|---|
//! | 0 | 4 | `L`, the number of bytes to commit, as a little-endian `u32` |

extern crate ceno_rt;

extern crate alloc;
use alloc::vec;

fn main() {
    let header = ceno_rt::read_slice();
    assert!(header.len() == 4, "probe header must be one u32 word");
    let len = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;

    // A cheap deterministic fill, so the buffer is not all zeros and the write traffic is
    // present. Keccak's cost does not depend on the bytes, only on their number.
    let mut buf = vec![0u8; len];
    let mut x: u32 = 0x9E37_79B9;
    for b in buf.iter_mut() {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *b = (x >> 24) as u8;
    }

    ceno_rt::commit(&buf);
}
