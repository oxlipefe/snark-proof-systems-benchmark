# gnark — how each task was expressed

> **This file cites no wall-clock time and no memory figure.** Constraint counts, MAC counts,
> proof sizes in bytes and correctness verdicts live here, because they are properties of the
> expression. **Every timing and every memory number lives in [`RESULTS.md`](RESULTS.md), with
> the conditions line that makes it meaningful.** A performance figure quoted without that line
> is not a figure this repository publishes.


`bench/TASKS.md` fixes each task by an **exact MAC count**. That count is the denominator of
both `MAC/s` and `bytes/MAC`, so it is never recomputed here. Every circuit in
[`bench/tasks/gnark/`](../../tasks/gnark/) counts the multiply-accumulates it actually emits
and **refuses to compile** if the count disagrees with the published one:

```go
if macs != s.MACs {
    return MACAssertionError(s.Label, macs, s.MACs)
}
```
[`circuit_matmul.go`](../../tasks/gnark/circuit_matmul.go), [`circuit_mlp.go`](../../tasks/gnark/circuit_mlp.go)

**WHAT THIS FILE MAY QUOTE.** Only figures that are *invariant across campaign runs* —
constraint counts, variable counts, FFT domain sizes, byte counts, MAC and activation counts,
measured magnitudes of the reference instance. **No wall-clock time and no memory figure**;
those move between runs and belong to [`RESULTS.md`](RESULTS.md) with its conditions line.
Every figure below was produced twice, by a harness smoke run and by the campaign's own
`run-all.sh`, and reproduced exactly.

Nothing in this directory is gnark code. gnark v0.16.2 is linked as an ordinary Go module
dependency, checksummed in `go.sum`, and called through its public API only.

---

## 1. THE DECLARATION THAT GOVERNS EVERY NUMBER BELOW: two regimes, never mixed

gnark is a circuit frontend over BN254, and a circuit gets to choose whether the model weights
are *witnessed* or *compiled in*. That choice changes the statement being proved, not just its
cost, so this entry measures both and labels every artifact with which one produced it.

**Regime A — "witness weights". THE HEADLINE, and the only cross-system comparable figure.**
The proved statement is:

> *There exist `x ∈ INT8^K` and `W ∈ INT8^(K×N)` such that `out = W·x`, and the prover knows
> them.*

Both the input vector and every weight are secret witness variables, and every INT8 value
entering the circuit is range-checked (§2). Each MAC is one real R1CS multiplication.

**Regime B — "baked weights". A DECLARED LEVER. Never a cross-system number.**
The proved statement is:

> *There exists `x ∈ INT8^K` such that `out = W₀·x`, where `W₀` is a **fixed, public matrix
> compiled into the circuit**.*

The weights are Go compile-time constants. In R1CS a linear combination with constant
coefficients folds into the linear expression for free, so a whole matmul collapses to about
one constraint per output (§3).

**B proves a WEAKER statement about `W` and a STRONGER one about the deployment.** Weaker:
nothing in a regime-B proof establishes anything about `W₀` — it is not proved, it is
*assumed*, because it is part of the circuit. Stronger: Groth16 runs a per-circuit setup, so
`W₀` is bound into the **verifying key**. A verifier holding that vk cannot be made to accept
a proof computed against different weights, because a different `W₀` is a different circuit
with a different vk. That is model-binding without a commitment scheme, paid for at setup time
and never at proving time.

**No other system in this bank can do that.** binius64, Ceno, DeepProve and jolt-atlas all
carry the weights as data — witness, hints, or tensor inputs — so for them "which model was
this?" is a question about a commitment or about nothing at all. gnark's per-circuit trusted
setup, normally counted as a cost, is here the mechanism. This is `PROTOCOL.md` §2's
never-evaluated lever *"explotación de pesos fijos por precómputo"*, and this entry is the
first measurement of it.

