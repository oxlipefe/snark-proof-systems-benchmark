//! The benchmark tasks of `bench/TASKS.md`, drawn to be the SAME INSTANCE binius64 measured.
//!
//! # Why the draw is copied rather than re-specified
//!
//! `bench/TASKS.md` fixes each task by an exact MAC count and a published seed, and
//! `systems/binius64/EXPRESSION.md` §7 fixes the RNG: `rand::rngs::StdRng::seed_from_u64`,
//! operands drawn as `i8` over the full `[-128, 127]`, `A` row-major first and then `B`
//! row-major. That draw is reproduced here verbatim, with the same `rand` major version, so
//! the two systems prove the same numbers and not merely the same shape.
//!
//! It is CHECKED rather than asserted by comment: `max_abs_intermediate` is the largest
//! magnitude any partial accumulator reaches, computed by the same running-max rule, and
//! `systems/binius64/EXPRESSION.md` §6 publishes it per task (T1-0: 270 167, T1-a: 421 915).
//! `Instance::assert_matches_binius64` compares against those published values and refuses
//! to hand back an instance that drifted.

use anyhow::Result;
use rand::{RngExt, SeedableRng, rngs::StdRng};

/// The INT32 range the task's output must stay inside, per `bench/TASKS.md`.
const INT32_MIN: i128 = i32::MIN as i128;
const INT32_MAX: i128 = i32::MAX as i128;

/// The benchmark tasks of `bench/TASKS.md`. Only the T1 ladder is expressible on the
/// sumcheck route; see `systems/plonky3/NOT_EXPRESSIBLE.md` for T2 and T3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Task {
    /// T1-0 — `[1x256] . [256x256]`, 65,536 MACs. Every dimension is a power of two.
    #[value(name = "t1-0")]
    T1_0,
    /// T1-a — `[1x768] . [768x768]`, 589,824 MACs. K and N are NOT powers of two.
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
}

impl Task {
    /// The task's name as `bench/TASKS.md` writes it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::T1_0 => "T1-0",
            Self::T1A => "T1-a",
            Self::T1B => "T1-b",
            Self::T1C => "T1-c",
            Self::T1D => "T1-d",
        }
    }

    /// The MAC count fixed by `bench/TASKS.md`. Never recomputed from the shape.
    pub const fn published_macs(self) -> usize {
        match self {
            Self::T1_0 => 65_536,
            Self::T1A => 589_824,
            Self::T1B => 2_359_296,
            Self::T1C => 9_437_184,
            Self::T1D => 37_748_736,
        }
    }

    /// The `[M x K] . [K x N]` shape.
    pub const fn shape(self) -> (usize, usize, usize) {
        match self {
            Self::T1_0 => (1, 256, 256),
            Self::T1A => (1, 768, 768),
            Self::T1B => (4, 768, 768),
            Self::T1C => (16, 768, 768),
            Self::T1D => (64, 768, 768),
        }
    }

    /// The published per-task witness seed, from `systems/binius64/EXPRESSION.md` §7.
    pub const fn seed(self) -> u64 {
        match self {
            Self::T1_0 => 0xE006_0100,
            Self::T1A => 0xE006_01A0,
            Self::T1B => 0xE006_01B0,
            Self::T1C => 0xE006_01C0,
            Self::T1D => 0xE006_01D0,
        }
    }

    /// `max |partial accumulator|` as published in `systems/binius64/EXPRESSION.md` §6.
    ///
    /// `None` where that file publishes no row (T1-d never built there).
    pub const fn binius64_max_abs_intermediate(self) -> Option<i128> {
        match self {
            Self::T1_0 => Some(270_167),
            Self::T1A => Some(421_915),
            Self::T1B => Some(636_963),
            Self::T1C => Some(611_801),
            Self::T1D => None,
        }
    }
}

/// One drawn task instance: the operands, the reference output, and the padded shape.
#[derive(Debug, Clone)]
pub struct Instance {
    pub task: Task,
    /// Published shape.
    pub m: usize,
    pub k: usize,
    pub n: usize,
    /// Padded shape — every dimension rounded up to a power of two, because a multilinear
    /// extension is indexed by a hypercube. `mp == m` etc. when the task is already aligned.
    pub mp: usize,
    pub kp: usize,
    pub np: usize,
    /// `A[M x K]`, signed INT8.
    pub a: Vec<Vec<i8>>,
    /// `B[K x N]`, signed INT8. The weights.
    pub b: Vec<Vec<i8>>,
    /// `C[M x N]`, the INT32 reference output, computed out of circuit in `i128`.
    pub c: Vec<Vec<i64>>,
    /// Largest absolute value any partial accumulator reached.
    pub max_abs_intermediate: i128,
}

impl Instance {
    /// Draws the published instance of `task`.
    pub fn draw(task: Task) -> Result<Self> {
        let (m, k, n) = task.shape();
        anyhow::ensure!(
            m * k * n == task.published_macs(),
            "{}: shape {m}x{k}x{n} gives {} MACs but bench/TASKS.md fixes {}",
            task.name(),
            m * k * n,
            task.published_macs()
        );

        let mut rng = StdRng::seed_from_u64(task.seed());
        // Draw order is A row-major then B row-major, exactly as the binius64 harness does.
        let a: Vec<Vec<i8>> = (0..m)
            .map(|_| (0..k).map(|_| rng.random()).collect())
            .collect();
        let b: Vec<Vec<i8>> = (0..k)
            .map(|_| (0..n).map(|_| rng.random()).collect())
            .collect();

        let (c, max_abs) = reference(task, &a, &b, m, n)?;

        Ok(Self {
            task,
            m,
            k,
            n,
            mp: m.next_power_of_two(),
            kp: k.next_power_of_two(),
            np: n.next_power_of_two(),
            a,
            b,
            c,
            max_abs_intermediate: max_abs,
        })
    }

