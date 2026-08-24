// Command negative is the correctness control.
//
// bench/README.md: "A corrupted trace must make verify() fail, in every system, on every
// task." Without it we would not be benchmarking proofs, only computations that happen to
// produce bytes.
//
// TWO POSITIVE CONTROLS RUN FIRST, and if either fails the tool prints
//
//	ROUND TRIP FAILED — every other result in this file is meaningless
//
// and aborts. A negative test that passes because nothing ever verifies establishes
// nothing; Ceno's control was nearly exactly that, and the reason it was not is that its
// positive control caught a vk that rejected every proof.
//
// # Two subcommands, because the sweep has to survive the process dying
//
//	prepare  compile → setup → prove → the two positive controls → the public_input_word and
//	         witness_word families. Persists the verifying key, the public witness and the
//	         serialized proof into a cache directory.
//	sweep    loads that cache and flips bytes. It does NOT re-prove: Groth16's prover is
//	         randomized, so a proof produced by a restarted process is a DIFFERENT proof and
//	         every offset in the file before the restart would refer to bytes that no longer
//	         exist. Persisting the artifact is what makes the sweep resumable AND coherent.
//
// # The families
//
//	public_input_word   one element of the PUBLIC witness — the claimed output — is
//	                    corrupted before verification. Exhaustive.
//	witness_word        one SECRET value is corrupted before proving. gnark is Apache-2.0
//	                    and we may instrument it, so this family is available here and was
//	                    not for jolt-atlas. Sampled, and the sample size is printed.
//	proof_byte          every byte of the serialized proof, exhaustively.
//
// # Two added verdicts, both declared
//
// WITNESS_INERT is the second, and §4.2 of NOT_EXPRESSIBLE.md explains the episode that
// produced it: a bump that the ReLU zeroes is a different witness for the SAME statement, so
// there is nothing for the verifier to reject and calling it VERIFY_ACCEPTED would report the
// prover's honesty as the verifier's failure.
//
// # PROVE_REJECTED is an added verdict, and it is declared
//
// The campaign's verdict vocabulary is {VERIFY_ACCEPTED, VERIFY_REJECTED,
// DESERIALIZE_REJECTED, DESERIALIZE_PANIC, DESERIALIZE_ABORT}. gnark's architecture forces a
// sixth: its solver evaluates every constraint while building the witness assignment, so a
// corrupted secret value is caught at PROVING time and no proof is ever produced. Recording
// that as VERIFY_REJECTED would claim the verifier rejected something it never saw. It is
// reported as PROVE_REJECTED, which is a different and weaker statement — it says the
// honest prover refuses, not that a dishonest one cannot succeed — and the RESULTS write-up
// must not upgrade it.
package main

import (
	"bufio"
	"bytes"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	gb "github.com/viaas/zk-prover-bench/gnark"

	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/backend/plonk"
	"github.com/consensys/gnark/backend/witness"
	"github.com/consensys/gnark/logger"
	"github.com/rs/zerolog"
)

var out = bufio.NewWriter(os.Stdout)

func main() {
	logger.Set(zerolog.New(os.Stderr).Level(zerolog.Disabled))
	defer out.Flush()

	if len(os.Args) < 3 {
		fmt.Fprintln(os.Stderr, "usage: negative <prepare|sweep> <task-label>")
		fmt.Fprintln(os.Stderr, "env: GNARK_BACKEND GNARK_REGIME GNARK_NEG_CACHE GNARK_NEG_OFFSETS_FILE GNARK_NEG_PATTERNS GNARK_NEG_WITNESS_SAMPLES")
		os.Exit(2)
	}
	mode, label := os.Args[1], os.Args[2]

	bk, err := gb.ParseBackend(env("GNARK_BACKEND", "groth16"))
	must(err)
	reg, err := gb.ParseRegime(env("GNARK_REGIME", "A"))
	must(err)
	cache := env("GNARK_NEG_CACHE", filepath.Join(os.TempDir(), "gnark-neg", label+"-"+string(bk)+"-"+string(reg)))

	switch mode {
	case "prepare":
		prepare(label, bk, reg, cache)
	case "sweep":
		sweep(label, bk, cache)
	default:
		fmt.Fprintf(os.Stderr, "unknown mode %q\n", mode)
		os.Exit(2)
	}
}

// ------------------------------------------------------------------ prepare