The trade is not free and is stated here rather than in a footnote: a regime-B circuit is
**one model**. Changing a single weight means recompiling, re-running setup, and distributing
a new vk. Regime B is what a deployed fixed-model service wants; it is not what a service that
updates its model wants.

Every ledger row, every CSV row and every grid in [`RESULTS.md`](RESULTS.md) carries its
regime, and [`report.py`](../../scripts/gnark/report.py) has no code path that can average
them.

## 2. Expressing an INT8 in a prime field, and what it costs

BN254's scalar field is ~2^254. An INT8 is carried as the field element it equals (negatives as
`r − |v|`), which costs nothing — **but nothing about that element says it is 8 bits wide.**
In a prime field, 8-bit-ness has to be *proved*. Every INT8 value entering the circuit is
shifted into `[0, 255]` and range-checked:

```go
for i := range c.X {
    rc.Check(api.Add(c.X[i], 128), 8)
}
if c.regime == RegimeA {
    for i := range c.W {
        rc.Check(api.Add(c.W[i], 128), 8)
    }
}
```
[`circuit_matmul.go`](../../tasks/gnark/circuit_matmul.go)

`rc` is `std/rangecheck`, constructed **once per circuit** in
[`relu.go`](../../tasks/gnark/relu.go) so that every check in the circuit shares one lookup
table. Because BN254's R1CS builder implements `frontend.Committer`, `rangecheck.New` returns
the commit-based variant — the log-derivative product argument — which is why a Groth16 proof
of any of these circuits carries a Pedersen commitment and measures 196 bytes instead of the
164 bytes gnark's own commitment-free examples produce (§3 of [`RESULTS.md`](RESULTS.md);
raw: [`probes-gnark-example.txt`](../../data/probes-gnark-example.txt)).

### The per-value cost is NOT a constant

It amortizes a shared lookup table. Quoting one number for "the cost of a range check" would
be false at every `n` but one. Measured, isolated, `n` 8-bit checks and nothing else
— [`probes-gnark-rangecheck.txt`](../../data/probes-gnark-rangecheck.txt):

| values checked | R1CS total | **R1CS / value** | SparseR1CS total | **SCS / value** |
|---:|---:|---:|---:|---:|
| 16 | 67 | **4.1875** | 209 | **13.0625** |
| 64 | 211 | **3.2969** | 689 | **10.7656** |
| 256 | 771 | **3.0117** | 2 305 | **9.0039** |
| 448 | 1 156 | **2.5804** | 3 457 | **7.7165** |
| 1 024 | 2 309 | **2.2549** | 6 913 | **6.7510** |
| 4 096 | 8 459 | **2.0652** | 25 345 | **6.1877** |
| 65 792 | 131 972 | **2.0059** | 395 521 | **6.0117** |

The per-value cost falls by 2.1× on R1CS and 2.2× on SparseR1CS across this range and is still
falling at the top. `65 792 = 256 + 65 536` is T1-0 regime A's own operand count, so the last
row is the rate that circuit actually pays. **Nothing in
[`report.py`](../../scripts/gnark/report.py) hardcodes a per-value figure; it reports totals.**

### The contrast, stated fairly, because it is an asymmetry in this benchmark

In a **binary field** or on a **byte-addressed machine**, 8-bit-ness is structural rather than
proved. Ceno reads each operand with a sign-extending `lb`: *"every byte trivially is [in
range], since a byte is 8 bits… here the 8-bit width is structural"*
([`ceno/EXPRESSION.md`](../ceno/EXPRESSION.md) §2). gnark cannot get that for free, and this
entry declines to take it for free either.

But the honest comparison is narrower than "gnark pays and others do not". **binius64 does not
pay it and says so**: its operands are witnessed as full 64-bit words with no range constraint,
and its own entry states *"A production deployment would need those range constraints, and
they are not in these numbers"* ([`binius64/EXPRESSION.md`](../binius64/EXPRESSION.md) §5).

