# gnark — tasks this system could not run, and why

> **This file cites no wall-clock time and no memory figure.** Constraint counts, MAC counts,
> proof sizes in bytes and correctness verdicts live here, because they are properties of the
> expression. **Every timing and every memory number lives in [`RESULTS.md`](RESULTS.md), with
> the conditions line that makes it meaningful.** A performance figure quoted without that line
> is not a figure this repository publishes.


`bench/README.md` commits to reporting a task a system cannot express as a **result**, not as a
gap. This file is that report for gnark.

The distinction the other four files draw is kept, and for this entry it is the whole
structure:

- **not expressible** — the frontend refuses to build the task;
- **expressible but not runnable** — the task builds and the prover, or the machine, then
  refuses it.

**For gnark the first column is empty, and that is the headline.** Every task in
`bench/TASKS.md`, in both regimes, on both backends — 28 of 28 — compiled to a constraint
system with no reshaping, no requantization, no width floor and no batch restriction. Where
gnark stops, it stops for reasons of *resources*, never of expressiveness.

| Task | Expressible? | Compiled? | Proved? | Where it stopped |
|---|---|---|---|---|
| **T1-0** | yes | **yes** | **yes** | — |
| **T1-a** | yes | **yes** | see RESULTS.md | — |
| **T1-b** | yes | **yes** | see RESULTS.md | — |
| **T1-c** | yes | **yes** | see RESULTS.md | 10 679 708 R1CS, FFT domain 2²⁴ — §2 |
| **T1-d** | yes | **yes** | see RESULTS.md | 39 175 652 R1CS, FFT domain 2²⁶ — §2 |
| **T2** | yes | **yes** | **yes** | — |
| **T3**, one proof | yes | **yes** | see RESULTS.md | — |

**WHAT THIS FILE MAY QUOTE.** Every figure below is one that is *invariant across campaign
runs*: constraint counts, variable counts, FFT domain sizes, proof and key byte counts, MAC and
activation counts, corruption verdicts. **No wall-clock time and no memory figure appears in
this file**, because those move between runs and belong to [`RESULTS.md`](RESULTS.md), which is
where the conditions line lives. This rule is not stylistic — it was adopted after a draft of
this file quoted compile times and heap sizes from a smoke run and the next campaign run
overwrote them, which is exactly how a benchmark acquires a figure with no file behind it.

That the rule is satisfiable is itself a measured result. Every compile-time figure in this
file and in [`EXPRESSION.md`](EXPRESSION.md) was produced twice — once by a harness smoke run
and once by the campaign's own `run-all.sh` — and **reproduced exactly**: all 28 constraint
counts, the `per_value` range-check curve, the four MAC-cost corners, both `pk_bytes` at the
padding boundary, `derivation_agrees` 20/20, and T2's and T3's `max_abs_intermediate` and
per-layer ReLU widths. The harness is deterministic in everything except the timings it is not
allowed to publish here.

