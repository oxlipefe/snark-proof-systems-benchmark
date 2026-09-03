//! A deliberate compile failure, kept so the absence is MEASURED rather than read.
//!
//! Rule 6 of this project's method: reading the source tells you WHERE to look, not what a
//! system costs — and its corollary for claims of absence is that a grep is not a proof. This
//! module asks the compiler instead. It is behind the `probe-binary-pcs` feature and is
//! expected to FAIL to build; `bench/scripts/plonky3/run-probe-binary-pcs.sh` builds it and
//! records rustc's diagnostics in `bench/data/probe-plonky3-whir-binary.txt`.
//!
//! Two instantiations are attempted, in increasing order of ambition:
//!
//! 1. [`config_over_binary128`] — just the WHIR configuration, over `BinaryField128` as both
//!    base and extension field.
//! 2. [`pcs_over_binary128`] — the `MultilinearPcs` impl itself.

use p3_binary_field::{BinaryChallenger, BinaryField128};
use p3_challenger::HashChallenger;
use p3_dft::Radix2DFTSmallBatch;
use p3_keccak::Keccak256Hash;
use p3_merkle_tree::MerkleTreeMmcs;
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher};
use p3_whir::parameters::{FoldingFactor, ProtocolParameters, SecurityAssumption, WhirConfig};
use p3_whir::pcs::prover::WhirProver;
use p3_sumcheck::layout::SuffixProver;

type F = BinaryField128;
type Challenger = BinaryChallenger<F, HashChallenger<u8, Keccak256Hash, 32>>;
type Hash = SerializingHasher<Keccak256Hash>;
type Compress = CompressionFunctionFromHasher<Keccak256Hash, 2, 32>;
type Mmcs = MerkleTreeMmcs<F, u8, Hash, Compress, 2, 32>;

/// Expected: `the trait bound `BinaryField128: TwoAdicField` is not satisfied`, pointing at
/// `whir/src/parameters/whir.rs:203-207`.
pub fn config_over_binary128() {
    let params = ProtocolParameters {
        security_level: 96,
        pow_bits: 16,
        folding_factor: FoldingFactor::Constant(4),
        soundness_type: SecurityAssumption::CapacityBound,
        starting_log_inv_rate: 1,
        round_log_inv_rates: vec![4, 7, 10],
    };
    let _ = WhirConfig::<F, F, Challenger>::new(17, params);
}

/// Expected: the same bound, now from `whir/src/pcs/adapter.rs:64-66`, plus the missing
/// `TwoAdicSubgroupDft<BinaryField128>`.
pub fn pcs_over_binary128() {
    type Dft = Radix2DFTSmallBatch<F>;
    type Pcs = WhirProver<F, F, Dft, Mmcs, Challenger, SuffixProver<F, F>>;
    let _ = core::mem::size_of::<Pcs>();
}