So on this axis there are three positions, not two, and a cross-system table must say which
one each column is in:

| System | 8-bit-ness | in the published numbers? |
|---|---|---|
| **gnark, regime A** | **proved**, `std/rangecheck` | **yes** |
| Ceno | structural (`lb` of a byte) | free, nothing to include |
| binius64 | **not established** | **no** — declared gap in its own entry |

gnark's regime-A figures therefore carry a cost binius64's do not. That is a real asymmetry,
it favours binius64 in any head-to-head, and it is declared here rather than discovered later.

## 3. What one MAC costs

Measured on a single 256-MAC dot product with **no range checks at all**, so the price of the
multiply is separated from the price of proving 8-bit-ness. Compiled, proved and verified in
all four corners — [`probes-gnark-maccost.txt`](../../data/probes-gnark-maccost.txt):

| backend | weights | constraints for 256 MACs | **per MAC** |
|---|---|---:|---:|
| Groth16 (`r1cs.NewBuilder`) | **constants** (regime B) | **1** | **0.0039** |
| Groth16 (`r1cs.NewBuilder`) | **witness** (regime A) | **257** | **1.0039** |
| PLONK (`scs.NewBuilder`) | **constants** (regime B) | **255** | **0.9961** |
| PLONK (`scs.NewBuilder`) | **witness** (regime A) | **512** | **2.0000** |

**Why R1CS folds a constant-coefficient linear combination for free.** An R1CS constraint is
`⟨a,z⟩ · ⟨b,z⟩ = ⟨c,z⟩` — a product of two *linear combinations* of wires. Scaling a wire by a
field constant only changes a coefficient inside a linear combination, so `api.Mul(x, 5)`
emits no constraint; it rewrites an expression. A dot product against constant weights is one
linear combination, and the only constraint is the `AssertIsEqual` that binds it to the public
output. Hence **1** constraint for 256 MACs. With witness weights each `api.Mul(x_i, w_i)` is a
genuine product of two non-constant linear combinations and must be its own constraint: 256
multiplications plus the final equality — **257**.

PLONK's arithmetization has no such fold. A SparseR1CS gate is
`qL·a + qR·b + qM·a·b + qO·c + qC = 0`: one gate holds one addition or one multiplication, so
even a constant-coefficient accumulation costs one gate per term (**255** for a 256-term sum),
and a witness-weighted MAC costs a multiply gate plus an add gate (**512**). PLONK is
therefore about 2× R1CS in regime A and about 255× in regime B — the lever is an R1CS lever,
and it barely exists on PLONK.

## 4. The ReLU gadget

`bench/TASKS.md` puts a ReLU after layers 1–3 of T2 and T3 and counts the 448 activations
separately from the 92 224 MACs. In a prime field there is no sign bit to read, so "is `x`
negative" must be decided by constraints that cannot see the integer. Two gadgets were built
and measured; **`hintedsign` is the one kept** (`DefaultReluGadget`,
[`relu.go`](../../tasks/gnark/relu.go)).

```go
func reluHintedSign(api frontend.API, rc frontend.Rangechecker, x frontend.Variable, b int) frontend.Variable {
    out, err := api.Compiler().NewHint(signHint, 1, x)
    if err != nil { panic(err) }
    s := out[0]
    api.AssertIsBoolean(s)
    y := api.Mul(s, x)
    rc.Check(y, b)
    rc.Check(api.Sub(y, x), b)
    return y
}
```

**The hint is UNCONSTRAINED by construction.** `signHint` runs outside the circuit and a lying
prover may return whatever it likes for `s`. What makes the gadget sound is that the two range
checks leave a liar no satisfying assignment, given `|x| < 2^B`:

- `s = 1` ⇒ `y = x` and `y − x = 0` ⇒ `rc.Check(y, B)` forces `x ∈ [0, 2^B)`;
- `s = 0` ⇒ `y = 0` and `y − x = −x` ⇒ `rc.Check(y − x, B)` forces `−x ∈ [0, 2^B)`, i.e. `x ≤ 0`.

