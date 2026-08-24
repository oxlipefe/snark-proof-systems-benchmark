//! T2 and T3 — the complete 200-256-128-64-1 MLP of `bench/TASKS.md`.
//!
//! T2 proves one input. T3 proves a batch of 8 independent inputs **in a single proof**,
//! over the same weights, which are committed once and shared across the batch. Both are
//! whole models: every layer, every ReLU, and the fixed per-proof overhead are inside the
//! measured circuit.
//!
//! # Requantisation, declared
//!
//! `bench/TASKS.md` fixes INT8 weights and an INT8 input, states for T1 that the output is
//! "INT32, not requantised", and says nothing about requantisation *between* layers of T2.
//! This expression **does not requantise**: each layer's INT32/INT64 accumulator is fed
//! straight into the next layer, exactly as T1's rule reads when extended down the network.
//! The consequence is stated rather than hidden — the accumulator grows by roughly
//! `log2(fan_in) + 7` bits per layer, so the *worst case* over all INT8 inputs exceeds
//! `i64` at layer 4 (bound ~1.44e19 against `i64::MAX` = 9.22e18). The published instance
//! does not come near it, and [`build`] refuses to emit a circuit unless every accumulator
//! of the actual instance fits in `i64` with a factor of two to spare; the observed maximum
//! is recorded with every measurement as `max_abs_intermediate`.
//!
//! The alternative — requantising between layers with a fixed right shift — is a modelling
//! decision `bench/TASKS.md` did not make, and making it here would change the task rather
//! than express it.

use anyhow::{Context, Result};
use binius_core::word::Word;
use binius_frontend::{CircuitBuilder, Wire};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use super::{Built, Task, finish, relu, word_of_i8};

/// Layer widths: input, then one entry per layer output.
const WIDTHS: [usize; 5] = [200, 256, 128, 64, 1];

/// Safety factor demanded of the instance's largest accumulator against `i64::MAX`.
const HEADROOM: i128 = 2;

