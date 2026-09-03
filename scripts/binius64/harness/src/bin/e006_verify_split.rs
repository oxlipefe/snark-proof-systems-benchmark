//! E-006 · diagnostic: where does binius64's verify time actually go?
//!
//! `bench/systems/binius64/RESULTS.md` §4 reports that verify time grows roughly in
//! proportion to circuit size — 1 389x from T1-0 to T1-c — which is not the shape a FRI
//! verifier is expected to have. That section names one untested candidate: our use of
//! `binius_examples::check_proof`.
//!
//! This binary settles it by **decomposing `check_proof` into its own terms, in the same
//! loop and the same run**. It reproduces, call for call, what
//! `binius_verifier::Verifier::verify` does internally (vendor
//! `crates/verifier/src/verify.rs`, `Verifier::verify`), timing each step:
//!
//!   A `channel`   — build the BaseFold verifier channel over the proof transcript and
//!                   observe the public statement (`observe_words`). This is the only
//!                   step that touches the caller-supplied value vector, and it touches
//!                   only its `inout()` slice.
//!   B `iop`       — `IOPVerifier::verify`: receive the trace oracle commitment, run the
//!                   constraint reduction (the zerocheck / shift / IntMul sumchecks) and
//!                   ring-switching. It does **not** contain the polynomial-commitment
//!                   opening: `verify_oracle_relation` only queues the relation, which
//!                   term D then discharges. Proof deserialization happens across B and D:
//!                   the transcript is read lazily by the protocol, so there is no
//!                   separable "deserialize" term to time —
//!                   `VerifierTranscript::new` is `Bytes::from(vec)` and parses nothing.
//!   C `wiring`    — `WiringEvalClaim::check_native`: evaluate the wiring ("monster")
//!                   multilinear from the constraint system and compare it against the
//!                   value the prover claimed.
//!   D `finish`    — `channel.finish()`, which is where the batched BaseFold/FRI opening
//!                   is actually verified (masking, the batched sumcheck, the combined FRI
//!                   opening and its Merkle paths — vendor
//!                   `crates/iop/src/basefold/channel.rs:103`), then
//!                   `VerifierTranscript::finalize()`, which only asserts the tape was
//!                   fully consumed. **D is the FRI verifier proper**, not a teardown step.
//!
//! A + B + C + D is `check_proof` minus only the `StdChallenger::default()` construction,
//! so the four terms are comparable against the `verify ms` column of the grid, and the
//! binary reports their total so a reader can check that they add up.
//!
//! Nothing here changes binius64's configuration, its constants, or the proof being
//! checked. The proof verified is produced by `create_proof` exactly as `e006-bench`
//! produces it, from the same circuit and the same seeded witness.

#[path = "../e006/mod.rs"]
mod e006;
#[path = "../stats.rs"]
mod stats;

use std::{fs, io::Write, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use binius_examples::{create_proof, setup};
use binius_hash::StdHashSuite;
use binius_ip::channel::WordIPVerifierChannel;
use binius_verifier::{
	SECURITY_BITS, Verifier,
	config::StdChallenger,
	fri::calculate_n_test_queries,
	transcript::VerifierTranscript,
};
use clap::Parser;

use crate::{e006::Task, stats::summarize};

const NANOS_PER_SEC: f64 = 1e9;

#[derive(Debug, Parser)]
#[command(
	name = "e006-verify-split",
	about = "Diagnostic: split binius64's check_proof into its own terms, same loop, same run"
)]
struct Args {
	#[arg(long, value_enum)]
	task: Task,

	#[arg(long, default_value_t = 1)]
	log_inv_rate: usize,

	/// Discarded verifications before the measured ones. The proof is produced once.
	#[arg(long, default_value_t = 1)]
	warmup: usize,

	/// Measured verifications of that one proof.
	#[arg(long, default_value_t = 5)]
	reps: usize,

	/// Free the prover before verifying. Off by default, so the process is in the same
	/// memory state `e006-bench` verifies in.
	#[arg(long)]
	drop_prover: bool,

	#[arg(long)]
	out_dir: Option<PathBuf>,