Exactly one branch is admissible for any `x` in range, except `x = 0` where both give `y = 0`
— two observationally identical witnesses, not a soundness hole. **That argument is a claim
about the code, so it is tested rather than asserted.**

### The evidence of correctness

[`relu_test.go`](../../tasks/gnark/relu_test.go), both gadgets, both backends
(`test.WithBackends(backend.GROTH16, backend.PLONK)`), run under `-tags=prover_checks` so
gnark's `test.NewAssert` performs the **full setup → prove → verify** rather than stopping at
the constraint solver. Without the tag these tests would establish that the constraints are
satisfiable, not that a proof of them verifies; `build.sh` §4 sets it and the check is blocking.

| Test | what it establishes | the witness that does the work |
|---|---|---|
| `TestReluGadgetsAreCorrect` | 13 cases — positive, negative, zero, and the boundaries ±(2²¹−1) — all `ProverSucceeded` | — |
| `TestReluGadgetsRejectWrongWitness` / `negative_passed_through` | **the gadget is not the identity.** `(in −7, out −7)` must fail | this one alone separates ReLU from `y = x`: every positive case above passes under the identity |
| … / `negative_claimed_positive` | `(−7, 7)` fails — the sign is not merely dropped | |
| … / `positive_zeroed` | `(7, 0)` fails — the gadget is not the constant zero | kills the other degenerate function that passes all negative cases |
| … / `positive_off_by_one` | `(7, 8)` fails — the pass-through is exact | |
| … / `zero_claimed_nonzero` | `(0, 1)` fails — the `x = 0` double-witness does not license an arbitrary output | |
| `TestReluHintedSignRejectsOutOfRange` | `x = 2^B`, one past the declared range, fails | pins the precondition the soundness argument above rests on |

`TestMACAssertionFires` in [`assert_test.go`](../../tasks/gnark/assert_test.go) does the same
job for the guard in §5: it compiles with a deliberately drifted count and checks the exact
error text. A guard nobody has watched fail is a guard nobody knows is wired up.

### The cost, which is a function of B and not a constant

`B` is derived per ReLU site from the magnitude that site actually reaches (§5), so T2 uses
**three different widths in one circuit**. A "R1CS per activation" figure quoted without its
`B` is not a property of the gadget. Marginal cost `(C(896) − C(448))/448`, R1CS —
[`probes-gnark-relubits.txt`](../../data/probes-gnark-relubits.txt):

| B | `tobinary` | `hintedsign` |
|---:|---:|---:|
| 8 | 12.000 | **7.002** |
| 16 | 20.000 | **9.009** |
| **19** (T2 L0) | 23.000 | **11.011** |
| **20** (T3 L0) | 24.000 | **9.009** |
| 24 | 28.000 | **11.011** |
| **29** (T2/T3 L1) | 33.000 | **13.016** |
| **38** (T2/T3 L2) | 42.000 | **15.018** |
| 44 | **48.000** | **13.016** |
| 48 | 52.000 | **17.025** |

`tobinary` is exactly **B + 4** R1CS per activation across the whole range — one constraint per
bit of the `B+1`-bit decomposition, plus the `Select`. `hintedsign` is cheaper everywhere and
is **non-monotonic** in `B` (13.016 at B=44 against 17.025 at B=48, 9.009 at B=20 against
11.011 at B=19), because `std/rangecheck` chooses its chunk decomposition automatically and
the chunk count does not move monotonically with the requested width. Reported as measured; we
did not chase the chunking rule.

**In situ on T2**, at its own per-layer widths — [`probes-gnark-relu.txt`](../../data/probes-gnark-relu.txt):

