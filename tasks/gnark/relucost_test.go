package gnarkbench

import (
	"fmt"
	"testing"

	"github.com/consensys/gnark/frontend"
)

// TestReluCostVsBitWidth measures how each gadget's MARGINAL cost moves with B.
//
// The campaign needs this measured rather than quoted, for one reason: B is derived from the
// magnitude each ReLU site actually reaches, so T2 uses three different widths — 19, 29 and
// 38 bits — inside one circuit. An "R1CS per activation" figure quoted without its B is not
// a property of the gadget.
//
// The marginal cost is (C(896) − C(448))/448, not C(448)/448. std/rangecheck's commit
// variant amortizes a shared lookup table across every check in the circuit, so a total
// divided by n prices that table into the gadget and reports a number that keeps moving with
// n.
func TestReluCostVsBitWidth(t *testing.T) {
	count := func(g ReluGadget, bits, n int) int {
		p := mkProbe(g, n)
		p.bits = bits
		ccs, err := frontend.Compile(Curve.ScalarField(), builderFor(Groth16), p)
		if err != nil {
			t.Fatal(err)
		}
		return ccs.GetNbConstraints()
	}
	// 19, 29 and 38 are T2's own measured widths; 44 and 48 extend the range past them.
	for _, g := range []ReluGadget{ReluToBinary, ReluHintedSign} {
		for _, b := range []int{8, 16, 19, 24, 29, 38, 44, 48} {
			c1, c2 := count(g, b, 448), count(g, b, 896)
			fmt.Printf("BCOST gadget=%-11s B=%-3d c448=%-7d c896=%-7d marginal_r1cs_per_activation=%.3f\n",
				g, b, c1, c2, float64(c2-c1)/448.0)
		}
	}
}
