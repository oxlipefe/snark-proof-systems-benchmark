# Task specifications

Every task is defined by an **exact operation count**, not by a shape that each system may
interpret differently. The MAC count is the denominator of `bytes/MAC` and of `MAC/s`, so
it is fixed here and never recomputed per system.

A **MAC** is one multiply-accumulate: one multiplication and one addition into an
accumulator. For `A[M×K] · B[K×N]` the count is exactly `M·K·N`.

---

## T1 — INT8 matrix multiply (ladder)

The shape of a transformer linear layer. `K = N = 768` is GPT-2's hidden size, so the rungs
are realistic rather than synthetic. `M` is the number of rows processed.

| Rung | Shape | **MACs** |
|---|---|---|
| T1-0 | `[1×256] · [256×256]` | **65,536** |
| T1-a | `[1×768] · [768×768]` | **589,824** |
| T1-b | `[4×768] · [768×768]` | **2,359,296** |
| T1-c | `[16×768] · [768×768]` | **9,437,184** |
| T1-d | `[64×768] · [768×768]` | **37,748,736** |

**Inputs:** INT8, values in `[-128, 127]`, fixed pseudo-random seed published per task.
**Accumulator:** INT32. **Output:** INT32, not requantized — requantization is a separate
concern and folding it in would make systems incomparable.

Systems that cannot express INT8 natively declare their encoding (e.g. field elements
constrained to an 8-bit range) in their `systems/<name>/EXPRESSION.md`.

## T2 — A complete MLP, end to end

A tabular decision network of the size actually deployed for credit scoring, fraud
detection and similar. **This is a whole model, not a tile** — it includes every layer,
every activation, and the fixed per-proof overhead.

| Layer | Shape | MACs |
|---|---|---|
| 1 | 200 → 256 | 51,200 |
| 2 | 256 → 128 | 32,768 |
| 3 | 128 → 64 | 8,192 |
| 4 | 64 → 1 | 64 |
| | **Total** | **92,224** |

