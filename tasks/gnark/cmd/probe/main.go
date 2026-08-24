// Command probe measures the things a benchmark of somebody else's system has to establish
// before it is allowed to report a limit as theirs.
//
// Modes:
//
//	minwidth    Does gnark have a minimum output-layer width? DeepProve cannot prove a dense
//	            layer with fewer than 4 outputs and jolt-atlas cannot below 2. T2 ends in a
//	            64→1 layer, so this runs FIRST, before anything else in the campaign. It is
//	            the single most transferable trap the campaign has found.
//	example     Runs gnark's OWN example circuits, unchanged, through the same
//	            compile→setup→prove→verify path a measured cell takes. This is the check
//	            that stopped jolt-atlas from publishing three of our own expression errors
//	            as somebody else's limits, and no limit in this campaign is attributed to
//	            gnark until it has passed.
//	padding     Does gnark pad to a power of two, and by how much? Reads the FFT domain
//	            cardinality OUT of the key the backend actually built and compares it with
//	            the size gnark's source says it should build. Measurement, not a formula.
//	relu        The cost of both ReLU gadgets, measured on T2 itself and in isolation.
//	rangecheck  How std/rangecheck's per-value cost moves with the number of values. It
//	            amortizes a shared lookup table, so a per-value figure is NOT a constant and
//	            nothing in this campaign may hardcode one.
//	maccost     What one MAC costs in each of the four corners: {R1CS, SparseR1CS} x
//	            {weights in the witness, weights as circuit constants}, with NO range checks,
//	            so the MAC's price is separated from the price of proving 8-bit-ness.
//	relubits    Each ReLU gadget's marginal cost as a function of the bit width B, which is
//	            not a constant and is derived per ReLU site from measured magnitudes.
//	filler      Compile/setup/prove a circuit of exactly N multiplication constraints, so
//	            the ceiling can be reported as a MEASURED INTERVAL — largest that worked,
//	            smallest that failed — and never interpolated.
package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"

	gb "github.com/viaas/zk-prover-bench/gnark"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/examples/cubic"
	"github.com/consensys/gnark/examples/mimc"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/logger"
	"github.com/consensys/gnark/std/rangecheck"
	"github.com/rs/zerolog"
)

func main() {
	logger.Set(zerolog.New(os.Stderr).Level(zerolog.Disabled))
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: probe <minwidth|example|padding|relu|relubits|maccost|rangecheck|filler> [args]")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "minwidth":
		minwidth()
	case "example":
		example()
	case "padding":
		padding()
	case "relu":
		relu()
	case "rangecheck":
		rangecheckProbe()
	case "maccost":
		maccost()
	case "relubits":
		relubits()
	case "filler":
		filler(os.Args[2:])
	default:
		fmt.Fprintf(os.Stderr, "unknown mode %q\n", os.Args[1])
		os.Exit(2)
	}
}

// ---------------------------------------------------------------- minwidth

func minwidth() {
	// Widths 1, 2 and 4 are the three thresholds the campaign has already hit in two other
	// systems. 1×1 is the degenerate case that finds a floor on the whole circuit rather
	// than on the layer.
	for _, label := range []string{"p1x1", "p2x1", "p64x1", "p64x2", "p64x4"} {
		for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
			for _, reg := range []gb.Regime{gb.RegimeA, gb.RegimeB} {
				runOneTask(label, bk, reg)
			}
		}
	}
	// And the real thing: T2 itself, whose last layer is 64→1. If the probes pass and T2
	// fails, the trap is not the width.
	for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
		runOneTask("t2", bk, gb.RegimeA)
	}
}

func runOneTask(label string, bk gb.Backend, reg gb.Regime) {
	spec, err := gb.Lookup(label)
	if err != nil {
		emit("MINWIDTH", label, bk, reg, "LOOKUP_FAILED", err, nil)
		return
	}
	b, err := gb.Compile(spec, reg, bk, gb.DefaultReluGadget)
	if err != nil {
		emit("MINWIDTH", label, bk, reg, "COMPILE_FAILED", err, nil)
		return
	}
	k, err := b.Setup()
	if err != nil {
		emit("MINWIDTH", label, bk, reg, "SETUP_FAILED", err, nil)
		return
	}
	full, public, err := b.Witnesses()
	if err != nil {
		emit("MINWIDTH", label, bk, reg, "WITNESS_FAILED", err, nil)
		return
	}
	p, d, err := b.Prove(k, full, 0)
	if err != nil {
		emit("MINWIDTH", label, bk, reg, "PROVE_FAILED", err, nil)
		return
	}
	vd, err := b.Verify(k, p, public)
	if err != nil {
		emit("MINWIDTH", label, bk, reg, "VERIFY_FAILED", err, nil)
		return
	}
	st := b.Stats()
	emit("MINWIDTH", label, bk, reg, "OK", nil, []string{
		"out_width=" + strconv.Itoa(spec.Layers[len(spec.Layers)-1].Out),
		"macs=" + strconv.Itoa(spec.MACs),
		"constraints=" + strconv.Itoa(st.Constraints),
		"domain_measured=" + strconv.FormatUint(k.DomainCardinality, 10),
		"prove_ms=" + ms(d), "verify_ms=" + ms(vd),
		"proof_bytes=" + strconv.FormatInt(p.Bytes, 10),
	})
}

