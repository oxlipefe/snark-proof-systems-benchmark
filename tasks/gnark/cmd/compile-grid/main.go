// Command compile-grid reports the constraint count of every task × regime × backend from
// COMPILATION ALONE.
//
// Why this is a separate binary and not a column of the runner: compiling is far cheaper
// than setup, and setup is far cheaper than proving. Constraint count is gnark's natural
// unit the way cycles were Ceno's, and it can be measured several rungs above where a
// Groth16 setup fits in memory. Reporting "T1-d: not measured" when the count was available
// for the price of a compile would be leaving a fact on the table.
//
// ONE PROCESS PER (task, regime, backend). A compile that dies of memory takes the process
// with it, and a grid driven from inside one process would lose every cell after the first
// death. The shell loop in bench/scripts/gnark/run-compile-grid.sh restarts it.
//
// Output is one CELL line per invocation on stdout, k=v, no commas. A failure prints a
// CELL line with status set and exits non-zero, so a died-at-this-rung is data.
//
// PADDING IS REPORTED, NOT ASSUMED. gnark builds an FFT domain whose size is the next power
// of two above the constraint count (Groth16) or above constraints+public (PLONK); this
// tool prints the derived padded size AND its source expression, and cmd/probe padding
// checks the derivation against the size the backend actually built in a real setup. Until
// that check has run for a given backend, the padded figure here is labelled derived.
package main

import (
	"fmt"
	"os"
	"runtime"
	"strconv"
	"strings"
	"time"

	gb "github.com/viaas/zk-prover-bench/gnark"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/logger"
	"github.com/rs/zerolog"
)

func main() {
	logger.Set(zerolog.New(os.Stderr).Level(zerolog.Disabled))
	if len(os.Args) != 4 {
		fmt.Fprintln(os.Stderr, "usage: compile-grid <task-label> <groth16|plonk> <A|B>")
		os.Exit(2)
	}
	label, bkName, regName := os.Args[1], os.Args[2], os.Args[3]

	spec, err := gb.Lookup(label)
	if err != nil {
		die(label, bkName, regName, "USAGE", err)
	}
	bk, err := gb.ParseBackend(bkName)
	if err != nil {
		die(label, bkName, regName, "USAGE", err)
	}
	reg, err := gb.ParseRegime(regName)
	if err != nil {
		die(label, bkName, regName, "USAGE", err)
	}

	gadget := gb.ReluGadget(env("GNARK_GADGET", string(gb.DefaultReluGadget)))
	fmt.Fprintf(os.Stderr, "[grid] compiling %s %s regime=%s gadget=%s GOMAXPROCS=%d\n",
		label, bk, reg, gadget, runtime.GOMAXPROCS(0))

	t0 := time.Now()
	b, err := gb.Compile(spec, reg, bk, gadget)
	if err != nil {
		status := "COMPILE_FAILED"
		if strings.Contains(err.Error(), "the expression drifted from the published task") {
			status = "MAC_ASSERTION_FAILED"
		}
		die(label, bkName, regName, status, err)
	}
	elapsed := time.Since(t0)
	st := b.Stats()

	// The FFT domain gnark will build. Derived here from gnark's own sizing expressions —
	// backend/groth16/bn254/setup.go:101 fft.NewDomain(nbConstraints) and
	// backend/plonk/bn254/setup.go:271-275 fft.NewDomain(nbConstraints+len(public)) — and
	// checked against a real setup by cmd/probe padding. Derived, and labelled derived.
	base := uint64(st.Constraints)
	src := "fft.NewDomain(nbConstraints)"
	if bk == gb.Plonk {
		base = uint64(st.Constraints + st.PublicVars)
		src = "fft.NewDomain(nbConstraints+nbPublic)"
	}
	padded := ecc.NextPowerOfTwo(base)

	var ms runtime.MemStats
	runtime.ReadMemStats(&ms)

	fmt.Println(strings.Join([]string{
		"CELL",
		"label=" + label,
		"backend=" + string(bk),
		"regime=" + string(reg),
		"gadget=" + string(gadget),
		"status=OK",
		"macs=" + itoa(spec.MACs),
		"macs_emitted=" + itoa(b.EmittedMACs),
		"relus=" + itoa(b.EmittedReLUs),
		"max_abs_intermediate=" + strconv.FormatInt(b.Ref.MaxAbsIntermediate, 10),
		"static_worst_case=" + b.Ref.StaticWorstCase.String(),
		"relu_bits=" + reluBits(b),
		"constraints=" + itoa(st.Constraints),
		"internal_vars=" + itoa(st.InternalVars),
		"secret=" + itoa(st.SecretVars),
		"public=" + itoa(st.PublicVars),
		"coefficients=" + itoa(st.Coefficients),
		"instructions=" + itoa(st.Instructions),
		"domain_derived=" + strconv.FormatUint(padded, 10),
		"domain_derived_from=" + src,
		"domain_measured=0",
		"padding_ratio=" + strconv.FormatFloat(float64(padded)/float64(base), 'f', 4, 64),
		"constraints_per_mac=" + strconv.FormatFloat(float64(st.Constraints)/float64(spec.MACs), 'f', 4, 64),
		"compile_ms=" + strconv.FormatFloat(float64(elapsed.Nanoseconds())/1e6, 'f', 3, 64),
		"go_heap_alloc_bytes=" + strconv.FormatUint(ms.HeapAlloc, 10),
		"go_sys_bytes=" + strconv.FormatUint(ms.Sys, 10),
	}, " "))
}

func die(label, bk, reg, status string, err error) {
	msg := strings.ReplaceAll(err.Error(), ",", ";")
	msg = strings.ReplaceAll(msg, " ", "_")
	if len(msg) > 400 {
		msg = msg[:400]
	}
	fmt.Fprintf(os.Stderr, "GNARK_FAIL class=%s msg=%v\n", status, err)
	fmt.Printf("CELL label=%s backend=%s regime=%s status=%s msg=%s\n", label, bk, reg, status, msg)
	os.Exit(1)
}

func itoa(v int) string { return strconv.Itoa(v) }

// reluBits renders the measured per-layer ReLU bit widths. They are derived from the
// reference forward pass, never guessed, and they are per-layer because layer 3's
// activations are an order of magnitude wider than layer 1's — a single number would hide
// exactly the fact that makes the gadget's cost grow through the network.
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

func env(k, def string) string {
	if v := os.Getenv(k); v != "" {
		return v
	}
	return def
}
