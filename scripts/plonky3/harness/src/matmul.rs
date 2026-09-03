//! Thaler's MATMULT, expressed on `p3-sumcheck`, generic over the field pair.
//!
//! # The protocol
//!
//! For `C = A[M x K] . B[K x N]`, the multilinear extension of `C` satisfies
//!
//! ```text
//!     C~(r1, r2) = sum_{k in {0,1}^log K}  A~(r1, k) * B~(k, r2)
//! ```
//!
//! which is a sum of a PRODUCT of two multilinears over `log K` variables — exactly the shape
//! [`p3_sumcheck::product_polynomial::ProductPolynomial`] drives. So:
//!
//! 1. the verifier samples `r1` and `r2` and computes `C~(r1, r2)` from the public output;
//! 2. `log K` sumcheck rounds reduce that to one claim at a random `r3`;
//! 3. the claim closes on `A~(r1, r3) * B~(r3, r2)`.
//!
//! No intermediate is committed and no constraint system is built: the prover's dominant work
//! is materialising `A~(r1, .)` and `B~(., r2)`, which costs `M*K + K*N` field multiplies —
//! i.e. **one field multiply per MAC**, on the padded shape.
//!
//! # What this proves, and what it does NOT
//!
//! Steps 1-3 bind the claim to the public `C`. They do NOT bind `A` and `B` to anything: the
//! two closing evaluations are numbers the prover sends. Turning this into a proof of
//! knowledge needs a multilinear PCS over the same field, which is the [`crate::pcs`] question
//! and is where the two fields part company. Every cell records which of the two it ran.
//!
//! # Padding
//!
//! A hypercube index is a power of two. `T1-a`'s `K = N = 768` is not, so it is padded to
//! `1024` with zeros: the prover does `1.778x` the task's arithmetic, and that factor is
//! reported in the cell, never absorbed into `MAC/s`. `T1-0` is aligned and pays nothing.

use anyhow::Result;
use p3_challenger::FieldChallenger;
use p3_field::{ExtensionField, Field, PrimeCharacteristicRing};

use p3_multilinear_util::poly::Poly;
use p3_sumcheck::SumcheckData;
use p3_sumcheck::product_polynomial::ProductPolynomial;
use p3_sumcheck::strategy::{Basis, SumcheckProver, VariableOrder};
use p3_util::log2_strict_usize;
use serde::{Deserialize, Serialize};

use crate::fields::FieldPair;
use crate::mle::{eq_table, eval_base, eval_ext};
use crate::tasks::Instance;

/// Sumcheck rounds bind variables from the most significant index bit down.
const ORDER: VariableOrder = VariableOrder::Prefix;

/// Basis for the round messages. `Evaluation` is the hypercube basis; `Projective` is
/// prefix-only and belongs to WHIR's internal path.
const BASIS: Basis = Basis::Evaluation;

/// Proof-of-work grinding per round. Zero: the campaign compares protocol cost, and a PoW
/// term would add a hash-rate measurement to a field-arithmetic one. Declared per cell.
const POW_BITS: usize = 0;

/// The statement, with every operand already embedded in the field and zero-padded.
pub struct Statement<P: FieldPair> {
    /// `A`, padded to `mp * kp`, row-major.
    pub a: Vec<P::F>,
    /// `B`, padded to `kp * np`, row-major. The weights.
    pub b: Vec<P::F>,
    /// `C`, padded to `mp * np`, row-major. **Public.**
    pub c: Vec<P::F>,
    pub log_m: usize,
    pub log_k: usize,
    pub log_n: usize,
    /// Did the field reproduce the task's INT32 reference output exactly?
    pub integer_faithful: bool,
}

/// A MATMULT proof: the sumcheck transcript plus the two closing evaluations.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "")]
pub struct MatmulProof<F: Field, EF: ExtensionField<F>> {
    pub sumcheck: SumcheckData<F, EF>,
    /// `A~(r1, r3)`.
    pub a_open: EF,
    /// `B~(r3, r2)`.
    pub b_open: EF,
}

