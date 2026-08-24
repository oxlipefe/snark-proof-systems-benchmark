//! zk-prover-bench · Ceno · generate the hint file for every task, and the manifest.
//!
//! `bench/TASKS.md` fixes each task by an **exact MAC count**. That count is the denominator
//! of both `MAC/s` and `bytes/MAC`, so it is never recomputed here: this generator *asserts*
//! that the work it is denominating is exactly the published number of multiply-accumulates,
//! and refuses to write a file that does not.
//!
//! Nothing in this file is Ceno code. It emits a byte layout that Ceno's own
//! `ceno_host::CenoStdin` defines (`ceno_host/src/lib.rs`, `Items::finalise`), so that
//! `cargo ceno prove --hints-file` loads it and the guest's `ceno_rt::read_slice()` walks it.
//!
//! # The one thing this generator does that no other system's generator could
//!
//! DeepProve's and jolt-atlas's task generators are Python and draw from numpy's PCG64, so
//! they share binius64's **seeds** but not its **stream**: same shapes, same MAC counts,
//! different instances. Ceno's guest is Rust, so this generator uses the same crate, the
//! same `StdRng::seed_from_u64`, and the same draw order as
//! `scripts/binius64/harness/src/e006/{matmul,mlp}.rs`. **Ceno and
//! binius64 therefore prove the same instance, value for value**, and `--verify-instance`
//! prints the digest that lets anyone check it. That is a stronger comparison than the
//! benchmark's other pairs get, and it is declared in EXPRESSION.md rather than assumed.

use std::{fs, path::PathBuf};

use rand::{RngExt, SeedableRng, rngs::StdRng};

/// `bench/TASKS.md`, frozen. Never recomputed.
const PUBLISHED_MACS: [(&str, u64); 7] = [
    ("t1-0", 65_536),
    ("t1-a", 589_824),
    ("t1-b", 2_359_296),
    ("t1-c", 9_437_184),
    ("t1-d", 37_748_736),
    ("t2", 92_224),
    ("t3", 737_792),
];

/// `bench/systems/binius64/EXPRESSION.md` §7.
const SEEDS: [(&str, u64); 7] = [
    ("t1-0", 0xE006_0100),
    ("t1-a", 0xE006_01A0),
    ("t1-b", 0xE006_01B0),
    ("t1-c", 0xE006_01C0),
    ("t1-d", 0xE006_01D0),
    ("t2", 0xE006_0200),
    ("t3", 0xE006_0300),
];

/// T1 rungs: (M, K, N).
const T1_SHAPES: [(&str, usize, usize, usize); 5] = [
    ("t1-0", 1, 256, 256),
    ("t1-a", 1, 768, 768),
    ("t1-b", 4, 768, 768),
    ("t1-c", 16, 768, 768),
    ("t1-d", 64, 768, 768),
];

/// T2/T3: 200 -> 256 -> 128 -> 64 -> 1, ReLU after layers 1-3, linear output.
const WIDTHS: [usize; 5] = [200, 256, 128, 64, 1];

/// Safety factor demanded of the instance's largest accumulator against `i64::MAX`, per
/// `bench/TASKS.md` Amendment A1. The same factor binius64's builder demands.
const HEADROOM: i128 = 2;

const INT32_MIN: i128 = i32::MIN as i128;
const INT32_MAX: i128 = i32::MAX as i128;

/// Word alignment of the hint region, from `ceno_host::WORD_ALIGNMENT`.
const WORD_ALIGNMENT: usize = 4;

fn seed_of(task: &str) -> u64 {
    SEEDS.iter().find(|(t, _)| *t == task).expect("unknown task").1
}

fn published_macs(task: &str) -> u64 {
    PUBLISHED_MACS.iter().find(|(t, _)| *t == task).expect("unknown task").1
}

/// Reproduce `ceno_host::Items::finalise`: a header of `u32` words
/// `[data_offset, alignment, len_0, .., len_{n-1}]`, then every record's bytes back to back,
/// each padded up to `alignment`. `ceno_rt`'s `read_slice()` walks exactly this.
fn finalise(records: &[Vec<u8>]) -> Vec<u8> {
    let header_words = records.len() + 2;
    let data_offset = (4 * header_words).next_multiple_of(WORD_ALIGNMENT);

    let mut header: Vec<u32> = Vec::with_capacity(header_words);
    header.push(data_offset as u32);
    header.push(WORD_ALIGNMENT as u32);
    header.extend(records.iter().map(|r| r.len() as u32));

    let mut bytes: Vec<u8> = header.iter().flat_map(|w| w.to_le_bytes()).collect();
    bytes.resize(data_offset, 0);
    for r in records {
        bytes.extend_from_slice(r);
        let pad = r.len().next_multiple_of(WORD_ALIGNMENT) - r.len();
        bytes.extend(std::iter::repeat(0u8).take(pad));
    }
    bytes
}