// ---------------------------------------------------------------- example

// The witness for the MiMC example is gnark's OWN published pair, copied verbatim from
// examples/mimc/mimc_test.go at v0.16.2. It is not recomputed here: a witness we computed
// ourselves would make a failure ambiguous between gnark's circuit and our arithmetic,
// which is precisely the ambiguity this mode exists to remove.
const (
	mimcPreImage = "16130099170765464552823636852555369511329944820189892919423002775646948828469"
	mimcHash     = "12886436712380113721405259596386800092738845035233065858332878701083870690753"
)

func example() {
	cases := []struct {
		name      string
		tmpl, asg frontend.Circuit
		source    string
	}{
		{"gnark-example-cubic", &cubic.Circuit{}, &cubic.Circuit{X: 3, Y: 35},
			"gnark_examples/cubic/cubic.go_v0.16.2_x^3+x+5=y_at_x=3"},
		{"gnark-example-mimc", &mimc.Circuit{}, &mimc.Circuit{PreImage: mimcPreImage, Hash: mimcHash},
			"gnark_examples/mimc/mimc_test.go_v0.16.2_verbatim"},
	}
	for _, c := range cases {
		for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
			b, err := gb.CompileGeneric(c.name, c.tmpl, c.asg, bk)
			if err != nil {
				emit("EXAMPLE", c.name, bk, "-", "COMPILE_FAILED", err, nil)
				continue
			}
			k, err := b.Setup()
			if err != nil {
				emit("EXAMPLE", c.name, bk, "-", "SETUP_FAILED", err, nil)
				continue
			}
			full, public, err := b.Witnesses()
			if err != nil {
				emit("EXAMPLE", c.name, bk, "-", "WITNESS_FAILED", err, nil)
				continue
			}
			p, d, err := b.Prove(k, full, 0)
			if err != nil {
				emit("EXAMPLE", c.name, bk, "-", "PROVE_FAILED", err, nil)
				continue
			}
			vd, err := b.Verify(k, p, public)
			if err != nil {
				emit("EXAMPLE", c.name, bk, "-", "VERIFY_FAILED", err, nil)
				continue
			}
			st := b.Stats()
			emit("EXAMPLE", c.name, bk, "-", "OK", nil, []string{
				"constraints=" + strconv.Itoa(st.Constraints),
				"public=" + strconv.Itoa(st.PublicVars),
				"domain_measured=" + strconv.FormatUint(k.DomainCardinality, 10),
				"setup_ms=" + ms(k.SetupD), "prove_ms=" + ms(d), "verify_ms=" + ms(vd),
				"proof_bytes=" + strconv.FormatInt(p.Bytes, 10),
				"proof_bytes_raw=" + strconv.FormatInt(p.BytesRaw, 10),
				"witness_source=" + c.source,
			})
		}
	}
}

// ---------------------------------------------------------------- padding

// fillerCircuit emits exactly N multiplication constraints and nothing else, so the
// relationship between constraint count and FFT domain size can be walked one constraint at
// a time across a power-of-two boundary.
type fillerCircuit struct {
	X frontend.Variable `gnark:",secret"`
	Y frontend.Variable `gnark:",public"`
	n int
}

func (c *fillerCircuit) Define(api frontend.API) error {
	acc := c.X
	for i := 0; i < c.n; i++ {
		acc = api.Mul(acc, c.X)
	}
	api.AssertIsEqual(acc, c.Y)
	return nil
}

func fillerAssignment(n int) (*fillerCircuit, error) {
	// x = 1 keeps x^(n+1) = 1 for any n, so the witness is valid at every size without a
	// modular exponentiation of our own that could itself be wrong.
	return &fillerCircuit{X: 1, Y: 1, n: n}, nil
}

