package gnarkbench

import (
	"fmt"
	"math"
	"math/big"
)

// A1Margin is the factor bench/TASKS.md Amendment A1 requires between the largest
// intermediate any instance actually reaches and the int64 bound. Two, not one: a witness
// generator that emits at exactly the bound is a witness generator that will overflow the
// first time anyone changes a seed.
const A1Margin = 2

// bn254ScalarFieldBits is BN254's scalar field modulus r, ~2^254. Recorded so that the two
// overflow facts stay two facts and never collapse into one:
//
//	(1) the int64 REFERENCE arithmetic in this file could overflow and is asserted not to;
//	(2) the FIELD the circuit computes in could not overflow at these magnitudes at all,
//	    because r ≈ 2.19e76 and the largest intermediate here is ~1e13.
//
// (2) does not license (1). The reference is what the assertion protects; the field is
// what a reader might otherwise assume the assertion is about.
const bn254ScalarFieldBits = 254

// Reference is the deterministic instance of a task: the INT8 witness, the INT32-and-wider
// reference outputs, and the measured magnitudes the circuit builder needs.
type Reference struct {
	Spec Spec

	// KindMatMul. X is M×K row-major, W is K×N row-major, Out is M×N row-major.
	X, W []int8
	Out  []int64

	// KindMLP. XB is Batch×In0 row-major; LW[l] is Layers[l].In × Layers[l].Out
	// row-major; OutB is Batch×(last layer's Out) row-major.
	XB   []int8
	LW   [][]int8
	OutB []int64

	// ReluIn[l][k] is the value entering ReLU site k of layer l, for every batch item,
	// flattened. Only layers with ReLU=true have an entry.
	ReluIn map[int][]int64

	// MaxAbsIntermediate is the largest absolute value any accumulator reached at any
	// step, over the whole instance. It is the number Amendment A1 asks for.
	MaxAbsIntermediate int64

	// StaticWorstCase is the data-INDEPENDENT bound over all admissible INT8 inputs and
	// weights. It is not this instance's magnitude; it is the magnitude the instance was
	// allowed to have. A1 quotes both because a benign seed is not a proof of safety.
	StaticWorstCase *big.Int

	// ReluBits[l] is the bit width B this build uses for layer l's ReLU gadget.
	ReluBits map[int]int

	// ReluMaxAbs[l] is the measured max |x| over layer l's ReLU inputs, which is where
	// ReluBits[l] comes from. Nothing here is guessed.
	ReluMaxAbs map[int]int64
}

// A1Report is the sentence the runner prints and the report reads.
func (r *Reference) A1Report() string {
	margin := "n/a"
	if r.MaxAbsIntermediate > 0 {
		margin = fmt.Sprintf("%.4g", float64(math.MaxInt64)/float64(r.MaxAbsIntermediate))
	}
	return fmt.Sprintf(
		"A1 max_abs_intermediate=%d int64_max=%d margin=%sx static_worst_case=%s "+
			"field=BN254_scalar_~2^%d field_overflow_impossible_at_these_magnitudes=true",
		r.MaxAbsIntermediate, int64(math.MaxInt64), margin, r.StaticWorstCase.String(), bn254ScalarFieldBits)
}

// assertA1 is the refusal. It runs before any circuit is built and before any witness is
// serialized, so a task that cannot be represented safely never becomes a measurement.
func (r *Reference) assertA1() error {
	limit := new(big.Int).SetInt64(math.MaxInt64)
	limit.Div(limit, big.NewInt(A1Margin))
	got := new(big.Int).SetInt64(r.MaxAbsIntermediate)
	if got.Cmp(limit) > 0 {
		return fmt.Errorf(
			"%s: max |intermediate| is %d, which leaves less than the factor-%d margin under int64 max (%d); "+
				"bench/TASKS.md Amendment A1 forbids emitting this circuit",
			r.Spec.Label, r.MaxAbsIntermediate, A1Margin, int64(math.MaxInt64))
	}
	return nil
}

// bitsFor returns the number of bits needed to hold |v|, i.e. the smallest B with |v| < 2^B.
func bitsFor(v int64) int {
	if v < 0 {
		v = -v
	}
	b := 0
	for u := uint64(v); u > 0; u >>= 1 {
		b++
	}
	return b
}

// NewReference builds the instance for a task and asserts A1 on it.
func NewReference(spec Spec) (*Reference, error) {
	r := &Reference{
		Spec:       spec,
		ReluIn:     map[int][]int64{},
		ReluBits:   map[int]int{},
		ReluMaxAbs: map[int]int64{},
	}
	switch spec.Kind {
	case KindMatMul:
		if err := r.buildMatMul(); err != nil {
			return nil, err
		}
	case KindMLP:
		if err := r.buildMLP(); err != nil {
			return nil, err
		}
	default:
		return nil, fmt.Errorf("%s: unknown kind %q", spec.Label, spec.Kind)
	}
	if err := r.assertA1(); err != nil {
		return nil, err
	}
	return r, nil
}

