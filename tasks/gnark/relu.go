package gnarkbench

import (
	"math/big"

	"github.com/consensys/gnark/constraint/solver"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/std/rangecheck"
)

// ReLU, two gadgets, both correct, one kept.
//
// bench/TASKS.md puts a ReLU after layers 1–3 of T2 and T3 and counts the activations
// separately from the MACs. In a prime field there is no sign bit to read, so "is x
// negative" has to be decided by a constraint system that cannot see the integer. Both
// gadgets below decide it; they differ in what they pay for the decision.
//
// GADGET 1 — binary decomposition. bits = ToBinary(x + 2^B, B+1); the top bit is the sign
// (1 when x ≥ 0); y = Select(sign, x, 0). The decomposition also enforces |x| ≤ 2^B, so the
// range check is not separate. It costs one constraint per bit, so its price is B.
//
// GADGET 2 — hinted sign with two range checks. A hint supplies s ∈ {0,1}; the circuit
// asserts s is boolean, sets y = s·x, and range-checks BOTH y and y−x to B bits.
//
//	s = 1  ⇒  y = x    and  y − x = 0     ⇒  x ∈ [0, 2^B)
//	s = 0  ⇒  y = 0    and  y − x = −x    ⇒  −x ∈ [0, 2^B), i.e. x ≤ 0
//
// So exactly one branch is admissible for any x with |x| < 2^B, except x = 0 where both
// branches give y = 0 — the two witnesses are observationally identical and neither is a
// soundness hole. The hint is UNCONSTRAINED by construction (a lying prover may set s
// freely); what makes the gadget sound is that the two range checks leave a liar no
// satisfying assignment. That argument is the reason relu_test.go exists: the wrong-witness
// test is not a formality here, it is the check on this paragraph.
//
// Gadget 2's price is not B but 2·(cost of one B-bit range check), and std/rangecheck's
// commit-based checker AMORTIZES a shared lookup table across every check in the circuit.
// Which gadget is cheaper therefore depends on B and on how many checks the rest of the
// circuit already pays for. Both are measured; nothing here is assumed.

// ReluGadget names the two implementations.
type ReluGadget string

const (
	ReluToBinary   ReluGadget = "tobinary"
	ReluHintedSign ReluGadget = "hintedsign"
)

// DefaultReluGadget is the one kept for the bank. It is set from measurement, in
// cmd/probe's `relu` mode; see bench/systems/gnark/EXPRESSION.md.
const DefaultReluGadget = ReluHintedSign

// signHint returns 1 when the input is a "non-negative" field element and 0 otherwise,
// where non-negative means "in [0, (p-1)/2]". The circuit does not trust this.
func signHint(field *big.Int, inputs, outputs []*big.Int) error {
	half := new(big.Int).Rsh(field, 1)
	if inputs[0].Cmp(half) <= 0 {
		outputs[0].SetUint64(1)
	} else {
		outputs[0].SetUint64(0)
	}
	return nil
}

func init() { solver.RegisterHint(signHint) }

// ReluHints is what the runner must hand the solver.
func ReluHints() []solver.Hint { return []solver.Hint{signHint} }

// Relu applies the named gadget. rc may be nil for ReluToBinary.
func Relu(api frontend.API, rc frontend.Rangechecker, gadget ReluGadget, x frontend.Variable, b int) frontend.Variable {
	switch gadget {
	case ReluToBinary:
		return reluToBinary(api, x, b)
	case ReluHintedSign:
		return reluHintedSign(api, rc, x, b)
	}
	panic("unknown relu gadget " + string(gadget))
}

func reluToBinary(api frontend.API, x frontend.Variable, b int) frontend.Variable {
	shift := new(big.Int).Lsh(big.NewInt(1), uint(b))
	bits := api.ToBinary(api.Add(x, shift), b+1)
	sign := bits[b] // 1 exactly when x + 2^B ≥ 2^B, i.e. x ≥ 0
	return api.Select(sign, x, 0)
}

func reluHintedSign(api frontend.API, rc frontend.Rangechecker, x frontend.Variable, b int) frontend.Variable {
	out, err := api.Compiler().NewHint(signHint, 1, x)
	if err != nil {
		panic(err)
	}
	s := out[0]
	api.AssertIsBoolean(s)
	y := api.Mul(s, x)
	rc.Check(y, b)
	rc.Check(api.Sub(y, x), b)
	return y
}

// newRangechecker is the single place the checker is constructed, so that every circuit in
// this package shares ONE lookup table. std/rangecheck's commit variant amortizes that
// table over every Check in the circuit; constructing a second checker would silently
// double a cost the campaign then reports as the task's.
func newRangechecker(api frontend.API) frontend.Rangechecker { return rangecheck.New(api) }
