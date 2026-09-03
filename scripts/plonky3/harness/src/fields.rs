//! The two fields of the same codebase, and the one property that separates them.
//!
//! # The asymmetry that must be read before any number
//!
//! `bench/TASKS.md` fixes an INTEGER matmul: signed INT8 operands, an INT32 accumulator, no
//! requantisation. A 31-bit prime field carries that arithmetic faithfully, because
//! `|acc| <= K * 128 * 128 < 2^24 < p`, so the field product IS the integer product.
//!
//! **`GF(2^128)` does not.** It has characteristic 2: `-1 == 1`, there is no order, and the
//! sum of two embedded bytes is their XOR, not their integer sum. Embedding the INT8 operands
//! in the binary tower and multiplying gives a well-defined bilinear form of exactly the same
//! SHAPE — same number of field multiplies, same multilinear structure, same sumcheck — but
//! it is **a different statement**. It is not the task's matmul.
//!
//! That is why [`FieldPair::INTEGER_FAITHFUL`] exists and why every cell carries it. The
//! binary cell measures the cost of the same protocol over a different substrate; it does not
//! prove T1. Stating that anywhere except in the same line as the number would be the failure
//! `bench/RESULTS.md` §7 exists to prevent.

use p3_binary_field::{BinaryChallenger, BinaryField8, BinaryField128};
use p3_challenger::{DuplexChallenger, FieldChallenger, GrindingChallenger, HashChallenger};
use p3_field::extension::BinomialExtensionField;
use p3_field::{ExtensionField, Field, PrimeCharacteristicRing};
use p3_keccak::Keccak256Hash;
use p3_koala_bear::{KoalaBear, Poseidon2KoalaBear};
use rand::{SeedableRng, rngs::SmallRng};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A (base field, extension field, challenger) triple from one codebase.
pub trait FieldPair {
    type F: Field + Serialize + DeserializeOwned;
    type EF: ExtensionField<Self::F> + Serialize + DeserializeOwned;
    type Challenger: FieldChallenger<Self::F>
        + GrindingChallenger<Witness = Self::F>
        + Clone;

    /// The name this cell is published under.
    const NAME: &'static str;

    /// Does the field carry the task's INTEGER arithmetic? See the module doc.
    const INTEGER_FAITHFUL: bool;

    /// `log2 |EF|`, the soundness denominator of every sumcheck round.
    const CHALLENGE_BITS: usize;

    /// One line naming the Fiat-Shamir primitive, recorded next to every cell.
    const CHALLENGER: &'static str;

    /// Embeds one signed INT8 operand.
    ///
    /// For a prime field this is the integer. For the binary tower it is the byte's image in
    /// `GF(2^8) subset GF(2^128)`, which is an injection of the BIT PATTERN and carries no
    /// integer meaning: `embed(-1)` and `embed(255)` are the same element.
    fn embed_i8(v: i8) -> Self::F;

    /// Embeds the task's INT32 reference output, when the field carries integers.
    fn embed_i64(v: i64) -> Option<Self::F>;

    /// A fresh transcript, seeded deterministically so a rerun replays the same challenges.
    fn challenger() -> Self::Challenger;
}

// ---------------------------------------------------------------------------------------
// KoalaBear, a 31-bit prime — the two-adic side.
// ---------------------------------------------------------------------------------------

/// KoalaBear (`p = 2^31 - 2^24 + 1`) with its degree-4 binomial extension.
///
/// The extension is not decoration: sumcheck challenges must be sampled from a set large
/// enough that a cheating prover cannot guess one. In the base field alone that set has 2^31
/// elements and the protocol would be worthless.
#[derive(Debug, Clone, Copy)]
pub struct KoalaBearPair;

type Kb = KoalaBear;
type KbExt = BinomialExtensionField<KoalaBear, 4>;
type KbPerm = Poseidon2KoalaBear<16>;

impl FieldPair for KoalaBearPair {
    type F = Kb;
    type EF = KbExt;
    type Challenger = DuplexChallenger<Kb, KbPerm, 16, 8>;

    const NAME: &'static str = "koala-bear";
    const INTEGER_FAITHFUL: bool = true;
    const CHALLENGE_BITS: usize = 124;
    const CHALLENGER: &'static str = "DuplexChallenger<KoalaBear, Poseidon2KoalaBear<16>, 16, 8>";

    #[inline]
    fn embed_i8(v: i8) -> Self::F {
        if v >= 0 {
            Self::F::from_u8(v as u8)
        } else {
            -Self::F::from_u8(v.unsigned_abs())
        }
    }

    #[inline]
    fn embed_i64(v: i64) -> Option<Self::F> {
        Some(if v >= 0 {
            Self::F::from_u64(v as u64)
        } else {
            -Self::F::from_u64(v.unsigned_abs())
        })
    }

    fn challenger() -> Self::Challenger {
        let mut rng = SmallRng::seed_from_u64(0xE006_13B0);
        DuplexChallenger::new(KbPerm::new_from_rng_128(&mut rng))
    }
}

// ---------------------------------------------------------------------------------------
// BinaryField128 — the binary tower of the SAME codebase.
// ---------------------------------------------------------------------------------------

/// `GF(2^128)`, the top of `p3-binary-field`'s Wiedemann tower.
///
/// `F == EF`: the field already carries 128 bits, so there is no extension to sample from and
/// `ExtensionField<F> for F` (Plonky3's blanket impl, `field/src/field.rs:1275`) applies.
#[derive(Debug, Clone, Copy)]
pub struct Binary128Pair;

impl FieldPair for Binary128Pair {
    type F = BinaryField128;
    type EF = BinaryField128;
    type Challenger = BinaryChallenger<BinaryField128, HashChallenger<u8, Keccak256Hash, 32>>;

    const NAME: &'static str = "binary128";
    const INTEGER_FAITHFUL: bool = false;
    const CHALLENGE_BITS: usize = 128;
    const CHALLENGER: &'static str =
        "BinaryChallenger<BinaryField128, HashChallenger<u8, Keccak256Hash, 32>>";

    #[inline]
    fn embed_i8(v: i8) -> Self::F {
        // The byte's bit pattern, read as an element of GF(2^8) and lifted into GF(2^128).
        // Sign is not preserved because it cannot be: characteristic 2 has no order.
        BinaryField128::from(BinaryField8::from_u8(v as u8))
    }

    #[inline]
    fn embed_i64(_v: i64) -> Option<Self::F> {
        // Deliberately absent. The reference INT32 output has no image in GF(2^128) that
        // makes the field product equal the integer product; the binary cell's public output
        // is computed IN THE FIELD, and that is exactly what makes it a different statement.
        None
    }

    fn challenger() -> Self::Challenger {
        BinaryChallenger::from_hasher(b"zk-prover-bench/plonky3/G-13b".to_vec(), Keccak256Hash)
    }
}
