package main

// maccost and relubits isolate two costs that EXPRESSION.md quotes as tables.
//
// Both write to bench/data/ through the campaign's own scripts rather than being computed
// in prose. Every figure in bench/systems/gnark/EXPRESSION.md §3 and §4 comes from one of
// these two outputs, so a reader can re-derive the table instead of trusting it.

import (
	"fmt"
	"strconv"

	gb "github.com/viaas/zk-prover-bench/gnark"

	"github.com/consensys/gnark/frontend"
)

// dotProduct is ONE dot product of length n, with NO range checks. That isolation is the
// point: §3 prices the MAC, and §2 prices the proof that the operands are 8-bit. Mixing them
// would make it impossible to say which of the two a prime field is paying for.
type dotProduct struct {
	X   []frontend.Variable `gnark:",secret"`
	W   []frontend.Variable `gnark:",secret"` // nil when the weights are Go constants
	Out frontend.Variable   `gnark:",public"`

	wConst []int
}

func (c *dotProduct) Define(api frontend.API) error {
	var acc frontend.Variable = 0
	for i := range c.X {
		if c.W != nil {
			acc = api.Add(acc, api.Mul(c.X[i], c.W[i]))
		} else {
			// A linear combination whose coefficients are CONSTANTS. In R1CS this folds into
			// the linear expression and emits no multiplication constraint at all; the whole
			// dot product then costs the single AssertIsEqual below.
			acc = api.Add(acc, api.Mul(c.X[i], c.wConst[i]))
		}
	}
	api.AssertIsEqual(acc, c.Out)
	return nil
}

func mkDot(n int, witnessWeights, fill bool) *dotProduct {
	c := &dotProduct{X: make([]frontend.Variable, n)}
	w := make([]int, n)
	for i := range w {
		w[i] = (i % 255) - 127
	}
	if witnessWeights {
		c.W = make([]frontend.Variable, n)
	} else {
		c.wConst = w
	}
	if fill {
		sum := 0
		for i := 0; i < n; i++ {
			x := (i*37)%255 - 127
			c.X[i] = x
			if c.W != nil {
				c.W[i] = w[i]
			}
			sum += x * w[i]
		}
		c.Out = sum
	}
	return c
}

// maccost measures the four corners of EXPRESSION.md §3: {R1CS, SparseR1CS} x {weights in the
// witness, weights as circuit constants}, on one 256-MAC dot product.
func maccost() {
	const n = 256
	for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
		for _, witnessW := range []bool{false, true} {
			regime := gb.RegimeB
			if witnessW {
				regime = gb.RegimeA
			}
			label := fmt.Sprintf("dot%d", n)
			b, err := gb.CompileGeneric(label, mkDot(n, witnessW, false), mkDot(n, witnessW, true), bk)
			if err != nil {
				emit("MACCOST", label, bk, regime, "COMPILE_FAILED", err, nil)
				continue
			}
			// Proved and verified, not merely compiled: a constraint count for a circuit
			// nobody proved is a count for a circuit that might not be satisfiable.
			k, err := b.Setup()
			if err != nil {
				emit("MACCOST", label, bk, regime, "SETUP_FAILED", err, nil)
				continue
			}
			full, public, err := b.Witnesses()
			if err != nil {
				emit("MACCOST", label, bk, regime, "WITNESS_FAILED", err, nil)
				continue
			}
			p, _, err := b.Prove(k, full, 0)
			if err != nil {
				emit("MACCOST", label, bk, regime, "PROVE_FAILED", err, nil)
				continue
			}
			if _, err := b.Verify(k, p, public); err != nil {
				emit("MACCOST", label, bk, regime, "VERIFY_FAILED", err, nil)
				continue
			}
			st := b.Stats()
			weights := "constant"
			if witnessW {
				weights = "witness"
			}
			emit("MACCOST", label, bk, regime, "OK", nil, []string{
				"macs=" + strconv.Itoa(n),
				"weights=" + weights,
				"range_checks=none_this_probe_prices_the_MAC_only",
				"constraints=" + strconv.Itoa(st.Constraints),
				"constraints_per_mac=" + strconv.FormatFloat(float64(st.Constraints)/float64(n), 'f', 4, 64),
				"verified=true",
			})
		}
	}
}

// relubits measures each gadget's MARGINAL cost as a function of B, on R1CS.
//
// Marginal, not total/n: std/rangecheck's commit variant amortizes a shared lookup table over
// every check in the circuit, so a total divided by n prices that table into the gadget and
// reports a figure that keeps moving with n.
func relubits() {
	count := func(g gb.ReluGadget, bits, n int) (int, error) {
		p := &activationOnly{
			In: make([]frontend.Variable, n), Out: make([]frontend.Variable, n),
			gadget: g, bits: bits,
		}
		b, err := gb.CompileGeneric(fmt.Sprintf("relu-%s-b%d-n%d", g, bits, n), p, p, gb.Groth16)
		if err != nil {
			return 0, err
		}
		return b.Stats().Constraints, nil
	}
	// 19, 29 and 38 are T2's own measured per-layer widths; 20 is T3's layer 0. The rest
	// bracket them.
	for _, g := range []gb.ReluGadget{gb.ReluToBinary, gb.ReluHintedSign} {
		for _, bits := range []int{8, 16, 19, 20, 24, 29, 38, 44, 48} {
			c1, err1 := count(g, bits, 448)
			c2, err2 := count(g, bits, 896)
			if err1 != nil || err2 != nil {
				emit("RELUBITS", string(g), gb.Groth16, "-", "COMPILE_FAILED", firstErr(err1, err2), nil)
				continue
			}
			emit("RELUBITS", string(g), gb.Groth16, "-", "OK", nil, []string{
				"bits=" + strconv.Itoa(bits),
				"c448=" + strconv.Itoa(c1),
				"c896=" + strconv.Itoa(c2),
				"marginal_r1cs_per_activation=" + strconv.FormatFloat(float64(c2-c1)/448.0, 'f', 3, 64),
			})
		}
	}
}

func firstErr(errs ...error) error {
	for _, e := range errs {
		if e != nil {
			return e
		}
	}
	return nil
}
