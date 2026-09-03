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
    ///
    /// `A` and `B` are STACKED into one multilinear, so the committed polynomial has
    /// `log2_ceil(2^a_vars + 2^b_vars)` variables — very nearly double the operands.
    #[value(name = "sumcheck-whir")]
    SumcheckWhir,
    /// The same statement, the same transcript order, the same bindings — with `A` and `B`
    /// committed under **two** WHIR schemes instead of one stacked one, so the committed
    /// element count is exactly `2^a_vars + 2^b_vars` and the stacking round-up disappears.
    /// `G-13b''` / `D-015`. **Prime field only**, for the same reason `sumcheck-whir` is.
    #[value(name = "sumcheck-whir-split")]
    SumcheckWhirSplit,
}

impl Route {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sumcheck => "sumcheck",
            Self::SumcheckWhir => "sumcheck-whir",
            Self::SumcheckWhirSplit => "sumcheck-whir-split",
        }
    }

    /// Does this route commit to the operands?
    pub const fn is_committed(self) -> bool {
        matches!(self, Self::SumcheckWhir | Self::SumcheckWhirSplit)
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
            Self::SumcheckWhirSplit => {
                "The operands, per proof. A and B are committed under two separate WHIR \
                 schemes, BOTH absorbed before the transcript fixes (r1, r2), and each opened \
                 at the point the sumcheck produced for it; weight regime `witness`, weight \
                 cost in PROVE. Same statement as `sumcheck-whir`, one fewer padding factor."
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

// -----------------------------------------------------------------------------------------
// The committed routes — one stacked commitment, or two separate ones
// -----------------------------------------------------------------------------------------

type BaseF = <KoalaBearPair as FieldPair>::F;
type Ext = <KoalaBearPair as FieldPair>::EF;

/// The commitment shape behind a committed route.
///
/// Both shapes prove the **same statement in the same order**: every commitment is absorbed
/// before the transcript fixes `(r1, r2)`, `C` stays public and is re-evaluated by the
/// verifier, and each opening is taken at exactly the point the sumcheck produced. They differ
/// only in how many WHIR schemes carry the two operands, which is the one thing `G-13b''`
/// measures.
pub enum WhirSetup {
    /// `A` and `B` stacked into one multilinear — the `sumcheck-whir` route, unchanged since
    /// the 2026-09-03 campaign so that its `…-n5` and `…-n6` rows stay reproducible.
    Stacked(pcs::Setup),
    /// `A` and `B` under two schemes — the `sumcheck-whir-split` route.
    Split {
        a: pcs::Setup,
        b: pcs::Setup,
    },
}

/// One committed-route proof: the sumcheck transcript, the points it closed at, and the
/// commitments and openings that bind the operands to it.
pub struct CommittedProven {
    pub proof: matmul::MatmulProof<BaseF, Ext>,
    pub a_point: Vec<Ext>,
    pub b_point: Vec<Ext>,
    /// `C~(r1, r2)`, the sum the sumcheck started from.
    pub claimed: Ext,
    /// In protocol order. One entry for `Stacked`, two for `Split`.
    pub commitments: Vec<pcs::Commitment>,
    /// In protocol order, one per commitment.
    pub openings: Vec<pcs::PcsProof>,
}

/// What each half of a committed verification concluded, kept apart so the negative control can
/// say WHICH check caught a corruption instead of only that something did.
#[derive(Debug, Clone, Copy)]
pub struct CommittedVerdict {
    pub sumcheck_ok: bool,
    pub opening_ok: bool,
    /// Did the values the commitment binds equal the ones the sumcheck closed on?
    pub bound_matches: bool,
}

impl WhirSetup {
    /// Builds the commitment scheme(s) `route` needs for this instance shape.
    pub fn build(route: Route, st: &Statement<KoalaBearPair>) -> Result<Self> {
        let (a_vars, b_vars) = (st.log_m + st.log_k, st.log_k + st.log_n);
        match route {
            Route::Sumcheck => anyhow::bail!(
                "`sumcheck` commits nothing; it has no commitment scheme to build"
            ),
            Route::SumcheckWhir => Ok(Self::Stacked(pcs::setup(a_vars, b_vars)?.0)),
            Route::SumcheckWhirSplit => Ok(Self::Split {
                a: pcs::setup_single(a_vars)?,
                b: pcs::setup_single(b_vars)?,
            }),
        }
    }

    pub const fn route(&self) -> Route {
        match self {
            Self::Stacked(_) => Route::SumcheckWhir,
            Self::Split { .. } => Route::SumcheckWhirSplit,
        }
    }

    /// A fresh transcript with every scheme's domain separator absorbed, in protocol order.
    pub fn challenger(&self) -> pcs::Challenger {
        match self {
            Self::Stacked(s) => pcs::challenger_for(&[s]),
            Self::Split { a, b } => pcs::challenger_for(&[a, b]),
        }
    }

    /// Variables per commitment, in protocol order.
    pub fn whir_vars(&self) -> Vec<usize> {
        match self {
            Self::Stacked(s) => vec![s.num_variables],
            Self::Split { a, b } => vec![a.num_variables, b.num_variables],
        }
    }

    /// Final STIR queries per commitment, in protocol order.
    pub fn final_queries(&self) -> Vec<usize> {
        match self {
            Self::Stacked(s) => vec![s.final_queries],
            Self::Split { a, b } => vec![a.final_queries, b.final_queries],
        }
    }

    /// Field elements actually committed, summed over the commitments. This is the number
    /// `G-13b''` exists to move.
    pub fn committed_elements(&self) -> usize {
        self.whir_vars().iter().map(|v| 1usize << v).sum()
    }

    /// Commit, prove, open — the whole prover half, on one transcript.
    ///
    /// The order is the one `EXPRESSION.md` §4 fixes and soundness rests on:
    /// **commit (all of them) -> absorb `C` -> sample `(r1, r2)` -> sumcheck rounds -> open**.
    pub fn prove(
        &self,
        st: &Statement<KoalaBearPair>,
        challenger: &mut pcs::Challenger,
    ) -> CommittedProven {
        self.prove_committing(st, st, challenger)
    }

    /// The same prover, with the operands it COMMITS taken from `committed` and the statement it
    /// PROVES taken from `proved`.
    ///
    /// An honest proof passes the same statement twice ([`Self::prove`]). The negative control
    /// passes two different ones, which is the only way to test the tie between the commitment
    /// and the sumcheck in isolation: with a corrupted operand in `committed` and the honest one
    /// in `proved`, the sumcheck is valid and the WHIR opening is a valid opening — of the wrong
    /// polynomial — so the only check that can catch it is the equality between the value the
    /// commitment binds and the value the sumcheck closed on.
    pub fn prove_committing(
        &self,
        committed: &Statement<KoalaBearPair>,
        proved: &Statement<KoalaBearPair>,
        challenger: &mut pcs::Challenger,
    ) -> CommittedProven {
        match self {
            Self::Stacked(setup) => {
                let (commitment, data) =
                    pcs::commit(setup, &committed.a, &committed.b, challenger);
                let proven = matmul::prove::<KoalaBearPair>(proved, challenger);
                let opening = pcs::open(setup, data, &proven.a_point, &proven.b_point, challenger);
                CommittedProven {
                    proof: proven.proof,
                    a_point: proven.a_point,
                    b_point: proven.b_point,
                    claimed: proven.claimed,
                    commitments: vec![commitment],
                    openings: vec![opening],
                }
            }
            Self::Split { a, b } => {
                let (a_commitment, a_data) = pcs::commit_single(a, &committed.a, challenger);
                let (b_commitment, b_data) = pcs::commit_single(b, &committed.b, challenger);
                let proven = matmul::prove::<KoalaBearPair>(proved, challenger);
                let a_opening = pcs::open_single(a, a_data, &proven.a_point, challenger);
                let b_opening = pcs::open_single(b, b_data, &proven.b_point, challenger);
                CommittedProven {
                    proof: proven.proof,
                    a_point: proven.a_point,
                    b_point: proven.b_point,
                    claimed: proven.claimed,
                    commitments: vec![a_commitment, b_commitment],
                    openings: vec![a_opening, b_opening],
                }
            }
        }
    }

    /// Absorbs every commitment, in protocol order, before the verifier draws anything.
    fn observe_commitments(&self, proven: &CommittedProven, challenger: &mut pcs::Challenger) {
        for commitment in &proven.commitments {
            challenger.observe(commitment.clone());
        }
    }

    /// Verifies the opening(s) and returns `(A~(a_point), B~(b_point))` as the commitments bind
    /// them. Called only after the sumcheck half has advanced the same transcript.
    fn verify_openings(
        &self,
        proven: &CommittedProven,
        challenger: &mut pcs::Challenger,
    ) -> Result<(Ext, Ext)> {
        match self {
            Self::Stacked(setup) => {
                anyhow::ensure!(proven.commitments.len() == 1 && proven.openings.len() == 1);
                pcs::verify_open(
                    setup,
                    &proven.commitments[0],
                    &proven.openings[0],
                    &proven.a_point,
                    &proven.b_point,
                    challenger,
                )
            }
            Self::Split { a, b } => {
                anyhow::ensure!(proven.commitments.len() == 2 && proven.openings.len() == 2);
                let a_bound = pcs::verify_open_single(
                    a,
                    &proven.commitments[0],
                    &proven.openings[0],
                    &proven.a_point,
                    challenger,
                )?;
                let b_bound = pcs::verify_open_single(
                    b,
                    &proven.commitments[1],
                    &proven.openings[1],
                    &proven.b_point,
                    challenger,
                )?;
                Ok((a_bound, b_bound))
            }
        }
    }

    /// The full verifier: absorb the commitment(s), replay the sumcheck, verify the opening(s),
    /// and check that what the commitments bind is what the sumcheck closed on.
    ///
    /// The transcript is supplied by the caller so that building it — which regenerates the
    /// Poseidon2 constants and absorbs the domain separator — stays OUTSIDE the measured
    /// bracket, exactly where the 2026-09-03 campaign had it.
    pub fn verify(
        &self,
        st: &Statement<KoalaBearPair>,
        proven: &CommittedProven,
        challenger: &mut pcs::Challenger,
    ) -> Result<()> {
        self.observe_commitments(proven, challenger);
        matmul::verify::<KoalaBearPair>(st, &proven.proof, challenger)?;
        let (a_bound, b_bound) = self.verify_openings(proven, challenger)?;
        // The commitment is only doing work if what it opens is what the sumcheck closed on.
        anyhow::ensure!(
            a_bound == proven.proof.a_open && b_bound == proven.proof.b_open,
            "the committed openings differ from the values the sumcheck closed on"
        );
        Ok(())
    }

    /// The same verifier, reporting each half separately instead of stopping at the first
    /// failure. For the negative control, which has to distinguish "the sumcheck caught it"
    /// from "the commitment caught it".
    pub fn verify_parts(
        &self,
        st: &Statement<KoalaBearPair>,
        proven: &CommittedProven,
    ) -> CommittedVerdict {
        let mut challenger = self.challenger();
        self.observe_commitments(proven, &mut challenger);
        let sumcheck_ok =
            matmul::verify::<KoalaBearPair>(st, &proven.proof, &mut challenger).is_ok();
        let bound = self.verify_openings(proven, &mut challenger).ok();
        CommittedVerdict {
            sumcheck_ok,
            opening_ok: bound.is_some(),
            bound_matches: bound
                .is_some_and(|(a, b)| a == proven.proof.a_open && b == proven.proof.b_open),
        }
    }

    /// Serialised proof size, in bytes.
    ///
    /// **The two shapes do not use the same accounting, deliberately.** `Stacked` reports the
    /// sumcheck transcript plus the WHIR opening body and OMITS the Merkle root, which is what
    /// the 2026-09-03 campaign published (`systems/plonky3/RESULTS.md`, the declared omission);
    /// changing it would silently move rows `…-n5` and `…-n6`. `Split` reports the sumcheck
    /// transcript plus BOTH opening bodies plus BOTH Merkle roots, because a route whose whole
    /// point is that it carries two commitments may not hide the second root. The per-root cost
    /// is published beside the cell as `whir_root_bytes` so either accounting can be recovered.
    pub fn proof_bytes(&self, proven: &CommittedProven) -> Result<usize> {
        let body: usize = proven
            .openings
            .iter()
            .map(pcs::proof_bytes)
            .sum::<Result<usize>>()?;
        let roots = match self {
            Self::Stacked(_) => 0,
            Self::Split { .. } => self.commitment_bytes(proven)?,
        };
        Ok(matmul::proof_bytes(&proven.proof)? + body + roots)
    }

    /// Serialised size of every Merkle root this route puts on the wire.
    pub fn commitment_bytes(&self, proven: &CommittedProven) -> Result<usize> {
        proven
            .commitments
            .iter()
            .map(pcs::commitment_bytes)
            .sum::<Result<usize>>()
    }
}

/// Everything one committed-route campaign produces: the measured rows and the structural
/// facts about the commitment(s) that produced them.
pub struct CommittedRun {
    pub rows: Vec<Rep>,
    pub setup_nanos: u128,
    pub setup: WhirSetup,
    /// Serialised bytes of every Merkle root this route puts on the wire. Included in
    /// `proof_bytes` for `sumcheck-whir-split`, excluded for `sumcheck-whir`; published here
    /// either way so both accountings are recoverable from the cell.
    pub root_bytes: usize,
}

/// Runs `reps` measured repetitions of a committed route. Prime field only.
pub fn run_committed(
    route: Route,
    st: &Statement<KoalaBearPair>,
    warmup: usize,
    reps: usize,
) -> Result<CommittedRun> {
    use std::time::Instant;

    let started = Instant::now();
    let setup = WhirSetup::build(route, st)?;
    let setup_nanos = started.elapsed().as_nanos();

    for _ in 0..warmup {
        one_committed(st, &setup)?;
    }
    let rows = (0..reps)
        .map(|_| one_committed(st, &setup))
        .collect::<Result<Vec<_>>>()?;

    // Measured once, outside every timed bracket, from a proof this cell actually produced.
    let mut ch = setup.challenger();
    let root_bytes = setup.commitment_bytes(&setup.prove(st, &mut ch))?;

    Ok(CommittedRun {
        rows,
        setup_nanos,
        setup,
        root_bytes,
    })
}

fn one_committed(st: &Statement<KoalaBearPair>, setup: &WhirSetup) -> Result<Rep> {
    use std::time::Instant;

    let mut prover_ch = setup.challenger();
    let started = Instant::now();
    let proven = setup.prove(st, &mut prover_ch);
    let prove_nanos = started.elapsed().as_nanos();

    let proof_bytes = setup.proof_bytes(&proven)?;

    let mut verifier_ch = setup.challenger();
    let started = Instant::now();
    setup.verify(st, &proven, &mut verifier_ch)?;
    let verify_nanos = started.elapsed().as_nanos();

    Ok(Rep {
        prove_nanos,
        verify_nanos,
        proof_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mle::{eq_table, eval_base};
    use crate::tasks::{Instance, Task};

    /// `C~(r1, r2)` recomputed from the PUBLIC output alone — the verifier's own quantity,
    /// written here a second time so the assertion below does not check `prove` against itself.
    fn claim_from_public_output(
        st: &Statement<KoalaBearPair>,
        r1: &[Ext],
        r2: &[Ext],
    ) -> Ext {
        use p3_field::PrimeCharacteristicRing;
        let (mp, np) = (1usize << st.log_m, 1usize << st.log_n);
        let eq1 = eq_table(r1);
        let eq2 = eq_table(r2);
        let mut claim = Ext::ZERO;
        for i in 0..mp {
            for j in 0..np {
                claim += eq1[i] * eq2[j] * st.c[i * np + j];
            }
        }
        claim
    }

    /// Runs one committed route end to end and checks everything that must hold of it
    /// regardless of how many commitments it used.
    fn exercise(route: Route, st: &Statement<KoalaBearPair>) -> CommittedProven {
        let setup = WhirSetup::build(route, st).expect("configures");
        let mut prover_ch = setup.challenger();
        let proven = setup.prove(st, &mut prover_ch);

        let mut verifier_ch = setup.challenger();
        setup
            .verify(st, &proven, &mut verifier_ch)
            .unwrap_or_else(|e| panic!("{} did not verify: {e}", route.name()));

        // The claim the sumcheck started from IS C~(r1, r2) for this route's own challenges.
        let r1 = &proven.a_point[..st.log_m];
        let r2 = &proven.b_point[st.log_k..];
        assert_eq!(
            proven.claimed,
            claim_from_public_output(st, r1, r2),
            "{}: the sumcheck did not start from the public output's own evaluation",
            route.name()
        );

        // The closing evaluations are the operands' multilinears at the opened points — and
        // `verify` has already checked that these are the values the commitment binds.
        assert_eq!(
            proven.proof.a_open,
            eval_base::<BaseF, Ext>(&st.a, &proven.a_point),
            "{}: A~ at the opened point",
            route.name()
        );
        assert_eq!(
            proven.proof.b_open,
            eval_base::<BaseF, Ext>(&st.b, &proven.b_point),
            "{}: B~ at the opened point",
            route.name()
        );
        proven
    }

    /// The two committed routes prove the SAME statement, and the split one commits exactly the
    /// operands.
    ///
    /// # What is asserted, and what cannot be
    ///
    /// The two routes' `claimed` field elements are **not** equal, and no correct
    /// implementation could make them so: the transcripts differ by construction — one Merkle
    /// root against two, one WHIR domain separator against two — so `(r1, r2)` differ and the
    /// claim is evaluated at a different point on each route. What IS asserted is the property
    /// that "same claimed" was standing in for: on each route the claim equals `C~(r1, r2)` for
    /// that route's own challenges, recomputed from the public output by a second
    /// implementation, and the two routes close on the same two multilinears of the same `A`
    /// and `B`. Both verify.
    fn routes_agree_on(task: Task) {
        let inst = Instance::draw(task).expect("draws");
        let st = matmul::embed::<KoalaBearPair>(&inst).expect("embeds");

        let stacked = exercise(Route::SumcheckWhir, &st);
        let split = exercise(Route::SumcheckWhirSplit, &st);

        // Same statement: same public output, same operands, same number of sumcheck rounds.
        assert_eq!(stacked.proof.sumcheck.polynomial_evaluations.len(), st.log_k);
        assert_eq!(split.proof.sumcheck.polynomial_evaluations.len(), st.log_k);
        assert_eq!(stacked.a_point.len(), st.log_m + st.log_k);
        assert_eq!(split.a_point.len(), st.log_m + st.log_k);
        assert_eq!(stacked.b_point.len(), st.log_k + st.log_n);
        assert_eq!(split.b_point.len(), st.log_k + st.log_n);

        // One commitment against two.
        assert_eq!(stacked.commitments.len(), 1);
        assert_eq!(split.commitments.len(), 2);
    }

    #[test]
    fn t1_0_both_committed_routes_prove_the_same_statement() {
        routes_agree_on(Task::T1_0);
    }

    #[test]
    fn t1_a_both_committed_routes_prove_the_same_statement() {
        routes_agree_on(Task::T1A);
    }

    /// The point of `G-13b''`, as an equality rather than a hope: the split route commits
    /// `2^a_vars + 2^b_vars` field elements — the operands and nothing else — while the stacked
    /// route rounds their SUM up to the next power of two.
    fn committed_elements_of(task: Task) {
        let inst = Instance::draw(task).expect("draws");
        let st = matmul::embed::<KoalaBearPair>(&inst).expect("embeds");
        let (a_vars, b_vars) = (st.log_m + st.log_k, st.log_k + st.log_n);
        let operands = (1usize << a_vars) + (1usize << b_vars);
        assert_eq!(operands, st.a.len() + st.b.len());

        let split = WhirSetup::build(Route::SumcheckWhirSplit, &st).expect("configures");
        assert_eq!(split.whir_vars(), vec![a_vars, b_vars]);
        assert_eq!(
            split.committed_elements(),
            operands,
            "the split route must commit the operands and nothing else"
        );

        let stacked = WhirSetup::build(Route::SumcheckWhir, &st).expect("configures");
        assert_eq!(stacked.whir_vars(), vec![operands.next_power_of_two().trailing_zeros() as usize]);
        assert_eq!(
            stacked.committed_elements(),
            operands.next_power_of_two(),
            "the stacked route commits log2_ceil of the SUM"
        );
        assert!(
            stacked.committed_elements() > split.committed_elements(),
            "the stacked route is supposed to be the expensive one"
        );
    }

    #[test]
    fn t1_0_split_commits_exactly_the_operands() {
        committed_elements_of(Task::T1_0);
    }

    #[test]
    fn t1_a_split_commits_exactly_the_operands() {
        committed_elements_of(Task::T1A);
    }
}
