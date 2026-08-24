// Command runner is the measured binary of the gnark cell.
//
// CONTRACT, matched to bench/scripts/jolt-atlas/:
//
//   - one positional argument, the task label;
//   - campaign parameters by environment, never by flag, so that the shell script that
//     writes the ledger row and the process that produces the numbers cannot disagree about
//     what was set;
//   - STDOUT IS THE STRUCTURED DATA CHANNEL. Uppercase-prefixed k=v pairs separated by
//     spaces, no commas anywhere — the ledger is CSV and a comma in a value is a corrupted
//     row that still parses;
//   - stderr is free-form log, shared with /usr/bin/time -l's own output.
//
// ONE PROCESS PER CELL. The N repetitions run INSIDE this process so that
// `/usr/bin/time -l` attributes exactly one memory peak to exactly one cell. Compile and
// setup also happen here, once, and are reported in their own fields. SETUP IS NEVER
// AMORTIZED INTO PROVE TIME — a Groth16 setup is 20× a prove at T1-0 and folding it in
// would turn a per-circuit one-off into a per-proof cost that nobody pays.
//
// EVERY REPETITION IS VERIFIED, WARMUP INCLUDED, and any failed verification exits
// non-zero. A prove-time figure for a proof nobody checked measures serialization.
//
// THE GO-RUNTIME MEMORY POLICY, DECLARED (the argument Ceno's BUILD.md §4 made about
// jemalloc, made here about the Go runtime):
//
//   - The garbage collector runs at its default settings. GOGC and GOMEMLIMIT are read from
//     the environment and REPORTED, because the campaign sweeps them as an axis; they are
//     not set by this program.
//   - This program NEVER calls debug.FreeOSMemory(). That call forces a collection and
//     hands pages back to the operating system on demand. Both metrics this benchmark
//     publishes — `maximum resident set size` and `peak memory footprint` — are what the
//     OS observed the process hold, so a FreeOSMemory() placed just before a measurement
//     would make peak memory a property of where we put the call. The peak reported here
//     is the peak the prover caused.
//   - runtime.ReadMemStats is called only for the stderr log, never for a published figure.
//     It reports the Go heap, which is not the process footprint.
//
// TIMING CLOCK. Every duration is time.Since on a time.Time from time.Now, which carries a
// monotonic reading; on Darwin that reading does not advance while the machine sleeps. The
// cell is bracketed by bench/scripts/clockprobe.py anyway, and the sleep verdict wins over
// every other status.
package main

import (
	"fmt"
	"os"
	"runtime"
	"runtime/debug"
	"sort"
	"strconv"
	"strings"
	"time"

	gb "github.com/viaas/zk-prover-bench/gnark"

	"github.com/consensys/gnark/logger"
	"github.com/rs/zerolog"
)

// Exit codes. Each failure class has its own code AND its own greppable line, because a
// campaign that only has exit codes cannot tell you from a log file which rung died of what.
const (
	exitOK        = 0
	exitUsage     = 2
	exitA1        = 11 // Amendment A1 refused the emit
	exitMACAssert = 12 // the expression drifted from bench/TASKS.md
	exitCompile   = 13 // compile failed for any other reason (includes OOM at compile)
	exitSetup     = 20 // setup failed, typically OOM
	exitWitness   = 21
	exitProve     = 30
	exitVerify    = 40 // A PROOF WAS PRODUCED AND DID NOT VERIFY. Blocking.
)

func fail(code int, class, msg string) {
	// One line, one shape, greppable across every log in bench/data/cells-gnark/.
	fmt.Fprintf(os.Stderr, "GNARK_FAIL class=%s exit=%d msg=%s\n", class, code, msg)
	fmt.Printf("FAIL class=%s exit=%d\n", class, code)
	os.Exit(code)
}

func envInt(name string, def int) int {
	v := os.Getenv(name)
	if v == "" {
		return def
	}
	n, err := strconv.Atoi(v)
	if err != nil {
		fail(exitUsage, "USAGE", fmt.Sprintf("%s=%q is not an integer", name, v))
	}
	return n
}

