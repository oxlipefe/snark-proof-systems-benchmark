//! Route 1's second half: binding `A` and `B` to a commitment with `p3-whir`.
//!
//! # Why this module is not generic over the field
//!
//! Everything in [`crate::matmul`] is generic over [`crate::fields::FieldPair`] and runs over
//! both fields. This module is written for the prime field alone, and that is the campaign's
//! result rather than a shortcut. At the pinned commit:
//!
//! * `p3-whir` is the **only** implementor of `p3_commit::MultilinearPcs` in the whole tree
//!   (`whir/src/pcs/adapter.rs:61` and its ZK twin `whir/src/pcs/zk/adapter.rs:92`);
//! * that impl requires `F: TwoAdicField + Ord`, `EF: ExtensionField<F> + TwoAdicField` and
//!   `Dft: TwoAdicSubgroupDft<F>` (`whir/src/pcs/adapter.rs:64-66`), and the same bounds gate
//!   `WhirConfig::new` (`whir/src/parameters/whir.rs:203-207`) and `PrescribedPointPcs`
//!   (`whir/src/pcs/adapter.rs:200-206`);
//! * `p3-binary-field` implements `TwoAdicField` nowhere, and cannot: the multiplicative group
//!   of `GF(2^128)` has order `2^128 - 1`, which is odd, so its two-adicity is zero and there
//!   is no subgroup of order `2^k` for any `k >= 1` to run a Reed-Solomon domain on.
//!
//! The binary side is not simply missing. It reaches **exactly** the commitment and stops:
//! `p3_sumcheck::commit::commit_base` and `Layout::commit` are generic over
//! `p3_commit::Encoder<F>` (`sumcheck/src/commit.rs:26-39`, `sumcheck/src/layout/prover/mod.rs:50-61`),
//! `p3-binary-dft` implements that `Encoder` for `BinaryField128`
//! (`binary-dft/src/encoder.rs:34-36`), and upstream's own test for it is titled
//! *"Phase 2 exit criterion: the multilinear commit path runs over a binary tower field"*
//! (`binary-dft/tests/commit.rs:1`). Commit yes; open no.

use anyhow::Result;
use p3_challenger::DuplexChallenger;
use p3_commit::MultilinearPcs;
use p3_dft::Radix2DFTSmallBatch;
use p3_field::{Field, PrimeCharacteristicRing};
use p3_field::extension::BinomialExtensionField;
use p3_koala_bear::{KoalaBear, Poseidon2KoalaBear};
use p3_matrix::dense::RowMajorMatrix;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_multilinear_util::point::Point;
use p3_sumcheck::layout::{Layout as _, SuffixProver, Table};
use p3_sumcheck::{OpeningBatch, OpeningProtocol, PointSchedule, PrescribedPointPcs, TableShape, TableSpec};
use p3_symmetric::{PaddingFreeSponge, TruncatedPermutation};
use p3_whir::fiat_shamir::domain_separator::DomainSeparator;
use p3_whir::parameters::{FoldingFactor, ProtocolParameters, SecurityAssumption, WhirConfig};
use p3_whir::pcs::prover::WhirProver;
use rand::SeedableRng;
use rand::rngs::SmallRng;

type F = KoalaBear;
type EF = BinomialExtensionField<KoalaBear, 4>;
type Poseidon16 = Poseidon2KoalaBear<16>;
type Poseidon24 = Poseidon2KoalaBear<24>;
type MerkleHash = PaddingFreeSponge<Poseidon24, 24, 16, 8>;
type MerkleCompress = TruncatedPermutation<Poseidon16, 2, 8, 16>;
pub type Challenger = DuplexChallenger<F, Poseidon16, 16, 8>;
type PackedF = <F as Field>::Packing;
type Mmcs = MerkleTreeMmcs<PackedF, PackedF, MerkleHash, MerkleCompress, 2, 8>;
type Dft = Radix2DFTSmallBatch<F>;
type LayoutMode = SuffixProver<F, EF>;
pub type Pcs = WhirProver<EF, F, Dft, Mmcs, Challenger, LayoutMode>;
pub type Commitment = <Pcs as MultilinearPcs<EF, Challenger>>::Commitment;
pub type PcsProof = <Pcs as MultilinearPcs<EF, Challenger>>::Proof;
type ProverData = <Pcs as MultilinearPcs<EF, Challenger>>::ProverData;

/// Target security, in bits. Declared per cell; never averaged with another system's.
pub const SECURITY_LEVEL: usize = 96;

/// Variables consumed by the first WHIR round.
pub const FOLDING_FACTOR: usize = 4;

/// `log_2(1/rho)` of the initial Reed-Solomon code. `1` is the rate binius64's primary cut uses.
pub const STARTING_LOG_INV_RATE: usize = 1;

