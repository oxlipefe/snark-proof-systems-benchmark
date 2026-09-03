//! Multilinear extensions, with ONE index convention, declared and tested.
//!
//! # The convention, and why it is ours rather than inherited
//!
//! A multilinear extension is indexed by a hypercube, and every bug in this kind of code is a
//! disagreement about which coordinate of the point is which bit of the index. Plonky3's
//! [`p3_multilinear_util::poly::Poly::fix_prefix_var_mut`] folds `p[i]` against `p[i + mid]`,
//! so under `VariableOrder::Prefix` the first bound variable is the **most significant** bit
//! of the index. Everything here uses that convention:
//!
//! ```text
//!     index = x_0 * 2^(n-1) + x_1 * 2^(n-2) + ... + x_(n-1)
//! ```
//!
//! so a row-major flattening `A[i][k] -> a[i * kp + k]` gives the variable split
//! `(i-variables, k-variables)` in exactly that order, and `A~(r1, r3)` is
//! [`eval`] at `r1` followed by `r3`. The invariant tying [`eq_table`] to [`eval`] is a test,
//! not a comment.

use p3_field::{ExtensionField, Field};

/// `eq[x] = prod_i eqbit(point_i, x_i)` over the hypercube, in the convention above.
///
/// `point[0]` is the most significant index bit.
pub fn eq_table<EF: Field>(point: &[EF]) -> Vec<EF> {
    let mut eq = vec![EF::ONE];
    for &r in point {
        let mut next = Vec::with_capacity(eq.len() * 2);
        for &e in &eq {
            let hi = e * r;
            next.push(e - hi); // x_i = 0  ->  (1 - r)
            next.push(hi); // x_i = 1  ->  r
        }
        eq = next;
    }
    eq
}

/// `f~(point)` for `f` given by its `2^n` hypercube evaluations, base-field values.
///
/// Folds the most significant variable first, mirroring `fix_prefix_var_mut`.
pub fn eval_base<F: Field, EF: ExtensionField<F>>(f: &[F], point: &[EF]) -> EF {
    assert_eq!(f.len(), 1 << point.len(), "arity does not match the table");
    if point.is_empty() {
        return EF::from(f[0]);
    }
    // First fold lifts the base-field table into the extension exactly once.
    let mid = f.len() / 2;
    let r = point[0];
    let mut cur: Vec<EF> = (0..mid)
        .map(|i| r * (f[i + mid] - f[i]) + f[i])
        .collect();
    for &r in &point[1..] {
        let mid = cur.len() / 2;
        for i in 0..mid {
            cur[i] = cur[i] + (cur[i + mid] - cur[i]) * r;
        }
        cur.truncate(mid);
    }
    cur[0]
}

/// `f~(point)` for `f` already in the extension field.
pub fn eval_ext<EF: Field>(f: &[EF], point: &[EF]) -> EF {
    assert_eq!(f.len(), 1 << point.len(), "arity does not match the table");
    let mut cur = f.to_vec();
    for &r in point {
        let mid = cur.len() / 2;
        for i in 0..mid {
            cur[i] = cur[i] + (cur[i + mid] - cur[i]) * r;
        }
        cur.truncate(mid);
    }
    cur[0]
}

#[cfg(test)]
mod tests {
    use p3_field::extension::BinomialExtensionField;
    use p3_koala_bear::KoalaBear;
    use p3_multilinear_util::point::Point;
    use p3_multilinear_util::poly::Poly;
    use rand::rngs::SmallRng;
    use rand::{RngExt, SeedableRng};

    use super::*;

    type F = KoalaBear;
    type EF = BinomialExtensionField<KoalaBear, 4>;

    /// The invariant that makes `eq_table` and `eval` one convention rather than two:
    /// contracting a table against `eq(point)` must equal evaluating it at `point`.
    #[test]
    fn eq_table_contracts_to_the_same_value_eval_gives() {
        let mut rng = SmallRng::seed_from_u64(7);
        for n in 0..8 {
            let f: Vec<F> = (0..1usize << n).map(|_| rng.random()).collect();
            let point: Vec<EF> = (0..n).map(|_| rng.random()).collect();
            let eq = eq_table(&point);
            let contracted: EF = eq
                .iter()
                .zip(&f)
                .map(|(&e, &v)| e * v)
                .sum();
            assert_eq!(contracted, eval_base(&f, &point), "n = {n}");
        }
    }

    /// And our convention must be Plonky3's, or every opened point is off by a bit reversal.
    #[test]
    fn our_convention_is_plonky3s() {
        let mut rng = SmallRng::seed_from_u64(11);
        for n in 1..8 {
            let f: Vec<F> = (0..1usize << n).map(|_| rng.random()).collect();
            let point: Vec<EF> = (0..n).map(|_| rng.random()).collect();
            let theirs = Poly::new(f.clone()).eval_base(&Point::new(point.clone()));
            assert_eq!(theirs, eval_base(&f, &point), "n = {n}");
        }
    }

    #[test]
    fn eval_ext_agrees_with_eval_base_on_lifted_tables() {
        let mut rng = SmallRng::seed_from_u64(13);
        for n in 0..7 {
            let f: Vec<F> = (0..1usize << n).map(|_| rng.random()).collect();
            let lifted: Vec<EF> = f.iter().map(|&v| EF::from(v)).collect();
            let point: Vec<EF> = (0..n).map(|_| rng.random()).collect();
            assert_eq!(eval_base(&f, &point), eval_ext(&lifted, &point));
        }
    }
}