    /// The instance binius64 proved, or an error naming the drift.
    ///
    /// This is the cross-system check that the two harnesses are on the same numbers: it
    /// compares `max |partial accumulator|` against the value `systems/binius64/EXPRESSION.md`
    /// §6 publishes, which is a function of every drawn operand.
    pub fn assert_matches_binius64(&self) -> Result<()> {
        let Some(published) = self.task.binius64_max_abs_intermediate() else {
            return Ok(());
        };
        anyhow::ensure!(
            self.max_abs_intermediate == published,
            "{}: max |accumulator| is {} but systems/binius64/EXPRESSION.md §6 publishes {} \
             for the same seed; the two harnesses are NOT proving the same instance",
            self.task.name(),
            self.max_abs_intermediate,
            published
        );
        Ok(())
    }

    /// Padded MAC count — what the prover actually does, which is not what the task fixes.
    pub const fn padded_macs(&self) -> usize {
        self.mp * self.kp * self.np
    }

    /// `A` flattened row-major into the padded hypercube `{0,1}^(log mp + log kp)`.
    pub fn a_padded(&self) -> Vec<i8> {
        let mut out = vec![0i8; self.mp * self.kp];
        for (i, row) in self.a.iter().enumerate() {
            out[i * self.kp..i * self.kp + self.k].copy_from_slice(row);
        }
        out
    }

    /// `B` flattened row-major into the padded hypercube `{0,1}^(log kp + log np)`.
    pub fn b_padded(&self) -> Vec<i8> {
        let mut out = vec![0i8; self.kp * self.np];
        for (kk, row) in self.b.iter().enumerate() {
            out[kk * self.np..kk * self.np + self.n].copy_from_slice(row);
        }
        out
    }

    /// `C` flattened row-major into the padded hypercube `{0,1}^(log mp + log np)`.
    pub fn c_padded(&self) -> Vec<i64> {
        let mut out = vec![0i64; self.mp * self.np];
        for (i, row) in self.c.iter().enumerate() {
            out[i * self.np..i * self.np + self.n].copy_from_slice(row);
        }
        out
    }

    /// Re-derives the reference output after `b` has been perturbed.
    ///
    /// Amendment A3 of `bench/TASKS.md`: a witness corruption counts as a test only if the
    /// output changes. For a matmul it almost always does, but "almost always" is not a
    /// measurement, so the corrupted instance recomputes and the control reports the verdict.
    pub fn recompute(&self) -> Result<(Vec<Vec<i64>>, i128)> {
        reference(self.task, &self.a, &self.b, self.m, self.n)
    }
}

/// The out-of-circuit reference product, in `i128` so the INT32 check cannot itself overflow.
fn reference(
    task: Task,
    a: &[Vec<i8>],
    b: &[Vec<i8>],
    m: usize,
    n: usize,
) -> Result<(Vec<Vec<i64>>, i128)> {
    let mut max_abs: i128 = 0;
    let mut c: Vec<Vec<i64>> = Vec::with_capacity(m);
    for row in a.iter() {
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
    Ok((c, max_abs))
}

/// Parses a task from its `bench/TASKS.md` name, for the negative control's CLI.
pub fn task_from_name(s: &str) -> Result<Task> {
    Ok(match s {
        "t1-0" => Task::T1_0,
        "t1-a" => Task::T1A,
        "t1-b" => Task::T1B,
        "t1-c" => Task::T1C,
        "t1-d" => Task::T1D,
        other => anyhow::bail!("unknown task {other}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of copying the draw: the instance must be binius64's instance.
    #[test]
    fn t1_0_is_the_instance_binius64_measured() {
        let inst = Instance::draw(Task::T1_0).expect("T1-0 draws");
        inst.assert_matches_binius64().expect("same instance");
        assert_eq!(inst.max_abs_intermediate, 270_167);
    }

    #[test]
    fn t1_a_is_the_instance_binius64_measured() {
        let inst = Instance::draw(Task::T1A).expect("T1-a draws");
        inst.assert_matches_binius64().expect("same instance");
    }

    /// A signed instance must actually be signed, or the encoding silently degraded.
    #[test]
    fn the_instance_exercises_negative_values() {
        let inst = Instance::draw(Task::T1_0).expect("T1-0 draws");
        assert!(inst.a[0].iter().any(|&v| v < 0));
        assert!(inst.a[0].iter().any(|&v| v > 0));
    }

    /// Padding is identity where the task is already aligned, and 1024 where it is not.
    #[test]
    fn padding_is_declared_not_hidden() {
        let t0 = Instance::draw(Task::T1_0).expect("draws");
        assert_eq!((t0.mp, t0.kp, t0.np), (1, 256, 256));
        assert_eq!(t0.padded_macs(), t0.task.published_macs());

        let ta = Instance::draw(Task::T1A).expect("draws");
        assert_eq!((ta.mp, ta.kp, ta.np), (1, 1024, 1024));
        assert_eq!(ta.padded_macs(), 1_048_576);
    }
}
