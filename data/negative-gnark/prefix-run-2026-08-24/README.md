# The pre-fix correctness-control run — PUBLISHED, not regenerated

These are the negative-control CSVs **as first produced**, before the control was corrected.
They are kept because `bench/CHALLENGE.md` commits this repository, in writing, to *"not
remove an unflattering number, including our own"*, and `bench/README.md` promises raw data
committed uncurated. **This run is the evidence for the defect; the re-run beside it is the
corrected control.** Silently regenerating would violate the rule we are about to hold five
other teams to.

## What was wrong with it

The `witness_word` family perturbed a witness value and recorded whether `verify()` accepted.
It did **not** assert that the perturbation changed the public output. On any task containing a
ReLU, a weight feeding a neuron whose pre-activation is negative before and after the change is
discarded by the ReLU, so the output is bit-identical and the perturbed witness still satisfies
the same public statement.

Two of eight positions in `t2-groth16-rA.csv` were therefore not tests of anything:

    t2,witness_word,W[46112],plus1,VERIFY_ACCEPTED
    t2,witness_word,W[69168],plus1,VERIFY_ACCEPTED

Mechanism established in [`../MECHANISM-t2-accepted-witness.md`](../MECHANISM-t2-accepted-witness.md),
four positions for four, with the pre-activations measured.

## What was NOT wrong with it

**`proof_byte` and `public_input_word` are unaffected and remain at zero acceptances.** The
exhaustive proof-artifact sweeps in this directory — every byte of the Groth16 proof and every
byte of the PLONK proof, both backends, both regimes — stand exactly as recorded.

## And what it turned out to be about

Not only our harness. The same corruption class on the same task **is rejected by binius64 and
accepted by gnark**, and both are correct: they bind different things. See `RESULTS.md` §6.