| backend | `tobinary` | `hintedsign` | Δ | Δ / activation |
|---|---:|---:|---:|---:|
| Groth16 | 289 864 | **283 408** | 6 456 | **14.411** |
| PLONK | 763 120 | **758 448** | 4 672 | **10.429** |

## 5. The two assertions that can refuse to emit a circuit

### The MAC assertion

Fires inside `Define`, so a drifted expression surfaces at compile time and never reaches a
timing. The text is fixed across the campaign so a drift is greppable in any log:

```
{task}: emitted {n} MACs but bench/TASKS.md fixes {published}; the expression drifted from the published task
```
[`spec.go`](../../tasks/gnark/spec.go)

A second guard of the same shape counts activations, because `bench/TASKS.md` requires them
reported separately from MACs and never folded in:

```
{task}: emitted {n} activations but bench/TASKS.md fixes {published}; activations are reported separately from MACs and the count drifted
```
[`errors.go`](../../tasks/gnark/errors.go)

### The A1 accumulator bound

Amendment A1 requires an implementation to *"assert this bound and refuse to emit a circuit
without at least a factor-2 margin, rather than relying on the published seed happening to be
benign."* [`reference.go`](../../tasks/gnark/reference.go) does that before any circuit is
built and before any witness is serialized.

**The MLP's reference forward pass is computed in `big.Int` and only then narrowed to `int64`.**
That is not caution for its own sake: A1's own numeric bound says the worst case over all
admissible INT8 inputs *exceeds* `int64` at layer 4, so computing the reference in `int64` and
then asserting on the `int64` result would be asserting with the arithmetic under suspicion.
For T1 the static bound `K·128·128` is checked in `big.Int` **before** the `int64` accumulation
runs, which is what makes `int64` provably safe there.

Measured on this campaign's instances — [`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv),
columns `max_abs_intermediate` and `static_worst_case`:

Provenance by column: `max |intermediate|` and `static worst case` are read from
`max_abs_intermediate` and `static_worst_case` in
[`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv); **`headroom` is derived here**, as
`int64::MAX` divided by the measured maximum.

| Task | max &#124;intermediate&#124; *(CSV)* | headroom under `int64::MAX` *(derived here)* | static worst case over all INT8 *(CSV)* |
|---|---:|---:|---:|
| **T2** | **14 623 789 560 139** (1.46·10¹³) | **630 710×** | 14 411 518 807 585 587 200 (1.44·10¹⁹) |
| **T3** | **19 286 700 869 800** (1.93·10¹³) | **478 224×** | same |
| T1-0 | 265 403 | 3.48·10¹³× | 4 194 304 |
| T1-a…T1-d | 532 359 – 614 632 | ≥1.50·10¹³× | 12 582 912 |

The static worst case exceeds `int64::MAX` by **1.562×** — exactly the overflow A1 records
(1.44·10¹⁹ against 9.22·10¹⁸). The instance does not approach it; the assertion is what
establishes that rather than the seed's good behaviour.
`TestA1RefusesWhenTheMarginIsGone` drives the refusal (accepts 2⁶¹, rejects 2⁶²) so the
campaign has watched it fire.

**Two overflow facts, kept separate.** (1) The `int64` *reference* arithmetic could overflow
and is asserted not to. (2) The *field* the circuit computes in cannot overflow at these
magnitudes at all: BN254's scalar modulus is ~2.19·10⁷⁶ against a largest intermediate of
~1.9·10¹³. (2) is not a reason to skip (1), and the runner prints both in one line
(`Reference.A1Report`).

## 6. Why `constraints/MAC` FALLS as the ladder rises — and why T3 is cheaper per MAC than T2

This is the natural-unit echo of `bench/README.md`'s finding that **`bytes/MAC` is not a
constant of a proof system**. In gnark's own unit it is visible from compilation alone, with no
prover involved — [`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv):

