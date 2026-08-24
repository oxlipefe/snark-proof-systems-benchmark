package gnarkbench

// Deterministic INT8 generator.
//
// This is SplitMix64, written out here rather than taken from math/rand, for one reason:
// the witness stream must be pinned to THIS FILE and not to a Go version. math/rand's
// documented stability has already moved once (Go 1.20 reseeded the global source, Go 1.22
// deprecated the old API), and a benchmark whose instances silently change when the
// toolchain moves is a benchmark that cannot be reproduced.
//
// The seeds are binius64's canonical seeds, reused verbatim so that the SHAPES and the MAC
// COUNTS line up across systems. The VALUES do not and cannot: this stream is not Rust's
// and not numpy's. Task-level comparison only — never witness-level.

type splitMix64 struct{ state uint64 }

func newRNG(seed uint32) *splitMix64 { return &splitMix64{state: uint64(seed)} }

func (r *splitMix64) next() uint64 {
	r.state += 0x9E3779B97F4A7C15
	z := r.state
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	return z ^ (z >> 31)
}

// int8 returns a value uniformly in [-128, 127], the range bench/TASKS.md specifies.
func (r *splitMix64) int8() int64 {
	return int64(int8(byte(r.next() >> 33)))
}