Every cell's exact constraint count, **including for rungs that were never proved**, is in
[`EXPRESSION.md`](EXPRESSION.md) §9 and in
[`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv). This is the same capability
Ceno's emulator gave that entry: compiling is far cheaper than proving, so a rung too large to
prove still yields an exact measurement of how much work it is.

---

## 1. What was expressed, without deviation

### 1.1 The grid: 28 of 28

7 tasks × 2 regimes × 2 backends. Every one compiled, and every one passed the MAC assertion —
`macs_emitted` equals the frozen `macs` in all 28 rows of
[`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv). Compilation never died, not even
at T1-d PLONK regime A, the largest circuit in the grid: **79 332 096 SparseR1CS
constraints**, on a 32 GiB machine. (What that compile cost in seconds and in heap is in
[`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv) and moves between runs; it is not
quoted here, per the rule above.)

### 1.2 No minimum output-layer width — the trap that caught two other systems

**This probe ran first, before any timing**, because T2 ends in a `64→1` layer and a width
floor would have made half the bank `NOT_EXPRESSIBLE` regardless of how fast anything was.
Raw: [`probes-gnark-minwidth.txt`](../../data/probes-gnark-minwidth.txt).

| Probe | shape | Groth16 A | Groth16 B | PLONK A | PLONK B |
|---|---|---|---|---|---|
| `p1x1` | `1 → 1` | **OK** | **OK** | **OK** | **OK** |
| `p2x1` | `2 → 1` | **OK** | **OK** | **OK** | **OK** |
| `p64x1` | `64 → 1` | **OK** | **OK** | **OK** | **OK** |
| `p64x2` | `64 → 2` | **OK** | **OK** | **OK** | **OK** |
| `p64x4` | `64 → 4` | **OK** | **OK** | **OK** | **OK** |
| **`t2`** | full MLP, `…→64→1` | **OK** | — | **OK** | — |

**OK** means compiled, set up, proved *and verified* — not merely built. 20 of 20 probe cells
plus T2 on both backends — 22 `status=OK` lines in
[`probes-gnark-minwidth.txt`](../../data/probes-gnark-minwidth.txt), zero failures. `p64x1`
Groth16 regime A: **467 constraints, proof 196 B, verified**.

The comparison, extending the table jolt-atlas's own entry opened
([`jolt-atlas/NOT_EXPRESSIBLE.md`](../jolt-atlas/NOT_EXPRESSIBLE.md) §2):

| System | fails below output width | mechanism |
|---|---|---|
| DeepProve | **< 4** | sumcheck has no rounds over a 1- or 2-element tensor; failure lands in its inserted `Requant` layer |
| jolt-atlas | **< 2** | `tract` folds the unit dimension; the contraction is not in `EINSUM_REGISTRY` |
| **gnark** | **none found** | a dense layer is a linear combination and an equality; there is no tensor rank for a frontend to fold away |

The mechanism is worth naming because it is what makes the result transferable rather than
lucky: gnark has no tensor layer at all. `Define` writes field arithmetic directly, so there is
no shape inference, no einsum registry and no rank-1 special case that could reject a width-1
output.

### 1.3 No padding of the task

gnark pads its **FFT domain** to a power of two — measured, and the derivation from gnark's own
sizing expressions agrees with the domain the backend actually built in **20 of 20** cases
across power-of-two boundaries ([`probes-gnark-padding.txt`](../../data/probes-gnark-padding.txt),
column `derivation_agrees`). The boundary bites hard: `filler-1023` compiles to 1 024
constraints, domain 1 024, proving key 100 957 B; `filler-1024` compiles to 1 025 constraints,
domain 2 048, proving key **133 791 B**. One constraint, **32.5 %** more key.

**But the task is not reshaped.** `768` stays `768`. The number of MACs `bench/TASKS.md`
publishes and the number gnark performs are the same number, which is not true of DeepProve
(768 → 1024, 1.778× the MACs actually proved) or of jolt-atlas (pads every dimension; the
switch that appears to disable it does not work). The padding is a cost gnark pays in *memory*,
and [`RESULTS.md`](RESULTS.md) reports which of the two figures — constraint count or padded
domain — the memory follows.

### 1.4 No implicit requantization, no batch restriction

Amendment A1 fixes **no requantization** between T2's layers, and nothing in gnark inserts a
rescale: accumulators carry full width, which is why the ReLU widths grow 19 → 29 → 38 bits
through the network ([`EXPRESSION.md`](EXPRESSION.md) §5). DeepProve and jolt-atlas both
requantize and cannot be told not to; for gnark this was not a fight, it was the absence of a
feature that would have caused one.

T3 is **8 independent inputs proven in one proof**, as specified: 737 792 MACs, 3 584
activations, 973 058 R1CS constraints, one Groth16 proof. DeepProve's ONNX parser pins
`batch_size = 1` and cannot express it at all.

## 2. Expressible ≠ provable — the distinction this file exists to keep

**Compiling survived the entire ladder. That says nothing about whether it proves.** A Groth16
setup allocates a proving key sized by the *padded domain*, and it does so before a single
proof is computed, so **[HIPÓTESIS]** gnark's ladder should die in **setup**, not in prove — a
prediction from the shape of the algorithm, not a measurement, and one the reja will confirm or
refute — which would be a
different failure from every other system in this bank.

The ceiling will be reported as a **measured interval — the largest that worked and the
smallest that failed — with the literal error message, and never interpolated**
(`bench/CHALLENGE.md`: *"We did that once, it cost us a strategic decision, and the rule now is
absolute."*).

### 2.1 What is established so far

**Nothing about the ceiling.** The four cells that existed when this file was drafted were
harness smoke tests, and the campaign's own `run-all.sh` has since truncated and repopulated
[`cells-gnark.csv`](../../data/cells-gnark.csv) with its own. Quoting the smoke cells here would
have put figures in this file that the very next run erased — which is what happened to a draft
of this section, and is why the rule at the top of this file exists.

What is established is only this: **the ceiling is above the largest rung that the campaign
records as `status=OK` and below the smallest it records as `FAIL_setup` or `FAIL_prove`**, and
both ends are read from the ledger, not from here.

### 2.2 The gaps

| | value | source |
|---|---|---|
| Largest rung whose Groth16 **setup** completed | `[PENDIENTE-REJA]` | `cells-gnark.csv` |
| Smallest rung whose Groth16 **setup** failed, with its literal message | `[PENDIENTE-REJA]` | `cells-gnark/<label>/log.txt` |
| Largest rung whose Groth16 **prove** completed | `[PENDIENTE-REJA]` | `cells-gnark.csv` |
| Smallest rung whose Groth16 **prove** failed, with its literal message | `[PENDIENTE-REJA]` | `cells-gnark/<label>/log.txt` |
| Same four rows for PLONK | `[PENDIENTE-REJA]` | as above |
| Whether the failure is an OOM kill, a Go runtime abort, or a gnark error | `[PENDIENTE-REJA]` | `run-cell.sh` records `FAIL_setup` / `FAIL_prove` / `FAIL_oom` separately |
| Whether the boot volume, not RAM, was the binding constraint | `[PENDIENTE-REJA]` | `run-cell-guarded.sh` records `KILLED_DISK` |

`run-cell.sh` classifies each failure into its own greppable status, and the runner writes one
`GNARK_FAIL class=… exit=…` line per failure class, so the interval can be filled from the raw
logs without re-running anything.

To bracket the ceiling more finely than the ladder's rungs allow,
[`cmd/probe`](../../tasks/gnark/cmd/probe/) has a `filler` mode that compiles, sets up and
proves a circuit of exactly *N* multiplication constraints:

```
gnark-probe filler <N> <groth16|plonk> [compile|setup|prove]
```

It exits with a distinct code per stage (13 compile, 20 setup, 30 prove) so a bisection can be
driven from the shell. **It has not been run.** Any ceiling stated before it has is a guess.

## 3. What was NOT measured, and why

### 3.1 GPU — out of scope, and the API is deprecated anyway

`bench/README.md` declares round one **CPU only**, *"so the comparison is between protocols
rather than between kernel-porting efforts."* That alone settles it. But the state of gnark's
GPU path is worth recording, because a reader may assume we simply did not try:
`backend.WithIcicleAcceleration` **no longer works**, by gnark's own docstring at v0.16.2:

```go
// DEPRECATED: we don't switch to ICICLE automatically anymore, the user has to
// explicitly use methods in the [github.com/consensys/gnark/backend/accelerated/icicle]
// package to use ICICLE acceleration. This option will be removed in a future release,
// but kept for now for API backward compatibility. It will error at runtime instead.
```
`backend/backend.go:125-138`

The option returns an error rather than accelerating anything. The live path is a separate
package, `backend/accelerated/icicle`, which this campaign did not build, did not link and did
not measure. **No figure here is a GPU figure and none should be read as an upper bound on what
gnark can do on a GPU.**

### 3.2 The PLONK SRS is not a ceremony, and that bounds what the PLONK numbers mean

Every PLONK figure in this entry uses an SRS from gnark's `test/unsafekzg`, whose own package
doc is unambiguous:

```go
// Package unsafekzg is a convenience package (to be use for test purposes only)
// to generate and cache SRS for the kzg scheme (and indirectly for PlonK setup).
```
`test/unsafekzg/kzgsrs.go:1-4`

**There is no multi-party ceremony behind it and its toxic waste is generated in the measuring
process.** The runner prints that on stderr for every PLONK cell rather than leaving it in a
README, and SRS generation time is reported in its **own field** (`srs_ms`), never inside
setup and never inside prove. **On the cells measured so far SRS generation dominates PLONK
setup by roughly an order of magnitude**, so folding it in would have misattributed most of the
one-off cost; the exact ratio per cell is `srs_ms` against `ms` on each `SETUP` line in
[`cells-gnark/`](../../data/cells-gnark/) and is `[PENDIENTE-REJA]` here, because it is a
timing and this file does not quote timings.

What this costs the comparison, stated plainly: the PLONK **timings and memory** are honest —
the prover does not know the SRS is unsafe and does the same work either way. The **security
column** is not: a real deployment needs a ceremony SRS of at least the padded domain size, and
obtaining one is a cost this benchmark did not pay and did not measure. `bench/README.md`
requires differences no normalization can fix to be *declared, not averaged*; this is one.

### 3.3 Not measured, not claimed

- **Zero-knowledge.** `backend.WithStatisticalZeroKnowledge` is off, so **gnark's default
  Groth16 path here is not statistical ZK** and every META line says `statistical_zk=false`.
  Nothing in this entry measures zero-knowledge; the benchmark's separation of privacy from
  verifiable integrity is strict and round one measures the latter.
- **Recursion / aggregation.** `std/recursion` exists and is untouched. It is the component
  that would matter most for chaining proofs, and DEC-7 puts recursion outside this project's
  stack anyway.
- **Solidity verification.** gnark generates Solidity verifiers; not exercised, not measured.
  The on-chain segment's cost is a separate question (D-009).
- **Other curves.** BN254 only. Changing the curve changes the security column, so it is a
  constant in [`build.go`](../../tasks/gnark/build.go) and not a flag.
- **LLM inference.** No task in `bench/TASKS.md` is a language model. T2's MLP is 92 224 MACs;
  **[INFERENCIA]** a GPT-2 forward pass is on the order of 10⁴× the top of this ladder. That
  is an order-of-magnitude estimate from the model's parameter count, not a measurement and not
  sourced to a primary reference; it is here to size the gap, and no claim in this benchmark
  rests on it.

## 4. Limits of OUR expression, not of gnark

Three, and they are ours to own.

### 4.1 The ReLU bit width comes from the measured instance, so T2/T3 are instance-bounded

The gadget range-checks each activation to `B` bits, and `B` is derived per ReLU site from the
magnitude that site **actually reaches in this instance**
([`reference.go`](../../tasks/gnark/reference.go)), not from a static bound over all admissible
INT8 inputs. `bench/TASKS.md` asks for the accumulator bound to be asserted; it does not say
where the circuit's range bound should come from, and we chose measured.

**What that means: the T2/T3 circuits are valid for instances no larger than this one**, not
for every admissible INT8 input. A different seed with larger activations would need a
recompile. This is normal practice for a fixed-model zkML circuit — static bounds analysis is
the alternative — but it is a property of our expression and a reader must not take the ReLU
cost in [`EXPRESSION.md`](EXPRESSION.md) §4 as the cost of a universally valid gadget.

The static bound is published beside the measured one so the other choice can be priced.
**Provenance differs by column and is marked, because two of these are ours and two are the
harness's.** `measured max |x|` and `B used` are read from `relu_bits` in
[`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv). The `static worst case` column is
**our own arithmetic**, layer by layer (`128 · Πᵢ 128·inᵢ`) — the CSV's `static_worst_case`
field is the whole-network product (1.44·10¹⁹), not a per-layer figure — and the last column is
`bits(static) + 1`, the same rule the harness applies to the measured value:

| ReLU site | measured max &#124;x&#124; *(CSV)* | **B used** *(CSV)* | static worst case *(derived here)* | B a static bound would need *(derived here)* |
|---|---:|---:|---:|---:|
| T2 layer 1 | 255 443 | **19** | 3 276 800 | 23 |
| T2 layer 2 | 151 720 706 | **29** | 1.07·10¹¹ | 38 |
| T2 layer 3 | 81 075 383 855 | **38** | 1.76·10¹⁵ | 52 |

At `tobinary`'s measured `B + 4` R1CS per activation, the static choice would cost 4, 9 and 14
more constraints per activation at the three layers. We did not compile it; that arithmetic is
an estimate of the *difference*, not a measurement, and it is labelled as such.

### 4.2 `witness_word` was sampled, and its first run reported an acceptance that was not one

The `public_input_word` family is **exhaustive** (256 of 256 rejected) and the `proof_byte`
sweep is **exhaustive** (every byte of the artifact, both backends). The `witness_word` family
— corrupting a secret value before proving — is **sampled**, because each sample costs a full
proof. The sample size is printed by the tool and recorded in
[`negative-gnark/`](../../data/negative-gnark/); **nothing is inferred about the positions not
touched.**

**The episode, because it is the third time this campaign has met an accepted corruption and
the first time the mechanism was established the same day.** The first campaign run reported:

```
t2,witness_word,W[46112],plus1,VERIFY_ACCEPTED
t2,witness_word,W[69168],plus1,VERIFY_ACCEPTED
```

That reads as a soundness finding. **It is not one, and the mechanism is measured rather than
argued: a ReLU is not injective.** A weight feeding a neuron whose pre-activation is negative is
zeroed by the activation and never reaches the output, so incrementing it produces a *different
witness for the same true statement*. Recomputing T2's reference forward pass with either weight
bumped gives the output bit-identical to the honest one — **14 623 789 560 139** in both cases.
A verifier that accepts such a proof is behaving correctly; there is nothing to reject.

**So the defect was ours, in the family's design.** "Corrupt a witness value" and "corrupt the
statement being proved" are the same operation only on a network with no activations — which is
why T1, a pure matmul, had produced 6 of 6 rejections and never exposed it. The fix
([`statement.go`](../../tasks/gnark/statement.go)) recomputes the reference forward pass
**before** proving and splits the outcomes:

| verdict | meaning |
|---|---|
| `PROVE_REJECTED` | the bump changed the statement (or left INT8 range) and the solver refused |
| **`WITNESS_INERT`** | the bump does not change the public output. Nothing to reject. **Not counted as an accepted corruption.** |
| `VERIFY_ACCEPTED` | reserved for what it is supposed to mean: **the statement changed and the verifier let it through.** That would be a soundness finding, and the tool now emits a `GNARK_ALERT` line when it happens |

Inert positions are replaced by live ones so the family still performs its intended number of
real tests, and three regression tests pin the behaviour
([`statement_test.go`](../../tasks/gnark/statement_test.go)): the two offending positions must
classify as `WITNESS_INERT`, a live position must not be excused as inert, and **a matmul must
have no inert weights at all** — the control on the control, since T1's arithmetic forbids a
dead weight.

**The inert fraction is a result, not noise — and the sample got it wrong.** It measures how
much of a ReLU network's weight tensor is dead for a given input. We first estimated it from
the 256 sampled probe positions, then computed it **exhaustively over all 92 224 weights**
([`inert-weights.txt`](../../data/repro-gnark/inert-weights.txt)). Both are published, because
the disagreement is the lesson:

| Task | 256-position sample | **exhaustive, all 92 224 weights** |
|---|---:|---:|
| T2 (batch 1) | 29.3 % | **52.27 %** (48 208) |
| T3 (batch 8) | 4.3 % | **3.27 %** (3 016) |

**The T2 sample is off by 23 points, far outside sampling error for n = 256** (standard error
~3 %), so the probe's position selection is not independent of the layer structure it is
sampling. The exhaustive figure is the one to cite; the sampled one is kept as the record of
how it was first mis-measured. Criterion, stated because the number is sensitive to it: a
weight is inert iff the neuron it feeds has pre-activation ≤ 0 both before and after the `+1`,
**for every batch item**.

The T2/T3 gap is the batch, and it is structural: a weight is dead in T3 only if its neuron is
negative for **all eight** inputs. So batching does not merely amortize the weights across
proofs (§6 of [`EXPRESSION.md`](EXPRESSION.md)) — **it hardens the witness-binding property by
about 16×.** **This figure is reported and not used**: no timing, memory or cost claim anywhere
in this entry depends on it.

**Both runs are published; neither was silently regenerated.** `bench/CHALLENGE.md` promises
this repository will not remove an unflattering number, *including our own*, and
`bench/README.md` promises raw data committed uncurated. So the pre-fix CSVs are preserved
verbatim under
[`negative-gnark/prefix-run-2026-08-24/`](../../data/negative-gnark/prefix-run-2026-08-24/),
with a README stating which run is which: **the old run is the evidence for the defect in our
control; the new one at the top level is the corrected control**, and it reports zero
non-control acceptances. Regenerating quietly would have broken a promise we are about to hold
five other teams to.

The `proof_byte` and `public_input_word` families are untouched by any of this: they corrupt
the artifact and the public statement directly, where inertness cannot arise, and both stand at
**zero acceptances** in both runs.

**And the phenomenon is not only about our harness — it separates the systems.** On the *same
task and the same corruption class*, binius64 returns `VERIFY_REJECTED` where gnark returns
`VERIFY_ACCEPTED`, including on a witness word whose original value was literally zero
(`bench/data/negative/t2/negative-control.csv`, `private_word/middle`). **Both are correct.**
binius64 **commits to the witness**, so altering any committed word breaks the commitment
whether or not the output moves; Groth16 with witness weights proves **existential
satisfiability**, and an inert weight is another valid witness for the same true statement.
Neither system is better on this axis — they bind different things, and
[`RESULTS.md`](RESULTS.md) §8.2 argues the comparative table needs a column for it.

### 4.3 The PLONK proof's byte layout was not mapped

The `proof_byte` sweep is exhaustive on both backends, but the **region map** — which offsets
belong to which field — exists only for Groth16, where it is read out of
`backend/groth16/bn254/marshal.go:33-57` and **verified against the artifact** (derived total
196 B = actual 196 B, 1 commitment). For PLONK the map records `NOT_DETERMINED`
([`negative-gnark/cache/t1-0-plonk-rA/regions.csv`](../../data/negative-gnark/)): a batched KZG
opening proof, and we did not reverse its layout. Since **zero** offsets were accepted on
either backend there is nothing the map would have had to explain — but had a byte been
accepted, PLONK is the backend where we could not have said what it was, and that is worth
knowing before the next campaign rather than after.

### 4.4 One number this entry deliberately does not stabilise

The Groth16 prover is randomized, so **every campaign run sweeps a different proof artifact**,
and the split of the 196 rejections between `DESERIALIZE_REJECTED` and `VERIFY_REJECTED` moves
with it: a flip lands in a point encoding that fails to decode, or in one that decodes to a
different valid point and fails the pairing check, depending on the bytes that run produced.

**The zero-accepted result is the stable one. The split is not.** Whatever
[`negative-gnark/t1-0-groth16-rA-exhaustive.csv`](../../data/negative-gnark/) currently holds is
one run's split and must be quoted as one run's, from that file, with its date — never as a
property of the format. A draft of this section quoted two different splits observed in one
working session; only one of them was ever written to
[`bench/data/`](../../data/), and a figure whose only home was a scratch directory is a figure
this benchmark does not have. It has been removed rather than reconstructed.
