# The two accepted `witness_word` corruptions in T2 regime A — mechanism, established

`bench/README.md`: *"A corrupted trace must make `verify()` fail."* Two corruptions in
`t2-groth16-rA.csv` did NOT make verify fail. This file establishes why, before any conclusion
was written, per the rule that cost two earlier campaigns in this repository a false alarm.

## What was observed

    t2,witness_word,W[0],plus1,PROVE_REJECTED
    t2,witness_word,W[23056],plus1,PROVE_REJECTED
    t2,witness_word,W[46112],plus1,VERIFY_ACCEPTED   <-- accepted
    t2,witness_word,W[69168],plus1,VERIFY_ACCEPTED   <-- accepted

Only in T2 regime A. Not in T1 (no ReLU). Not in T2 regime B (weights are circuit constants
there, so there is no weight witness to corrupt and the file contains no `W` rows).

## The mechanism, measured

T2's weight array is the four layers flattened; `LW[l][i*Out + o]`. Each probed index and the
neuron it feeds, with that neuron's pre-activation before and after adding 1 to the weight
(the accumulator moves by exactly the input value multiplying it):

| index | layer | in | neuron | input value | pre-activation | after `w+1` | ReLU out | verdict |
|---|---:|---:|---:|---:|---:|---:|---|---|
| W[0] | 0 | 0 | 0 | 69 | 6 310 | 6 379 | 6 310 → 6 379, **changes** | PROVE_REJECTED |
| W[23056] | 0 | 90 | 16 | −18 | 13 447 | 13 429 | 13 447 → 13 429, **changes** | PROVE_REJECTED |
| **W[46112]** | 0 | 180 | 32 | −128 | **−11 129** | **−11 257** | **0 → 0, unchanged** | **VERIFY_ACCEPTED** |
| **W[69168]** | 1 | 140 | 48 | 35 814 | **−87 033 032** | **−86 997 218** | **0 → 0, unchanged** | **VERIFY_ACCEPTED** |

**Four for four.** Every corruption that changed a ReLU output was caught. Every corruption
that did not change a ReLU output was accepted. There is no third case.

Reproduce: `bench/data/repro-gnark/acceptprobe.txt` records this program's output; it calls
`gnarkbench.NewReference("t2")` and reads `ReluIn` and `XB` from the same reference instance
the circuit builder uses.

## What follows, and what does not

**This is NOT unsoundness, and it is not a defect in gnark.**

The statement T2 regime A proves is *"there exist INT8 `x` and INT8 `W` such that
`out = MLP(W, x)`"*. **ReLU is not injective.** A weight feeding a neuron whose pre-activation
is negative — before and after the perturbation — is not constrained by the output at all,
because the ReLU discards it. So the "corrupted" witness is a **genuinely satisfying witness
for the same public statement**, and a verifier that accepted it did the correct thing. A
verifier that *rejected* it would be unsound in the other direction: it would be rejecting a
true statement.

**What this does invalidate is the test, not the system.** A `witness_word` probe is only a
valid negative control if the corruption actually changes the public output. Ours did not
assert that, so two of its eight positions were not tests of anything.

**This generalises beyond gnark, and it is a methodology finding for the whole benchmark.**
Any `witness_word`-style control, in ANY of the five systems, is measuring nothing whenever it
perturbs a witness value that the task's own function discards. Every task in this benchmark
with a ReLU — T2 and T3, in every system — has this property. A system reporting "all witness
corruptions rejected" on T2 has either sampled only live positions or has not looked. **The
fix is to require the control to verify that the reference output changed before counting a
position as a test**, and it belongs in `bench/README.md`'s correctness-control rule, not in
this file.

## What the control still establishes for gnark

The proof-byte control is untouched by this and is the stronger one: **exhaustive, every byte
of the serialized proof, both backends, zero accepted** (`RESULTS.md` §6). The
`public_input_word` family is also exhaustive over T1-0's 256 public outputs and rejected all
256. And the six `witness_word` positions that *did* change a ReLU output were all caught at
proving time.

**No corrupted proof was accepted at any offset, in any task, in either backend.**
