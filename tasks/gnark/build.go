package gnarkbench

import (
	"bytes"
	"fmt"
	"io"
	"math/big"
	"time"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend"
	"github.com/consensys/gnark/backend/groth16"
	groth16_bn254 "github.com/consensys/gnark/backend/groth16/bn254"
	"github.com/consensys/gnark/backend/plonk"
	plonk_bn254 "github.com/consensys/gnark/backend/plonk/bn254"
	"github.com/consensys/gnark/backend/witness"
	"github.com/consensys/gnark/constraint"
	"github.com/consensys/gnark/constraint/solver"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/consensys/gnark/frontend/cs/scs"
	"github.com/consensys/gnark/test/unsafekzg"
)

// Backend is which proof system the cell measures.
type Backend string

const (
	Groth16 Backend = "groth16"
	Plonk   Backend = "plonk"
)

func ParseBackend(s string) (Backend, error) {
	switch Backend(s) {
	case Groth16:
		return Groth16, nil
	case Plonk:
		return Plonk, nil
	}
	return "", fmt.Errorf("unknown backend %q: want groth16 or plonk", s)
}

// Curve is fixed for the whole campaign. Changing it changes the security column of the
// conditions line, so it is a constant and not a flag.
var Curve = ecc.BN254

// Build is a compiled circuit plus everything derived from it that does not need a proving
// key. Compilation is far cheaper than setup, so the compile-only grid can climb the ladder
// past the rung where setup dies.
type Build struct {
	Spec     Spec
	Regime   Regime
	Backend  Backend
	Gadget   ReluGadget
	Ref      *Reference
	CCS      constraint.ConstraintSystem
	Template frontend.Circuit
	CompileD time.Duration

	EmittedMACs  int
	EmittedReLUs int

	// asgOverride is set only by CompileGeneric. It exists so that a circuit that is NOT
	// one of the bank tasks — gnark's own example circuits, the minimum-width probes —
	// travels through exactly the same Setup / Prove / Verify code as a measured cell.
	// bench/systems/jolt-atlas stopped three of our own expression errors from being
	// published as somebody else's limits precisely because the control circuit went
	// through the harness unchanged; a second code path would have voided that check.
	asgOverride frontend.Circuit
	name        string
}

// Name is the label this build reports under.
func (b *Build) Name() string {
	if b.name != "" {
		return b.name
	}
	return b.Spec.Label
}

// CompileGeneric compiles an arbitrary circuit through the same path a bank task takes.
// spec is synthesised with MACs = 0 and is never a published denominator; callers that
// report a rate for such a circuit must supply their own count and say where it came from.
func CompileGeneric(name string, tmpl, asg frontend.Circuit, backend Backend) (*Build, error) {
	t0 := time.Now()
	ccs, err := frontend.Compile(Curve.ScalarField(), builderFor(backend), tmpl)
	d := time.Since(t0)
	if err != nil {
		return nil, fmt.Errorf("compile %s/%s: %w", name, backend, err)
	}
	return &Build{
		Spec: Spec{Label: name}, Regime: "-", Backend: backend, Gadget: DefaultReluGadget,
		Ref: &Reference{StaticWorstCase: big.NewInt(0)},
		CCS: ccs, Template: tmpl, CompileD: d, asgOverride: asg, name: name,
	}, nil
}

func builderFor(b Backend) frontend.NewBuilder {
	if b == Plonk {
		return scs.NewBuilder
	}
	return r1cs.NewBuilder
}

// Template constructs the circuit template for a task, regime and gadget.
func Template(spec Spec, regime Regime, gadget ReluGadget, ref *Reference) frontend.Circuit {
	if spec.Kind == KindMatMul {
		return NewMatMulCircuit(spec, regime, ref)
	}
	return NewMLPCircuit(spec, regime, gadget, ref)
}

// Assignment constructs the filled witness circuit.
func Assignment(spec Spec, regime Regime, gadget ReluGadget, ref *Reference) frontend.Circuit {
	if spec.Kind == KindMatMul {
		return AssignMatMul(spec, regime, ref)
	}
	return AssignMLP(spec, regime, gadget, ref)
}

func emitted(c frontend.Circuit) (macs, relus int) {
	switch t := c.(type) {
	case *MatMulCircuit:
		return t.EmittedMACs(), 0
	case *MLPCircuit:
		return t.EmittedMACs(), t.EmittedReLUs()
	}
	return 0, 0
}