| Task | shape | MACs | Groth16 regime A constraints | **constraints / MAC** |
|---|---|---:|---:|---:|
| T1-0 | `[1×256]·[256×256]` | 65 536 | 197 763 | **3.0176** |
| T1-a | `[1×768]·[768×768]` | 589 824 | 1 774 726 | **3.0089** |
| T1-b | `[4×768]·[768×768]` | 2 359 296 | 3 555 722 | **1.5071** |
| T1-c | `[16×768]·[768×768]` | 9 437 184 | 10 679 708 | **1.1317** |
| T1-d | `[64×768]·[768×768]` | 37 748 736 | 39 175 652 | **1.0378** |

**The mechanism is the range checks, and it is arithmetic.** A rung's constraint count is
roughly `M·K·N` multiplications plus the range checks on `M·K + K·N` operands. From T1-a to
T1-d the weight matrix never changes: it is always 768×768 = 589 824 values, each checked. What
grows is `M`. So a fixed range-check bill of ~1.18M constraints is spread over 589 824 MACs at
T1-a and over 37 748 736 MACs at T1-d, and the ratio walks from 3.01 down toward the asymptote
of ~1.0 that a bare multiplication costs.

**T3 against T2 is the same effect, in the batch dimension.** T3 is *the same MLP, batch of 8,
in one proof*, so the weights are shared across the eight items: `secret = 92 224 + 8·200 =
93 824`, against T2's `92 224 + 200 = 92 424`. The 92 224 weight range checks are paid once and
amortized over eight forward passes:

| Task | MACs | Groth16 regime A constraints | **constraints / MAC** |
|---|---:|---:|---:|
| T2 | 92 224 | 283 408 | **3.0730** |
| **T3** | **737 792** | **973 058** | **1.3189** |

8× the MACs for 3.43× the constraints. **Whether that survives into `bytes/MAC` and `MAC/s` is
a question about the prover, not about the circuit, and it is answered in
[`RESULTS.md`](RESULTS.md), not here.** What §6 establishes is that the denominator-dependence
is present in the frontend before any prover runs — so a single `constraints/MAC` figure quoted
without its rung is a property of the *pair*, exactly as `bench/README.md` says of `bytes/MAC`.

Regime B shows the same shape more sharply (0.0157 at T1-0 down to 0.0065 at T1-d) because
there the only constraints left are the input range checks and one equality per output.

## 7. Deviations from `bench/TASKS.md`: none

Stated with that force, because it is a result. **There is no task in `bench/TASKS.md` that
gnark's frontend made us change in order to express it.**

- **No padding of the task.** gnark pads its FFT *domain* to a power of two (measured, 20/20
  agreement in [`probes-gnark-padding.txt`](../../data/probes-gnark-padding.txt)), but the
  **task is not reshaped**. `768` stays `768`. Unlike DeepProve (768 → 1024, 1.778× the MACs
  actually proved) and jolt-atlas (which pads every dimension and whose disable switch does not
  work), the MACs `bench/TASKS.md` publishes and the MACs gnark performs are the same number.
- **No requantization anywhere**, per Amendment A1. Accumulators carry full width from one
  layer to the next. Unlike DeepProve and jolt-atlas, this is not a configuration we had to
  fight; nothing in gnark inserts a rescale.
