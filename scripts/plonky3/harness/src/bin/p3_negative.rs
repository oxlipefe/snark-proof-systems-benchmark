//! G-13b · the correctness control. **Blocking**: it runs before the timings it licenses.
//!
//! A benchmark that never checks whether the verifier rejects a bad proof measures the speed of
//! a system that might accept anything. Five corruptions are applied, each to one honest cell,
//! and each verdict is reported with what it does and does not establish.
//!
//! # Amendment A3 of `bench/TASKS.md`, applied
//!
//! A witness corruption counts as a test only if the OUTPUT changes. Every operand corruption
//! here recomputes the reference forward pass first; a position that leaves the output
//! bit-identical is reported `WITNESS_INERT` and counted as neither a pass nor a failure.
//! For a matmul with no activations an inert position is rare, but "rare" is not "measured".
//!
//! # What each corruption establishes, which is not the same for the two routes
//!
//! * `weight_bit` / `input_bit` — the prover proves the CORRUPTED operands against the
//!   PUBLISHED output. On the `sumcheck` route this tests that the claim is bound to `C`; it
//!   does **not** test any binding of the operands, because there is none. On the
//!   `sumcheck-whir` route the same corruption additionally has to survive the commitment.
//! * `public_output_bit` — the verifier is handed a different `C` from the one proved. This is
//!   the strongest control available on either route: it corrupts the statement itself.
//! * `round_message` and `closing_opening` — the proof bytes are altered after the fact. These
//!   always corrupt the transcript and are route-independent.

use std::{fs, io::Write, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use p3_field::PrimeCharacteristicRing;
use plonky3_bench_harness::fields::{Binary128Pair, FieldPair, KoalaBearPair};
use plonky3_bench_harness::matmul::{self, Statement};
use plonky3_bench_harness::route::{Route, WhirSetup};
use plonky3_bench_harness::tasks::{Instance, Task, task_from_name};

#[derive(Debug, Parser)]
#[command(name = "p3-negative", about = "The blocking correctness control for Plonky3")]
struct Args {
    /// Task names as `bench/TASKS.md` writes them, e.g. `t1-0`.
    #[arg(required = true)]
    tasks: Vec<String>,

    #[arg(long)]
    out_dir: PathBuf,
}

/// One corruption's verdict.
struct Verdict {
    task: &'static str,
    field: &'static str,
    route: &'static str,
    kind: &'static str,
    detail: String,
    /// `REJECTED` (the control passed), `ACCEPTED` (an alert), or `WITNESS_INERT`.
    outcome: &'static str,
}

fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.out_dir)?;

    let mut verdicts = Vec::new();
    for name in &args.tasks {
        let task = task_from_name(name)?;
        verdicts.extend(sumcheck_controls::<KoalaBearPair>(task)?);
        verdicts.extend(sumcheck_controls::<Binary128Pair>(task)?);
        verdicts.extend(whir_controls(task)?);
    }

    let mut report = String::new();
    let mut csv = String::from("task,field,route,kind,detail,outcome\n");
    let mut accepted = 0usize;
    for v in &verdicts {
        report.push_str(&format!(
            "{:<6} {:<11} {:<14} {:<18} {:<40} {}\n",
            v.task, v.field, v.route, v.kind, v.detail, v.outcome
        ));
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            v.task, v.field, v.route, v.kind, v.detail, v.outcome
        ));
        if v.outcome == "ACCEPTED" {
            accepted += 1;
        }
    }
    let summary = format!(
        "\n{} corruptions applied; {} REJECTED, {} ACCEPTED, {} WITNESS_INERT.\n",
        verdicts.len(),
        verdicts.iter().filter(|v| v.outcome == "REJECTED").count(),
        accepted,
        verdicts
            .iter()
            .filter(|v| v.outcome == "WITNESS_INERT")
            .count()
    );
    report.push_str(&summary);
    print!("{report}");

    fs::File::create(args.out_dir.join("report.txt"))?.write_all(report.as_bytes())?;
    fs::File::create(args.out_dir.join("negative.csv"))?.write_all(csv.as_bytes())?;

    if accepted > 0 {
        anyhow::bail!(
            "{accepted} corruption(s) were ACCEPTED. No timing may be published from this build."
        );
    }
    Ok(())
}