	#[arg(long)]
	label: Option<String>,
}

/// One verification, decomposed. Nanoseconds.
#[derive(Debug, Clone, Copy)]
struct Split {
	channel: u128,
	iop: u128,
	wiring: u128,
	finish: u128,
	total: u128,
}

fn verify_split(
	verifier: &Verifier<StdHashSuite>,
	inout_words: &[binius_core::word::Word],
	proof_bytes: Vec<u8>,
) -> Result<Split> {
	let challenger = StdChallenger::default();
	let mut transcript = VerifierTranscript::new(challenger, proof_bytes);

	let started_total = Instant::now();

	let (channel_nanos, iop_nanos, wiring_nanos, channel_finish_nanos) = {
		let t_channel = Instant::now();
		let mut channel = verifier
			.iop_compiler()
			.create_channel_from_transcript::<StdHashSuite, StdChallenger, _>(&mut transcript);
		let inout = channel.observe_words(inout_words);
		let channel_nanos = t_channel.elapsed().as_nanos();

		let t_iop = Instant::now();
		let claim = verifier
			.iop_verifier()
			.verify(&inout, &mut channel)
			.map_err(|err| anyhow::anyhow!("IOPVerifier::verify: {err}"))?;
		let iop_nanos = t_iop.elapsed().as_nanos();

		let t_wiring = Instant::now();
		claim
			.check_native()
			.map_err(|err| anyhow::anyhow!("WiringEvalClaim::check_native: {err}"))?;
		let wiring_nanos = t_wiring.elapsed().as_nanos();

		let t_finish = Instant::now();
		let inner = channel
			.finish()
			.map_err(|err| anyhow::anyhow!("channel.finish: {err}"))?;
		drop(inner);
		let channel_finish_nanos = t_finish.elapsed().as_nanos();

		(channel_nanos, iop_nanos, wiring_nanos, channel_finish_nanos)
	};

	let t_transcript = Instant::now();
	transcript
		.finalize()
		.map_err(|err| anyhow::anyhow!("VerifierTranscript::finalize: {err}"))?;
	let finish_nanos = channel_finish_nanos + t_transcript.elapsed().as_nanos();

	let total = started_total.elapsed().as_nanos();

	Ok(Split {
		channel: channel_nanos,
		iop: iop_nanos,
		wiring: wiring_nanos,
		finish: finish_nanos,
		total,
	})
}

