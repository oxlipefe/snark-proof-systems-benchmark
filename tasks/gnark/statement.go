package gnarkbench

// Recomputing the statement, so the correctness control can tell a corrupted WITNESS from a
// corrupted STATEMENT.
//
// THE EPISODE THIS FILE EXISTS BECAUSE OF. The first campaign run of the correctness control
// reported two accepted corruptions on T2:
//
//	t2,witness_word,W[46112],plus1,VERIFY_ACCEPTED
//	t2,witness_word,W[69168],plus1,VERIFY_ACCEPTED
//
// They are not a soundness finding. **A ReLU is not injective.** A weight feeding a neuron
// whose pre-activation is negative is zeroed by the activation and therefore does not reach
// the output at all; incrementing it produces a DIFFERENT WITNESS FOR THE SAME TRUE
// STATEMENT. The verifier accepting it is correct behaviour — there is nothing to reject.
// Measured, not argued: recomputing T2's reference forward pass with either weight bumped
// gives the identical output, 14 623 789 560 139.
//
// So "corrupt a witness value" and "corrupt the statement being proved" are not the same
// operation on a network with activations, and a control that conflates them reports the
// prover's honesty as the verifier's failure. This file separates them BEFORE the proof is
// attempted, so the sweep can label an inert bump as inert and reserve VERIFY_ACCEPTED for
// what it is supposed to mean: a corruption of the statement that the verifier let through.
//
// The inert fraction is not noise to be discarded. It is a measurement of how much of a
// ReLU network's weight tensor is dead for a given input, and it is reported.

// StatementEffect is what a one-value bump does to the proved statement, decided before
// proving by recomputing the reference in int64.
type StatementEffect struct {
	Position SecretPosition
	Layer    int   // -1 for a matmul task or an input position
	Honest   int64 // the honest value at that position
	InRange  bool  // whether honest+1 is still a valid INT8, i.e. survives the range check
	Changes  bool  // whether honest+1 changes the public output
}

// Verdict names the outcome this bump SHOULD produce, so the sweep can flag a disagreement
// rather than silently record whatever happened.
func (e StatementEffect) Expected() string {
	switch {
	case !e.InRange:
		return "PROVE_REJECTED_range" // w+1 = 128 is not an INT8; the range check refuses it
	case e.Changes:
		return "PROVE_REJECTED_statement" // the output moved; AssertIsEqual refuses it
	default:
		return "WITNESS_INERT" // a valid witness for the same statement; nothing to reject
	}
}

// EffectOfBump determines, without proving, what incrementing one secret value does.
func EffectOfBump(spec Spec, regime Regime, ref *Reference, pos SecretPosition) (StatementEffect, error) {
	e := StatementEffect{Position: pos, Layer: -1}

	base, err := recomputeOutputs(spec, ref)
	if err != nil {
		return e, err
	}
	mod, err := cloneWithBump(spec, regime, ref, pos, &e)
	if err != nil {
		return e, err
	}
	after, err := recomputeOutputs(spec, mod)
	if err != nil {
		return e, err
	}
	e.InRange = e.Honest+1 <= 127
	for i := range base {
		if base[i] != after[i] {
			e.Changes = true
			break
		}
	}
	return e, nil
}

// cloneWithBump returns a copy of ref with one operand incremented, and records which layer
// and which honest value were touched.
func cloneWithBump(spec Spec, regime Regime, ref *Reference, pos SecretPosition, e *StatementEffect) (*Reference, error) {
	c := &Reference{Spec: spec}
	switch spec.Kind {
	case KindMatMul:
		c.X = append([]int8(nil), ref.X...)
		c.W = append([]int8(nil), ref.W...)
		tgt := c.X
		if pos.Slice == "W" {
			tgt = c.W
		}
		if pos.Index < 0 || pos.Index >= len(tgt) {
			return nil, errPositionOutOfRange(pos, len(tgt))
		}
		e.Honest = int64(tgt[pos.Index])
		tgt[pos.Index]++
	case KindMLP:
		c.XB = append([]int8(nil), ref.XB...)
		c.LW = make([][]int8, len(ref.LW))
		for i := range ref.LW {
			c.LW[i] = append([]int8(nil), ref.LW[i]...)
		}
		if pos.Slice == "X" {
			if pos.Index < 0 || pos.Index >= len(c.XB) {
				return nil, errPositionOutOfRange(pos, len(c.XB))
			}
			e.Honest = int64(c.XB[pos.Index])
			c.XB[pos.Index]++
			break
		}
		off := 0
		found := false
		for l, lay := range spec.Layers {
			n := lay.In * lay.Out
			if pos.Index >= off && pos.Index < off+n {
				e.Layer = l
				e.Honest = int64(c.LW[l][pos.Index-off])
				c.LW[l][pos.Index-off]++
				found = true
				break
			}
			off += n
		}
		if !found {
			return nil, errPositionOutOfRange(pos, off)
		}
	}
	return c, nil
}

// recomputeOutputs runs the reference forward pass in int64. Amendment A1's bound has already
// been asserted on this instance, so int64 is safe here; a bump of one is far inside it.
func recomputeOutputs(spec Spec, r *Reference) ([]int64, error) {
	switch spec.Kind {
	case KindMatMul:
		out := make([]int64, spec.M*spec.N)
		for m := 0; m < spec.M; m++ {
			for n := 0; n < spec.N; n++ {
				var acc int64
				for k := 0; k < spec.K; k++ {
					acc += int64(r.X[m*spec.K+k]) * int64(r.W[k*spec.N+n])
				}
				out[m*spec.N+n] = acc
			}
		}
		return out, nil
	case KindMLP:
		last := spec.Layers[len(spec.Layers)-1].Out
		out := make([]int64, spec.Batch*last)
		for b := 0; b < spec.Batch; b++ {
			cur := make([]int64, spec.Layers[0].In)
			for i := range cur {
				cur[i] = int64(r.XB[b*spec.Layers[0].In+i])
			}
			for l, lay := range spec.Layers {
				next := make([]int64, lay.Out)
				for o := 0; o < lay.Out; o++ {
					var acc int64
					for i := 0; i < lay.In; i++ {
						acc += cur[i] * int64(r.LW[l][i*lay.Out+o])
					}
					if lay.ReLU && acc < 0 {
						acc = 0
					}
					next[o] = acc
				}
				cur = next
			}
			for o := 0; o < last; o++ {
				out[b*last+o] = cur[o]
			}
		}
		return out, nil
	}
	return nil, errUnknownKind(spec)
}
