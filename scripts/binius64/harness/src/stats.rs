//! Order statistics for repeated measurements.
//!
//! The protocol's completion criterion demands error bars, so every reported quantity
//! carries its dispersion. Median plus interquartile range is used rather than mean plus
//! standard deviation: a prover run on a laptop occasionally picks up an OS scheduling
//! hiccup, and the median is not moved by it.

/// Median, quartiles and range of a sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
	pub n: usize,
	pub min: f64,
	pub q1: f64,
	pub median: f64,
	pub q3: f64,
	pub max: f64,
}

impl Summary {
	/// Interquartile range.
	pub fn iqr(&self) -> f64 {
		self.q3 - self.q1
	}

}

/// Summarizes a non-empty sample. Returns `None` for an empty one rather than panicking,
/// so a bucket that never fired is reported as absent instead of as zero.
pub fn summarize(samples: &[f64]) -> Option<Summary> {
	if samples.is_empty() {
		return None;
	}
	let mut sorted = samples.to_vec();
	sorted.sort_by(|a, b| a.partial_cmp(b).expect("measurements are never NaN"));
	Some(Summary {
		n: sorted.len(),
		min: sorted[0],
		q1: quantile(&sorted, 0.25),
		median: quantile(&sorted, 0.5),
		q3: quantile(&sorted, 0.75),
		max: sorted[sorted.len() - 1],
	})
}

/// Linear-interpolation quantile of an already-sorted sample.
fn quantile(sorted: &[f64], p: f64) -> f64 {
	debug_assert!(!sorted.is_empty());
	if sorted.len() == 1 {
		return sorted[0];
	}
	let pos = p * (sorted.len() - 1) as f64;
	let lo = pos.floor() as usize;
	let hi = pos.ceil() as usize;
	if lo == hi {
		return sorted[lo];
	}
	let weight = pos - lo as f64;
	sorted[lo] * (1.0 - weight) + sorted[hi] * weight
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn returns_none_for_an_empty_sample() {
		assert!(summarize(&[]).is_none());
	}

	#[test]
	fn reports_the_median_of_an_odd_sample() {
		let summary = summarize(&[3.0, 1.0, 2.0]).expect("non-empty");
		assert_eq!(summary.median, 2.0);
		assert_eq!(summary.min, 1.0);
		assert_eq!(summary.max, 3.0);
	}

	#[test]
	fn interpolates_the_median_of_an_even_sample() {
		let summary = summarize(&[1.0, 2.0, 3.0, 4.0]).expect("non-empty");
		assert_eq!(summary.median, 2.5);
	}
}