/// A cheap order-sensitive digest of the drawn instance, so that "Ceno and binius64 prove
/// the same instance" is checkable rather than asserted. FNV-1a over the operand bytes in
/// draw order.
fn instance_digest(bytes: &[i8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u8 as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

struct Emitted {
    hints: Vec<u8>,
    /// The committed output, as the `u32` words `--public-io` expects. The guest hashes the
    /// little-endian bytes of its output with Keccak-256 and emits the digest as the proof's
    /// public values; `ceno_zkvm::e2e::public_io_words_to_digest_words` hashes the
    /// little-endian bytes of these words the same way. The two must agree or the proof does
    /// not verify — see systems/ceno/EXPRESSION.md §6.
    public_io: Vec<u32>,
    macs: u64,
    max_abs: i128,
    digest: u64,
    note: String,
}

/// T1. Draw order matches `harness/src/e006/matmul.rs`: `A` row-major `[M][K]`, then `B`
/// row-major `[K][N]`.
fn build_t1(task: &str, m: usize, k: usize, n: usize) -> Emitted {
    let mut rng = StdRng::seed_from_u64(seed_of(task));
    let a: Vec<Vec<i8>> = (0..m).map(|_| (0..k).map(|_| rng.random()).collect()).collect();
    let b: Vec<Vec<i8>> = (0..k).map(|_| (0..n).map(|_| rng.random()).collect()).collect();

    // Reference product in i128, so the range check cannot itself overflow. This is also
    // where the MAC count is counted rather than assumed.
    let mut macs: u64 = 0;
    let mut max_abs: i128 = 0;
    let mut public_io: Vec<u32> = Vec::with_capacity(m * n);
    for row in &a {
        for j in 0..n {
            let mut acc: i128 = 0;
            for (kk, &a_ik) in row.iter().enumerate() {
                acc += i128::from(a_ik) * i128::from(b[kk][j]);
                macs += 1;
                max_abs = max_abs.max(acc.abs());
            }
            assert!(
                (INT32_MIN..=INT32_MAX).contains(&acc),
                "{task}: an output element left the INT32 range bench/TASKS.md fixes"
            );
            // One INT32 output element is one `u32` word, little-endian, matching the guest's
            // `acc.to_le_bytes()`.
            public_io.push(acc as i32 as u32);
        }
    }

    let mut flat: Vec<i8> = Vec::with_capacity(m * k + k * n);
    for row in &a {
        flat.extend_from_slice(row);
    }
    for row in &b {
        flat.extend_from_slice(row);
    }
    let digest = instance_digest(&flat);

    let mut header = Vec::with_capacity(12);
    header.extend((m as u32).to_le_bytes());
    header.extend((k as u32).to_le_bytes());
    header.extend((n as u32).to_le_bytes());

    let a_bytes: Vec<u8> = a.iter().flatten().map(|&v| v as u8).collect();
    let b_bytes: Vec<u8> = b.iter().flatten().map(|&v| v as u8).collect();

    Emitted {
        hints: finalise(&[header, a_bytes, b_bytes]),
        public_io,
        macs,
        max_abs,
        digest,
        note: format!("A[{m}x{k}] . B[{k}x{n}], INT8 operands, INT32 accumulator, not requantised"),
    }
}

/// T2/T3. Draw order matches `harness/src/e006/mlp.rs`: weights layer by layer, each
/// `[out][in]`, then the batch inputs, each of length 200.
fn build_mlp(task: &str, batch: usize) -> Emitted {
    let mut rng = StdRng::seed_from_u64(seed_of(task));
    let weights: Vec<Vec<Vec<i8>>> = (0..WIDTHS.len() - 1)
        .map(|layer| {
            (0..WIDTHS[layer + 1])
                .map(|_| (0..WIDTHS[layer]).map(|_| rng.random()).collect())
                .collect()
        })
        .collect();
    let inputs: Vec<Vec<i8>> = (0..batch)
        .map(|_| (0..WIDTHS[0]).map(|_| rng.random()).collect())
        .collect();

    let mut macs: u64 = 0;
    let mut max_abs: i128 = 0;
    let mut public_io: Vec<u32> = Vec::with_capacity(batch * 2);
    for input in &inputs {
        let mut act: Vec<i128> = input.iter().map(|&v| i128::from(v)).collect();
        for (layer, matrix) in weights.iter().enumerate() {
            let is_last = layer == weights.len() - 1;
            let mut next = Vec::with_capacity(matrix.len());
            for row in matrix {
                let mut acc: i128 = 0;
                for (i, &w) in row.iter().enumerate() {
                    acc += act[i] * i128::from(w);
                    macs += 1;
                    max_abs = max_abs.max(acc.abs());
                }
                next.push(if is_last { acc } else { acc.max(0) });
            }
            act = next;
        }
        assert_eq!(act.len(), 1, "the network must end in a single output");
        // One i64 output is two `u32` words, little-endian, matching the guest's
        // `act[0].to_le_bytes()`.
        let out = act[0] as i64 as u64;
        public_io.push(out as u32);
        public_io.push((out >> 32) as u32);
    }

    // Amendment A1: the worst case over all admissible INT8 operands overflows i64 at layer
    // 4. The published instance must not come near it, and this refuses to emit if it does.
    assert!(
        max_abs.saturating_mul(HEADROOM) < i128::from(i64::MAX),
        "{task}: largest intermediate {max_abs} has less than the factor-{HEADROOM} margin \
         under i64::MAX that bench/TASKS.md A1 demands"
    );

    let mut flat: Vec<i8> = Vec::new();
    for layer in &weights {
        for row in layer {
            flat.extend_from_slice(row);
        }
    }
    for input in &inputs {
        flat.extend_from_slice(input);
    }
    let digest = instance_digest(&flat);

    let header = (batch as u32).to_le_bytes().to_vec();
    let mut w_bytes: Vec<u8> = Vec::new();
    for layer in &weights {
        for row in layer {
            w_bytes.extend(row.iter().map(|&v| v as u8));
        }
    }
    let in_bytes: Vec<u8> = inputs.iter().flatten().map(|&v| v as u8).collect();

    Emitted {
        hints: finalise(&[header, w_bytes, in_bytes]),
        public_io,
        macs,
        max_abs,
        digest,
        note: format!(
            "200-256-128-64-1 MLP, batch {batch}, ReLU after layers 1-3, linear output, \
             no requantisation"
        ),
    }
}

fn main() {
    let out_dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("usage: ceno-bench-tasks <out-dir>");
    fs::create_dir_all(&out_dir).expect("could not create the output directory");

    let mut manifest = String::from("{\n");
    let mut first = true;

    let mut emit = |task: &str, e: Emitted, extra: String| {
        let published = published_macs(task);
        assert_eq!(
            e.macs, published,
            "{task}: counted {} MACs, bench/TASKS.md publishes {published}. The published \
             count is frozen; this generator does not get to disagree with it.",
            e.macs
        );
        let path = out_dir.join(format!("{task}.hints.bin"));
        fs::write(&path, &e.hints).expect("could not write the hint file");
        let pio: Vec<String> = e.public_io.iter().map(|w| w.to_string()).collect();
        let pio_text = pio.join(",");
        fs::write(out_dir.join(format!("{task}.public-io.txt")), &pio_text)
            .expect("could not write the public-io file");
        if !first {
            manifest.push_str(",\n");
        }
        first = false;
        manifest.push_str(&format!(
            "  \"{task}\": {{\n    \"hints\": \"{task}.hints.bin\",\n    \
             \"hints_bytes\": {},\n    \"published_macs\": {published},\n    \
             \"counted_macs\": {},\n    \"seed\": \"0x{:08x}\",\n    \
             \"max_abs_intermediate\": {},\n    \"instance_digest_fnv1a64\": \"0x{:016x}\",\n    \
             \"public_io_words\": {},\n    \"public_io_argv_bytes\": {},\n    \
             \"expression\": \"{}\"{}\n  }}",
            e.hints.len(),
            e.macs,
            seed_of(task),
            e.max_abs,
            e.digest,
            e.public_io.len(),
            pio_text.len(),
            e.note,
            extra
        ));
        println!(
            "{task}: {} MACs asserted, hints {} B, max|acc| {}, instance digest 0x{:016x}",
            e.macs,
            e.hints.len(),
            e.max_abs,
            e.digest
        );
    };

    for (task, m, k, n) in T1_SHAPES {
        let e = build_t1(task, m, k, n);
        emit(task, e, format!(",\n    \"shape\": [{m}, {k}, {n}],\n    \"output_bytes\": {}", m * n * 4));
    }
    for (task, batch) in [("t2", 1usize), ("t3", 8usize)] {
        let e = build_mlp(task, batch);
        emit(task, e, format!(",\n    \"batch\": {batch},\n    \"output_bytes\": {}", batch * 8));
    }

    manifest.push_str("\n}\n");
    fs::write(out_dir.join("manifest.json"), manifest).expect("could not write the manifest");
    println!("wrote {}", out_dir.display());
}
