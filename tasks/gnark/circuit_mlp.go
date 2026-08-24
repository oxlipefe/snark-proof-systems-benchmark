package gnarkbench

import (
	"github.com/consensys/gnark/frontend"
)

// MLPCircuit expresses T2 (200-256-128-64-1, ReLU after layers 1–3) and T3 (the same
// network, batch of 8, in ONE proof).
//
// Amendment A1 governs the arithmetic: NO requantization between layers, accumulators
// carry full width. The ReLU bit widths therefore grow layer by layer, and they are taken
// from the measured magnitudes in Reference, never guessed. Reference.assertA1 has already
// refused the emit if this instance came within a factor of two of int64.
//
// Regimes are exactly as in MatMulCircuit: A puts every weight in the witness and
// range-checks it; B bakes the weights in as Go constants and range-checks only the
// network input.
type MLPCircuit struct {
	X   []frontend.Variable `gnark:",secret"` // Batch × Layers[0].In
	W   []frontend.Variable `gnark:",secret"` // regime A only: all layers, concatenated
	Out []frontend.Variable `gnark:",public"` // Batch × last layer's Out

	spec     Spec
	regime   Regime
	gadget   ReluGadget
	wConst   [][]int8
	reluBits map[int]int
	macs     *int
	relus    *int
}

func mlpWeightLen(s Spec) int {
	n := 0
	for _, l := range s.Layers {
		n += l.In * l.Out
	}
	return n
}

func newMLP(spec Spec, regime Regime, gadget ReluGadget, ref *Reference) *MLPCircuit {
	last := spec.Layers[len(spec.Layers)-1].Out
	c := &MLPCircuit{
		X:        make([]frontend.Variable, spec.Batch*spec.Layers[0].In),
		Out:      make([]frontend.Variable, spec.Batch*last),
		spec:     spec,
		regime:   regime,
		gadget:   gadget,
		reluBits: ref.ReluBits,
		macs:     new(int),
		relus:    new(int),
	}
	if regime == RegimeA {
		c.W = make([]frontend.Variable, mlpWeightLen(spec))
	} else {
		c.wConst = ref.LW
	}
	return c
}

// NewMLPCircuit returns the compile-time template.
func NewMLPCircuit(spec Spec, regime Regime, gadget ReluGadget, ref *Reference) *MLPCircuit {
	return newMLP(spec, regime, gadget, ref)
}

// AssignMLP fills the template with the reference instance.
func AssignMLP(spec Spec, regime Regime, gadget ReluGadget, ref *Reference) *MLPCircuit {
	c := newMLP(spec, regime, gadget, ref)
	for i, v := range ref.XB {
		c.X[i] = int(v)
	}
	if regime == RegimeA {
		off := 0
		for l := range spec.Layers {
			for i, v := range ref.LW[l] {
				c.W[off+i] = int(v)
			}
			off += len(ref.LW[l])
		}
	}
	for i, v := range ref.OutB {
		c.Out[i] = v
	}
	return c
}

func (c *MLPCircuit) EmittedMACs() int {
	if c.macs == nil {
		return 0
	}
	return *c.macs
}

func (c *MLPCircuit) EmittedReLUs() int {
	if c.relus == nil {
		return 0
	}
	return *c.relus
}

func (c *MLPCircuit) Define(api frontend.API) error {
	s := c.spec
	rc := newRangechecker(api)

	for i := range c.X {
		rc.Check(api.Add(c.X[i], 128), 8)
	}
	if c.regime == RegimeA {
		for i := range c.W {
			rc.Check(api.Add(c.W[i], 128), 8)
		}
	}

	// Offsets of each layer's weight block inside the flat W slice.
	offs := make([]int, len(s.Layers))
	off := 0
	for l, lay := range s.Layers {
		offs[l] = off
		off += lay.In * lay.Out
	}

	macs, relus := 0, 0
	last := s.Layers[len(s.Layers)-1].Out

	for b := 0; b < s.Batch; b++ {
		cur := make([]frontend.Variable, s.Layers[0].In)
		for i := range cur {
			cur[i] = c.X[b*s.Layers[0].In+i]
		}
		for l, lay := range s.Layers {
			next := make([]frontend.Variable, lay.Out)
			for o := 0; o < lay.Out; o++ {
				var acc frontend.Variable = 0
				for i := 0; i < lay.In; i++ {
					var term frontend.Variable
					if c.regime == RegimeA {
						term = api.Mul(cur[i], c.W[offs[l]+i*lay.Out+o])
					} else {
						term = api.Mul(cur[i], int(c.wConst[l][i*lay.Out+o]))
					}
					acc = api.Add(acc, term)
					macs++
				}
				next[o] = acc
			}
			if lay.ReLU {
				nbBits := c.reluBits[l]
				if nbBits == 0 {
					// A ReLU site with no measured bound is a build that would range-check
					// to zero bits and silently constrain the activation to be zero. Refuse.
					return errReluBitsMissing(s.Label, l)
				}
				for o := range next {
					next[o] = Relu(api, rc, c.gadget, next[o], nbBits)
					relus++
				}
			}
			cur = next
		}
		for o := 0; o < last; o++ {
			api.AssertIsEqual(cur[o], c.Out[b*last+o])
		}
	}

	if c.macs != nil {
		*c.macs = macs
	}
	if c.relus != nil {
		*c.relus = relus
	}
	if macs != s.MACs {
		return MACAssertionError(s.Label, macs, s.MACs)
	}
	// Activations are counted and reported separately from MACs and are NEVER folded into
	// them; bench/TASKS.md is explicit about that and this is where it is enforced.
	if relus != s.ReLUs {
		return errReluCount(s.Label, relus, s.ReLUs)
	}
	return nil
}