func prepare(label string, bk gb.Backend, reg gb.Regime, cache string) {
	spec, err := gb.Lookup(label)
	must(err)
	must(os.MkdirAll(cache, 0o755))

	b, err := gb.Compile(spec, reg, bk, gb.DefaultReluGadget)
	must(err)
	keys, err := b.Setup()
	must(err)
	full, public, err := b.Witnesses()
	must(err)
	p, _, err := b.Prove(keys, full, 0)
	must(err)

	// ---- positive control 1: the honest proof verifies ----
	if _, err := b.Verify(keys, p, public); err != nil {
		roundTripFailed(fmt.Sprintf("honest proof did not verify: %v", err))
	}
	row(label, "none", "-", "honest", "VERIFY_ACCEPTED")

	// ---- positive control 2: serialize → deserialize → verify, UNMODIFIED ----
	// This is what proves the method itself does not corrupt. Every proof_byte verdict
	// below travels the same deserialize path, so if this one failed, every rejection in
	// the sweep would be a rejection of our round trip rather than of the corruption.
	rt, verdict := verifyBlob(bk, keys, public, p.Blob)
	if verdict != "VERIFY_ACCEPTED" {
		roundTripFailed(fmt.Sprintf("unmodified round trip gave %s (%v)", verdict, rt))
	}
	row(label, "none", "-", "roundtrip", "VERIFY_ACCEPTED")

	// ---- persist, so `sweep` can resume without re-proving ----
	writeTo(filepath.Join(cache, "vk.bin"), keys.VerifyingKeyAny().(io.WriterTo))
	writeTo(filepath.Join(cache, "public.bin"), public)
	must(os.WriteFile(filepath.Join(cache, "proof.bin"), p.Blob, 0o644))
	must(os.WriteFile(filepath.Join(cache, "backend.txt"), []byte(string(bk)), 0o644))
	writeRegions(filepath.Join(cache, "regions.csv"), label, bk, b, p.Blob)

	fmt.Fprintf(os.Stderr, "[neg] %s %s regime=%s proof=%d bytes (raw %d) cached in %s\n",
		label, bk, reg, p.Bytes, p.BytesRaw, cache)

	// ---- family: public_input_word, EXHAUSTIVE ----
	pubBytes, err := public.MarshalBinary()
	must(err)
	nElems, elemOff, elemSize := witnessLayout(pubBytes)
	fmt.Fprintf(os.Stderr, "[neg] public witness: %d elements of %d bytes starting at offset %d\n",
		nElems, elemSize, elemOff)
	for i := 0; i < nElems; i++ {
		// Flip the LOW bit of the element: the smallest change that is certainly a change,
		// applied to values that are small integers so it cannot wrap the modulus.
		c := append([]byte(nil), pubBytes...)
		last := elemOff + i*elemSize + elemSize - 1
		orig := c[last]
		c[last] ^= 0x01
		w, werr := witness.New(gb.Curve.ScalarField())
		if werr != nil {
			row(label, "public_input_word", strconv.Itoa(i), detail("xor01", orig), "DESERIALIZE_REJECTED")
			continue
		}
		if err := w.UnmarshalBinary(c); err != nil {
			row(label, "public_input_word", strconv.Itoa(i), detail("xor01", orig), "DESERIALIZE_REJECTED")
			continue
		}
		_, v := verifyBlob(bk, keys, w, p.Blob)
		row(label, "public_input_word", strconv.Itoa(i), detail("xor01", orig), v)
	}

	// ---- family: witness_word, SAMPLED (proving is not free) ----
	nSamples := envInt("GNARK_NEG_WITNESS_SAMPLES", 8)
	fmt.Fprintf(os.Stderr, "[neg] witness_word: %d samples (proving is not free; coverage is declared not inferred)\n", nSamples)
	witnessWordFamily(label, spec, reg, bk, b, keys, public, nSamples)

	out.Flush()
}

