package gnarkbench

import (
	"strings"
	"testing"

	"github.com/consensys/gnark/frontend"
)

// TestMACAssertionFires is the test on the guard itself. A guard nobody has watched fail is
// a guard nobody knows is wired up — E-001's third failure was exactly an assertion that
// was never exercised.
func TestMACAssertionFires(t *testing.T) {
	spec := Specs["t1-0"]
	ref, err := NewReference(spec)
	if err != nil {
		t.Fatal(err)
	}
	drifted := spec
	drifted.MACs = spec.MACs - 1 // pretend TASKS.md said one less
	c := NewMatMulCircuit(drifted, RegimeA, ref)
	_, err = frontend.Compile(Curve.ScalarField(), builderFor(Groth16), c)
	if err == nil {
		t.Fatal("compile succeeded with a drifted MAC count; the guard is not wired up")
	}
	want := "t1-0: emitted 65536 MACs but bench/TASKS.md fixes 65535; the expression drifted from the published task"
	if !strings.Contains(err.Error(), want) {
		t.Fatalf("guard fired with the wrong text.\n got: %v\nwant substring: %s", err, want)
	}
}

// TestFrozenCountsMatchTheEmittedExpression compiles nothing: it walks the reference forward
// pass and checks that the arithmetic the generator performs is the arithmetic TASKS.md
// froze. If this fails, every bytes/MAC figure in the campaign has the wrong denominator.
func TestFrozenCountsMatchTheEmittedExpression(t *testing.T) {
	for _, label := range BankTasks {
		spec := Specs[label]
		want := spec.MACs
		got := 0
		switch spec.Kind {
		case KindMatMul:
			got = spec.M * spec.K * spec.N
		case KindMLP:
			for _, l := range spec.Layers {
				got += spec.Batch * l.In * l.Out
			}
		}
		if got != want {
			t.Errorf("%s: shape implies %d MACs, bench/TASKS.md fixes %d", label, got, want)
		}
	}
}

// TestReLUCountIsSeparateFromMACs pins bench/TASKS.md's rule that activations are counted
// and reported separately and never folded into the MAC count.
func TestReLUCountIsSeparateFromMACs(t *testing.T) {
	for _, label := range []string{"t2", "t3"} {
		spec := Specs[label]
		ref, err := NewReference(spec)
		if err != nil {
			t.Fatalf("%s: %v", label, err)
		}
		if got := ref.ReluSites(); got != spec.ReLUs {
			t.Errorf("%s: reference performs %d activations, spec says %d", label, got, spec.ReLUs)
		}
	}
}

// TestA1RefusesWhenTheMarginIsGone drives the A1 assertion into its refusal, so that the
// campaign has watched it fire rather than assumed it would.
func TestA1RefusesWhenTheMarginIsGone(t *testing.T) {
	r := &Reference{Spec: Specs["t2"], MaxAbsIntermediate: 1 << 62}
	if err := r.assertA1(); err == nil {
		t.Fatal("A1 accepted an intermediate at 2^62, which leaves less than a factor-2 margin")
	}
	r.MaxAbsIntermediate = 1 << 61
	if err := r.assertA1(); err != nil {
		t.Fatalf("A1 refused an intermediate at 2^61, which has the margin: %v", err)
	}
}

// TestReferenceIsDeterministic is the reproducibility check. The witness stream is pinned to
// rng.go, not to a Go version, and this is what would notice if that stopped being true.
func TestReferenceIsDeterministic(t *testing.T) {
	for _, label := range BankTasks {
		if label == "t1-c" || label == "t1-d" {
			continue // same code path as t1-b, and this test should stay cheap
		}
		a, err := NewReference(Specs[label])
		if err != nil {
			t.Fatalf("%s: %v", label, err)
		}
		b, err := NewReference(Specs[label])
		if err != nil {
			t.Fatalf("%s: %v", label, err)
		}
		if a.MaxAbsIntermediate != b.MaxAbsIntermediate {
			t.Errorf("%s: two references disagree on max |intermediate|", label)
		}
		if len(a.Out) != len(b.Out) {
			t.Fatalf("%s: output length differs", label)
		}
		for i := range a.Out {
			if a.Out[i] != b.Out[i] {
				t.Fatalf("%s: outputs differ at %d", label, i)
			}
		}
		for i := range a.OutB {
			if a.OutB[i] != b.OutB[i] {
				t.Fatalf("%s: batched outputs differ at %d", label, i)
			}
		}
	}
}