// Compile builds the constraint system. The MAC assertion fires inside Define, so a drifted
// expression surfaces here and never reaches a timing.
func Compile(spec Spec, regime Regime, backend Backend, gadget ReluGadget) (*Build, error) {
	ref, err := NewReference(spec)
	if err != nil {
		return nil, err
	}
	tmpl := Template(spec, regime, gadget, ref)
	t0 := time.Now()
	ccs, err := frontend.Compile(Curve.ScalarField(), builderFor(backend), tmpl)
	d := time.Since(t0)
	if err != nil {
		return nil, fmt.Errorf("compile %s/%s/%s: %w", spec.Label, backend, regime, err)
	}
	macs, relus := emitted(tmpl)
	return &Build{
		Spec: spec, Regime: regime, Backend: backend, Gadget: gadget, Ref: ref,
		CCS: ccs, Template: tmpl, CompileD: d, EmittedMACs: macs, EmittedReLUs: relus,
	}, nil
}

// Stats are the shape numbers that go in the META line.
type Stats struct {
	Constraints   int
	InternalVars  int
	SecretVars    int
	PublicVars    int
	Coefficients  int
	Instructions  int
	DomainCardin  uint64 // FFT domain size the backend actually built; 0 until Setup
	DomainMeasure string // where the number was read from, so it is never mistaken for a formula
}

func (b *Build) Stats() Stats {
	return Stats{
		Constraints:  b.CCS.GetNbConstraints(),
		InternalVars: b.CCS.GetNbInternalVariables(),
		SecretVars:   b.CCS.GetNbSecretVariables(),
		PublicVars:   b.CCS.GetNbPublicVariables(),
		Coefficients: b.CCS.GetNbCoefficients(),
		Instructions: b.CCS.GetNbInstructions(),
	}
}

// Keys carries a proving/verifying key pair for either backend, plus the timings that must
// stay OUT of prove time.
type Keys struct {
	Backend Backend

	G16pk groth16.ProvingKey
	G16vk groth16.VerifyingKey

	PLpk plonk.ProvingKey
	PLvk plonk.VerifyingKey

	SetupD time.Duration
	SRSD   time.Duration // PLONK only; reported separately from setup and from prove
	PkSize int64
	VkSize int64

	// DomainCardinality is READ OUT of the key the backend built — pk.Domain.Cardinality
	// for Groth16, vk.Size for PLONK — not computed from a formula about what gnark
	// "should" do. Padding is a measurement here.
	DomainCardinality uint64
	DomainSource      string
}

// Setup runs the one-off key generation. It happens once per cell, inside the measured
// process, and its duration is reported in its own field. It is NEVER amortized into prove.
func (b *Build) Setup() (*Keys, error) {
	k := &Keys{Backend: b.Backend}
	switch b.Backend {
	case Groth16:
		t0 := time.Now()
		pk, vk, err := groth16.Setup(b.CCS)
		k.SetupD = time.Since(t0)
		if err != nil {
			return nil, fmt.Errorf("groth16 setup: %w", err)
		}
		k.G16pk, k.G16vk = pk, vk
		if p, ok := pk.(*groth16_bn254.ProvingKey); ok {
			k.DomainCardinality = p.Domain.Cardinality
			k.DomainSource = "groth16_bn254.ProvingKey.Domain.Cardinality"
		}
	case Plonk:
		// test/unsafekzg's own package doc: "a convenience package (to be use for test
		// purposes only)". There is no ceremony behind this SRS and its toxic waste is
		// generated in process. Every PLONK figure in this campaign inherits that, and the
		// runner prints it to stderr on every cell rather than burying it in a README.
		t0 := time.Now()
		srs, srsLagrange, err := unsafekzg.NewSRS(b.CCS)
		k.SRSD = time.Since(t0)
		if err != nil {
			return nil, fmt.Errorf("unsafekzg SRS: %w", err)
		}
		t1 := time.Now()
		pk, vk, err := plonk.Setup(b.CCS, srs, srsLagrange)
		k.SetupD = time.Since(t1)
		if err != nil {
			return nil, fmt.Errorf("plonk setup: %w", err)
		}
		k.PLpk, k.PLvk = pk, vk
		if v, ok := vk.(*plonk_bn254.VerifyingKey); ok {
			k.DomainCardinality = v.Size
			k.DomainSource = "plonk_bn254.VerifyingKey.Size"
		}
	}
	k.PkSize = sizeOf(k.provingKeyWriter())
	k.VkSize = sizeOf(k.verifyingKeyWriter())
	return k, nil
}

func (k *Keys) provingKeyWriter() io.WriterTo {
	if k.Backend == Groth16 {
		return k.G16pk
	}
	return k.PLpk
}

func (k *Keys) verifyingKeyWriter() io.WriterTo {
	if k.Backend == Groth16 {
		return k.G16vk
	}
	return k.PLvk
}

func sizeOf(w io.WriterTo) int64 {
	if w == nil {
		return 0
	}
	n, err := w.WriteTo(io.Discard)
	if err != nil {
		return 0
	}
	return n
}