/// Proof-of-work grinding budget, in bits: Plonky3's own `DEFAULT_MAX_POW`.
///
/// **Zero is not available, and that is a measured fact, not a preference.** At 96-bit
/// security, rate 1 and folding 4 on this instance, `WhirConfig::new` rejects a budget of zero
/// with `PowBitsExceedBudget { required: 7, budget: 0 }`; at folding 5 it asks for 11. The
/// sumcheck-only route runs with no grinding at all, so **the two routes do not carry the same
/// Fiat-Shamir cost** and the difference between them is not purely the commitment.
pub const POW_BITS: usize = 7;

/// The soundness assumption. G-13b' (2026-09-03): `UniqueDecoding` — no conjectures — because
/// binius64's 232 FRI queries at rate 1 for 96 bits are exactly the unique-decoding count, and
/// `CapacityBound` (Plonky3's WHIR-example default, used for the 2026-09-03 CAMPAIGN rows)
/// rests on the mutual-correlated-agreement-up-to-capacity conjecture that ePrint 2025/2046
/// disproves. POW_BITS lowered from 16 to the minimum `WhirConfig::new` accepts at folding 4 (7),
/// because WHIR subtracts the PoW budget from the security level before deriving queries.
pub const SOUNDNESS: SecurityAssumption = SecurityAssumption::UniqueDecoding;

/// Everything the two sides need to agree on, built once per cell.
pub struct Setup {
    pub pcs: Pcs,
    pub protocol: OpeningProtocol,
    /// Variables in the stacked committed polynomial.
    pub num_variables: usize,
    /// STIR queries in the final proximity test, at this configuration.
    pub final_queries: usize,
}

/// Builds the commitment scheme and the two-table opening protocol for one instance shape.
pub fn setup(a_vars: usize, b_vars: usize) -> Result<(Setup, Vec<Table<F>>)> {
    // Deterministic permutation constants, so a rerun commits the same way.
    let mut rng = SmallRng::seed_from_u64(0xE006_13B1);
    let poseidon16 = Poseidon16::new_from_rng_128(&mut rng);
    let poseidon24 = Poseidon24::new_from_rng_128(&mut rng);
    let mmcs = Mmcs::new(
        MerkleHash::new(poseidon24),
        MerkleCompress::new(poseidon16),
        0,
    );

    // Two tables, one column each: A and B. They are stacked into ONE commitment, which is
    // what `p3-sumcheck`'s layout exists for; committing them separately would pay the FFT
    // and Merkle overhead twice.
    let tables = vec![
        Table::new(RowMajorMatrix::new(vec![F::ZERO; 1 << a_vars], 1 << a_vars)),
        Table::new(RowMajorMatrix::new(vec![F::ZERO; 1 << b_vars], 1 << b_vars)),
    ];
    let witness = LayoutMode::new_witness(tables.clone(), FOLDING_FACTOR);
    let num_variables = witness.num_variables();

    let one_opening: PointSchedule = vec![OpeningBatch::new(vec![0], Vec::new())];
    let protocol = OpeningProtocol::new(vec![
        TableSpec::new(TableShape::new(a_vars, 1), one_opening.clone()),
        TableSpec::new(TableShape::new(b_vars, 1), one_opening),
    ])
    .pad_to_min_num_variables(FOLDING_FACTOR);

    let folding = FoldingFactor::Constant(FOLDING_FACTOR);
    let (num_rounds, _) = folding
        .compute_number_of_rounds(num_variables)
        .map_err(|e| anyhow::anyhow!("folding schedule invalid: {e:?}"))?;
    let mut round_log_inv_rates = Vec::with_capacity(num_rounds);
    let mut rate = STARTING_LOG_INV_RATE;
    for round in 0..num_rounds {
        rate += folding.at_round(round) - 1;
        round_log_inv_rates.push(rate);
    }

    let params = ProtocolParameters {
        security_level: SECURITY_LEVEL,
        pow_bits: POW_BITS,
        folding_factor: folding,
        soundness_type: SOUNDNESS,
        starting_log_inv_rate: STARTING_LOG_INV_RATE,
        round_log_inv_rates,
    };
    let config = WhirConfig::<EF, F, Challenger>::new(num_variables, params)
        .map_err(|e| anyhow::anyhow!("WhirConfig rejected the parameters: {e:?}"))?;
    let final_queries = config.final_queries;
    let dft = Dft::new(1 << config.max_fft_size());
    let pcs = Pcs::new(config, dft, mmcs);

    Ok((
        Setup {
            pcs,
            protocol,
            num_variables,
            final_queries,
        },
        vec![],
    ))
}

/// A fresh transcript with the scheme's domain separator already absorbed.
pub fn challenger(setup: &Setup) -> Challenger {
    let mut rng = SmallRng::seed_from_u64(0xE006_13B1);
    let poseidon16 = Poseidon16::new_from_rng_128(&mut rng);
    let mut challenger = DuplexChallenger::new(poseidon16);
    let mut domainsep = DomainSeparator::new(vec![]);
    setup.pcs.add_domain_separator::<8>(&mut domainsep);
    domainsep.observe_domain_separator(&mut challenger);
    challenger
}

