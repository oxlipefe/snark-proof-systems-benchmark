package gnarkbench

import (
	"testing"

	"github.com/consensys/gnark/backend"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/test"
)

// reluProbe is a standalone circuit whose only job is to exercise one ReLU gadget. Out is
// public, so a wrong Out is a statement the prover must fail to satisfy — which is what
// makes ProverFailed below a real test and not a tautology.
type reluProbe struct {
	In  []frontend.Variable `gnark:",secret"`
	Out []frontend.Variable `gnark:",public"`

	gadget ReluGadget
	bits   int
}

func (c *reluProbe) Define(api frontend.API) error {
	rc := newRangechecker(api)
	for i := range c.In {
		api.AssertIsEqual(Relu(api, rc, c.gadget, c.In[i], c.bits), c.Out[i])
	}
	return nil
}

// reluCases covers the three regions that matter and the two boundaries the gadgets are
// most likely to get wrong: the extremes of the declared range, and zero, where both
// admissible hint values collapse to the same output.
var reluCases = []struct{ in, out int64 }{
	{0, 0},
	{1, 1},
	{-1, 0},
	{127, 127},
	{-128, 0},
	{1000, 1000},
	{-1000, 0},
	{32767, 32767},
	{-32768, 0},
	{1 << 20, 1 << 20},
	{-(1 << 20), 0},
	{(1 << 21) - 1, (1 << 21) - 1},
	{-((1 << 21) - 1), 0},
}

const reluTestBits = 22 // every case above satisfies |x| < 2^21 < 2^22

func mkProbe(g ReluGadget, n int) *reluProbe {
	return &reluProbe{
		In: make([]frontend.Variable, n), Out: make([]frontend.Variable, n),
		gadget: g, bits: reluTestBits,
	}
}

func TestReluGadgetsAreCorrect(t *testing.T) {
	for _, g := range []ReluGadget{ReluToBinary, ReluHintedSign} {
		g := g
		t.Run(string(g), func(t *testing.T) {
			assert := test.NewAssert(t)
			tmpl := mkProbe(g, len(reluCases))
			good := mkProbe(g, len(reluCases))
			for i, c := range reluCases {
				good.In[i] = c.in
				good.Out[i] = c.out
			}
			assert.ProverSucceeded(tmpl, good,
				test.WithCurves(ecc.BN254),
				test.WithBackends(backend.GROTH16, backend.PLONK))
		})
	}
}

// TestReluGadgetsRejectWrongWitness is the check that the positive test is not vacuous. A
// gadget that returned x unchanged would pass every positive case above with out = x for
// x >= 0; it is only the NEGATIVE inputs claimed to pass through unchanged that separate a
// real ReLU from an identity.
func TestReluGadgetsRejectWrongWitness(t *testing.T) {
	wrong := []struct {
		name    string
		in, out int64
	}{
		{"negative_passed_through", -7, -7},
		{"negative_claimed_positive", -7, 7},
		{"positive_zeroed", 7, 0},
		{"positive_off_by_one", 7, 8},
		{"zero_claimed_nonzero", 0, 1},
	}
	for _, g := range []ReluGadget{ReluToBinary, ReluHintedSign} {
		g := g
		for _, w := range wrong {
			w := w
			t.Run(string(g)+"/"+w.name, func(t *testing.T) {
				assert := test.NewAssert(t)
				tmpl := mkProbe(g, 1)
				bad := mkProbe(g, 1)
				bad.In[0] = w.in
				bad.Out[0] = w.out
				assert.ProverFailed(tmpl, bad,
					test.WithCurves(ecc.BN254),
					test.WithBackends(backend.GROTH16, backend.PLONK))
			})
		}
	}
}

// TestReluHintedSignRejectsOutOfRange pins the precondition the hinted-sign gadget's
// soundness argument rests on: |x| < 2^B. A value outside the declared range must not
// prove, or the gadget's argument would be false for exactly the inputs it excludes.
func TestReluHintedSignRejectsOutOfRange(t *testing.T) {
	assert := test.NewAssert(t)
	tmpl := mkProbe(ReluHintedSign, 1)
	bad := mkProbe(ReluHintedSign, 1)
	bad.In[0] = int64(1) << reluTestBits // == 2^B, one past the declared range
	bad.Out[0] = int64(1) << reluTestBits
	assert.ProverFailed(tmpl, bad, test.WithCurves(ecc.BN254), test.WithBackends(backend.GROTH16, backend.PLONK))
}