func padding() {
	// Sizes chosen to straddle 2^k boundaries from both sides. If gnark pads, the domain is
	// flat across each interval and jumps at the boundary; if it does not, the domain
	// tracks the count.
	for _, n := range []int{100, 1000, 1023, 1024, 1025, 2000, 4095, 4096, 4097, 10000} {
		for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
			tmpl := &fillerCircuit{n: n}
			asg, _ := fillerAssignment(n)
			b, err := gb.CompileGeneric(fmt.Sprintf("filler-%d", n), tmpl, asg, bk)
			if err != nil {
				emit("PADDING", fmt.Sprintf("filler-%d", n), bk, "-", "COMPILE_FAILED", err, nil)
				continue
			}
			k, err := b.Setup()
			if err != nil {
				emit("PADDING", fmt.Sprintf("filler-%d", n), bk, "-", "SETUP_FAILED", err, nil)
				continue
			}
			st := b.Stats()
			base := uint64(st.Constraints)
			src := "fft.NewDomain(nbConstraints)"
			if bk == gb.Plonk {
				base = uint64(st.Constraints + st.PublicVars)
				src = "fft.NewDomain(nbConstraints+nbPublic)"
			}
			derived := ecc.NextPowerOfTwo(base)
			agree := "true"
			if derived != k.DomainCardinality {
				agree = "false"
			}
			emit("PADDING", fmt.Sprintf("filler-%d", n), bk, "-", "OK", nil, []string{
				"constraints=" + strconv.Itoa(st.Constraints),
				"public=" + strconv.Itoa(st.PublicVars),
				"domain_measured=" + strconv.FormatUint(k.DomainCardinality, 10),
				"domain_measured_from=" + k.DomainSource,
				"domain_derived=" + strconv.FormatUint(derived, 10),
				"domain_derived_from=" + src,
				"derivation_agrees=" + agree,
				"padding_ratio=" + strconv.FormatFloat(float64(k.DomainCardinality)/float64(base), 'f', 4, 64),
				"pk_bytes=" + strconv.FormatInt(k.PkSize, 10),
			})
		}
	}
}

// ---------------------------------------------------------------- relu

// activationOnly isolates the gadget: n activations at bit width bits, and nothing else.
// The DELTA between n and 2n is the marginal cost, which is the honest per-activation
// figure — std/rangecheck amortizes a shared lookup table, so total/n at a single n prices
// the table into the gadget.
type activationOnly struct {
	In  []frontend.Variable `gnark:",secret"`
	Out []frontend.Variable `gnark:",public"`

	gadget gb.ReluGadget
	bits   int
}

func (c *activationOnly) Define(api frontend.API) error {
	rc := rangecheck.New(api)
	for i := range c.In {
		api.AssertIsEqual(gb.Relu(api, rc, c.gadget, c.In[i], c.bits), c.Out[i])
	}
	return nil
}

func mkActivation(g gb.ReluGadget, n, bits int, fill bool) *activationOnly {
	c := &activationOnly{In: make([]frontend.Variable, n), Out: make([]frontend.Variable, n), gadget: g, bits: bits}
	if fill {
		for i := range c.In {
			v := int64(i%7) - 3 // covers negative, zero and positive
			c.In[i] = v
			if v < 0 {
				c.Out[i] = 0
			} else {
				c.Out[i] = v
			}
		}
	}
	return c
}

