package gnarkbench

import (
	"github.com/consensys/gnark/frontend"
)

// MatMulCircuit expresses one rung of the T1 ladder: A[M×K] · B[K×N], INT8 in, full-width
// accumulator out, no requantization.
//
// Only the exported fields are circuit variables — gnark discovers them by reflection, and
// everything unexported below is build configuration that never becomes a wire.
//
// REGIME A: W is allocated and secret, and every element of X and W is range-checked to
// 8 bits after a +128 shift into [0, 255]. Each MAC is api.Mul(variable, variable), which
// is one R1CS multiplication constraint.
//
// REGIME B: W is nil; the weights come from wConst as Go integers. api.Mul(variable,
// constant) scales a linear expression instead of emitting a constraint, so the entire
// dot product folds into the AssertIsEqual at the end. Only X is range-checked — there is
// nothing to prove about a constant. Groth16's per-circuit setup then binds those weights
// into the verifying key, which is the lever this regime exists to price.
type MatMulCircuit struct {
	X   []frontend.Variable `gnark:",secret"`
	W   []frontend.Variable `gnark:",secret"`
	Out []frontend.Variable `gnark:",public"`

	spec   Spec
	regime Regime
	wConst []int8
	macs   *int
}

// NewMatMulCircuit returns the compile-time template. Slices are allocated because gnark
// sizes the witness from the template; their elements stay nil.
func NewMatMulCircuit(spec Spec, regime Regime, ref *Reference) *MatMulCircuit {
	c := &MatMulCircuit{
		X:      make([]frontend.Variable, spec.M*spec.K),
		Out:    make([]frontend.Variable, spec.M*spec.N),
		spec:   spec,
		regime: regime,
		macs:   new(int),
	}
	if regime == RegimeA {
		c.W = make([]frontend.Variable, spec.K*spec.N)
	} else {
		c.wConst = ref.W
	}
	return c
}

// AssignMatMul fills the template with the reference instance.
func AssignMatMul(spec Spec, regime Regime, ref *Reference) *MatMulCircuit {
	c := &MatMulCircuit{
		X:      make([]frontend.Variable, spec.M*spec.K),
		Out:    make([]frontend.Variable, spec.M*spec.N),
		spec:   spec,
		regime: regime,
		macs:   new(int),
	}
	for i, v := range ref.X {
		c.X[i] = int(v)
	}
	if regime == RegimeA {
		c.W = make([]frontend.Variable, spec.K*spec.N)
		for i, v := range ref.W {
			c.W[i] = int(v)
		}
	}
	for i, v := range ref.Out {
		c.Out[i] = v
	}
	return c
}

// EmittedMACs is what Define counted. Zero before compilation.
func (c *MatMulCircuit) EmittedMACs() int {
	if c.macs == nil {
		return 0
	}
	return *c.macs
}

func (c *MatMulCircuit) Define(api frontend.API) error {
	s := c.spec
	rc := newRangechecker(api)

	// Every INT8 value entering the circuit is PROVED 8-bit. A binary-field system gets
	// this from the representation; a prime field has to pay for it, and the campaign
	// declines to hand gnark a discount the other four systems did not get.
	for i := range c.X {
		rc.Check(api.Add(c.X[i], 128), 8)
	}
	if c.regime == RegimeA {
		for i := range c.W {
			rc.Check(api.Add(c.W[i], 128), 8)
		}
	}

	macs := 0
	for m := 0; m < s.M; m++ {
		for n := 0; n < s.N; n++ {
			var acc frontend.Variable = 0
			for k := 0; k < s.K; k++ {
				var term frontend.Variable
				if c.regime == RegimeA {
					term = api.Mul(c.X[m*s.K+k], c.W[k*s.N+n])
				} else {
					term = api.Mul(c.X[m*s.K+k], int(c.wConst[k*s.N+n]))
				}
				acc = api.Add(acc, term)
				macs++
			}
			api.AssertIsEqual(acc, c.Out[m*s.N+n])
		}
	}

	if c.macs != nil {
		*c.macs = macs
	}
	// THE MANDATORY GUARD. For a published bank task s.MACs is bench/TASKS.md's frozen
	// figure and this is the assertion that the expression still computes the task the
	// denominator refers to. For a probe s.MACs is the probe's own arithmetic; the check
	// still runs, because a probe whose expression drifted is a probe that measures
	// nothing, and the error text names bench/TASKS.md either way so a drift is greppable.
	if macs != s.MACs {
		return MACAssertionError(s.Label, macs, s.MACs)
	}
	return nil
}
