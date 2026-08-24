// Package gnarkbench expresses the zk-prover-bench task bank (bench/TASKS.md) as gnark
// circuits over BN254, for both the R1CS/Groth16 and the SparseR1CS/PLONK frontends.
//
// # Two regimes, never mixed
//
// Regime A ("witness weights") is the HEADLINE and the only cross-system comparable one.
// Both the input vector and every weight are secret witness variables, and every INT8
// value entering the circuit is range-checked with std/rangecheck, because in a prime
// field 8-bit-ness has to be PROVED. A binary-field system gets it for free from the
// representation; BN254 does not, and pretending otherwise would be the benchmark paying
// gnark a discount no other system got. Each MAC is one real R1CS multiplication.
//
// Regime B ("baked weights") is a DECLARED LEVER and must never enter a cross-system
// number. Weights are Go compile-time constants folded into the circuit, so a linear
// combination with constant coefficients is free in R1CS and a whole matmul collapses to
// about one constraint per output. Groth16's per-circuit setup binds those weights into
// the verifying key, which is what a deployed fixed-model zkML service actually wants.
// This is PROTOCOL.md §2's never-evaluated lever "explotación de pesos fijos por
// precómputo".
//
// Every artifact this package produces carries its regime. There is no code path that can
// emit a Regime B figure without the label.
//
// # MAC counts are frozen
//
// The MAC counts below are copied from bench/TASKS.md and are NEVER recomputed. They are
// the denominator of bytes/MAC and of MAC/s across five systems. Each circuit counts the
// multiply-accumulates it actually emits and refuses to compile if the count disagrees.
package gnarkbench

import "fmt"

// Regime selects how the weights enter the circuit. See the package doc.
type Regime string

const (
	// RegimeA — weights are secret witness variables, every INT8 value range-checked.
	RegimeA Regime = "A"
	// RegimeB — weights are Go constants baked into the circuit; only inputs range-checked.
	RegimeB Regime = "B"
)

func ParseRegime(s string) (Regime, error) {
	switch Regime(s) {
	case RegimeA:
		return RegimeA, nil
	case RegimeB:
		return RegimeB, nil
	}
	return "", fmt.Errorf("unknown regime %q: want A (witness weights) or B (baked weights)", s)
}

// Kind distinguishes the two circuit shapes in the bank.
type Kind string

const (
	KindMatMul Kind = "matmul" // T1 ladder
	KindMLP    Kind = "mlp"    // T2, T3
)

// Layer is one dense layer of the MLP. No bias: bench/TASKS.md specifies none.
type Layer struct {
	In, Out int
	ReLU    bool
}

// Spec is one frozen task. MACs and ReLUs come from bench/TASKS.md and are never derived
// from the code that builds the circuit — that is the whole point of the assertion.
type Spec struct {
	Label string
	Kind  Kind

	// KindMatMul: A[M×K] · B[K×N].
	M, K, N int

	// KindMLP.
	Layers []Layer
	Batch  int

	MACs  int    // FROZEN, bench/TASKS.md
	ReLUs int    // FROZEN for T2 (448); T3 is 8×T2 and says so
	Seed  uint32 // binius64's canonical seed, reused verbatim
}

// mlpLayers is the T2 network: 200-256-128-64-1, ReLU after layers 1–3, linear output.
// 448 activations = 256 + 128 + 64.
func mlpLayers() []Layer {
	return []Layer{
		{In: 200, Out: 256, ReLU: true},
		{In: 256, Out: 128, ReLU: true},
		{In: 128, Out: 64, ReLU: true},
		{In: 64, Out: 1, ReLU: false},
	}
}