func relu() {
	// 1) In isolation, at one declared bit width, with the marginal cost separated from the
	//    amortized table.
	const bits = 24
	for _, g := range []gb.ReluGadget{gb.ReluToBinary, gb.ReluHintedSign} {
		for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
			var c1, c2 int
			for _, n := range []int{448, 896} {
				tmpl := mkActivation(g, n, bits, false)
				asg := mkActivation(g, n, bits, true)
				b, err := gb.CompileGeneric(fmt.Sprintf("relu-%s-%d", g, n), tmpl, asg, bk)
				if err != nil {
					emit("RELU", string(g), bk, "-", "COMPILE_FAILED", err, nil)
					continue
				}
				// Correctness is not assumed from the unit test alone: the isolated gadget
				// is proved and verified here too, at the size the bank actually uses.
				k, err := b.Setup()
				if err != nil {
					emit("RELU", string(g), bk, "-", "SETUP_FAILED", err, nil)
					continue
				}
				full, public, err := b.Witnesses()
				if err != nil {
					emit("RELU", string(g), bk, "-", "WITNESS_FAILED", err, nil)
					continue
				}
				p, _, err := b.Prove(k, full, 0)
				if err != nil {
					emit("RELU", string(g), bk, "-", "PROVE_FAILED", err, nil)
					continue
				}
				if _, err := b.Verify(k, p, public); err != nil {
					emit("RELU", string(g), bk, "-", "VERIFY_FAILED", err, nil)
					continue
				}
				st := b.Stats()
				if n == 448 {
					c1 = st.Constraints
				} else {
					c2 = st.Constraints
				}
				emit("RELU", string(g), bk, "-", "OK", nil, []string{
					"mode=isolated", "n=" + strconv.Itoa(n), "bits=" + strconv.Itoa(bits),
					"constraints=" + strconv.Itoa(st.Constraints),
					"total_per_activation=" + strconv.FormatFloat(float64(st.Constraints)/float64(n), 'f', 3, 64),
					"verified=true",
				})
			}
			if c1 > 0 && c2 > 0 {
				emit("RELU", string(g), bk, "-", "OK", nil, []string{
					"mode=marginal", "bits=" + strconv.Itoa(bits),
					"c448=" + strconv.Itoa(c1), "c896=" + strconv.Itoa(c2),
					"marginal_per_activation=" + strconv.FormatFloat(float64(c2-c1)/448.0, 'f', 3, 64),
				})
			}
		}
	}

	// 2) On T2 itself, at T2's own measured per-layer bit widths, which is the number that
	//    actually decides which gadget the bank keeps.
	for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
		base := map[gb.ReluGadget]int{}
		for _, g := range []gb.ReluGadget{gb.ReluToBinary, gb.ReluHintedSign} {
			b, err := gb.Compile(gb.Specs["t2"], gb.RegimeA, bk, g)
			if err != nil {
				emit("RELU", string(g), bk, gb.RegimeA, "COMPILE_FAILED", err, nil)
				continue
			}
			st := b.Stats()
			base[g] = st.Constraints
			emit("RELU", string(g), bk, gb.RegimeA, "OK", nil, []string{
				"mode=t2_in_situ", "constraints=" + strconv.Itoa(st.Constraints),
				"relus=" + strconv.Itoa(b.EmittedReLUs),
				"relu_bits=" + reluBits(b),
			})
		}
		if base[gb.ReluToBinary] > 0 && base[gb.ReluHintedSign] > 0 {
			d := base[gb.ReluToBinary] - base[gb.ReluHintedSign]
			emit("RELU", "delta", bk, gb.RegimeA, "OK", nil, []string{
				"mode=t2_delta",
				"tobinary=" + strconv.Itoa(base[gb.ReluToBinary]),
				"hintedsign=" + strconv.Itoa(base[gb.ReluHintedSign]),
				"delta=" + strconv.Itoa(d),
				"delta_per_activation=" + strconv.FormatFloat(float64(d)/448.0, 'f', 3, 64),
				"cheaper=" + cheaper(base[gb.ReluToBinary], base[gb.ReluHintedSign]),
			})
		}
	}
}

func cheaper(tobin, hinted int) string {
	if hinted < tobin {
		return "hintedsign"
	}
	return "tobinary"
}

func reluBits(b *gb.Build) string {
	parts := []string{}
	for l := 0; l < 8; l++ {
		if bits, ok := b.Ref.ReluBits[l]; ok {
			parts = append(parts, fmt.Sprintf("L%d:%d(max=%d)", l, bits, b.Ref.ReluMaxAbs[l]))
		}
	}
	if len(parts) == 0 {
		return "-"
	}
	return strings.Join(parts, "|")
}

// ---------------------------------------------------------------- rangecheck

// rcOnly performs exactly n 8-bit range checks and nothing else.
type rcOnly struct {
	In []frontend.Variable `gnark:",secret"`
	Y  frontend.Variable   `gnark:",public"`
	n  int
}

func (c *rcOnly) Define(api frontend.API) error {
	rc := rangecheck.New(api)
	for i := range c.In {
		rc.Check(api.Add(c.In[i], 128), 8)
	}
	api.AssertIsEqual(c.Y, c.Y)
	return nil
}