func (r *Reference) buildMatMul() error {
	s := r.Spec
	// The static worst case for a K-term INT8 dot product: K · 128 · 128. Computed in
	// big.Int and checked against int64 BEFORE the int64 accumulation below runs, so the
	// accumulation cannot be the thing that hides its own overflow.
	r.StaticWorstCase = new(big.Int).Mul(big.NewInt(int64(s.K)), big.NewInt(128*128))
	if r.StaticWorstCase.Cmp(new(big.Int).SetInt64(math.MaxInt64/A1Margin)) > 0 {
		return fmt.Errorf("%s: static worst case %s exceeds the int64 budget; the int64 reference below would be unsound",
			s.Label, r.StaticWorstCase)
	}

	rng := newRNG(s.Seed)
	r.X = make([]int8, s.M*s.K)
	for i := range r.X {
		r.X[i] = int8(rng.int8())
	}
	r.W = make([]int8, s.K*s.N)
	for i := range r.W {
		r.W[i] = int8(rng.int8())
	}

	r.Out = make([]int64, s.M*s.N)
	var maxAbs int64
	for m := 0; m < s.M; m++ {
		for n := 0; n < s.N; n++ {
			var acc int64
			for k := 0; k < s.K; k++ {
				acc += int64(r.X[m*s.K+k]) * int64(r.W[k*s.N+n])
				if a := abs64(acc); a > maxAbs {
					maxAbs = a
				}
			}
			r.Out[m*s.N+n] = acc
		}
	}
	r.MaxAbsIntermediate = maxAbs
	return nil
}

func (r *Reference) buildMLP() error {
	s := r.Spec

	// The MLP's static worst case DOES exceed int64 at layer 4 — that is exactly what
	// Amendment A1 records (1.44e19 against a 9.22e18 bound), and it is why the reference
	// forward pass below is computed in big.Int and only then narrowed to int64. Computing
	// it in int64 and then asserting on the int64 result would be asserting with the
	// arithmetic under suspicion.
	bound := big.NewInt(128)
	for _, l := range s.Layers {
		// per layer: In terms, each |prev| · 127-or-128
		bound = new(big.Int).Mul(bound, big.NewInt(128))
		bound = new(big.Int).Mul(bound, big.NewInt(int64(l.In)))
	}
	r.StaticWorstCase = bound

	rng := newRNG(s.Seed)
	in0 := s.Layers[0].In
	r.XB = make([]int8, s.Batch*in0)
	for i := range r.XB {
		r.XB[i] = int8(rng.int8())
	}
	r.LW = make([][]int8, len(s.Layers))
	for l, lay := range s.Layers {
		r.LW[l] = make([]int8, lay.In*lay.Out)
		for i := range r.LW[l] {
			r.LW[l][i] = int8(rng.int8())
		}
	}

	last := s.Layers[len(s.Layers)-1].Out
	r.OutB = make([]int64, s.Batch*last)
	maxAbs := new(big.Int)
	tmp := new(big.Int)

	for b := 0; b < s.Batch; b++ {
		cur := make([]*big.Int, in0)
		for i := 0; i < in0; i++ {
			cur[i] = big.NewInt(int64(r.XB[b*in0+i]))
		}
		for l, lay := range s.Layers {
			next := make([]*big.Int, lay.Out)
			for o := 0; o < lay.Out; o++ {
				acc := new(big.Int)
				for i := 0; i < lay.In; i++ {
					tmp.Mul(cur[i], big.NewInt(int64(r.LW[l][i*lay.Out+o])))
					acc.Add(acc, tmp)
					if a := new(big.Int).Abs(acc); a.Cmp(maxAbs) > 0 {
						maxAbs = a
					}
				}
				next[o] = acc
			}
			if lay.ReLU {
				for o := 0; o < lay.Out; o++ {
					v, ok := toInt64(next[o])
					if !ok {
						return fmt.Errorf("%s: ReLU input at layer %d does not fit int64 (%s); A1 refuses the emit",
							s.Label, l, next[o])
					}
					r.ReluIn[l] = append(r.ReluIn[l], v)
					if a := abs64(v); a > r.ReluMaxAbs[l] {
						r.ReluMaxAbs[l] = a
					}
					if next[o].Sign() < 0 {
						next[o] = new(big.Int)
					}
				}
			}
			cur = next
		}
		for o := 0; o < last; o++ {
			v, ok := toInt64(cur[o])
			if !ok {
				return fmt.Errorf("%s: output %d does not fit int64 (%s); A1 refuses the emit", s.Label, o, cur[o])
			}
			r.OutB[b*last+o] = v
		}
	}

	m64, ok := toInt64(maxAbs)
	if !ok {
		return fmt.Errorf("%s: max |intermediate| is %s, which does not fit int64 at all; A1 refuses the emit",
			s.Label, maxAbs)
	}
	r.MaxAbsIntermediate = m64

	// B is MEASURED, not guessed: one bit above the largest magnitude this instance's ReLU
	// site actually reached. It is a per-site bound, so the range check cost tracks the
	// data rather than the 51-bit static worst case.
	//
	// THE TRADE, DECLARED. A static bound would make the circuit valid for every admissible
	// INT8 instance; a measured bound makes it valid for instances no larger than this one.
	// The campaign chose measured — it is what the gadget's cost should be reported at, and
	// the static bound is published beside it so the reader can price the other choice.
	for l := range r.ReluIn {
		r.ReluBits[l] = bitsFor(r.ReluMaxAbs[l]) + 1
	}
	return nil
}

func abs64(v int64) int64 {
	if v < 0 {
		return -v
	}
	return v
}

func toInt64(v *big.Int) (int64, bool) {
	if !v.IsInt64() {
		return 0, false
	}
	return v.Int64(), true
}

// ReluSites is the number of activations this instance actually performs, counted from the
// reference forward pass. The runner asserts it against Spec.ReLUs.
func (r *Reference) ReluSites() int {
	n := 0
	for _, v := range r.ReluIn {
		n += len(v)
	}
	return n
}