func main() {
	// gnark's own logger writes progress to stderr at INFO. Silenced: stdout is the data
	// channel and stderr already carries /usr/bin/time -l's report; gnark's per-span lines
	// are re-enabled with GNARK_LOG=1 when a cell is being diagnosed rather than measured.
	if os.Getenv("GNARK_LOG") == "1" {
		logger.Set(zerolog.New(zerolog.NewConsoleWriter(func(w *zerolog.ConsoleWriter) { w.Out = os.Stderr })))
	} else {
		logger.Set(zerolog.New(os.Stderr).Level(zerolog.Disabled))
	}

	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: runner <task-label>")
		fmt.Fprintln(os.Stderr, "env: GNARK_BACKEND=groth16|plonk GNARK_REGIME=A|B GNARK_REPS GNARK_WARMUP")
		fmt.Fprintln(os.Stderr, "     GNARK_NB_TASKS (solver.WithNbTasks; 0 = gnark default) GNARK_GADGET=hintedsign|tobinary")
		fmt.Fprintln(os.Stderr, "     GOMAXPROCS GOGC GOMEMLIMIT are read from the environment and reported, never set here")
		os.Exit(exitUsage)
	}
	label := os.Args[1]

	spec, err := gb.Lookup(label)
	if err != nil {
		fail(exitUsage, "USAGE", err.Error())
	}
	bk, err := gb.ParseBackend(envStr("GNARK_BACKEND", "groth16"))
	if err != nil {
		fail(exitUsage, "USAGE", err.Error())
	}
	regime, err := gb.ParseRegime(envStr("GNARK_REGIME", "A"))
	if err != nil {
		fail(exitUsage, "USAGE", err.Error())
	}
	gadget := gb.ReluGadget(envStr("GNARK_GADGET", string(gb.DefaultReluGadget)))
	reps := envInt("GNARK_REPS", 5)
	warmup := envInt("GNARK_WARMUP", 1)
	nbTasks := envInt("GNARK_NB_TASKS", 0)

	fmt.Fprintf(os.Stderr, "[gnark] %s backend=%s regime=%s gadget=%s reps=%d warmup=%d\n",
		label, bk, regime, gadget, reps, warmup)
	fmt.Fprintf(os.Stderr, "[gnark] go=%s %s/%s GOMAXPROCS=%d GOGC=%s GOMEMLIMIT=%s(bytes=%d)\n",
		runtime.Version(), runtime.GOOS, runtime.GOARCH, runtime.GOMAXPROCS(0),
		envStr("GOGC", "unset(default 100)"), envStr("GOMEMLIMIT", "unset(off)"), debug.SetMemoryLimit(-1))
	fmt.Fprintf(os.Stderr, "[gnark] memory policy: default GC, no debug.FreeOSMemory() call anywhere in this binary; "+
		"peak RSS and peak footprint are what the OS observed, not what we asked it to observe\n")
	fmt.Fprintf(os.Stderr, "[gnark] clock: time.Now/time.Since (monotonic reading; does not advance during Darwin sleep); "+
		"cell also bracketed by clockprobe.py\n")
	if bk == gb.Plonk {
		fmt.Fprintf(os.Stderr, "[gnark] PLONK SRS comes from gnark's test/unsafekzg, whose own package doc says "+
			"\"a convenience package (to be use for test purposes only)\". There is no ceremony behind it and its "+
			"toxic waste is generated in this process. SRS time is reported in its own field and is never part of prove.\n")
	}

	// ---- reference instance + A1 ----
	ref, err := gb.NewReference(spec)
	if err != nil {
		fail(exitA1, "A1_ASSERTION", err.Error())
	}
	fmt.Fprintf(os.Stderr, "[gnark] %s\n", ref.A1Report())

	// ---- compile ----
	build, err := gb.Compile(spec, regime, bk, gadget)
	if err != nil {
		if strings.Contains(err.Error(), "the expression drifted from the published task") ||
			strings.Contains(err.Error(), "activations are reported separately") {
			fail(exitMACAssert, "MAC_ASSERTION", err.Error())
		}
		fail(exitCompile, "COMPILE", err.Error())
	}
	st := build.Stats()

	// ---- setup ----
	keys, err := build.Setup()
	if err != nil {
		fail(exitSetup, "SETUP", err.Error())
	}

	meta := []string{
		"META",
		kv("label", label),
		kv("backend", string(bk)),
		kv("regime", string(regime)),
		kv("gadget", string(gadget)),
		kvi("macs", spec.MACs),
		kvi("macs_emitted", build.EmittedMACs),
		kvi("constraints", st.Constraints),
		kvi("internal_vars", st.InternalVars),
		kvi("secret", st.SecretVars),
		kvi("public", st.PublicVars),
		kvi("coefficients", st.Coefficients),
		kvi("instructions", st.Instructions),
		kvi("relus", build.EmittedReLUs),
		kvi64("max_abs_intermediate", ref.MaxAbsIntermediate),
		kv("static_worst_case", ref.StaticWorstCase.String()),
		kv("relu_bits", reluBitsField(ref)),
		kvu("domain_cardinality", keys.DomainCardinality),
		kv("domain_source", keys.DomainSource),
		kvi("gomaxprocs", runtime.GOMAXPROCS(0)),
		kv("gogc", envStr("GOGC", "default")),
		kv("gomemlimit", envStr("GOMEMLIMIT", "off")),
		kvi64("gomemlimit_bytes", debug.SetMemoryLimit(-1)),
		kvi("nb_tasks", nbTasks),
		kvi("reps", reps),
		kvi("warmup", warmup),
		kv("curve", "BN254"),
		kv("statistical_zk", "false"),
		kv("clock", "go_time.Since_monotonic"),
		kv("go", runtime.Version()),
		kvf("compile_ms", build.CompileD),
	}
	fmt.Println(strings.Join(meta, " "))

	setup := []string{"SETUP", kvf("ms", keys.SetupD), kvi64("pk_bytes", keys.PkSize), kvi64("vk_bytes", keys.VkSize)}
	if bk == gb.Plonk {
		setup = append(setup, kvf("srs_ms", keys.SRSD), kv("srs", "test/unsafekzg_TEST_PURPOSES_ONLY"))
	}
	fmt.Println(strings.Join(setup, " "))

	// ---- witness ----
	full, public, err := build.Witnesses()
	if err != nil {
		fail(exitWitness, "WITNESS", err.Error())
	}

	// ---- repetitions ----
	var proveMs []float64
	lastBytes := int64(0)
	total := warmup + reps
	for i := 0; i < total; i++ {
		isWarm := i < warmup
		tag := "REP"
		idx := i - warmup + 1
		if isWarm {
			tag = "WARMUP"
			idx = i
		}
		p, d, err := build.Prove(keys, full, nbTasks)
		if err != nil {
			fail(exitProve, "PROVE", fmt.Sprintf("%s rep=%d: %v", tag, idx, err))
		}
		vd, verr := build.Verify(keys, p, public)
		ok := verr == nil
		if !ok {
			// BLOCKING. A produced proof that does not verify invalidates the cell and
			// every figure derived from it.
			fmt.Printf("%s i=%d %s %s %s %s %s\n", tag, idx, kvf("prove_ms", d), kvf("verify_ms", vd),
				kvi64("proof_bytes", p.Bytes), kvi64("proof_bytes_raw", p.BytesRaw), kv("verify_ok", "false"))
			fail(exitVerify, "VERIFY", fmt.Sprintf("%s rep=%d did not verify: %v", tag, idx, verr))
		}
		fmt.Printf("%s i=%d %s %s %s %s %s\n", tag, idx, kvf("prove_ms", d), kvf("verify_ms", vd),
			kvi64("proof_bytes", p.Bytes), kvi64("proof_bytes_raw", p.BytesRaw), kv("verify_ok", "true"))
		lastBytes = p.Bytes
		if !isWarm {
			proveMs = append(proveMs, float64(d.Nanoseconds())/1e6)
		}

		if os.Getenv("GNARK_DUMP_PROOF") != "" && i == total-1 {
			if err := os.WriteFile(os.Getenv("GNARK_DUMP_PROOF"), p.Blob, 0o644); err != nil {
				fmt.Fprintf(os.Stderr, "[gnark] could not dump proof: %v\n", err)
			}
		}
	}

	var ms runtime.MemStats
	runtime.ReadMemStats(&ms)
	fmt.Fprintf(os.Stderr, "[gnark] go-heap only (NOT the published figure): HeapAlloc=%.3fGB Sys=%.3fGB TotalAlloc=%.3fGB NumGC=%d\n",
		float64(ms.HeapAlloc)/1e9, float64(ms.Sys)/1e9, float64(ms.TotalAlloc)/1e9, ms.NumGC)

	sort.Float64s(proveMs)
	fmt.Printf("DONE %s\n", kvi64("proof_bytes", lastBytes))
	os.Exit(exitOK)
}

