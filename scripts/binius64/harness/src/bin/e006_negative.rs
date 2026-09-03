//! E-006 · the correctness control, and it is blocking.
//!
//! **A corrupted trace must make `verify()` fail.** Without this control the benchmark is
//! not measuring proofs, it is measuring a computation that happens to emit bytes, and no
//! number from this system may be published. `bench/README.md` states the rule; this binary
//! is how binius64 answers it, for every task.
//!
//! Three corruptions are exercised per task, each on a freshly cloned honest witness:
//!
//! | Mode | What is corrupted | Why it is the right thing to corrupt |
//! |---|---|---|
//! | `private_word` | one **committed private word** — an INT8 weight or an internal wire | This is "a corrupted trace": the prover's own secret data no longer satisfies the constraint system, while the public claim is untouched |
//! | `inout_word` | one **public output word** | The prover claims a different result for the same model and input |
//! | `proof_byte` | one byte of the **serialized proof** | The transcript itself is tampered with after an honest proof |
//!
//! An attempt **passes** when no accepted proof exists at the end of it. Two distinct ways
//! of passing are recorded separately rather than merged, because they say different things
//! about the system: `PROVER_ERROR` (the prover refused to produce a proof at all) and
//! `VERIFY_REJECTED` (a proof was produced and the verifier rejected it). Only
//! `VERIFY_ACCEPTED` is a failure, and it fails the whole system.

#[path = "../e006/mod.rs"]
mod e006;

use std::{
	fs,
	io::Write,
	panic::{AssertUnwindSafe, catch_unwind},
	path::PathBuf,
	process::ExitCode,
};

use anyhow::Result;
use binius_core::constraint_system::ValueVec;
use binius_examples::{check_proof, create_proof, setup};
use binius_hash::StdHashSuite;
use clap::Parser;

use crate::e006::Task;

#[derive(Debug, Parser)]
#[command(name = "e006-negative", about = "Blocking correctness control: a corrupt trace must not verify")]
struct Args {
	#[arg(long, value_enum)]
	task: Task,

	#[arg(long, default_value_t = 1)]
	log_inv_rate: usize,

	#[arg(long)]
	out_dir: PathBuf,

	/// One corruption per family instead of three. Each attempt costs a full proof, so at the
	/// largest rungs the exhaustive form costs tens of minutes; the reduced form still
	/// exercises all three families. Cells run this way say so in the results.
	#[arg(long)]
	quick: bool,
}

/// What became of one corruption attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
	/// The prover refused to produce a proof. Passes: no accepted proof exists.
	ProverError,
	/// A proof was produced and the verifier rejected it. Passes, and is the stronger result.
	VerifyRejected,
	/// A proof was produced and the verifier accepted it. **Fails.**
	VerifyAccepted,
}

impl Outcome {
	const fn passed(self) -> bool {
		!matches!(self, Outcome::VerifyAccepted)
	}
	const fn as_str(self) -> &'static str {
		match self {
			Outcome::ProverError => "PROVER_ERROR",
			Outcome::VerifyRejected => "VERIFY_REJECTED",
			Outcome::VerifyAccepted => "VERIFY_ACCEPTED",
		}
	}
}

fn main() -> ExitCode {
	match run() {
		Ok(true) => ExitCode::SUCCESS,
		Ok(false) => {
			eprintln!("FAIL: a corrupted trace produced an accepted proof. No number from this \
			           system may be published.");
			ExitCode::FAILURE
		}
		Err(err) => {
			eprintln!("ERROR: {err:#}");
			ExitCode::from(2)
		}
	}
}

