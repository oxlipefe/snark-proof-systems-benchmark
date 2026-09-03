//! G-13b · one benchmark cell of `bench/`, for Plonky3.
//!
//! One process per cell (task x field x route x threads), so `/usr/bin/time -l` outside it
//! attributes peak RSS and peak memory footprint to that cell alone — the convention every
//! other system in this benchmark follows.
//!
//! All timing uses [`Instant`], which on macOS is monotonic and does **not** advance while the
//! machine sleeps. The wrapper script compares the total against `/usr/bin/time -l`'s
//! wall-clock `real`; a gap means the cell spanned a sleep and the cell is invalid.

use std::{fs, io::Write, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use plonky3_bench_harness::fields::{Binary128Pair, FieldPair, KoalaBearPair};
use plonky3_bench_harness::matmul::{self, Shape};
use plonky3_bench_harness::route::{Rep, Route, run_sumcheck, run_sumcheck_whir};
use plonky3_bench_harness::stats::summarize;
use plonky3_bench_harness::tasks::{Instance, Task};

const NANOS_PER_SEC: f64 = 1e9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FieldChoice {
    /// KoalaBear (31-bit prime) with its degree-4 extension.
    #[value(name = "koala-bear")]
    KoalaBear,
    /// `GF(2^128)`, the top of p3-binary-field's Wiedemann tower.
    #[value(name = "binary128")]
    Binary128,
}

#[derive(Debug, Parser)]
#[command(name = "p3-bench", about = "One cell of the public zk-prover-bench, on Plonky3")]
struct Args {
    #[arg(long, value_enum)]
    task: Task,

    #[arg(long, value_enum)]
    field: FieldChoice,

    #[arg(long, value_enum, default_value = "sumcheck")]
    route: Route,

    /// Discarded repetitions before the measured ones.
    #[arg(long, default_value_t = 1)]
    warmup: usize,

    /// Measured repetitions. The protocol asks for N >= 5; a lower N is declared per cell.
    #[arg(long, default_value_t = 5)]
    reps: usize,

    /// Build the statement and report its shape without proving.
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

    let threads: usize = std::env::var("RAYON_NUM_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Statement construction: drawing the published instance, embedding it in the field, and
    // computing the public output IN THE FIELD. Timed, reported apart, never in prove.
    let build_started = Instant::now();
    let inst = Instance::draw(args.task)
        .with_context(|| format!("drawing the {} instance", args.task.name()))?;
    inst.assert_matches_binius64()?;

    let (shape, integer_faithful, rows, setup_nanos, extra) = match (args.field, args.route) {
        (FieldChoice::KoalaBear, Route::Sumcheck) => {
            let st = matmul::embed::<KoalaBearPair>(&inst)?;
            let shape = st.shape(args.task.published_macs());
            let build_nanos = build_started.elapsed().as_nanos();
            report_build(&args, &shape, KoalaBearPair::NAME, build_nanos);
            if args.stat_only {
                return Ok(());
            }
            let rows = run_sumcheck::<KoalaBearPair>(&st, args.warmup, args.reps)?;
            (shape, st.integer_faithful, rows, 0u128, String::new())
        }
        (FieldChoice::Binary128, Route::Sumcheck) => {
            let st = matmul::embed::<Binary128Pair>(&inst)?;
            let shape = st.shape(args.task.published_macs());
            let build_nanos = build_started.elapsed().as_nanos();
            report_build(&args, &shape, Binary128Pair::NAME, build_nanos);
            if args.stat_only {
                return Ok(());
            }
            let rows = run_sumcheck::<Binary128Pair>(&st, args.warmup, args.reps)?;
            (shape, st.integer_faithful, rows, 0u128, String::new())
        }
        (FieldChoice::KoalaBear, Route::SumcheckWhir) => {
            let st = matmul::embed::<KoalaBearPair>(&inst)?;
            let shape = st.shape(args.task.published_macs());
            let build_nanos = build_started.elapsed().as_nanos();
            report_build(&args, &shape, KoalaBearPair::NAME, build_nanos);
            if args.stat_only {
                return Ok(());
            }
            let (rows, setup_nanos, setup) = run_sumcheck_whir(&st, args.warmup, args.reps)?;
            let extra = format!(
                "whir_stacked_vars={} whir_final_queries={} security={} rate={} pow_budget={} folding={}",
                setup.num_variables,
                setup.final_queries,
                plonky3_bench_harness::pcs::SECURITY_LEVEL,
                plonky3_bench_harness::pcs::STARTING_LOG_INV_RATE,
                plonky3_bench_harness::pcs::POW_BITS,
                plonky3_bench_harness::pcs::FOLDING_FACTOR,
            );
            (shape, st.integer_faithful, rows, setup_nanos, extra)
        }
        (FieldChoice::Binary128, Route::SumcheckWhir) => {
            anyhow::bail!(
                "route `sumcheck-whir` is not available over BinaryField128, and this is the \
                 campaign's result rather than a limitation of this harness. p3-whir is the \
                 only implementor of p3_commit::MultilinearPcs at the pinned commit, and both \
                 that impl (whir/src/pcs/adapter.rs:64-66) and WhirConfig::new \
                 (whir/src/parameters/whir.rs:203-207) require `F: TwoAdicField` and \
                 `EF: ExtensionField<F> + TwoAdicField`. The multiplicative group of GF(2^128) \
                 has odd order 2^128-1, so its two-adicity is zero and no such impl can exist. \
                 See systems/plonky3/NOT_EXPRESSIBLE.md and the recorded compiler error in \
                 data/probe-plonky3-whir-binary.txt."
            );
        }
    };

    for (rep, row) in rows.iter().enumerate() {
        eprintln!(
            "rep={rep} prove={:.4}s verify={:.5}s proof={}B",
            row.prove_nanos as f64 / NANOS_PER_SEC,
            row.verify_nanos as f64 / NANOS_PER_SEC,
            row.proof_bytes
        );
    }

    write_rows(&args, &shape, threads, &rows)?;
    write_json(
        &args,
        &shape,
        threads,
        integer_faithful,
        setup_nanos,
        &extra,
        &rows,
    )?;
    Ok(())
}

fn report_build(args: &Args, shape: &Shape, field: &str, build_nanos: u128) {
    eprintln!(
        "task={} field={field} route={} macs={} padded_macs={} padding={:.4} \
         log_m={} log_k={} log_n={} rounds={} reduced_poly_elements={} \
         reduction_field_muls={} build={:.4}s",
        args.task.name(),
        args.route.name(),
        args.task.published_macs(),
        shape.padded_macs,
        shape.padding_factor,
        shape.log_m,
        shape.log_k,
        shape.log_n,
        shape.rounds,
        shape.reduced_poly_elements,
        shape.reduction_field_muls,
        build_nanos as f64 / NANOS_PER_SEC
    );
}

/// Raw per-repetition rows. Uncurated: every repetition that ran is here.
fn write_rows(args: &Args, shape: &Shape, threads: usize, rows: &[Rep]) -> Result<()> {
    let path = args.out_dir.join("reps.csv");
    let mut file = fs::File::create(&path)?;
    writeln!(
        file,
        "label,task,field,route,n_macs,padded_macs,padding_factor,rounds,threads,rep,\
         prove_nanos,verify_nanos,proof_bytes"
    )?;
    let field = field_name(args.field);
    for (rep, row) in rows.iter().enumerate() {
        writeln!(
            file,
            "{},{},{field},{},{},{},{:.6},{},{threads},{rep},{},{},{}",
            args.label,
            args.task.name(),
            args.route.name(),
            args.task.published_macs(),
            shape.padded_macs,
            shape.padding_factor,
            shape.rounds,
            row.prove_nanos,
            row.verify_nanos,
            row.proof_bytes
        )?;
    }
    eprintln!("wrote {}", path.display());
    Ok(())
}

const fn field_name(f: FieldChoice) -> &'static str {
    match f {
        FieldChoice::KoalaBear => "koala-bear",
        FieldChoice::Binary128 => "binary128",
    }
}

fn write_json(
    args: &Args,
    shape: &Shape,
    threads: usize,
    integer_faithful: bool,
    setup_nanos: u128,
    extra: &str,
    rows: &[Rep],
) -> Result<()> {
    let proves: Vec<f64> = rows.iter().map(|r| r.prove_nanos as f64).collect();
    let verifies: Vec<f64> = rows.iter().map(|r| r.verify_nanos as f64).collect();
    let sizes: Vec<f64> = rows.iter().map(|r| r.proof_bytes as f64).collect();
    let prove = summarize(&proves);
    let verify = summarize(&verifies);
    let size = summarize(&sizes);

    let cell = serde_json::json!({
        "label": args.label,
        "task": args.task.name(),
        "field": field_name(args.field),
        "route": args.route.name(),
        "binds": args.route.binds(),
        "integer_faithful": integer_faithful,
        "threads_env": threads,
        "n_macs": args.task.published_macs(),
        "padded_macs": shape.padded_macs,
        "padding_factor": shape.padding_factor,
        "log_m": shape.log_m,
        "log_k": shape.log_k,
        "log_n": shape.log_n,
        "sumcheck_rounds": shape.rounds,
        "reduced_poly_elements": shape.reduced_poly_elements,
        "reduction_field_muls": shape.reduction_field_muls,
        "warmup": args.warmup,
        "reps": args.reps,
        "setup_nanos": setup_nanos,
        "extra": extra,
        "prove_median_nanos": prove.map(|s| s.median),
        "prove_min_nanos": prove.map(|s| s.min),
        "prove_max_nanos": prove.map(|s| s.max),
        "verify_median_nanos": verify.map(|s| s.median),
        "verify_min_nanos": verify.map(|s| s.min),
        "verify_max_nanos": verify.map(|s| s.max),
        "proof_bytes_median": size.map(|s| s.median),
    });
    let path = args.out_dir.join("cell.json");
    fs::write(&path, serde_json::to_string_pretty(&cell)?)?;
    eprintln!("wrote {}", path.display());
    Ok(())
}