/// Corrupts one operand and proves the corrupted instance against the published output.
fn corrupt_operand<P: FieldPair>(
    task: Task,
    which: &'static str,
) -> Result<Verdict> {
    let mut inst = Instance::draw(task)?;
    let honest = matmul::embed::<P>(&inst)?;

    let detail = match which {
        "weight_bit" => {
            inst.b[0][0] ^= 1;
            format!("B[0][0] low bit, {} -> {}", inst.b[0][0] ^ 1, inst.b[0][0])
        }
        _ => {
            inst.a[0][0] ^= 1;
            format!("A[0][0] low bit, {} -> {}", inst.a[0][0] ^ 1, inst.a[0][0])
        }
    };

    // A3: the corruption is a test only if the output moved.
    let (c_new, max_abs) = inst.recompute()?;
    if c_new == inst.c {
        return Ok(Verdict {
            task: task.name(),
            field: P::NAME,
            route: "sumcheck",
            kind: which,
            detail,
            outcome: "WITNESS_INERT",
        });
    }
    inst.c = c_new;
    inst.max_abs_intermediate = max_abs;

    let mut bad = matmul::embed::<P>(&inst)?;
    bad.c = honest.c.clone();

    let mut prover_ch = P::challenger();
    let proven = matmul::prove::<P>(&bad, &mut prover_ch);
    let mut verifier_ch = P::challenger();
    let outcome = if matmul::verify::<P>(&honest, &proven.proof, &mut verifier_ch).is_err() {
        "REJECTED"
    } else {
        "ACCEPTED"
    };
    Ok(Verdict {
        task: task.name(),
        field: P::NAME,
        route: "sumcheck",
        kind: which,
        detail,
        outcome,
    })
}

fn sumcheck_controls<P: FieldPair>(task: Task) -> Result<Vec<Verdict>> {
    let mut out = vec![
        corrupt_operand::<P>(task, "weight_bit")?,
        corrupt_operand::<P>(task, "input_bit")?,
    ];

    let inst = Instance::draw(task)?;
    let st: Statement<P> = matmul::embed::<P>(&inst)?;
    let mut prover_ch = P::challenger();
    let proven = matmul::prove::<P>(&st, &mut prover_ch);

    // The verifier is handed a different public output from the one proved.
    let mut tampered = matmul::embed::<P>(&inst)?;
    tampered.c[0] += P::F::ONE;
    let mut ch = P::challenger();
    out.push(Verdict {
        task: task.name(),
        field: P::NAME,
        route: "sumcheck",
        kind: "public_output_bit",
        detail: "C[0][0] + 1 on the verifier's side".to_string(),
        outcome: if matmul::verify::<P>(&tampered, &proven.proof, &mut ch).is_err() {
            "REJECTED"
        } else {
            "ACCEPTED"
        },
    });

    // One sumcheck round message is altered after the proof was produced.
    let mut bad = proven.proof.clone();
    bad.sumcheck.polynomial_evaluations[0][0] += P::EF::ONE;
    let mut ch = P::challenger();
    out.push(Verdict {
        task: task.name(),
        field: P::NAME,
        route: "sumcheck",
        kind: "round_message",
        detail: "round 0, h(0) + 1".to_string(),
        outcome: if matmul::verify::<P>(&st, &bad, &mut ch).is_err() {
            "REJECTED"
        } else {
            "ACCEPTED"
        },
    });

    // One closing evaluation is altered.
    let mut bad = proven.proof.clone();
    bad.a_open += P::EF::ONE;
    let mut ch = P::challenger();
    out.push(Verdict {
        task: task.name(),
        field: P::NAME,
        route: "sumcheck",
        kind: "closing_opening",
        detail: "A~(r1,r3) + 1".to_string(),
        outcome: if matmul::verify::<P>(&st, &bad, &mut ch).is_err() {
            "REJECTED"
        } else {
            "ACCEPTED"
        },
    });

    Ok(out)
}

/// Every committed route, corrupted three ways.
///
/// The routes differ only in how many WHIR commitments carry the operands, so they get the same
/// corruptions from the same helpers rather than a second copy of the control. A route that
/// binds its operands has to catch a flipped weight AT THE OPENING, not merely at the sumcheck,
/// and the verdict records both halves so that distinction is visible in the report.
const COMMITTED_ROUTES: [Route; 2] = [Route::SumcheckWhir, Route::SumcheckWhirSplit];

fn whir_controls(task: Task) -> Result<Vec<Verdict>> {
    let mut out = Vec::new();
    for route in COMMITTED_ROUTES {
        out.push(committed_operand_control(task, route, "weight_bit")?);
        out.push(committed_operand_control(task, route, "input_bit")?);
        out.push(committed_output_control(task, route)?);
        out.push(committed_binding_control(task, route)?);
    }
    Ok(out)
}