// witnessWordFamily corrupts one SECRET value and re-proves.
//
// EVERY POSITION IS CLASSIFIED BEFORE IT IS PROVED. "Corrupt a witness value" and "corrupt the
// statement being proved" are not the same operation on a network with activations: a ReLU is
// not injective, so a weight feeding a neuron whose pre-activation is negative is zeroed and
// never reaches the output. Incrementing such a weight yields a DIFFERENT WITNESS FOR THE SAME
// TRUE STATEMENT, and a verifier that accepts it is behaving correctly.
//
// The first campaign run of this control conflated the two and reported two such positions as
// VERIFY_ACCEPTED on T2 — which reads as a soundness finding and is not one. gnarkbench's
// EffectOfBump now recomputes the reference forward pass first, so:
//
//	WITNESS_INERT   the bump does not change the public output. Nothing to reject. Reported,
//	                and NOT counted as an accepted corruption.
//	VERIFY_ACCEPTED reserved for what it is supposed to mean: the statement DID change and the
//	                verifier let it through. That would be a soundness finding.
//
// When a sampled position turns out inert, a live replacement is drawn so the family still
// performs its intended number of real tests. The inert fraction is itself reported: it
// measures how much of a ReLU network's weight tensor is dead for a given input.
func witnessWordFamily(label string, spec gb.Spec, reg gb.Regime, bk gb.Backend,
	b *gb.Build, keys *gb.Keys, public witness.Witness, nSamples int) {

	// Draw a generous pool so inert positions can be replaced without re-sampling logic.
	pool := gb.SecretPositions(spec, reg, nSamples*8)
	live, inert := 0, 0

	for _, pos := range pool {
		if live >= nSamples {
			break
		}
		eff, err := gb.EffectOfBump(spec, reg, b.Ref, pos)
		if err != nil {
			continue
		}
		if !eff.Changes {
			// Inert: a valid witness for the same statement. Recorded once per occurrence so
			// the fraction is visible, and never proved — proving it would only re-establish
			// that an honest statement verifies, which the positive control already did.
			inert++
			row(label, "witness_word", pos.String(),
				fmt.Sprintf("plus1_from%d_INERT_relu_zeroes_it", eff.Honest), "WITNESS_INERT")
			continue
		}
		live++

		asg, err := gb.CorruptedAssignment(spec, reg, gb.DefaultReluGadget, b.Ref, pos)
		if err != nil {
			row(label, "witness_word", pos.String(), detailBump(eff), "PROVE_REJECTED")
			continue
		}
		w, err := gb.WitnessOf(asg)
		if err != nil {
			row(label, "witness_word", pos.String(), detailBump(eff), "PROVE_REJECTED")
			continue
		}
		p, _, err := b.Prove(keys, w, 0)
		if err != nil {
			// The solver evaluated the constraints while assigning and refused. Declared
			// verdict; see the package doc.
			row(label, "witness_word", pos.String(), detailBump(eff), "PROVE_REJECTED")
			continue
		}
		_, v := verifyBlob(bk, keys, public, p.Blob)
		row(label, "witness_word", pos.String(), detailBump(eff), v)
		if v == "VERIFY_ACCEPTED" {
			// This one WOULD be a soundness finding: the statement changed and the proof
			// still verified. Make it impossible to miss in a log.
			fmt.Fprintf(os.Stderr,
				"GNARK_ALERT class=WITNESS_ACCEPTED task=%s pos=%s: the bump CHANGED the public output "+
					"and the proof still verified. This is not inertness. Investigate before publishing.\n",
				label, pos)
		}
	}
	fmt.Fprintf(os.Stderr, "[neg] witness_word: %d live corruptions tested, %d inert positions skipped "+
		"(inert = the ReLU zeroes that weight, so the bump does not change the statement)\n", live, inert)
}

func detailBump(e gb.StatementEffect) string {
	if !e.InRange {
		return fmt.Sprintf("plus1_from%d_OUTOFRANGE", e.Honest)
	}
	return fmt.Sprintf("plus1_from%d", e.Honest)
}

// ------------------------------------------------------------------ sweep

func sweep(label string, bk gb.Backend, cache string) {
	vk := loadVK(bk, filepath.Join(cache, "vk.bin"))
	public := loadPublic(filepath.Join(cache, "public.bin"))
	blob, err := os.ReadFile(filepath.Join(cache, "proof.bin"))
	must(err)

	patterns := parsePatterns(env("GNARK_NEG_PATTERNS", "01"))
	offsets := parseOffsets(env("GNARK_NEG_OFFSETS_FILE", ""), len(blob))

	fmt.Fprintf(os.Stderr, "[neg] sweeping %d offsets x %d patterns over a %d-byte proof\n",
		len(offsets), len(patterns), len(blob))

	for _, off := range offsets {
		if off < 0 || off >= len(blob) {
			continue
		}
		for _, pat := range patterns {
			c := append([]byte(nil), blob...)
			orig := c[off]
			c[off] ^= pat
			if c[off] == orig {
				// XOR 0 would be a no-op and would report the honest proof as an accepted
				// corruption. Never emitted, but the guard is cheap and the failure mode is
				// the exact shape of a false unsoundness finding.
				continue
			}
			_, v := verifyBlobVK(bk, vk, public, c)
			row(label, "proof_byte", strconv.Itoa(off), detail(fmt.Sprintf("xor%02x", pat), orig), v)
			out.Flush()
		}
	}
	out.Flush()
}