// Witnesses builds the full and public witnesses for the reference instance.
func (b *Build) Witnesses() (full, public witness.Witness, err error) {
	asg := b.asgOverride
	if asg == nil {
		asg = Assignment(b.Spec, b.Regime, b.Gadget, b.Ref)
	}
	full, err = frontend.NewWitness(asg, Curve.ScalarField())
	if err != nil {
		return nil, nil, fmt.Errorf("witness: %w", err)
	}
	public, err = full.Public()
	if err != nil {
		return nil, nil, fmt.Errorf("public witness: %w", err)
	}
	return full, public, nil
}

// Proof is one repetition's result, with both serializations measured.
type Proof struct {
	G16 groth16.Proof
	PL  plonk.Proof

	Bytes    int64 // WriteTo, points COMPRESSED — gnark's default wire format
	BytesRaw int64 // WriteRawTo, points UNCOMPRESSED
	Blob     []byte
}

// THE MEMORY-KNOB QUESTION, at the API level.
//
// backend.ProverConfig at v0.16.2 has exactly five fields — SolverOpts, HashToFieldFn,
// ChallengeHash, KZGFoldingHash, StatisticalZK — and NONE of them is a memory knob. There
// is no segmentation, no streaming, no shard cap, nothing with the shape of Ceno's
// --max-cycle-per-shard. solver.WithNbTasks caps the number of solver goroutines, which is
// a parallelism knob that happens to move memory; the runner exposes it as its own axis
// and does not call it a memory knob.
//
// WithStatisticalZeroKnowledge is NOT set. It INCREASES memory, and the campaign's reason
// for leaving it off is not that: it is that gnark's default Groth16 path is therefore not
// statistical ZK, and the ZK column of the conditions line has to say so rather than
// inherit a "y" from the fact that Groth16 is a zk-SNARK on paper.
func backendProverOpts(solverOpts []solver.Option) []backend.ProverOption {
	return []backend.ProverOption{backend.WithSolverOptions(solverOpts...)}
}

// Prove runs one proof. nbTasks <= 0 leaves gnark's default (runtime.NumCPU()).
func (b *Build) Prove(k *Keys, full witness.Witness, nbTasks int) (*Proof, time.Duration, error) {
	var solverOpts []solver.Option
	solverOpts = append(solverOpts, solver.WithHints(ReluHints()...))
	if nbTasks > 0 {
		solverOpts = append(solverOpts, solver.WithNbTasks(nbTasks))
	}

	p := &Proof{}
	t0 := time.Now()
	switch b.Backend {
	case Groth16:
		pr, err := groth16.Prove(b.CCS, k.G16pk, full, backendProverOpts(solverOpts)...)
		d := time.Since(t0)
		if err != nil {
			return nil, d, fmt.Errorf("groth16 prove: %w", err)
		}
		p.G16 = pr
	case Plonk:
		pr, err := plonk.Prove(b.CCS, k.PLpk, full, backendProverOpts(solverOpts)...)
		d := time.Since(t0)
		if err != nil {
			return nil, d, fmt.Errorf("plonk prove: %w", err)
		}
		p.PL = pr
	}
	d := time.Since(t0)

	var buf bytes.Buffer
	n, err := p.writerTo().WriteTo(&buf)
	if err != nil {
		return nil, d, fmt.Errorf("serialize proof: %w", err)
	}
	p.Bytes = n
	p.Blob = append([]byte(nil), buf.Bytes()...)

	raw, err := p.rawWriterTo().WriteTo(io.Discard)
	if err != nil {
		return nil, d, fmt.Errorf("serialize proof raw: %w", err)
	}
	p.BytesRaw = raw
	return p, d, nil
}

type rawWriterTo interface {
	WriteRawTo(io.Writer) (int64, error)
}

type rawAdapter struct{ w rawWriterTo }

func (a rawAdapter) WriteTo(w io.Writer) (int64, error) { return a.w.WriteRawTo(w) }

func (p *Proof) writerTo() io.WriterTo {
	if p.G16 != nil {
		return p.G16
	}
	return p.PL
}

func (p *Proof) rawWriterTo() io.WriterTo {
	if p.G16 != nil {
		return rawAdapter{p.G16.(rawWriterTo)}
	}
	return rawAdapter{p.PL.(rawWriterTo)}
}

// Verify checks a proof against the public witness. EVERY repetition is verified, warmup
// included: a prove-time figure for a proof nobody checked is a figure about serialization,
// not about proving.
func (b *Build) Verify(k *Keys, p *Proof, public witness.Witness) (time.Duration, error) {
	t0 := time.Now()
	var err error
	switch b.Backend {
	case Groth16:
		err = groth16.Verify(p.G16, k.G16vk, public)
	case Plonk:
		err = plonk.Verify(p.PL, k.PLvk, public)
	}
	return time.Since(t0), err
}

// NewEmptyProof returns a zero proof of the right concrete type, for deserialization.
func NewEmptyProof(bk Backend) interface {
	io.ReaderFrom
	io.WriterTo
} {
	if bk == Groth16 {
		return groth16.NewProof(Curve)
	}
	return plonk.NewProof(Curve)
}