func rangecheckProbe() {
	// std/rangecheck's commit variant AMORTIZES a shared lookup table across every Check in
	// the circuit. The per-value cost is therefore a function of how many values the circuit
	// checks, NOT a constant of the gadget. This mode exists so that no script in this
	// campaign ever hardcodes one — report.py reports totals.
	for _, n := range []int{16, 64, 256, 448, 1024, 4096, 65792} {
		for _, bk := range []gb.Backend{gb.Groth16, gb.Plonk} {
			tmpl := &rcOnly{In: make([]frontend.Variable, n), n: n}
			asg := &rcOnly{In: make([]frontend.Variable, n), Y: 0, n: n}
			for i := range asg.In {
				asg.In[i] = int64(i%255) - 127
			}
			b, err := gb.CompileGeneric(fmt.Sprintf("rc-%d", n), tmpl, asg, bk)
			if err != nil {
				emit("RANGECHECK", fmt.Sprintf("rc-%d", n), bk, "-", "COMPILE_FAILED", err, nil)
				continue
			}
			st := b.Stats()
			emit("RANGECHECK", fmt.Sprintf("rc-%d", n), bk, "-", "OK", nil, []string{
				"n=" + strconv.Itoa(n),
				"constraints=" + strconv.Itoa(st.Constraints),
				"per_value=" + strconv.FormatFloat(float64(st.Constraints)/float64(n), 'f', 4, 64),
				"note=per_value_is_NOT_a_constant_it_amortizes_a_shared_table",
			})
		}
	}
}

// ---------------------------------------------------------------- filler

// filler walks the ceiling. The campaign reports the ceiling as an INTERVAL — the largest
// size that completed and the smallest that failed — and never as an interpolated point.
func filler(args []string) {
	if len(args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: probe filler <n-constraints> <groth16|plonk> [compile|setup|prove]")
		os.Exit(2)
	}
	n, err := strconv.Atoi(args[0])
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(2)
	}
	bk, err := gb.ParseBackend(args[1])
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(2)
	}
	stage := "prove"
	if len(args) > 2 {
		stage = args[2]
	}
	label := fmt.Sprintf("filler-%d", n)

	tmpl := &fillerCircuit{n: n}
	asg, _ := fillerAssignment(n)
	t0 := time.Now()
	b, err := gb.CompileGeneric(label, tmpl, asg, bk)
	if err != nil {
		emit("FILLER", label, bk, "-", "COMPILE_FAILED", err, nil)
		os.Exit(13)
	}
	st := b.Stats()
	fields := []string{"n=" + strconv.Itoa(n), "constraints=" + strconv.Itoa(st.Constraints),
		"compile_ms=" + ms(time.Since(t0))}
	if stage == "compile" {
		emit("FILLER", label, bk, "-", "OK", nil, fields)
		return
	}
	k, err := b.Setup()
	if err != nil {
		emit("FILLER", label, bk, "-", "SETUP_FAILED", err, fields)
		os.Exit(20)
	}
	fields = append(fields, "setup_ms="+ms(k.SetupD), "domain_measured="+strconv.FormatUint(k.DomainCardinality, 10))
	if stage == "setup" {
		emit("FILLER", label, bk, "-", "OK", nil, fields)
		return
	}
	full, public, err := b.Witnesses()
	if err != nil {
		emit("FILLER", label, bk, "-", "WITNESS_FAILED", err, fields)
		os.Exit(21)
	}
	p, d, err := b.Prove(k, full, 0)
	if err != nil {
		emit("FILLER", label, bk, "-", "PROVE_FAILED", err, fields)
		os.Exit(30)
	}
	vd, err := b.Verify(k, p, public)
	if err != nil {
		emit("FILLER", label, bk, "-", "VERIFY_FAILED", err, fields)
		os.Exit(40)
	}
	fields = append(fields, "prove_ms="+ms(d), "verify_ms="+ms(vd),
		"proof_bytes="+strconv.FormatInt(p.Bytes, 10))
	emit("FILLER", label, bk, "-", "OK", nil, fields)
}

// ---------------------------------------------------------------- output

func emit(kind, label string, bk gb.Backend, reg gb.Regime, status string, err error, extra []string) {
	parts := []string{kind, "label=" + label, "backend=" + string(bk), "regime=" + string(reg), "status=" + status}
	parts = append(parts, extra...)
	if err != nil {
		msg := strings.ReplaceAll(err.Error(), ",", ";")
		msg = strings.ReplaceAll(msg, " ", "_")
		msg = strings.ReplaceAll(msg, "\n", "/")
		if len(msg) > 300 {
			msg = msg[:300]
		}
		parts = append(parts, "msg="+msg)
	}
	fmt.Println(strings.Join(parts, " "))
	os.Stdout.Sync()
}

func ms(d time.Duration) string {
	return strconv.FormatFloat(float64(d.Nanoseconds())/1e6, 'f', 3, 64)
}