/// Commits the CORRUPTED operands and proves them against the PUBLISHED output.
fn committed_operand_control(
    task: Task,
    route: Route,
    which: &'static str,
) -> Result<Verdict> {
    let mut inst = Instance::draw(task)?;
    let honest = matmul::embed::<KoalaBearPair>(&inst)?;
    let setup = WhirSetup::build(route, &honest)?;

    let detail = if which == "weight_bit" {
        inst.b[0][0] ^= 1;
        format!("B[0][0] low bit, {} -> {}", inst.b[0][0] ^ 1, inst.b[0][0])
    } else {
        inst.a[0][0] ^= 1;
        format!("A[0][0] low bit, {} -> {}", inst.a[0][0] ^ 1, inst.a[0][0])
    };

    // A3: the corruption is a test only if the output moved.
    let (c_new, max_abs) = inst.recompute()?;
    if c_new == inst.c {
        return Ok(Verdict {
            task: task.name(),
            field: KoalaBearPair::NAME,
            route: route.name(),
            kind: which,
            detail,
            outcome: "WITNESS_INERT",
        });
    }
    inst.c = c_new;
    inst.max_abs_intermediate = max_abs;

    let mut bad = matmul::embed::<KoalaBearPair>(&inst)?;
    bad.c = honest.c.clone();

    let mut prover_ch = setup.challenger();
    let proven = setup.prove(&bad, &mut prover_ch);
    let v = setup.verify_parts(&honest, &proven);

    Ok(Verdict {
        task: task.name(),
        field: KoalaBearPair::NAME,
        route: route.name(),
        kind: which,
        detail: format!(
            "{detail}; sumcheck_ok={} opening_ok={} bound_matches={}",
            v.sumcheck_ok, v.opening_ok, v.bound_matches
        ),
        outcome: verdict_of(v),
    })
}

/// An honest proof handed to a verifier holding a DIFFERENT public output.
fn committed_output_control(task: Task, route: Route) -> Result<Verdict> {
    let inst = Instance::draw(task)?;
    let honest = matmul::embed::<KoalaBearPair>(&inst)?;
    let setup = WhirSetup::build(route, &honest)?;

    let mut prover_ch = setup.challenger();
    let proven = setup.prove(&honest, &mut prover_ch);

    let mut tampered = matmul::embed::<KoalaBearPair>(&inst)?;
    tampered.c[0] += <KoalaBearPair as FieldPair>::F::ONE;
    let v = setup.verify_parts(&tampered, &proven);

    Ok(Verdict {
        task: task.name(),
        field: KoalaBearPair::NAME,
        route: route.name(),
        kind: "public_output_bit",
        detail: format!(
            "C[0][0] + 1 on the verifier's side; sumcheck_ok={} opening_ok={} bound_matches={}",
            v.sumcheck_ok, v.opening_ok, v.bound_matches
        ),
        outcome: verdict_of(v),
    })
}

/// The control that isolates what the committed routes exist for.
///
/// The three controls above all corrupt something the SUMCHECK already catches, so on a
/// committed route they also desynchronise the transcript and the opening fails with it — which
/// proves the proof is rejected but says nothing about whether the commitment binds anything.
/// This one commits a corrupted `B` and runs the honest sumcheck: the sumcheck is valid, the
/// WHIR opening is a valid opening of the committed polynomial, and the ONLY thing standing
/// between the verifier and a proof about operands nobody committed is the equality between the
/// value the commitment binds and the value the sumcheck closed on. It must fail there.
fn committed_binding_control(task: Task, route: Route) -> Result<Verdict> {
    let inst = Instance::draw(task)?;
    let honest = matmul::embed::<KoalaBearPair>(&inst)?;
    let setup = WhirSetup::build(route, &honest)?;

    let mut corrupted = inst.clone();
    corrupted.b[0][0] ^= 1;
    let detail = format!(
        "B[0][0] low bit COMMITTED ONLY, {} -> {}",
        corrupted.b[0][0] ^ 1,
        corrupted.b[0][0]
    );

    // A3: the flip counts only if it would have moved the output.
    let (c_new, max_abs) = corrupted.recompute()?;
    if c_new == inst.c {
        return Ok(Verdict {
            task: task.name(),
            field: KoalaBearPair::NAME,
            route: route.name(),
            kind: "committed_binding",
            detail,
            outcome: "WITNESS_INERT",
        });
    }
    // The corrupted instance's own consistent output, so that `embed`'s INT32 cross-check
    // passes. Only `.a` and `.b` of the result are used: this statement is COMMITTED, never
    // proved, and the proved statement stays the honest one.
    corrupted.c = c_new;
    corrupted.max_abs_intermediate = max_abs;
    let committed = matmul::embed::<KoalaBearPair>(&corrupted)?;

    let mut prover_ch = setup.challenger();
    let proven = setup.prove_committing(&committed, &honest, &mut prover_ch);
    let v = setup.verify_parts(&honest, &proven);

    Ok(Verdict {
        task: task.name(),
        field: KoalaBearPair::NAME,
        route: route.name(),
        kind: "committed_binding",
        detail: format!(
            "{detail}; sumcheck_ok={} opening_ok={} bound_matches={}",
            v.sumcheck_ok, v.opening_ok, v.bound_matches
        ),
        outcome: verdict_of(v),
    })
}

/// A committed proof is ACCEPTED only when every half accepted it.
const fn verdict_of(v: plonky3_bench_harness::route::CommittedVerdict) -> &'static str {
    if v.sumcheck_ok && v.opening_ok && v.bound_matches {
        "ACCEPTED"
    } else {
        "REJECTED"
    }
}