// ------------------------------------------------------------------ verification plumbing

// verifyBlob deserializes a proof from bytes and verifies it, classifying every way the
// attempt can end. A panic inside gnark-crypto's decoder is caught and reported as
// DESERIALIZE_PANIC; a runtime abort (an allocation the runtime cannot satisfy) cannot be
// caught from inside the process at all, which is why sweep-proof-bytes.sh drives this in a
// restarting loop and records the killing offset as DESERIALIZE_ABORT.
func verifyBlob(bk gb.Backend, keys *gb.Keys, public witness.Witness, blob []byte) (err error, verdict string) {
	return verifyBlobVK(bk, keys.VerifyingKeyAny(), public, blob)
}

func verifyBlobVK(bk gb.Backend, vk any, public witness.Witness, blob []byte) (rerr error, verdict string) {
	defer func() {
		if r := recover(); r != nil {
			rerr = fmt.Errorf("panic: %v", r)
			verdict = "DESERIALIZE_PANIC"
		}
	}()

	p := gb.NewEmptyProof(bk)
	if _, err := p.ReadFrom(bytes.NewReader(blob)); err != nil {
		return err, "DESERIALIZE_REJECTED"
	}
	var verr error
	if bk == gb.Groth16 {
		verr = groth16.Verify(p.(groth16.Proof), vk.(groth16.VerifyingKey), public)
	} else {
		verr = plonk.Verify(p.(plonk.Proof), vk.(plonk.VerifyingKey), public)
	}
	if verr != nil {
		return verr, "VERIFY_REJECTED"
	}
	return nil, "VERIFY_ACCEPTED"
}

// ------------------------------------------------------------------ regions

// writeRegions maps every offset of the SERIALIZED artifact to the field it belongs to.
//
// bench/README.md's correctness control is about the proof; a benchmark that sweeps an
// artifact without knowing which of its bytes are the proof body and which are envelope
// cannot say what an accepted flip means. gnark is Apache-2.0 and the layout is read
// straight out of backend/groth16/bn254/marshal.go:33-57 — Ar (G1) | Bs (G2) | Krs (G1) |
// Commitments (a LENGTH-PREFIXED SLICE of G1) | CommitmentPok (G1) — with sizes taken from
// the artifact actually produced, not from a formula about what they should be.
//
// The Commitments slice is the reason a proof of this circuit is 196 bytes and not 164:
// std/rangecheck's commit variant calls api.Commit, which adds a Pedersen commitment and
// its proof of knowledge to every Groth16 proof of a circuit that range-checks anything.
func writeRegions(path, label string, bk gb.Backend, b *gb.Build, blob []byte) {
	f, err := os.Create(path)
	must(err)
	defer f.Close()
	fmt.Fprintln(f, "task,backend,region,start,end,bytes,source")

	if bk != gb.Groth16 {
		// PLONK's proof is a batch KZG opening whose byte layout this campaign has not
		// mapped. Saying "unknown" is the honest output; inventing boundaries would be
		// exactly the guess DeepProve's episode punished.
		fmt.Fprintf(f, "%s,%s,NOT_DETERMINED,0,%d,%d,plonk_proof_layout_not_mapped_by_this_campaign\n",
			label, bk, len(blob)-1, len(blob))
		return
	}

	nCommit := len(b.CCS.GetCommitments().CommitmentIndexes())
	const g1c, g2c, sliceLen = 32, 64, 4
	expected := g1c + g2c + g1c + sliceLen + nCommit*g1c + g1c
	regions := []struct {
		name        string
		start, size int
	}{
		{"Ar_G1_compressed", 0, g1c},
		{"Bs_G2_compressed", g1c, g2c},
		{"Krs_G1_compressed", g1c + g2c, g1c},
		{"Commitments_slice_length_prefix", g1c + g2c + g1c, sliceLen},
		{"Commitments_G1_compressed_x" + strconv.Itoa(nCommit), g1c + g2c + g1c + sliceLen, nCommit * g1c},
		{"CommitmentPok_G1_compressed", g1c + g2c + g1c + sliceLen + nCommit*g1c, g1c},
	}
	src := "gnark_v0.16.2_backend/groth16/bn254/marshal.go:33-57"
	for _, r := range regions {
		if r.size == 0 {
			continue
		}
		fmt.Fprintf(f, "%s,%s,%s,%d,%d,%d,%s\n", label, bk, r.name, r.start, r.start+r.size-1, r.size, src)
	}
	agree := "true"
	if expected != len(blob) {
		agree = "false"
	}
	fmt.Fprintf(f, "%s,%s,LAYOUT_TOTAL_MATCHES_ARTIFACT_%s,0,%d,%d,expected=%d_actual=%d_commitments=%d\n",
		label, bk, strings.ToUpper(agree), len(blob)-1, len(blob), expected, len(blob), nCommit)
	fmt.Fprintf(os.Stderr, "[neg] region map: %d commitment(s), derived total %d vs artifact %d (agrees=%s)\n",
		nCommit, expected, len(blob), agree)
}

