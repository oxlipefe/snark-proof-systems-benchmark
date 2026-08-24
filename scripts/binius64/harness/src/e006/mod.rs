//! E-006 · the three public benchmark tasks, expressed in Binius64.
//!
//! The task specifications are frozen in `bench/TASKS.md` and are **not** recomputed here:
//! the MAC count of each task is a constant of this module, and every builder asserts that
//! the circuit it produced emits exactly that many IMUL constraints. A builder that drifts
//! from the published count fails loudly instead of quietly renormalising `bytes/MAC`.
//!
//! # INT8 encoding, declared
//!
//! `bench/TASKS.md` specifies INT8 operands in `[-128, 127]`. Binius64's word is a 64-bit
//! two's-complement integer, so a signed 8-bit value is carried **sign-extended into a
//! 64-bit word**. The product of two such words is taken from the **low word of `imul`**,
//! which for two's-complement operands is exactly the low 64 bits of the signed product;
//! the sign corrections that [`binius_frontend::CircuitBuilder::smul`] applies touch only
//! the high word, which this circuit discards. So a signed INT8 multiply-accumulate costs
//! **1 IMUL constraint**, the same as the unsigned one measured in E-001, and no range
//! constraint is needed because the operands are witnessed as full 64-bit words whose
//! product cannot overflow at these depths (asserted per task in `bounds`).
//!
//! Accumulation is [`CircuitBuilder::iadd`], whose two's-complement sum is exact while the
//! running total stays inside `i64`. Every builder checks that against the reference
//! computation before it emits anything, and records the observed maximum magnitude.

pub mod matmul;
pub mod mlp;

use anyhow::Result;
use binius_core::constraint_system::{ConstraintSystem, ValueVec};
use binius_frontend::{CircuitBuilder, Wire};

/// The benchmark tasks of `bench/TASKS.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Task {
	/// T1-0 — `[1x256] . [256x256]`, 65,536 MACs.
	#[value(name = "t1-0")]
	T1_0,
	/// T1-a — `[1x768] . [768x768]`, 589,824 MACs.
	#[value(name = "t1-a")]
	T1A,
	/// T1-b — `[4x768] . [768x768]`, 2,359,296 MACs.
	#[value(name = "t1-b")]
	T1B,
	/// T1-c — `[16x768] . [768x768]`, 9,437,184 MACs.
	#[value(name = "t1-c")]
	T1C,
	/// T1-d — `[64x768] . [768x768]`, 37,748,736 MACs.
	#[value(name = "t1-d")]
	T1D,
	/// T2 — the 200-256-128-64-1 MLP, one input, 92,224 MACs plus 448 ReLU.
	#[value(name = "t2")]
	T2,
	/// T3 — the same MLP over a batch of 8, in one proof. 737,792 MACs plus 3,584 ReLU.
	#[value(name = "t3")]
	T3,
}

impl Task {
	pub const ALL: [Task; 7] = [
		Task::T1_0,
		Task::T1A,
		Task::T1B,
		Task::T1C,
		Task::T1D,
		Task::T2,
		Task::T3,
	];

