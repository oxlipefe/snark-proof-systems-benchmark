//! One measured repetition, for each of the two routes, with the brackets declared.
//!
//! # What is inside `prove` and inside `verify`, per route
//!
//! | route | `prove` | `verify` |
//! |---|---|---|
//! | `sumcheck` | absorb `C`, sample `(r1,r2)`, build `A~(r1,.)` and `B~(.,r2)`, `log K` sumcheck rounds, evaluate the two closing openings | absorb `C`, sample `(r1,r2)`, recompute `C~(r1,r2)`, replay the rounds, check the closing product |
//! | `sumcheck-whir` | the above **plus** the WHIR commitment to `A` and `B` and the prescribed-point opening | the above **plus** absorbing the commitment and verifying the opening |
//!
//! Statement construction — drawing the instance, embedding it, computing `C` in the field —
//! is **outside both**, timed once and reported as `build`. The WHIR configuration, the
//! Poseidon2 constants, the Merkle scheme and the DFT twiddles are also outside, reported as
//! `setup`. Neither is amortised into a prove time or into any derived rate.

use anyhow::Result;
use p3_challenger::CanObserve;

use crate::fields::{FieldPair, KoalaBearPair};
use crate::matmul::{self, Statement};
use crate::pcs;

/// One repetition's raw measurements. Numerator and denominator of every derived rate come
/// from the same repetition of the same run; no ratio is formed here.
#[derive(Debug, Clone, Copy)]
pub struct Rep {
    pub prove_nanos: u128,
    pub verify_nanos: u128,
    pub proof_bytes: usize,
}

/// The two routes of `systems/plonky3/EXPRESSION.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Route {
    /// Sumcheck only. `A` and `B` are **not committed**: the two closing evaluations are
    /// numbers the prover sends, so the proof binds the claim to the public output and to
    /// nothing else. Available over both fields, which is what makes the cross-field cell
    /// possible at all.
    #[value(name = "sumcheck")]
    Sumcheck,
    /// Sumcheck plus a WHIR commitment to `A` and `B`, opened at the prescribed points the
    /// sumcheck produced. A proof of knowledge of the operands. **Prime field only.**
    #[value(name = "sumcheck-whir")]
    SumcheckWhir,
}

impl Route {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sumcheck => "sumcheck",
            Self::SumcheckWhir => "sumcheck-whir",
        }
    }

    /// What the proof binds about the weights, in `bench/RESULTS.md` §2's vocabulary.
    pub const fn binds(self) -> &'static str {
        match self {
            Self::Sumcheck => {
                "NOTHING about the operands. The closing evaluations A~(r1,r3) and B~(r3,r2) \
                 are unbound prover-supplied values; the proof binds the sumcheck claim to the \
                 public output C only."
            }
            Self::SumcheckWhir => {
                "The operands, per proof. A and B are committed as one stacked multilinear and \
                 opened at points the transcript fixes after the commitment; weight regime \
                 `witness`, weight cost in PROVE."
            }
        }
    }
}

/// Runs `reps` measured repetitions of the sumcheck-only route over the field pair `P`.
pub fn run_sumcheck<P: FieldPair>(
    st: &Statement<P>,
    warmup: usize,
    reps: usize,
) -> Result<Vec<Rep>> {
    for _ in 0..warmup {
        one_sumcheck::<P>(st)?;
    }
    (0..reps).map(|_| one_sumcheck::<P>(st)).collect()
}

fn one_sumcheck<P: FieldPair>(st: &Statement<P>) -> Result<Rep> {
    use std::time::Instant;

    let mut prover_ch = P::challenger();
    let started = Instant::now();
    let proven = matmul::prove::<P>(st, &mut prover_ch);
    let prove_nanos = started.elapsed().as_nanos();

    let proof_bytes = matmul::proof_bytes(&proven.proof)?;

    let mut verifier_ch = P::challenger();
    let started = Instant::now();
    matmul::verify::<P>(st, &proven.proof, &mut verifier_ch)?;
    let verify_nanos = started.elapsed().as_nanos();

    Ok(Rep {
        prove_nanos,
        verify_nanos,
        proof_bytes,
    })
}

/// Runs `reps` measured repetitions of the committed route. Prime field only.
pub fn run_sumcheck_whir(
    st: &Statement<KoalaBearPair>,
    warmup: usize,
    reps: usize,
) -> Result<(Vec<Rep>, u128, pcs::Setup)> {
    use std::time::Instant;

    let started = Instant::now();
    let (setup, _) = pcs::setup(st.log_m + st.log_k, st.log_k + st.log_n)?;
    let setup_nanos = started.elapsed().as_nanos();

    for _ in 0..warmup {
        one_whir(st, &setup)?;
    }
    let rows = (0..reps)
        .map(|_| one_whir(st, &setup))
        .collect::<Result<Vec<_>>>()?;
    Ok((rows, setup_nanos, setup))
}

fn one_whir(st: &Statement<KoalaBearPair>, setup: &pcs::Setup) -> Result<Rep> {
    use std::time::Instant;

    let mut prover_ch = pcs::challenger(setup);
    let started = Instant::now();
    let (commitment, data) = pcs::commit(setup, &st.a, &st.b, &mut prover_ch);
    let proven = matmul::prove::<KoalaBearPair>(st, &mut prover_ch);
    let pcs_proof = pcs::open(
        setup,
        data,
        &proven.a_point,
        &proven.b_point,
        &mut prover_ch,
    );
    let prove_nanos = started.elapsed().as_nanos();

    let proof_bytes = matmul::proof_bytes(&proven.proof)? + pcs::proof_bytes(&pcs_proof)?;

    let mut verifier_ch = pcs::challenger(setup);
    let started = Instant::now();
    verifier_ch.observe(commitment.clone());
    matmul::verify::<KoalaBearPair>(st, &proven.proof, &mut verifier_ch)?;
    let (a_bound, b_bound) = pcs::verify_open(
        setup,
        &commitment,
        &pcs_proof,
        &proven.a_point,
        &proven.b_point,
        &mut verifier_ch,
    )?;
    // The commitment is only doing work if what it opens is what the sumcheck closed on.
    anyhow::ensure!(
        a_bound == proven.proof.a_open && b_bound == proven.proof.b_open,
        "the committed openings differ from the values the sumcheck closed on"
    );
    let verify_nanos = started.elapsed().as_nanos();

    Ok(Rep {
        prove_nanos,
        verify_nanos,
        proof_bytes,
    })
}