/// Commits `A` and `B` as one stacked polynomial. Absorbs the commitment into `challenger`.
pub fn commit(
    setup: &Setup,
    a: &[F],
    b: &[F],
    challenger: &mut Challenger,
) -> (Commitment, ProverData) {
    let tables = vec![
        Table::new(RowMajorMatrix::new(a.to_vec(), a.len())),
        Table::new(RowMajorMatrix::new(b.to_vec(), b.len())),
    ];
    let witness = LayoutMode::new_witness(tables, FOLDING_FACTOR);
    <Pcs as MultilinearPcs<EF, Challenger>>::commit(&setup.pcs, witness, challenger)
}

/// Opens the two committed columns at the prescribed points the sumcheck produced.
pub fn open(
    setup: &Setup,
    data: ProverData,
    a_point: &[EF],
    b_point: &[EF],
    challenger: &mut Challenger,
) -> PcsProof {
    let points = [
        Point::new(a_point.to_vec()),
        Point::new(b_point.to_vec()),
    ];
    setup
        .pcs
        .open_at(data, &setup.protocol, &points, challenger)
}

/// Verifies the opening and returns `(A~(a_point), B~(b_point))` as the commitment binds them.
pub fn verify_open(
    setup: &Setup,
    commitment: &Commitment,
    proof: &PcsProof,
    a_point: &[EF],
    b_point: &[EF],
    challenger: &mut Challenger,
) -> Result<(EF, EF)> {
    let points = [
        Point::new(a_point.to_vec()),
        Point::new(b_point.to_vec()),
    ];
    let evals = setup
        .pcs
        .verify_at(commitment, proof, &setup.protocol, &points, challenger)
        .map_err(|e| anyhow::anyhow!("WHIR rejected the opening: {e:?}"))?;
    anyhow::ensure!(evals.len() == 2, "expected two opening batches, got {}", evals.len());
    let a = *evals[0]
        .current()
        .first()
        .ok_or_else(|| anyhow::anyhow!("no value opened for A"))?;
    let b = *evals[1]
        .current()
        .first()
        .ok_or_else(|| anyhow::anyhow!("no value opened for B"))?;
    Ok((a, b))
}

/// Serialised size of the PCS opening proof, in bytes.
pub fn proof_bytes(proof: &PcsProof) -> Result<usize> {
    Ok(postcard::to_allocvec(proof)
        .map_err(|e| anyhow::anyhow!("serialising the WHIR proof: {e}"))?
        .len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::KoalaBearPair;
    use crate::matmul;
    use crate::mle::eval_base;
    use crate::tasks::{Instance, Task};

    /// The whole of route 1 on the smallest rung: commit, sumcheck, open, verify.
    ///
    /// This is also the test that pins the OPENING POINT CONVENTION. `verify_open` returns the
    /// values the commitment binds; they are compared against our own multilinear evaluation
    /// at the same point. A bit-reversal or a prefix/suffix disagreement between our indexing
    /// and the layout's would show up here as two different field elements, not as a passing
    /// test with a wrong meaning.
    #[test]
    fn t1_0_closes_on_a_commitment() {
        let inst = Instance::draw(Task::T1_0).expect("draws");
        let st = matmul::embed::<KoalaBearPair>(&inst).expect("embeds");
        let (setup, _) = setup(st.log_m + st.log_k, st.log_k + st.log_n).expect("configures");

        let mut prover_ch = challenger(&setup);
        let (commitment, data) = commit(&setup, &st.a, &st.b, &mut prover_ch);
        let proven = matmul::prove::<KoalaBearPair>(&st, &mut prover_ch);
        let pcs_proof = open(
            &setup,
            data,
            &proven.a_point,
            &proven.b_point,
            &mut prover_ch,
        );

        let mut verifier_ch = challenger(&setup);
        <Challenger as p3_challenger::CanObserve<Commitment>>::observe(
            &mut verifier_ch,
            commitment.clone(),
        );
        matmul::verify::<KoalaBearPair>(&st, &proven.proof, &mut verifier_ch)
            .expect("the sumcheck verifies");
        let (a_bound, b_bound) = verify_open(
            &setup,
            &commitment,
            &pcs_proof,
            &proven.a_point,
            &proven.b_point,
            &mut verifier_ch,
        )
        .expect("the opening verifies");

        assert_eq!(
            a_bound,
            eval_base::<F, EF>(&st.a, &proven.a_point),
            "the value the commitment binds for A is not A~ at the opened point"
        );
        assert_eq!(b_bound, eval_base::<F, EF>(&st.b, &proven.b_point));
        assert_eq!(a_bound, proven.proof.a_open);
        assert_eq!(b_bound, proven.proof.b_open);
    }
}