**Activations:** ReLU after layers 1–3 — **448 elements**, reported separately and never
folded into the MAC count. **Output layer:** linear, no activation.
**Weights:** INT8, fixed published seed. **Input:** INT8 vector of length 200.
**Requantization: none.** Accumulators stay at full width between layers, extending T1's
explicit rule. See [Amendment A1](#a1--t2-requantization-2026-08-23) for the reasoning and
for the limitation this creates.

## T3 — The same MLP, batch of 8

Identical network, 8 independent inputs proven together in **one** proof.

**Total: 737,792 MACs** (8 × 92,224).

Isolates whether batching independent requests into a single proof is sublinear in the
number of requests. A system that produces 8 separate proofs reports it that way, and that
is a result about the system, not a failure of the task.

---

## Reference points

Our own prover's measured behaviour, for orientation — **these are our numbers, not a
standard**:

- Flat rate up to **524,288 MACs**; degradation begins above it on a 32 GiB machine.
- So **T1-0 and T2 fall inside** that regime, and **T1-a through T1-d and T3 fall outside**
  it, deliberately. The ladder is built to find where each system breaks, not to flatter any
  of them.

## What is published per system

For every system, `systems/<name>/`:

- `COMMIT` — pinned revision, with date
- `BUILD.md` — build configuration and the integrity check that was run before measuring
- `EXPRESSION.md` — how each task was expressed, with the code
- `REPRODUCTION.md` — the system's own published reference number, and whether we
  reproduced it (and if not, by how much)
- `NOT_EXPRESSIBLE.md` — any task this system cannot express, and why


---

# Amendments

This specification is frozen before measurement so that it can be attacked independently of
the results. It is not immutable — but every change is logged here with its date, its
reason, and its effect on systems already measured. **Silent edits are the failure mode this
log exists to prevent.**

## A1 · T2 requantization (2026-08-23)

**Gap.** The original T2 specification did not state whether accumulators are requantized
between layers. Real deployed INT8 networks normally do requantize; the spec was silent, and
silence is not a specification.

**Resolution: no requantization.** Accumulators carry full width from one layer to the next,
extending the rule T1 already stated explicitly ("INT32, not requantized").

**Why.** Requantization is a separate cost with a separate design space — scale selection,
rounding mode, clipping — and every system implements it differently. Folding it into T2
would mean the systems are no longer proving the same arithmetic, which defeats the point of
a same-task benchmark. It is a legitimate future task (a `T2-b`), not a hidden variable
inside this one.

**Limitation this creates, stated plainly.** T2 is therefore *not* a faithful reproduction of
a deployed INT8 pipeline. It is a fixed, fully specified arithmetic workload chosen so that
different proof systems can be compared on identical work. Anyone using these numbers to
predict the cost of a production model must add requantization themselves.

**Numerical bound, declared.** Without requantization, a worst case taken over all admissible
INT8 inputs and weights would overflow a 64-bit accumulator at layer 4 (1.44·10¹⁹ against a
9.22·10¹⁸ bound). **The published instance does not approach it:** its maximum absolute
intermediate is 8.96·10¹², a margin of roughly 10⁶×. Implementations must assert this bound
and refuse to emit a circuit without at least a factor-2 margin, rather than relying on the
published seed happening to be benign.

**Effect on systems already measured.** None. `binius64` was measured under exactly this
rule, its builder already asserts the bound, and its measured maximum intermediate
(8.96·10¹²) is recorded in its cell metadata. No figure changes.

## A2 · Weight regime — the spec never fixed it (2026-08-24)

**Gap, and it is the serious one.** T2 said only *"Weights: INT8, fixed published seed."* It
never said **what the weights are to the proof system**: private witness, circuit constants,
or preprocessed-and-committed. Five systems resolved that silence in three different ways —
and **the cost of the weights therefore lands in three different columns**:

| System | Weight regime chosen | Where the weight cost lands |
|---|---|---|
| binius64 | witness (declared) | prove time and prover memory |
| gnark, regime A | witness | prove time and prover memory |
| gnark, regime B | circuit constants | compile; bound into the verifying key |
| DeepProve | preprocessed | **`setup`** |
| jolt-atlas | preprocessed | **`setup`** |
| Ceno | program data | the cycle count |

**Consequence: `bytes/MAC` and `MAC/s` do not cover the same envelope across systems.** This
benchmark reports setup separately and never folds it into derived metrics — a rule that is
correct in isolation and **silently excludes the weight cost for two of the five systems**.
Measured inside one system, the choice is worth **192.8×** in constraints.

**Resolution: declare, do not standardize.** Forcing one regime would exclude systems that
cannot express the others, and would privilege whichever regime our own system happens to use.
Instead, from this amendment on:

1. **Every system declares its weight regime** from this vocabulary: `witness` ·
   `circuit-constant` · `preprocessed` · `program-data`. A system that supports more than one
   **is measured in each**, reported as separate regimes, never averaged.
2. **Every system declares where the weight cost lands** — prove, setup, compile, or cycles.
3. **Derived metrics are comparable only within a weight regime.** A `bytes/MAC` from a
   `witness` system and one from a `preprocessed` system are **not** the same quantity, and any
   table placing them in one column must say so in the column header, not in a footnote.
4. **A new column is required: what the proof binds.** Regimes differ in a property that has
   nothing to do with cost. A system that commits the witness rejects any change to it; a
   system proving existential satisfiability accepts a change that does not alter the output.
   **Both are correct and they are different theorems.** See A3.

**Open question, not a property.** In `circuit-constant` regime the verifier needs only the
verifying key. That the weights are absent from the verifier's inputs **does not establish that
the verifying key reveals nothing about them.** Nobody in this benchmark has established that,
and it must not be stated as a privacy property until someone does.

**Effect on systems already measured.** No figure changes. Every system's weight regime was
recorded in its `EXPRESSION.md` and is now surfaced as a first-class column. Ceno never named
its position; that is now required.

## A3 · A corruption counts as a test only if it changes the output (2026-08-24)

**Gap.** The correctness control corrupts a witness value and asks whether `verify()` fails.
That silently assumes corrupting the witness corrupts the **statement** — true for a matmul,
**false for any network with activations**. Measured exhaustively: **52.27 % of T2's weights and
3.27 % of T3's are inert** — they feed neurons whose pre-activation is negative, so ReLU
discards them and the output is bit-identical. A perturbed witness is then still a valid witness
for the same true statement, and **accepting it is correct behaviour**.

**Consequence:** in any task with activations, a `witness_word` "pass" does not distinguish *the
system caught it* from *the position did not matter*. The control was measuring luck.

**Rule, from this amendment on.** Before a witness corruption is counted as a test, the
reference forward pass **must be recomputed** and the output shown to change. Positions that
leave the output unchanged are reported as `WITNESS_INERT` and are **not** counted as passes or
failures. A verdict of accepted-and-the-output-changed is a genuine alert and must be reported
as such.

**Artifact corruption is unaffected** and remains the strong control: corrupting proof bytes or
public inputs always corrupts the statement.

**Effect on systems already measured.** No published verdict is withdrawn: no system accepted a
corruption that changed the output. But `witness_word` results on T2 and T3 are re-labelled as
weak evidence, and the pre-fix runs are published alongside the corrected ones — they are the
evidence for this amendment.