/// What the cell reports alongside its timings, in the same line as the number.
#[derive(Serialize, Clone, Debug)]
pub struct Shape {
    pub log_m: usize,
    pub log_k: usize,
    pub log_n: usize,
    /// Sumcheck rounds = `log K` (padded).
    pub rounds: usize,
    /// Field elements the prover materialises for the two reduced polynomials.
    pub reduced_poly_elements: usize,
    /// Field multiplies in the reduction phase: `mp*kp + kp*np + 2*mp*np + mp + np`.
    /// Multiplies INSIDE `p3-sumcheck`'s rounds are NOT instrumented and are not in here.
    pub reduction_field_muls: usize,
    /// Padded MACs — what the prover actually does.
    pub padded_macs: usize,
    /// Padded / published. `1.0` when the task's shape is already a power of two.
    pub padding_factor: f64,
}

/// Embeds a drawn instance into the field pair `P`, zero-padded to the hypercube.
///
/// The public output `C` is computed **in the field**. When the field carries integers
/// faithfully this is checked against the task's INT32 reference and the cell fails if they
/// disagree; when it does not, the in-field product IS the statement and the check is skipped
/// with `integer_faithful = false` recorded on the cell.
pub fn embed<P: FieldPair>(inst: &Instance) -> Result<Statement<P>> {
    let (mp, kp, np) = (inst.mp, inst.kp, inst.np);

    let a: Vec<P::F> = inst.a_padded().into_iter().map(P::embed_i8).collect();
    let b: Vec<P::F> = inst.b_padded().into_iter().map(P::embed_i8).collect();

    // The field product, computed once and outside every timed section.
    let mut c = vec![P::F::ZERO; mp * np];
    for i in 0..mp {
        for kk in 0..kp {
            let a_ik = a[i * kp + kk];
            if a_ik.is_zero() {
                continue;
            }
            for j in 0..np {
                c[i * np + j] += a_ik * b[kk * np + j];
            }
        }
    }

    if P::INTEGER_FAITHFUL {
        let reference = inst.c_padded();
        for (idx, &want) in reference.iter().enumerate() {
            let want = P::embed_i64(want).expect("a faithful field embeds its own reference");
            anyhow::ensure!(
                c[idx] == want,
                "{}: field product disagrees with the INT32 reference at index {idx}; the \
                 field does not carry this task's arithmetic and the cell must not be published",
                inst.task.name()
            );
        }
    }

    Ok(Statement {
        a,
        b,
        c,
        log_m: log2_strict_usize(mp),
        log_k: log2_strict_usize(kp),
        log_n: log2_strict_usize(np),
        integer_faithful: P::INTEGER_FAITHFUL,
    })
}

impl<P: FieldPair> Statement<P> {
    pub fn shape(&self, published_macs: usize) -> Shape {
        let (mp, kp, np) = (1 << self.log_m, 1 << self.log_k, 1 << self.log_n);
        let padded = mp * kp * np;
        Shape {
            log_m: self.log_m,
            log_k: self.log_k,
            log_n: self.log_n,
            rounds: self.log_k,
            reduced_poly_elements: 2 * kp,
            reduction_field_muls: mp * kp + kp * np + 2 * mp * np + mp + np,
            padded_macs: padded,
            padding_factor: padded as f64 / published_macs as f64,
        }
    }

    /// Absorbs the public statement and draws `(r1, r2)`.
    ///
    /// Prover and verifier call this, in this order, on their own transcripts. The output is
    /// public, so it is observed in full: a verifier that never saw `C` would be checking a
    /// claim about a matrix the prover chose after seeing the challenges.
    fn absorb_and_sample(&self, challenger: &mut P::Challenger) -> (Vec<P::EF>, Vec<P::EF>) {
        for &value in &self.c {
            challenger.observe_algebra_element(value);
        }
        let r1: Vec<P::EF> = (0..self.log_m)
            .map(|_| challenger.sample_algebra_element())
            .collect();
        let r2: Vec<P::EF> = (0..self.log_n)
            .map(|_| challenger.sample_algebra_element())
            .collect();
        (r1, r2)
    }
}

