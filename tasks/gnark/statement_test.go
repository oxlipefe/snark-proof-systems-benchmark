package gnarkbench

import (
	"fmt"
	"testing"
)

// TestInertBumpsAreClassifiedAsInert pins the episode that produced the WITNESS_INERT verdict.
//
// The first campaign run of the correctness control reported W[46112] and W[69168] on T2 as
// VERIFY_ACCEPTED, which reads as a soundness finding and is not one: both weights feed neurons
// whose pre-activation is negative, the ReLU zeroes them, and incrementing them leaves the
// public output bit-identical. This test is the guard that keeps the control from ever again
// reporting the prover's honesty as the verifier's failure.
func TestInertBumpsAreClassifiedAsInert(t *testing.T) {
	spec := Specs["t2"]
	ref, err := NewReference(spec)
	if err != nil {
		t.Fatal(err)
	}
	for _, idx := range []int{46112, 69168} {
		pos := SecretPosition{Slice: "W", Index: idx}
		eff, err := EffectOfBump(spec, RegimeA, ref, pos)
		if err != nil {
			t.Fatal(err)
		}
		if eff.Changes {
			t.Errorf("W[%d]: expected the bump to leave the output unchanged, got Changes=true", idx)
		}
		if got := eff.Expected(); got != "WITNESS_INERT" {
			t.Errorf("W[%d]: expected WITNESS_INERT, got %s", idx, got)
		}
	}
}

// TestLiveBumpsAreClassifiedAsLive is the other half: a position that DOES move the output must
// not be silently excused as inert, or the family would test nothing at all.
func TestLiveBumpsAreClassifiedAsLive(t *testing.T) {
	for _, label := range []string{"t2", "t1-0"} {
		spec := Specs[label]
		ref, err := NewReference(spec)
		if err != nil {
			t.Fatal(err)
		}
		pos := SecretPosition{Slice: "W", Index: 0}
		eff, err := EffectOfBump(spec, RegimeA, ref, pos)
		if err != nil {
			t.Fatal(err)
		}
		if !eff.Changes {
			t.Errorf("%s W[0]: expected the bump to move the output", label)
		}
		if got := eff.Expected(); got != "PROVE_REJECTED_statement" {
			t.Errorf("%s W[0]: expected PROVE_REJECTED_statement, got %s", label, got)
		}
	}
}

// TestMatMulHasNoInertWeights is the control on the control. T1 has no activations, so every
// weight reaches the output linearly and NO bump can be inert. If this ever fails, the
// inertness machinery is finding inertness where the arithmetic forbids it.
func TestMatMulHasNoInertWeights(t *testing.T) {
	spec := Specs["t1-0"]
	ref, err := NewReference(spec)
	if err != nil {
		t.Fatal(err)
	}
	for _, pos := range SecretPositions(spec, RegimeA, 32) {
		eff, err := EffectOfBump(spec, RegimeA, ref, pos)
		if err != nil {
			t.Fatal(err)
		}
		if !eff.Changes {
			t.Errorf("%s: bump at %s left the output unchanged; a matmul has no dead weights", spec.Label, pos)
		}
	}
}

// TestInertFractionIsReported measures how much of T2's and T3's weight tensor is dead for the
// published input. It is not an assertion about a threshold — it is the number the correctness
// control reports, and it belongs in the record rather than in a log nobody reads.
func TestInertFractionIsReported(t *testing.T) {
	for _, label := range []string{"t2", "t3"} {
		spec := Specs[label]
		ref, err := NewReference(spec)
		if err != nil {
			t.Fatal(err)
		}
		pool := SecretPositions(spec, RegimeA, 256)
		live, inert := 0, 0
		for _, pos := range pool {
			eff, err := EffectOfBump(spec, RegimeA, ref, pos)
			if err != nil {
				t.Fatal(err)
			}
			if eff.Changes {
				live++
			} else {
				inert++
			}
		}
		fmt.Printf("INERTFRACTION task=%s sampled=%d live=%d inert=%d inert_pct=%.1f\n",
			label, live+inert, live, inert, 100*float64(inert)/float64(live+inert))
	}
}
