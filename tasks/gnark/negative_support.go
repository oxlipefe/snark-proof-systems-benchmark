package gnarkbench

import (
	"fmt"

	"github.com/consensys/gnark/backend/witness"
	"github.com/consensys/gnark/frontend"
)

// VerifyingKeyAny returns the verifying key as an untyped value, for the correctness
// control, which has to hold keys of either backend in the same variable.
func (k *Keys) VerifyingKeyAny() any {
	if k.Backend == Groth16 {
		return k.G16vk
	}
	return k.PLvk
}

// SecretPosition names one secret value of an assignment: which slice, and which index.
type SecretPosition struct {
	Slice string // "X" or "W"
	Index int
}

func (p SecretPosition) String() string { return fmt.Sprintf("%s[%d]", p.Slice, p.Index) }

// SecretPositions picks n secret positions to corrupt, SPREAD ACROSS the whole of each
// slice rather than clustered at its start. A sweep that only ever corrupts index 0 tests
// one wire; the campaign reports the sample size and where the samples fell, and infers
// nothing about the positions it did not touch.
func SecretPositions(spec Spec, regime Regime, n int) []SecretPosition {
	if n <= 0 {
		return nil
	}
	var xLen, wLen int
	switch spec.Kind {
	case KindMatMul:
		xLen = spec.M * spec.K
		if regime == RegimeA {
			wLen = spec.K * spec.N
		}
	case KindMLP:
		xLen = spec.Batch * spec.Layers[0].In
		if regime == RegimeA {
			wLen = mlpWeightLen(spec)
		}
	}

	half := n / 2
	if wLen == 0 {
		half = n
	}
	var out []SecretPosition
	out = append(out, spread("X", xLen, half)...)
	if wLen > 0 {
		out = append(out, spread("W", wLen, n-half)...)
	}
	return out
}

func spread(name string, length, n int) []SecretPosition {
	if length == 0 || n <= 0 {
		return nil
	}
	if n > length {
		n = length
	}
	out := make([]SecretPosition, 0, n)
	for i := 0; i < n; i++ {
		out = append(out, SecretPosition{Slice: name, Index: i * length / n})
	}
	return out
}

// CorruptedAssignment returns the honest assignment with ONE secret value incremented by
// one, leaving the public claimed output untouched. The proof then either fails to be
// produced — gnark's solver evaluates the constraints while assigning — or is produced and
// must be rejected. Both outcomes are recorded; only the second would be a soundness
// statement, and the control's write-up must not confuse them.
func CorruptedAssignment(spec Spec, regime Regime, gadget ReluGadget, ref *Reference, pos SecretPosition) (frontend.Circuit, error) {
	asg := Assignment(spec, regime, gadget, ref)
	switch c := asg.(type) {
	case *MatMulCircuit:
		return c, bump(sliceOf(c.X, c.W, pos), pos)
	case *MLPCircuit:
		return c, bump(sliceOf(c.X, c.W, pos), pos)
	}
	return nil, fmt.Errorf("unknown circuit type for %s", spec.Label)
}

func sliceOf(x, w []frontend.Variable, pos SecretPosition) []frontend.Variable {
	if pos.Slice == "W" {
		return w
	}
	return x
}

func bump(s []frontend.Variable, pos SecretPosition) error {
	if pos.Index < 0 || pos.Index >= len(s) {
		return fmt.Errorf("position %s is outside the assignment (len %d)", pos, len(s))
	}
	v, ok := s[pos.Index].(int)
	if !ok {
		return fmt.Errorf("position %s does not hold an int", pos)
	}
	s[pos.Index] = v + 1
	return nil
}

// WitnessOf builds a full witness from an assignment.
func WitnessOf(asg frontend.Circuit) (witness.Witness, error) {
	return frontend.NewWitness(asg, Curve.ScalarField())
}