/// A proof together with the two points its closing evaluations were taken at.
///
/// The points are returned rather than recomputed because a PCS route has to open exactly
/// them, and recomputing a point is how the two halves of a protocol drift apart.
pub struct Proven<P: FieldPair> {
    pub proof: MatmulProof<P::F, P::EF>,
    pub a_point: Vec<P::EF>,
    pub b_point: Vec<P::EF>,
}

/// Proves `C = A . B` for the embedded statement.
pub fn prove<P: FieldPair>(st: &Statement<P>, challenger: &mut P::Challenger) -> Proven<P> {
    let (mp, kp, np) = (1 << st.log_m, 1 << st.log_k, 1 << st.log_n);
    let (r1, r2) = st.absorb_and_sample(challenger);

    let eq1 = eq_table(&r1);
    let eq2 = eq_table(&r2);

    // claimed = C~(r1, r2).
    let mut claimed = P::EF::ZERO;
    for i in 0..mp {
        for j in 0..np {
            claimed += eq1[i] * eq2[j] * st.c[i * np + j];
        }
    }

    // evals[k]   = A~(r1, k)   — mp * kp multiplies.
    let evals: Vec<P::EF> = (0..kp)
        .map(|kk| {
            (0..mp)
                .map(|i| eq1[i] * st.a[i * kp + kk])
                .sum::<P::EF>()
        })
        .collect();

    // weights[k] = B~(k, r2)   — kp * np multiplies. This is the dominant term.
    let weights: Vec<P::EF> = (0..kp)
        .map(|kk| {
            let row = &st.b[kk * np..(kk + 1) * np];
            eq2.iter()
                .zip(row)
                .map(|(&e, &v)| e * v)
                .sum::<P::EF>()
        })
        .collect();

    let poly = ProductPolynomial::new_unpacked(ORDER, Poly::new(evals), Poly::new(weights));
    let mut prover = SumcheckProver::new(poly, claimed);
    let mut sumcheck = SumcheckData::default();
    let r3 = prover.compute_sumcheck_polynomials(
        &mut sumcheck,
        challenger,
        st.log_k,
        POW_BITS,
        None,
    );

    // The two closing evaluations, read off the ORIGINAL tables rather than off the folded
    // state, so that a folding-convention error shows up as a rejected proof instead of as a
    // proof of the wrong statement.
    let mut a_point = r1.clone();
    a_point.extend_from_slice(r3.as_slice());
    let mut b_point = r3.as_slice().to_vec();
    b_point.extend_from_slice(&r2);

    let a_open = eval_base::<P::F, P::EF>(&st.a, &a_point);
    let b_open = eval_base::<P::F, P::EF>(&st.b, &b_point);

    challenger.observe_algebra_element(a_open);
    challenger.observe_algebra_element(b_open);

    Proven {
        proof: MatmulProof {
            sumcheck,
            a_open,
            b_open,
        },
        a_point,
        b_point,
    }
}

/// The verifier. Returns the point the two openings were taken at, so a PCS route can bind
/// them; the sumcheck-only route discards it and is weaker by exactly that much.
pub fn verify<P: FieldPair>(
    st: &Statement<P>,
    proof: &MatmulProof<P::F, P::EF>,
    challenger: &mut P::Challenger,
) -> Result<(Vec<P::EF>, Vec<P::EF>)> {
    let (mp, np) = (1 << st.log_m, 1 << st.log_n);
    let (r1, r2) = st.absorb_and_sample(challenger);

    let eq1 = eq_table(&r1);
    let eq2 = eq_table(&r2);
    let mut claimed = P::EF::ZERO;
    for i in 0..mp {
        for j in 0..np {
            claimed += eq1[i] * eq2[j] * st.c[i * np + j];
        }
    }

    let r3 = proof
        .sumcheck
        .verify_rounds(challenger, &mut claimed, st.log_k, POW_BITS, BASIS)
        .map_err(|e| anyhow::anyhow!("sumcheck rejected: {e:?}"))?;

    anyhow::ensure!(
        proof.a_open * proof.b_open == claimed,
        "the closing product A~(r1,r3) * B~(r3,r2) does not equal the folded claim"
    );

    challenger.observe_algebra_element(proof.a_open);
    challenger.observe_algebra_element(proof.b_open);

    let mut a_point = r1;
    a_point.extend_from_slice(r3.as_slice());
    let mut b_point = r3.as_slice().to_vec();
    b_point.extend_from_slice(&r2);
    Ok((a_point, b_point))
}

