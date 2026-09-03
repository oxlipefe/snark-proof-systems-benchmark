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
use p3_challenger::CanObserve;
use p3_field::PrimeCharacteristicRing;
use plonky3_bench_harness::fields::{Binary128Pair, FieldPair, KoalaBearPair};
use plonky3_bench_harness::matmul::{self, Statement};
use plonky3_bench_harness::pcs;
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

/// The committed route's own control: a corrupted weight must fail the WHIR opening as well.
fn whir_controls(task: Task) -> Result<Vec<Verdict>> {
    let mut inst = Instance::draw(task)?;
    let honest = matmul::embed::<KoalaBearPair>(&inst)?;
    let (setup, _) = pcs::setup(honest.log_m + honest.log_k, honest.log_k + honest.log_n)?;

    inst.b[0][0] ^= 1;
    let (c_new, max_abs) = inst.recompute()?;
    if c_new == inst.c {
        return Ok(vec![Verdict {
            task: task.name(),
            field: KoalaBearPair::NAME,
            route: "sumcheck-whir",
            kind: "weight_bit",
            detail: "B[0][0] low bit".to_string(),
            outcome: "WITNESS_INERT",
        }]);
    }
    inst.c = c_new;
    inst.max_abs_intermediate = max_abs;
    let mut bad = matmul::embed::<KoalaBearPair>(&inst)?;
    bad.c = honest.c.clone();

    // The prover commits to the CORRUPTED operands and proves against the published output.
    let mut prover_ch = pcs::challenger(&setup);
    let (commitment, data) = pcs::commit(&setup, &bad.a, &bad.b, &mut prover_ch);
    let proven = matmul::prove::<KoalaBearPair>(&bad, &mut prover_ch);
    let pcs_proof = pcs::open(
        &setup,
        data,
        &proven.a_point,
        &proven.b_point,
        &mut prover_ch,
    );

    let mut ch = pcs::challenger(&setup);
    ch.observe(commitment.clone());
    let sumcheck_ok = matmul::verify::<KoalaBearPair>(&honest, &proven.proof, &mut ch).is_ok();
    let opening_ok = pcs::verify_open(
        &setup,
        &commitment,
        &pcs_proof,
        &proven.a_point,
        &proven.b_point,
        &mut ch,
    )
    .is_ok();

    Ok(vec![Verdict {
        task: task.name(),
        field: KoalaBearPair::NAME,
        route: "sumcheck-whir",
        kind: "weight_bit",
        detail: format!(
            "B[0][0] low bit; sumcheck_ok={sumcheck_ok} opening_ok={opening_ok}"
        ),
        outcome: if sumcheck_ok && opening_ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        },
    }])
}