fn main() -> Result<()> {
	let args = Args::parse();

	let n_threads = std::env::var("RAYON_NUM_THREADS")
		.ok()
		.and_then(|v| v.parse::<usize>().ok())
		.unwrap_or(0);

	let built = e006::build(args.task)
		.with_context(|| format!("building {}", args.task.name()))?;
	eprintln!(
		"task={} macs={} imul={} and={} zero={} bmul={} private={} inout={}",
		args.task.name(),
		built.n_macs,
		built.n_imul_constraints,
		built.n_and_constraints,
		built.n_zero_constraints,
		built.n_bmul_constraints,
		built.n_private_values,
		built.n_inout_values
	);

	let (verifier, prover) = setup::<StdHashSuite>(
		built.constraint_system.clone(),
		args.log_inv_rate,
		None,
	)
	.with_context(|| format!("setting up for {}", args.task.name()))?;

	// One proof, verified many times. The verify terms do not depend on which honest proof
	// they are handed, and proving T1-c once already costs minutes.
	let proof = create_proof(&prover, &built.witness).context("producing the proof to verify")?;
	let proof_bytes = proof.len();
	eprintln!("proof={proof_bytes}B");

	if args.drop_prover {
		drop(prover);
		eprintln!("prover dropped before verifying");
	}

	let inout_words = built.witness.inout().to_vec();

	for w in 0..args.warmup {
		verify_split(&verifier, &inout_words, proof.clone())
			.with_context(|| format!("warmup verify {w}"))?;
	}

	let mut rows = Vec::with_capacity(args.reps);
	for rep in 0..args.reps {
		let split = verify_split(&verifier, &inout_words, proof.clone())
			.with_context(|| format!("measured verify {rep}"))?;
		eprintln!(
			"rep={rep} total={:.4}s channel={:.6}s iop={:.4}s wiring={:.4}s finish={:.6}s \
			 wiring_share={:.4}",
			split.total as f64 / NANOS_PER_SEC,
			split.channel as f64 / NANOS_PER_SEC,
			split.iop as f64 / NANOS_PER_SEC,
			split.wiring as f64 / NANOS_PER_SEC,
			split.finish as f64 / NANOS_PER_SEC,
			split.wiring as f64 / split.total as f64
		);
		rows.push(split);
	}

	let label = args
		.label
		.clone()
		.unwrap_or_else(|| format!("{}-r{}-t{}", args.task.name(), args.log_inv_rate, n_threads));

	if let Some(dir) = &args.out_dir {
		fs::create_dir_all(dir)?;
		let csv = dir.join(format!("{label}.verify-split.csv"));
		let mut f = fs::File::create(&csv)?;
		writeln!(
			f,
			"label,task,log_inv_rate,threads,rep,channel_nanos,iop_nanos,wiring_nanos,\
			 finish_nanos,total_nanos,proof_bytes,n_imul,n_and,n_zero,n_bmul,n_private,n_inout"
		)?;
		for (rep, r) in rows.iter().enumerate() {
			writeln!(
				f,
				"{label},{},{},{n_threads},{rep},{},{},{},{},{},{proof_bytes},{},{},{},{},{},{}",
				args.task.name(),
				args.log_inv_rate,
				r.channel,
				r.iop,
				r.wiring,
				r.finish,
				r.total,
				built.n_imul_constraints,
				built.n_and_constraints,
				built.n_zero_constraints,
				built.n_bmul_constraints,
				built.n_private_values,
				built.n_inout_values
			)?;
		}
		eprintln!("wrote {}", csv.display());

		let totals = rows.iter().map(|r| r.total as f64).collect::<Vec<_>>();
		let iops = rows.iter().map(|r| r.iop as f64).collect::<Vec<_>>();
		let wirings = rows.iter().map(|r| r.wiring as f64).collect::<Vec<_>>();
		let channels = rows.iter().map(|r| r.channel as f64).collect::<Vec<_>>();
		let finishes = rows.iter().map(|r| r.finish as f64).collect::<Vec<_>>();
		let json = serde_json::json!({
			"label": label,
			"task": args.task.name(),
			"log_inv_rate": args.log_inv_rate,
			"threads": n_threads,
			"security_bits": SECURITY_BITS,
			"n_test_queries": calculate_n_test_queries(SECURITY_BITS, args.log_inv_rate),
			"reps": args.reps,
			"warmup": args.warmup,
			"drop_prover": args.drop_prover,
			"proof_bytes": proof_bytes,
			"n_macs": built.n_macs,
			"n_imul": built.n_imul_constraints,
			"n_and": built.n_and_constraints,
			"n_zero": built.n_zero_constraints,
			"n_bmul": built.n_bmul_constraints,
			"n_private": built.n_private_values,
			"n_inout": built.n_inout_values,
			"total_nanos_median": summarize(&totals).map(|s| s.median),
			"total_nanos_min": summarize(&totals).map(|s| s.min),
			"total_nanos_max": summarize(&totals).map(|s| s.max),
			"channel_nanos_median": summarize(&channels).map(|s| s.median),
			"iop_nanos_median": summarize(&iops).map(|s| s.median),
			"iop_nanos_min": summarize(&iops).map(|s| s.min),
			"iop_nanos_max": summarize(&iops).map(|s| s.max),
			"wiring_nanos_median": summarize(&wirings).map(|s| s.median),
			"wiring_nanos_min": summarize(&wirings).map(|s| s.min),
			"wiring_nanos_max": summarize(&wirings).map(|s| s.max),
			"finish_nanos_median": summarize(&finishes).map(|s| s.median),
		});
		let path = dir.join(format!("{label}.verify-split.json"));
		fs::write(&path, serde_json::to_string_pretty(&json)?)?;
		eprintln!("wrote {}", path.display());
	}

	Ok(())
}
