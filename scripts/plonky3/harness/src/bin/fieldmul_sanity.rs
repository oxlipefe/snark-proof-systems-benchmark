//! G-13b · build sanity check: are the two fields' multiplies running at the rate their own
//! algorithms and this machine's primitives allow?
//!
//! Exits non-zero when they are not, so the measurement scripts stop before producing numbers
//! that look plausible and are not. See `src/sanity/mod.rs` for the two failure modes this
//! catches and `systems/plonky3/BUILD.md` §2 for the recorded results.

use std::process::ExitCode;

use clap::Parser;
use plonky3_bench_harness::sanity::{
    MAX_U32_MULS_PER_KB_MUL, MIN_B128_RATIO_VS_HAND, p3_b128_mul_rate, p3_koalabear_mul_rate,
    u32_mul_rate,
};

#[derive(Debug, Parser)]
#[command(name = "p3-fieldmul-sanity", about = "Reject a build whose field multiply is crippled")]
struct Args {
    /// Iterations per accumulator lane.
    #[arg(long, default_value_t = 2_000_000)]
    iters: u64,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let b128 = p3_b128_mul_rate(args.iters);
    let kb = p3_koalabear_mul_rate(args.iters);
    let u32mul = u32_mul_rate(args.iters);
    let u32_per_kb = u32mul / kb;

    #[cfg(not(target_arch = "aarch64"))]
    {
        println!("p3 BinaryField128 mul {b128:>10.1} Mmul/s");
        println!("p3 KoalaBear mul      {kb:>10.1} Mmul/s");
        println!("raw u32 mul           {u32mul:>10.1} Mops/s");
        println!();
        println!("NO BINARY REFERENCE on this architecture: the hand-written 6-PMULL kernel and");
        println!("the raw carryless-multiply ceiling are implemented for aarch64 only. Port");
        println!("`sanity::handmul` before trusting a binary-field measurement here.");
        return ExitCode::from(2);
    }

    #[cfg(target_arch = "aarch64")]
    {
        let hand = plonky3_bench_harness::sanity::hand_mul_rate(args.iters);
        let pmull = plonky3_bench_harness::sanity::pmull_rate(args.iters);
        let ratio = b128 / hand;
        let pmull_per_b128 = pmull / b128;

        // The raw loops round-trip each product through general registers, so both raw figures
        // understate what the units can issue. They are lower bounds on the machine.
        println!("raw PMULL (lower bd)  {pmull:>10.1} Mops/s");
        println!("raw u32 mul (lower bd){u32mul:>10.1} Mops/s");
        println!("hand-written 6-PMULL  {hand:>10.1} Mmul/s   (GHASH basis; binius64's algorithm)");
        println!("p3 BinaryField128 mul {b128:>10.1} Mmul/s   (Wiedemann tower basis + 4 clmul)");
        println!("p3 KoalaBear mul      {kb:>10.1} Mmul/s");
        println!();
        println!("p3-B128 / hand-6PMULL {ratio:>10.3}   (floor {MIN_B128_RATIO_VS_HAND:.2})");
        println!("PMULL per p3-B128 mul {pmull_per_b128:>10.1}   (the algorithm issues 4 clmul plus \
                  three byte-table basis maps; a figure far above that is the bit-serial fallback)");
        println!("u32 muls per KB mul   {u32_per_kb:>10.1}   (ceiling {MAX_U32_MULS_PER_KB_MUL:.0}; \
                  Montgomery costs 2)");
        println!();

        let mut failed = false;
        if ratio < MIN_B128_RATIO_VS_HAND {
            failed = true;
            println!("FAIL: BinaryField128's multiply is far below what any implementation of it");
            println!("      costs. Two causes, both silent:");
            println!("      1. `[profile.release]` in Cargo.toml.in must set `lto = \"thin\"`.");
            println!("      2. RUSTFLAGS must carry `-C target-cpu=native`: without the `aes`");
            println!("         target feature, p3-binary-field compiles the BIT-SERIAL clmul");
            println!("         fallback (binary-field/src/clmul/mod.rs:14-31).");
            println!("      Edit Cargo.toml.in, not the generated Cargo.toml: setup.sh");
            println!("      overwrites the latter on every run.");
        }
        if u32_per_kb > MAX_U32_MULS_PER_KB_MUL {
            failed = true;
            println!("FAIL: KoalaBear's multiply costs {u32_per_kb:.1} raw 32-bit multiplies.");
            println!("      A Montgomery multiply in a 31-bit field costs two. Check LTO.");
        }
        if failed {
            return ExitCode::FAILURE;
        }

        println!("OK: the measured build is fit to produce numbers.");
        ExitCode::SUCCESS
    }
}
