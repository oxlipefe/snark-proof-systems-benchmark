//! E-001 · build sanity check: is the field multiply running at the rate its algorithm
//! and this machine's carryless multiplier allow?
//!
//! Exits non-zero when it is not, so the measurement scripts stop before producing
//! numbers that look plausible and are not. See `src/sanity/mod.rs` for the history that
//! made this necessary.

#[path = "../sanity/mod.rs"]
mod sanity;

use std::process::ExitCode;

use clap::Parser;

use crate::sanity::{MIN_RATIO_VS_HAND, PMULL_PER_FIELD_MUL};

#[derive(Debug, Parser)]
#[command(name = "e001-fieldmul-sanity", about = "Reject a build whose field multiply is crippled")]
struct Args {
	/// Iterations per accumulator lane.
	#[arg(long, default_value_t = 2_000_000)]
	iters: u64,
}

fn main() -> ExitCode {
	let args = Args::parse();

	let binius = sanity::binius_mul_rate(args.iters);

	#[cfg(not(target_arch = "aarch64"))]
	{
		println!("binius B128 mul       {binius:>10.1} Mmul/s");
		println!();
		println!("NO REFERENCE on this architecture: the hand-written kernel and the raw");
		println!("carryless-multiply ceiling are implemented for aarch64 only. The rate above");
		println!("is unchecked — port `sanity::handmul` before trusting a measurement here.");
		return ExitCode::from(2);
	}

	#[cfg(target_arch = "aarch64")]
	{
		let hand = sanity::hand_mul_rate(args.iters);
		let pmull = sanity::pmull_rate(args.iters);
		let ratio = binius / hand;
		let pmull_per_mul = pmull / binius;

		// The raw loop round-trips each product through general registers, so this figure
		// understates what the vector units can issue. It is a lower bound on the machine,
		// which is why `pmull_per_mul` below can read under the algorithm's true 6.
		println!("raw PMULL (lower bd)  {pmull:>10.1} Mops/s");
		println!("hand-written 6-PMULL  {hand:>10.1} Mmul/s");
		println!("binius B128 mul       {binius:>10.1} Mmul/s");
		println!();
		println!("binius / hand-written {ratio:>10.3}   (floor {MIN_RATIO_VS_HAND:.2})");
		println!(
			"PMULL per field mul   {pmull_per_mul:>10.1}   (algorithm costs {PMULL_PER_FIELD_MUL:.0}; \
			 below that means the PMULL row is understated, not that the multiply is free)"
		);
		println!();

		if ratio < MIN_RATIO_VS_HAND {
			println!("FAIL: the field multiply is far below what its own algorithm costs.");
			println!("      Check `[profile.release]` in Cargo.toml.in — it must set");
			println!("      `lto = \"thin\"`, as binius64's own Cargo.toml does. Without it");
			println!("      the multiply emits eight non-inlined calls around its six PMULL.");
			println!("      Edit Cargo.toml.in, not the generated Cargo.toml: setup.sh");
			println!("      overwrites the latter on every run.");
			return ExitCode::FAILURE;
		}

		println!("OK: the measured build is fit to produce numbers.");
		ExitCode::SUCCESS
	}
}