- **No minimum output width.** T2's final `64→1` layer is expressed directly. See
  [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1.
- **No batch restriction.** T3 is one proof over eight inputs, as specified.
- **MAC counts match exactly**, asserted at compile time for all 7 tasks × 2 regimes ×
  2 backends — 28 of 28 (`macs_emitted` = `macs` in every row of
  [`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv)).

## 8. Witness seeds — same seeds, different instance

Fixed per task, and they are **binius64's canonical seeds reused verbatim**
([`spec.go`](../../tasks/gnark/spec.go)):

| Task | seed | Task | seed |
|---|---|---|---|
| T1-0 | `0xE0060100` | T1-c | `0xE00601C0` |
| T1-a | `0xE00601A0` | T1-d | `0xE00601D0` |
| T1-b | `0xE00601B0` | T2 | `0xE0060200` |
| | | T3 | `0xE0060300` |

**The RNG is NOT the same one, and this is a task-level comparison only.** Ceno is the one
system in this bank that reproduces binius64's instance value for value
([`ceno/EXPRESSION.md`](../ceno/EXPRESSION.md) §7); gnark does not, and could not without
porting Rust's `StdRng` into Go. This generator uses an explicit SplitMix64 written out in
[`rng.go`](../../tasks/gnark/rng.go) rather than Go's `math/rand`, for one reason: `math/rand`'s
documented stability has already moved once, and a benchmark whose instances change when the
toolchain moves cannot be reproduced. The stream is pinned to that file.

The consequence, stated the way the other entries state theirs: **same shapes, same MAC counts,
same activation counts, DIFFERENT INSTANCE.** Comparison at the level of the task is valid;
comparison at the level of the witness is not, and no figure in
[`RESULTS.md`](RESULTS.md) may rest on the two systems having drawn the same numbers.

The measurable trace of that difference is the largest intermediate each instance reaches:
binius64 and Ceno both record **8 955 951 054 519** for T2; this generator reaches
**14 623 789 560 139**. Same order of magnitude, different instance — which is what a correct
implementation of "same seed, different stream" should look like, and is checked by
`TestReferenceIsDeterministic`.

## 9. Circuit shape, measured

Read off the compiled constraint systems, not derived —
[`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv). Groth16, regime A:

| Task | MACs | activations | constraints | internal vars | secret | public | FFT domain |
|---|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 0 | 197 763 | 197 763 | 65 792 | 257 | 262 144 |
| T2 | 92 224 | 448 | 283 408 | 285 967 | 92 424 | 2 | 524 288 |
| T1-a | 589 824 | 0 | 1 774 726 | 1 774 214 | 590 592 | 769 | 2 097 152 |
| T3 | 737 792 | 3 584 | 973 058 | 991 738 | 93 824 | 9 | 1 048 576 |
| T1-b | 2 359 296 | 0 | 3 555 722 | 3 552 906 | 592 896 | 3 073 | 4 194 304 |
| T1-c | 9 437 184 | 0 | 10 679 708 | 10 667 676 | 602 112 | 12 289 | 16 777 216 |
| T1-d | 37 748 736 | 0 | 39 175 652 | 39 126 756 | 638 976 | 49 153 | 67 108 864 |

The other three quadrants (Groth16/B, PLONK/A, PLONK/B) are in the same file. `3 584 = 8 × 448`
is derived from `bench/TASKS.md`'s frozen T2 figure, which is the only activation count the
spec states, and is labelled as a derivation in [`spec.go`](../../tasks/gnark/spec.go).

## 10. Hint registration and the solver

One hint function is registered, `signHint`, used only by the ReLU gadget. It is passed to the
solver explicitly on every prove (`solver.WithHints(ReluHints()...)`,
[`build.go`](../../tasks/gnark/build.go)) rather than relied upon through the global registry,
so a proof cannot silently depend on init-order.

`solver.WithNbTasks` caps the number of solver goroutines and is exposed as its own campaign
axis. It is a **parallelism** knob that happens to move memory, and this entry does not call it
a memory knob; see [`RESULTS.md`](RESULTS.md) and
[`run-memory-knob.sh`](../../scripts/gnark/run-memory-knob.sh).

`backend.WithStatisticalZeroKnowledge` is **not** set. gnark's own docstring says it *"makes
the prover more memory costly"*, but that is not why it is off: it is off because leaving it
off is the default path, and the consequence is that **gnark's default Groth16 path here is not
statistical zero-knowledge.** The ZK column of the conditions line says so rather than
inheriting a "yes" from the fact that Groth16 is a zk-SNARK on paper. Every META line carries
`statistical_zk=false`.