// Specs is the frozen bank. Seeds are binius64's canonical seeds, reused verbatim.
//
// WITNESS-LEVEL COMPARISON IS INVALID. Go's pseudo-random stream is not Rust's and not
// numpy's, and this package does not even use Go's math/rand — it uses the explicit
// SplitMix64 in rng.go so the stream is pinned to this file rather than to a standard
// library version. Same seed, same shapes, same MAC counts, DIFFERENT INSTANCE. Task-level
// comparison only.
var Specs = map[string]Spec{
	"t1-0": {Label: "t1-0", Kind: KindMatMul, M: 1, K: 256, N: 256, MACs: 65_536, Seed: 0xE0060100},
	"t1-a": {Label: "t1-a", Kind: KindMatMul, M: 1, K: 768, N: 768, MACs: 589_824, Seed: 0xE00601A0},
	"t1-b": {Label: "t1-b", Kind: KindMatMul, M: 4, K: 768, N: 768, MACs: 2_359_296, Seed: 0xE00601B0},
	"t1-c": {Label: "t1-c", Kind: KindMatMul, M: 16, K: 768, N: 768, MACs: 9_437_184, Seed: 0xE00601C0},
	"t1-d": {Label: "t1-d", Kind: KindMatMul, M: 64, K: 768, N: 768, MACs: 37_748_736, Seed: 0xE00601D0},

	"t2": {Label: "t2", Kind: KindMLP, Layers: mlpLayers(), Batch: 1, MACs: 92_224, ReLUs: 448, Seed: 0xE0060200},
	// T3 is "the same MLP, batch of 8, in ONE proof". bench/TASKS.md freezes 737 792 MACs
	// and states the 448 activations only for T2; 3 584 = 8 × 448 is stated here as a
	// derivation of the frozen T2 figure, not as an independent count.
	"t3": {Label: "t3", Kind: KindMLP, Layers: mlpLayers(), Batch: 8, MACs: 737_792, ReLUs: 3_584, Seed: 0xE0060300},

	// --- Not part of the published bank. Probes, labelled so they can never be mistaken
	// for a bank task in a ledger. Their MACs are their own, not TASKS.md's.
	"p64x1": {Label: "p64x1", Kind: KindMLP, Layers: []Layer{{In: 64, Out: 1, ReLU: false}},
		Batch: 1, MACs: 64, ReLUs: 0, Seed: 0xE0060400},
	"p2x1": {Label: "p2x1", Kind: KindMLP, Layers: []Layer{{In: 2, Out: 1, ReLU: false}},
		Batch: 1, MACs: 2, ReLUs: 0, Seed: 0xE0060401},
	"p64x4": {Label: "p64x4", Kind: KindMLP, Layers: []Layer{{In: 64, Out: 4, ReLU: false}},
		Batch: 1, MACs: 256, ReLUs: 0, Seed: 0xE0060402},
	"p64x2": {Label: "p64x2", Kind: KindMLP, Layers: []Layer{{In: 64, Out: 2, ReLU: false}},
		Batch: 1, MACs: 128, ReLUs: 0, Seed: 0xE0060403},
	"p1x1": {Label: "p1x1", Kind: KindMLP, Layers: []Layer{{In: 1, Out: 1, ReLU: false}},
		Batch: 1, MACs: 1, ReLUs: 0, Seed: 0xE0060404},
}

// BankTasks are the seven published tasks, in ladder order.
var BankTasks = []string{"t1-0", "t2", "t1-a", "t3", "t1-b", "t1-c", "t1-d"}

// IsBankTask reports whether label is one of bench/TASKS.md's seven, i.e. whether its MAC
// count is a frozen published figure rather than a probe's own arithmetic.
func IsBankTask(label string) bool {
	for _, t := range BankTasks {
		if t == label {
			return true
		}
	}
	return false
}

func Lookup(label string) (Spec, error) {
	s, ok := Specs[label]
	if !ok {
		return Spec{}, fmt.Errorf("unknown task %q", label)
	}
	return s, nil
}

// MACAssertionError is the mandatory guard. Its text is fixed by the campaign so that a
// drifted expression is greppable across all five systems' logs.
func MACAssertionError(task string, emitted, published int) error {
	return fmt.Errorf("%s: emitted %d MACs but bench/TASKS.md fixes %d; the expression drifted from the published task",
		task, emitted, published)
}
