//! T1 — the INT8 matrix-multiply ladder of `bench/TASKS.md`.
//!
//! `C = A[M x K] . B[K x N]`, signed INT8 operands, INT32 accumulator, output **not**
//! requantised. Each output element is one depth-`K` dot product: the first product seeds
//! the accumulator, so a dot product costs `K` IMUL constraints and `K - 1` accumulating
//! additions. Total IMUL = `M * K * N`, which is exactly the MAC count `bench/TASKS.md`
//! fixes, and [`super::finish`] refuses to return a circuit where it is not.
//!
//! Every output element is bound to its own `inout` wire, so no dot product can be removed
//! as dead code and the verifier sees the full INT32 output matrix.

use anyhow::{Context, Result};
use binius_core::word::Word;
use binius_frontend::{CircuitBuilder, Wire};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use super::{Built, Task, finish, word_of_i8};

/// The INT32 range the task's output must stay inside, per `bench/TASKS.md`.
const INT32_MIN: i128 = i32::MIN as i128;
const INT32_MAX: i128 = i32::MAX as i128;

pub fn build(task: Task) -> Result<Built> {
	let (m, k, n) = task
		.matmul_shape()
		.with_context(|| format!("{} is not a matmul task", task.name()))?;

	let mut rng = StdRng::seed_from_u64(task.seed());
	// Signed INT8 in [-128, 127], as bench/TASKS.md specifies.
	let a: Vec<Vec<i8>> = (0..m).map(|_| (0..k).map(|_| rng.random()).collect()).collect();
	let b: Vec<Vec<i8>> = (0..k).map(|_| (0..n).map(|_| rng.random()).collect()).collect();

	// Reference product, computed out of circuit in i128 so the range check cannot itself
	// overflow. This is also the source of the claimed outputs, so a wrong circuit shows up
	// as a witness-population failure rather than as a plausible number.
	let mut max_abs: i128 = 0;
	let mut c: Vec<Vec<i64>> = Vec::with_capacity(m);
	for row in &a {
		let mut out_row = Vec::with_capacity(n);
		for j in 0..n {
			let mut acc: i128 = 0;
			for (kk, &a_ik) in row.iter().enumerate() {
				acc += i128::from(a_ik) * i128::from(b[kk][j]);
				max_abs = max_abs.max(acc.abs());
			}
			anyhow::ensure!(
				(INT32_MIN..=INT32_MAX).contains(&acc),
				"{}: output element [{}][{j}] = {acc} left the INT32 range that \
				 bench/TASKS.md fixes for this task",
				task.name(),
				out_row.len()
			);
			out_row.push(acc as i64);
		}
		c.push(out_row);
	}

	let builder = CircuitBuilder::new();
	let a_wires: Vec<Vec<Wire>> = (0..m)
		.map(|_| (0..k).map(|_| builder.add_witness()).collect())
		.collect();
	let b_wires: Vec<Vec<Wire>> = (0..k)
		.map(|_| (0..n).map(|_| builder.add_witness()).collect())
		.collect();
	let out_wires: Vec<Vec<Wire>> = (0..m)
		.map(|_| (0..n).map(|_| builder.add_inout()).collect())
		.collect();

	for (i, out_row) in out_wires.iter().enumerate() {
		for (j, &out) in out_row.iter().enumerate() {
			let dot = dot_product(&builder, &a_wires[i], &b_wires, j, k);
			builder.assert_eq(format!("t1_out[{i}][{j}]"), dot, out);
		}
	}

	let circuit = builder.build();
	let mut filler = circuit.new_witness_filler();
	for (wire_row, value_row) in a_wires.iter().zip(&a) {
		for (&wire, &value) in wire_row.iter().zip(value_row) {
			filler[wire] = Word(word_of_i8(value));
		}
	}
	for (wire_row, value_row) in b_wires.iter().zip(&b) {
		for (&wire, &value) in wire_row.iter().zip(value_row) {
			filler[wire] = Word(word_of_i8(value));
		}
	}
	for (wire_row, value_row) in out_wires.iter().zip(&c) {
		for (&wire, &value) in wire_row.iter().zip(value_row) {
			filler[wire] = Word(value as u64);
		}
	}
	circuit
		.populate_wire_witness(&mut filler)
		.with_context(|| format!("{}: evaluating the circuit to fill the witness", task.name()))?;
	let witness = filler.into_value_vec();

	finish(task, circuit, witness, max_abs)
}

/// One depth-`k` signed dot product `sum_kk a[kk] * b[kk][col]`.
///
/// The low word of `imul` is the low 64 bits of the product of the two operands read as
/// unsigned, which for sign-extended two's-complement operands is exactly the low 64 bits of
/// the signed product. `|a * b| <= 128 * 128`, so it fits with 47 bits to spare and the high
/// word — the only part `smul` would correct — is discarded.
fn dot_product(
	builder: &CircuitBuilder,
	a_row: &[Wire],
	b_wires: &[Vec<Wire>],
	col: usize,
	k: usize,
) -> Wire {
	let product = |kk: usize| builder.imul(a_row[kk], b_wires[kk][col]).1;
	(1..k).fold(product(0), |acc, kk| builder.iadd(acc, product(kk)).0)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The MAC count of the smallest rung must be exactly what bench/TASKS.md publishes,
	/// and it must be the IMUL count — not a number computed from the shape after the fact.
	#[test]
	fn t1_0_emits_the_published_mac_count() {
		let built = build(Task::T1_0).expect("T1-0 builds");
		assert_eq!(built.n_imul_constraints, 65_536);
		assert_eq!(built.n_macs, 65_536);
		assert_eq!(built.n_inout_values, 256);
	}

	/// A signed dot product must actually be signed: with operands drawn from [-128, 127]
	/// the reference must produce negative outputs, or the encoding silently degraded to
	/// the unsigned one E-001 measured.
	#[test]
	fn the_instance_exercises_negative_values() {
		let mut rng = StdRng::seed_from_u64(Task::T1_0.seed());
		let a: Vec<i8> = (0..256).map(|_| rng.random()).collect();
		assert!(a.iter().any(|&v| v < 0), "no negative operand in the instance");
		assert!(a.iter().any(|&v| v > 0), "no positive operand in the instance");
	}
}