fn run() -> Result<bool> {
	let args = Args::parse();
	fs::create_dir_all(&args.out_dir)?;

	let built = e006::build(args.task)?;
	let (verifier, prover) =
		setup::<StdHashSuite>(built.constraint_system.clone(), args.log_inv_rate, None)?;

	// Control on the control: the honest witness must verify. A negative test that passes
	// because nothing ever verifies proves nothing at all.
	let honest_proof = create_proof(&prover, &built.witness)?;
	let honest_proof_len = honest_proof.len();
	check_proof(&verifier, &built.witness, honest_proof)
		.map_err(|err| anyhow::anyhow!("the HONEST proof failed to verify: {err:#}"))?;
	eprintln!(
		"{}: honest proof verifies ({honest_proof_len} bytes) — the control is live",
		args.task.name()
	);

	let n_private = built.witness.non_public().len();
	let n_public = built.witness.public().len();
	let n_inout = built.witness.inout().len();
	let n_const = n_public - n_inout;

	let mut results: Vec<(String, usize, Outcome, String)> = Vec::new();

	// --- Mode 1: a committed private word. This is the trace corruption proper. ---
	// Three positions, so the answer cannot depend on one lucky wire: the first private
	// word, one in the middle, and the last.
	let private_positions: Vec<(&str, usize)> = if args.quick {
		vec![("middle", n_private / 2)]
	} else {
		vec![("first", 0), ("middle", n_private / 2), ("last", n_private - 1)]
	};
	for (tag, index) in private_positions {
		let mut witness = built.witness.clone();
		let offset = (n_public + index) as u32;
		let before = witness.word(offset).0;
		witness.word_mut(offset).0 ^= 1; // flip the low bit
		let after = witness.word(offset).0;
		let outcome = attempt(&verifier, &prover, &witness, None);
		results.push((
			format!("private_word/{tag}"),
			index,
			outcome,
			format!("private[{index}] {before:#018x} -> {after:#018x} (low bit flipped)"),
		));
	}

	// --- Mode 2: a public output word. The prover claims a different result. ---
	let inout_positions: Vec<(&str, usize)> = if args.quick {
		vec![("first", 0)]
	} else {
		vec![("first", 0), ("last", n_inout - 1)]
	};
	for (tag, index) in inout_positions {
		let mut witness = built.witness.clone();
		let offset = (n_const + index) as u32;
		let before = witness.word(offset).0;
		witness.word_mut(offset).0 ^= 1;
		let after = witness.word(offset).0;
		let outcome = attempt(&verifier, &prover, &witness, None);
		results.push((
			format!("inout_word/{tag}"),
			index,
			outcome,
			format!("inout[{index}] {before:#018x} -> {after:#018x} (low bit flipped)"),
		));
	}

	// --- Mode 3: one byte of an otherwise honest serialized proof. ---
	let proof_positions: Vec<(&str, usize)> = if args.quick {
		vec![("middle", honest_proof_len / 2)]
	} else {
		vec![("head", 0), ("middle", honest_proof_len / 2), ("tail", honest_proof_len - 1)]
	};
	for (tag, position) in proof_positions {
		let outcome = attempt(&verifier, &prover, &built.witness, Some(position));
		results.push((
			format!("proof_byte/{tag}"),
			position,
			outcome,
			format!("proof byte {position} of {honest_proof_len} XOR 0x01"),
		));
	}

	let all_passed = results.iter().all(|(_, _, outcome, _)| outcome.passed());

	let path = args.out_dir.join("negative-control.csv");
	let mut file = fs::File::create(&path)?;
	writeln!(file, "task,log_inv_rate,mode,index,outcome,passed,detail")?;
	for (mode, index, outcome, detail) in &results {
		writeln!(
			file,
			"{},{},{mode},{index},{},{},\"{detail}\"",
			args.task.name(),
			args.log_inv_rate,
			outcome.as_str(),
			outcome.passed()
		)?;
	}

	println!("\n{} — corrupted-trace control, log_inv_rate={}", args.task.name(), args.log_inv_rate);
	println!("{:<22} {:<18} {:<7} {}", "mode", "outcome", "passed", "detail");
	for (mode, _, outcome, detail) in &results {
		println!("{mode:<22} {:<18} {:<7} {detail}", outcome.as_str(), outcome.passed());
	}
	println!(
		"\n{}: {}/{} corruptions produced no accepted proof — {}",
		args.task.name(),
		results.iter().filter(|r| r.2.passed()).count(),
		results.len(),
		if all_passed { "PASS" } else { "FAIL" }
	);
	eprintln!("wrote {}", path.display());

	Ok(all_passed)
}

/// Proves `witness`, optionally flips one byte of the resulting proof, and verifies.
///
/// The prover is run under `catch_unwind` because a constraint system that is not satisfied
/// is outside its contract: it is entitled to panic rather than return an error, and a panic
/// still means no accepted proof was produced.
fn attempt(
	verifier: &binius_verifier::Verifier<StdHashSuite>,
	prover: &binius_prover::Prover<binius_prover::OptimalPackedB128, StdHashSuite>,
	witness: &ValueVec,
	corrupt_proof_byte: Option<usize>,
) -> Outcome {
	let proved = catch_unwind(AssertUnwindSafe(|| create_proof(prover, witness)));
	let mut proof = match proved {
		Ok(Ok(proof)) => proof,
		Ok(Err(_)) | Err(_) => return Outcome::ProverError,
	};
	if let Some(position) = corrupt_proof_byte {
		proof[position] ^= 0x01;
	}
	let verified = catch_unwind(AssertUnwindSafe(|| check_proof(verifier, witness, proof)));
	match verified {
		Ok(Ok(())) => Outcome::VerifyAccepted,
		Ok(Err(_)) | Err(_) => Outcome::VerifyRejected,
	}
}