// ------------------------------------------------------------------ io helpers

func roundTripFailed(why string) {
	out.Flush()
	fmt.Fprintln(os.Stderr, "ROUND TRIP FAILED — every other result in this file is meaningless")
	fmt.Fprintln(os.Stderr, "  "+why)
	fmt.Println("ROUND TRIP FAILED — every other result in this file is meaningless")
	os.Stdout.Sync()
	os.Exit(9)
}

func row(task, family, offset, det, verdict string) {
	fmt.Fprintf(out, "%s,%s,%s,%s,%s\n", task, family, offset, det, verdict)
}

func detail(pattern string, orig byte) string {
	return pattern + "_from" + hex.EncodeToString([]byte{orig})
}

func witnessLayout(b []byte) (nElems, elemOff, elemSize int) {
	// backend/witness: [uint32(nbPublic) | uint32(nbSecret) | uint32(len) | elements]
	if len(b) < 12 {
		return 0, 0, 0
	}
	nElems = int(uint32(b[8])<<24 | uint32(b[9])<<16 | uint32(b[10])<<8 | uint32(b[11]))
	elemOff = 12
	if nElems == 0 {
		return 0, elemOff, 0
	}
	elemSize = (len(b) - elemOff) / nElems
	return nElems, elemOff, elemSize
}

func writeTo(path string, w io.WriterTo) {
	f, err := os.Create(path)
	must(err)
	defer f.Close()
	_, err = w.WriteTo(f)
	must(err)
}

func loadVK(bk gb.Backend, path string) any {
	f, err := os.Open(path)
	must(err)
	defer f.Close()
	if bk == gb.Groth16 {
		vk := groth16.NewVerifyingKey(gb.Curve)
		_, err = vk.ReadFrom(f)
		must(err)
		return vk
	}
	vk := plonk.NewVerifyingKey(gb.Curve)
	_, err = vk.ReadFrom(f)
	must(err)
	return vk
}

func loadPublic(path string) witness.Witness {
	f, err := os.Open(path)
	must(err)
	defer f.Close()
	w, err := witness.New(gb.Curve.ScalarField())
	must(err)
	_, err = w.ReadFrom(f)
	must(err)
	return w
}

func parsePatterns(s string) []byte {
	var pats []byte
	for _, p := range strings.Split(s, ",") {
		p = strings.TrimSpace(strings.TrimPrefix(p, "0x"))
		if p == "" {
			continue
		}
		v, err := strconv.ParseUint(p, 16, 8)
		must(err)
		pats = append(pats, byte(v))
	}
	if len(pats) == 0 {
		pats = []byte{0x01}
	}
	return pats
}

func parseOffsets(path string, total int) []int {
	if path == "" {
		all := make([]int, total)
		for i := range all {
			all[i] = i
		}
		return all
	}
	data, err := os.ReadFile(path)
	must(err)
	var offs []int
	for _, line := range strings.Fields(string(data)) {
		n, err := strconv.Atoi(line)
		if err != nil {
			continue
		}
		offs = append(offs, n)
	}
	return offs
}

func env(k, def string) string {
	if v := os.Getenv(k); v != "" {
		return v
	}
	return def
}

func envInt(k string, def int) int {
	if v := os.Getenv(k); v != "" {
		n, err := strconv.Atoi(v)
		if err == nil {
			return n
		}
	}
	return def
}

func must(err error) {
	if err != nil {
		out.Flush()
		fmt.Fprintf(os.Stderr, "GNARK_FAIL class=NEGATIVE msg=%v\n", err)
		os.Exit(1)
	}
}