func envStr(name, def string) string {
	if v := os.Getenv(name); v != "" {
		return v
	}
	return def
}

func kv(k, v string) string {
	if v == "" {
		v = "-"
	}
	// The ledger is CSV. A comma inside a value produces a row that parses and lies.
	v = strings.ReplaceAll(v, ",", ";")
	v = strings.ReplaceAll(v, " ", "_")
	return k + "=" + v
}

func kvi(k string, v int) string     { return k + "=" + strconv.Itoa(v) }
func kvi64(k string, v int64) string { return k + "=" + strconv.FormatInt(v, 10) }
func kvu(k string, v uint64) string  { return k + "=" + strconv.FormatUint(v, 10) }
func kvf(k string, d time.Duration) string {
	return k + "=" + strconv.FormatFloat(float64(d.Nanoseconds())/1e6, 'f', 3, 64)
}

// reluBitsField renders the per-layer measured bit widths as layer:bits pairs joined by
// '|', so the META line stays comma-free and the widths stay per-layer. A single number
// here would hide that layer 3's activations are an order of magnitude wider than layer 1's.
func reluBitsField(ref *gb.Reference) string {
	if len(ref.ReluBits) == 0 {
		return "-"
	}
	ls := make([]int, 0, len(ref.ReluBits))
	for l := range ref.ReluBits {
		ls = append(ls, l)
	}
	sort.Ints(ls)
	parts := make([]string, 0, len(ls))
	for _, l := range ls {
		parts = append(parts, fmt.Sprintf("L%d:%d(max=%d)", l, ref.ReluBits[l], ref.ReluMaxAbs[l]))
	}
	return strings.Join(parts, "|")
}
