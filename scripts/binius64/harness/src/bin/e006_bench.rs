//! E-006 · one benchmark cell of the public `bench/` comparison, for binius64.
//!
//! One process per cell (task x rate x threads), so that `/usr/bin/time -l` outside it
//! attributes peak RSS and peak footprint to that cell alone — the convention E-005 fixed.
//! Inside the process this measures, per repetition and from the same run:
//!
//! - **prove time** — wall time of `create_proof`
//! - **proof size** — bytes of the serialized transcript that same call returned
//! - **verify time** — wall time of `check_proof` on that same proof
//!
//! and once per cell, reported separately and never amortised into prove time:
//!
//! - **circuit build time** and **setup time**
//!
//! Numerator and denominator of every derived rate come from one repetition of one run;
//! this binary emits the raw per-repetition rows and leaves every ratio to the consumer.
//!
//! All timing uses [`Instant`], which on macOS is a monotonic clock that does **not**
//! advance while the machine sleeps. The wrapper script compares the total against
//! `/usr/bin/time -l`'s wall-clock `real`; a gap between them means the cell spanned a
//! sleep and the cell is invalid. See `bench/systems/binius64/BUILD.md`.

#[path = "../e006/mod.rs"]
mod e006;
#[path = "../stats.rs"]
mod stats;