/// Serialised proof bytes. Postcard, as Plonky3's own WHIR example measures proof size with.
pub fn proof_bytes<F: Field + Serialize, EF: ExtensionField<F> + Serialize>(
    proof: &MatmulProof<F, EF>,
) -> Result<usize> {
    Ok(postcard::to_allocvec(proof)
        .map_err(|e| anyhow::anyhow!("serialising the proof: {e}"))?
        .len())
}

/// Independent check of the closing evaluations against a direct contraction. Not part of the
/// protocol; a self-test that the two ways of computing `A~(r1,r3)` agree.
#[allow(dead_code)]
pub fn cross_check_opening<F: Field, EF: ExtensionField<F>>(
    table: &[F],
    point: &[EF],
    claimed: EF,
) -> bool {
    let eq = eq_table(point);
    let contracted: EF = eq.iter().zip(table).map(|(&e, &v)| e * v).sum();
    contracted == claimed && eval_ext(&eq, &[]) == eq[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::{Binary128Pair, KoalaBearPair};
    use crate::tasks::{Instance, Task};

    fn round_trip<P: FieldPair>(task: Task) {
        let inst = Instance::draw(task).expect("draws");
        let st = embed::<P>(&inst).expect("embeds");
        let mut prover_ch = P::challenger();
        let proven = prove::<P>(&st, &mut prover_ch);
        let mut verifier_ch = P::challenger();
        verify::<P>(&st, &proven.proof, &mut verifier_ch).expect("an honest proof verifies");
    }

    #[test]
    fn t1_0_round_trips_over_the_prime_field() {
        round_trip::<KoalaBearPair>(Task::T1_0);
    }

    #[test]
    fn t1_0_round_trips_over_the_binary_field() {
        round_trip::<Binary128Pair>(Task::T1_0);
    }

    /// The padded rung must round-trip too, or the zero padding is not neutral.
    #[test]
    fn t1_a_round_trips_over_the_prime_field() {
        round_trip::<KoalaBearPair>(Task::T1A);
    }

    /// A corrupted weight must be rejected. This is the §10 control, run as a unit test as
    /// well as from the negative binary, because a benchmark whose control only runs in a
    /// script is a benchmark whose control can be forgotten.
    fn corruption_is_rejected<P: FieldPair>(task: Task) {
        let mut inst = Instance::draw(task).expect("draws");
        let st_honest = embed::<P>(&inst).expect("embeds");

        // Flip the low bit of one weight, then recompute the reference: A3 requires that the
        // output actually change before the corruption counts as a test.
        inst.b[0][0] ^= 1;
        let (c_new, max_abs) = inst.recompute().expect("recomputes");
        assert_ne!(c_new, inst.c, "the flip left the output unchanged: WITNESS_INERT");
        inst.c = c_new;
        inst.max_abs_intermediate = max_abs;

        // The prover proves the CORRUPTED operands against the ORIGINAL public output.
        let mut st_bad = embed::<P>(&inst).expect("embeds");
        st_bad.c = st_honest.c.clone();

        let mut prover_ch = P::challenger();
        let proven = prove::<P>(&st_bad, &mut prover_ch);
        let mut verifier_ch = P::challenger();
        assert!(
            verify::<P>(&st_honest, &proven.proof, &mut verifier_ch).is_err(),
            "a proof of a corrupted product was ACCEPTED against the published output"
        );
    }

    #[test]
    fn corrupted_weight_is_rejected_over_the_prime_field() {
        corruption_is_rejected::<KoalaBearPair>(Task::T1_0);
    }

    #[test]
    fn corrupted_weight_is_rejected_over_the_binary_field() {
        corruption_is_rejected::<Binary128Pair>(Task::T1_0);
    }
}