	/// The task's name as `bench/TASKS.md` writes it.
	pub const fn name(self) -> &'static str {
		match self {
			Task::T1_0 => "T1-0",
			Task::T1A => "T1-a",
			Task::T1B => "T1-b",
			Task::T1C => "T1-c",
			Task::T1D => "T1-d",
			Task::T2 => "T2",
			Task::T3 => "T3",
		}
	}

	/// The MAC count fixed by `bench/TASKS.md`. Never recomputed from the shape.
	pub const fn published_macs(self) -> usize {
		match self {
			Task::T1_0 => 65_536,
			Task::T1A => 589_824,
			Task::T1B => 2_359_296,
			Task::T1C => 9_437_184,
			Task::T1D => 37_748_736,
			Task::T2 => 92_224,
			Task::T3 => 737_792,
		}
	}

	/// ReLU applications, counted separately and never folded into the MAC count.
	pub const fn published_relus(self) -> usize {
		match self {
			Task::T2 => 448,
			Task::T3 => 3_584,
			_ => 0,
		}
	}

	/// The `[M x K] . [K x N]` shape, for the T1 rungs.
	pub const fn matmul_shape(self) -> Option<(usize, usize, usize)> {
		match self {
			Task::T1_0 => Some((1, 256, 256)),
			Task::T1A => Some((1, 768, 768)),
			Task::T1B => Some((4, 768, 768)),
			Task::T1C => Some((16, 768, 768)),
			Task::T1D => Some((64, 768, 768)),
			_ => None,
		}
	}

	/// Batch size, for the MLP tasks.
	pub const fn mlp_batch(self) -> Option<usize> {
		match self {
			Task::T2 => Some(1),
			Task::T3 => Some(8),
			_ => None,
		}
	}

	/// The published per-task witness seed. Fixed here so a rerun proves the same instance.
	pub const fn seed(self) -> u64 {
		match self {
			Task::T1_0 => 0xE006_0100,
			Task::T1A => 0xE006_01A0,
			Task::T1B => 0xE006_01B0,
			Task::T1C => 0xE006_01C0,
			Task::T1D => 0xE006_01D0,
			Task::T2 => 0xE006_0200,
			Task::T3 => 0xE006_0300,
		}
	}

	/// One line describing how the task is expressed, recorded next to every measurement.
	pub const fn expression(self) -> &'static str {
		match self {
			Task::T2 | Task::T3 => {
				"MLP 200-256-128-64-1, signed INT8 weights and input sign-extended to 64-bit \
				 words; 1 IMUL per MAC (low word of imul); iadd accumulation; ReLU = \
				 band(x, bnot(sar(x, 63))); no requantisation between layers"
			}
			_ => {
				"INT8 matmul A[MxK].B[KxN], signed operands sign-extended to 64-bit words; \
				 1 IMUL per MAC (low word of imul); iadd accumulation; INT32 output asserted \
				 against one inout wire per output element"
			}
		}
	}
}

/// A built task circuit with its witness, ready to be proved repeatedly.
pub struct Built {
	pub task: Task,
	pub constraint_system: ConstraintSystem,
	pub witness: ValueVec,
	pub n_and_constraints: usize,
	pub n_imul_constraints: usize,
	pub n_zero_constraints: usize,
	pub n_bmul_constraints: usize,
	pub n_macs: usize,
	pub n_relus: usize,
	/// Largest absolute value any accumulator reached, from the out-of-circuit reference.
	pub max_abs_intermediate: i128,
	/// Private (committed) words, the quantity `MAX_VALUES_PER_SEGMENT` bounds.
	pub n_private_values: usize,
	pub n_inout_values: usize,
}

/// Builds the named task.
pub fn build(task: Task) -> Result<Built> {
	match task {
		Task::T2 | Task::T3 => mlp::build(task),
		_ => matmul::build(task),
	}
}

/// Sign-extends a signed 8-bit value into the 64-bit word Binius64 carries it in.
#[inline]
pub fn word_of_i8(v: i8) -> u64 {
	v as i64 as u64
}

/// ReLU on a two's-complement 64-bit accumulator: `x & !(x >> 63)`.
///
/// `sar(x, 63)` is all-ones exactly when `x` is negative, so masking with its complement
/// keeps `x` when it is non-negative and yields zero when it is not.
pub fn relu(builder: &CircuitBuilder, x: Wire) -> Wire {
	let sign_mask = builder.sar(x, 63);
	builder.band(x, builder.bnot(sign_mask))
}

/// Shared tail: reads the constraint mix off the built circuit and checks it against the
/// frozen MAC count of `bench/TASKS.md`.
pub(crate) fn finish(
	task: Task,
	circuit: binius_frontend::Circuit,
	witness: ValueVec,
	max_abs_intermediate: i128,
) -> Result<Built> {
	let cs = circuit.constraint_system().clone();
	let n_imul = cs.imul_constraints.len();
	anyhow::ensure!(
		n_imul == task.published_macs(),
		"{}: emitted {n_imul} IMUL constraints but bench/TASKS.md fixes {} MACs; the \
		 expression drifted from the published task",
		task.name(),
		task.published_macs()
	);
	Ok(Built {
		task,
		n_and_constraints: cs.and_constraints.len(),
		n_imul_constraints: n_imul,
		n_zero_constraints: cs.zero_constraints.len(),
		n_bmul_constraints: cs.bmul_constraints.len(),
		n_private_values: witness.non_public().len(),
		n_inout_values: witness.inout().len(),
		constraint_system: cs,
		witness,
		n_macs: task.published_macs(),
		n_relus: task.published_relus(),
		max_abs_intermediate,
	})
}