pub fn build(task: Task) -> Result<Built> {
	let batch = task
		.mlp_batch()
		.with_context(|| format!("{} is not an MLP task", task.name()))?;

	let mut rng = StdRng::seed_from_u64(task.seed());
	// Weights, one matrix per layer, shape [out][in]. Shared across the whole batch: this is
	// one model, and in T3 it is committed once for all 8 inputs.
	let weights: Vec<Vec<Vec<i8>>> = (0..WIDTHS.len() - 1)
		.map(|layer| {
			(0..WIDTHS[layer + 1])
				.map(|_| (0..WIDTHS[layer]).map(|_| rng.random()).collect())
				.collect()
		})
		.collect();
	let inputs: Vec<Vec<i8>> = (0..batch)
		.map(|_| (0..WIDTHS[0]).map(|_| rng.random()).collect())
		.collect();

	// Reference forward pass in i128, layer by layer, tracking the largest magnitude any
	// accumulator reaches. This is what the range check below judges, and it is the source
	// of the claimed outputs.
	let mut max_abs: i128 = 0;
	let mut outputs: Vec<i64> = Vec::with_capacity(batch);
	let mut activations: Vec<Vec<i64>> = Vec::with_capacity(batch);
	for input in &inputs {
		let mut act: Vec<i128> = input.iter().map(|&v| i128::from(v)).collect();
		for (layer, matrix) in weights.iter().enumerate() {
			let is_last = layer == weights.len() - 1;
			let mut next = Vec::with_capacity(matrix.len());
			for row in matrix {
				let mut acc: i128 = 0;
				for (i, &w) in row.iter().enumerate() {
					acc += act[i] * i128::from(w);
					max_abs = max_abs.max(acc.abs());
				}
				// ReLU after layers 1-3; the output layer is linear.
				next.push(if is_last { acc } else { acc.max(0) });
			}
			act = next;
		}
		debug_assert_eq!(act.len(), 1);
		outputs.push(act[0] as i64);
		activations.push(Vec::new());
	}
	let _ = &activations;

	anyhow::ensure!(
		max_abs.saturating_mul(HEADROOM) <= i128::from(i64::MAX),
		"{}: the instance's largest accumulator is {max_abs}, which leaves less than {HEADROOM}x \
		 headroom under i64::MAX = {}; the un-requantised expression is not exact for this \
		 instance and no number may be produced from it",
		task.name(),
		i64::MAX
	);

	let builder = CircuitBuilder::new();
	let weight_wires: Vec<Vec<Vec<Wire>>> = weights
		.iter()
		.map(|matrix| {
			matrix
				.iter()
				.map(|row| row.iter().map(|_| builder.add_witness()).collect())
				.collect()
		})
		.collect();
	let input_wires: Vec<Vec<Wire>> = (0..batch)
		.map(|_| (0..WIDTHS[0]).map(|_| builder.add_witness()).collect())
		.collect();
	// One public output per batch element: the model's scalar prediction. Nothing else about
	// the forward pass is revealed, which is the shape a real inference proof has.
	let out_wires: Vec<Wire> = (0..batch).map(|_| builder.add_inout()).collect();

	for (b, input_row) in input_wires.iter().enumerate() {
		let mut act: Vec<Wire> = input_row.clone();
		for (layer, matrix) in weight_wires.iter().enumerate() {
			let is_last = layer == weight_wires.len() - 1;
			act = matrix
				.iter()
				.map(|row| {
					let acc = dot_product(&builder, &act, row);
					if is_last { acc } else { relu(&builder, acc) }
				})
				.collect();
		}
		builder.assert_eq(format!("t2_out[{b}]"), act[0], out_wires[b]);
	}

	let circuit = builder.build();
	let mut filler = circuit.new_witness_filler();
	for (matrix_wires, matrix) in weight_wires.iter().zip(&weights) {
		for (row_wires, row) in matrix_wires.iter().zip(matrix) {
			for (&wire, &w) in row_wires.iter().zip(row) {
				filler[wire] = Word(word_of_i8(w));
			}
		}
	}
	for (row_wires, row) in input_wires.iter().zip(&inputs) {
		for (&wire, &v) in row_wires.iter().zip(row) {
			filler[wire] = Word(word_of_i8(v));
		}
	}
	for (&wire, &value) in out_wires.iter().zip(&outputs) {
		filler[wire] = Word(value as u64);
	}
	circuit
		.populate_wire_witness(&mut filler)
		.with_context(|| format!("{}: evaluating the MLP to fill the witness", task.name()))?;
	let witness = filler.into_value_vec();

	finish(task, circuit, witness, max_abs)
}

/// One dot product of an activation vector against one weight row.
fn dot_product(builder: &CircuitBuilder, act: &[Wire], row: &[Wire]) -> Wire {
	let product = |i: usize| builder.imul(act[i], row[i]).1;
	(1..row.len()).fold(product(0), |acc, i| builder.iadd(acc, product(i)).0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t2_emits_the_published_mac_and_relu_counts() {
		let built = build(Task::T2).expect("T2 builds");
		assert_eq!(built.n_imul_constraints, 92_224);
		assert_eq!(built.n_relus, 448);
		assert_eq!(built.n_inout_values, 1);
	}

	#[test]
	fn t3_is_eight_times_t2_in_one_circuit() {
		let built = build(Task::T3).expect("T3 builds");
		assert_eq!(built.n_imul_constraints, 737_792);
		assert_eq!(built.n_imul_constraints, 8 * 92_224);
		assert_eq!(built.n_relus, 3_584);
		// Eight outputs, one proof: the batch is not eight proofs stapled together.
		assert_eq!(built.n_inout_values, 8);
	}

	/// The un-requantised accumulator must be checked, not assumed. If this ever fails the
	/// expression is wrong for the instance and the task must be re-expressed.
	#[test]
	fn the_instance_stays_inside_i64() {
		for task in [Task::T2, Task::T3] {
			let built = build(task).expect("MLP builds");
			assert!(
				built.max_abs_intermediate * HEADROOM <= i128::from(i64::MAX),
				"{}: max |acc| = {}",
				task.name(),
				built.max_abs_intermediate
			);
		}
	}
}