use std::{fs, io::Write, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use binius_examples::{check_proof, create_proof, setup};
use binius_hash::StdHashSuite;
use binius_verifier::{SECURITY_BITS, fri::calculate_n_test_queries};
use clap::Parser;

use crate::{e006::Task, stats::summarize};

const NANOS_PER_SEC: f64 = 1e9;

#[derive(Debug, Parser)]
#[command(name = "e006-bench", about = "One cell of the public zk-prover-bench, on binius64")]
struct Args {
	#[arg(long, value_enum)]
	task: Task,

	/// Inverse-rate exponent. One cell is one rate.
	#[arg(long)]
	log_inv_rate: usize,

	/// Discarded proofs before the measured repetitions.
	#[arg(long, default_value_t = 1)]
	warmup: usize,

	/// Measured repetitions. Protocol asks for N >= 5; a lower N must be declared per cell.
	#[arg(long, default_value_t = 5)]
	reps: usize,

	/// Build the circuit and report its shape without proving.
	#[arg(long)]
	stat_only: bool,

	#[arg(long)]
	out_dir: PathBuf,

	#[arg(long, default_value = "unlabelled")]
	label: String,
}

fn main() -> Result<()> {
	let args = Args::parse();
	fs::create_dir_all(&args.out_dir)
		.with_context(|| format!("creating {}", args.out_dir.display()))?;

	let pinned_single_thread = binius_utils::rayon::config::adjust_thread_pool().is_ok();
	let n_threads = binius_utils::rayon::current_num_threads();

	let build_started = Instant::now();
	let built = e006::build(args.task)
		.with_context(|| format!("building the {} circuit", args.task.name()))?;
	let build_nanos = build_started.elapsed().as_nanos();

	eprintln!(
		"task={} macs={} relus={} IMUL={} AND={} ZERO={} BMUL={} private_values={} inout={} \
		 max_abs_acc={} build={:.3}s",
		args.task.name(),
		built.n_macs,
		built.n_relus,
		built.n_imul_constraints,
		built.n_and_constraints,
		built.n_zero_constraints,
		built.n_bmul_constraints,
		built.n_private_values,
		built.n_inout_values,
		built.max_abs_intermediate,
		build_nanos as f64 / NANOS_PER_SEC
	);

	if args.stat_only {
		eprintln!("--stat-only: nothing was proved");
		return Ok(());
	}

	// Setup is a one-off. It is timed here and reported in its own column; it is never
	// folded into, or amortised across, the prove times below.
	let setup_started = Instant::now();
	let (verifier, prover) = setup::<StdHashSuite>(
		built.constraint_system.clone(),
		args.log_inv_rate,
		None,
	)
	.with_context(|| format!("setting up the prover for {}", args.task.name()))?;
	let setup_nanos = setup_started.elapsed().as_nanos();
	eprintln!("setup={:.3}s", setup_nanos as f64 / NANOS_PER_SEC);

	for w in 0..args.warmup {
		let proof = create_proof(&prover, &built.witness)
			.with_context(|| format!("warmup proof {w}"))?;
		check_proof(&verifier, &built.witness, proof)
			.with_context(|| format!("warmup verify {w}"))?;
	}

	let mut rows = Vec::with_capacity(args.reps);
	for rep in 0..args.reps {
		let prove_started = Instant::now();
		let proof = create_proof(&prover, &built.witness)
			.with_context(|| format!("measured proof {rep}"))?;
		let prove_nanos = prove_started.elapsed().as_nanos();
		let proof_bytes = proof.len();

		let verify_started = Instant::now();
		check_proof(&verifier, &built.witness, proof)
			.with_context(|| format!("measured verify {rep}"))?;
		let verify_nanos = verify_started.elapsed().as_nanos();

		eprintln!(
			"rep={rep} prove={:.3}s verify={:.4}s proof={proof_bytes}B",
			prove_nanos as f64 / NANOS_PER_SEC,
			verify_nanos as f64 / NANOS_PER_SEC
		);
		rows.push((prove_nanos, verify_nanos, proof_bytes));
	}

	write_rows(&args, &built, n_threads, &rows)?;
	write_json(
		&args,
		&built,
		n_threads,
		pinned_single_thread,
		build_nanos,
		setup_nanos,
		&rows,
	)?;
	Ok(())
}

/// Raw per-repetition rows. Uncurated: every repetition that ran is here.
fn write_rows(
	args: &Args,
	built: &e006::Built,
	n_threads: usize,
	rows: &[(u128, u128, usize)],
) -> Result<()> {
	let path = args.out_dir.join("reps.csv");
	let mut file = fs::File::create(&path)?;
	writeln!(
		file,
		"label,task,n_macs,n_relus,log_inv_rate,threads,security_bits,n_test_queries,rep,\
		 prove_nanos,verify_nanos,proof_bytes"
	)?;
	for (rep, (prove, verify, size)) in rows.iter().enumerate() {
		writeln!(
			file,
			"{},{},{},{},{},{n_threads},{SECURITY_BITS},{},{rep},{prove},{verify},{size}",
			args.label,
			args.task.name(),
			built.n_macs,
			built.n_relus,
			args.log_inv_rate,
			calculate_n_test_queries(SECURITY_BITS, args.log_inv_rate)
		)?;
	}
	eprintln!("wrote {}", path.display());
	Ok(())
}

fn write_json(
	args: &Args,
	built: &e006::Built,
	n_threads: usize,
	pinned_single_thread: bool,
	build_nanos: u128,
	setup_nanos: u128,
	rows: &[(u128, u128, usize)],
) -> Result<()> {
	let proves: Vec<f64> = rows.iter().map(|r| r.0 as f64).collect();
	let verifies: Vec<f64> = rows.iter().map(|r| r.1 as f64).collect();
	let sizes: Vec<f64> = rows.iter().map(|r| r.2 as f64).collect();
	let prove = summarize(&proves);
	let verify = summarize(&verifies);
	let size = summarize(&sizes);

	let value = serde_json::json!({
		"experiment": "E-006",
		"system": "binius64",
		"label": args.label,
		"task": args.task.name(),
		"task_expression": args.task.expression(),
		"witness_seed": format!("{:#x}", args.task.seed()),
		"n_macs_published": args.task.published_macs(),
		"n_macs_measured_imul": built.n_imul_constraints,
		"n_relus": built.n_relus,
		"n_and_constraints": built.n_and_constraints,
		"n_zero_constraints": built.n_zero_constraints,
		"n_bmul_constraints": built.n_bmul_constraints,
		"n_private_values": built.n_private_values,
		"n_inout_values": built.n_inout_values,
		"max_abs_intermediate": built.max_abs_intermediate.to_string(),
		"log_inv_rate": args.log_inv_rate,
		"security_bits": SECURITY_BITS,
		"n_test_queries": calculate_n_test_queries(SECURITY_BITS, args.log_inv_rate),
		"field": "GF(2^128), BinaryField128bGhash",
		"hash_suite": "StdHashSuite = Sha256HashSuite",
		"trusted_setup": false,
		"zk": false,
		"quantization": "signed INT8 operands sign-extended into 64-bit two's-complement words",
		"rayon_threads": n_threads,
		"rayon_pinned_single_thread": pinned_single_thread,
		"warmup_proofs": args.warmup,
		"measured_reps": args.reps,
		"circuit_build_nanos": build_nanos.to_string(),
		"setup_nanos": setup_nanos.to_string(),
		"setup_note": "one-off; reported separately and never amortised into prove time",
		"prove_nanos_median": prove.map(|s| s.median),
		"prove_nanos_min": prove.map(|s| s.min),
		"prove_nanos_max": prove.map(|s| s.max),
		"verify_nanos_median": verify.map(|s| s.median),
		"verify_nanos_min": verify.map(|s| s.min),
		"verify_nanos_max": verify.map(|s| s.max),
		"proof_bytes_median": size.map(|s| s.median),
		"timing_clock": "std::time::Instant (macOS: monotonic, does not advance during sleep)",
	});
	let path = args.out_dir.join("cell.json");
	fs::write(&path, serde_json::to_string_pretty(&value)?)?;
	eprintln!("wrote {}", path.display());
	Ok(())
}
