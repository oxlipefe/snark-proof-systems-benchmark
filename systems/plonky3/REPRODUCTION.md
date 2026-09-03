# Plonky3 — the fairness protocol's primary check, and why it is largely undischarged

`bench/TASKS.md` requires, per system, *"the system's own published reference number, and
whether we reproduced it (and if not, by how much)"*. For this system the honest answer has
three parts.

---

## 1. There is no published reference number for what was measured

Plonky3 is a **toolkit**, not a prover. It publishes no benchmark for "an INT8 matmul", because
it implements no INT8 matmul: `bench/scripts/plonky3/harness/src/matmul.rs` is our expression
of the task on their sumcheck engine (`EXPRESSION.md` §1). There is nothing upstream to
reproduce, and inventing a comparison against one of their component benchmarks would be
`bench/RESULTS.md` §0's rule 6 violated in the other direction.

What the tree does carry, at the pinned commit:

| upstream artifact | what it measures | comparable to a cell here? |
|---|---|---|
| `whir/examples/whir.rs` | commit + open + verify of **one random single-column table** at a chosen variable count | **No.** No task, no operands, one table, sampled rather than prescribed opening points |
| `whir/benches/{whir_pcs,fri_vs_whir,sumcheck,zk_overhead}.rs` | Criterion micro-benchmarks of the PCS and its parts | **No.** Component costs, not a proof of a statement |
| `sumcheck/benches/sumcheck.rs` | Criterion micro-benchmark of the round kernels | **No** |
| `binary-field/benches/arithmetic.rs` | field arithmetic throughput | **Partly — see §2** |

**No figure from any of them is carried into this directory**, and no cell here is described as
a reproduction of one.

## 2. The continuity check that WAS discharged, and it is the useful one

The check this benchmark actually needs is not against Plonky3's numbers; it is against **our
own earlier measurements on the same machine**, because that is what makes a Plonky3 cell and a
binius64 cell admissible in the same document.

`bench/scripts/plonky3/harness/src/sanity/handmul.rs` is **byte-identical** to the 6-PMULL
kernel that produced E-001's and E-006's reference figures, and `src/stats.rs` is byte-identical
to their order-statistics module. The probe was rerun on 2026-09-03:

| row | E-006 (2026-08-23) | this campaign (2026-09-03) | ratio |
|---|---:|---:|---:|
| raw PMULL, lower bound | 3 130.2–3 226.7 Mops/s | 3 220.0–3 220.8 Mops/s | 1.00–1.03 |
| hand-written 6-PMULL | 1 001.8–1 014.6 Mmul/s | 1 012.2–1 015.5 Mmul/s | 1.00–1.01 |

**Reproduced to within 1 % on both rows, eleven days apart, on a machine that is not
dedicated.** That is the strongest statement available here, and it is about the instrument
rather than about Plonky3.

It also carries E-006's unexplained anomaly forward unchanged: the hand-written kernel still
reads ≈ 0.81× of its E-001 level while the raw PMULL loop is unchanged. Cause still not
established; no number in this directory depends on it, because every criterion is a ratio
taken inside one process.

## 3. What a third party can and cannot reproduce from this directory

**Can, exactly, from a clean clone:** the instance (`BUILD.md` §1 — the drawn operands are
checked against binius64's published `max |accumulator|`), the circuit-free structural
quantities of every cell (`sumcheck rounds`, `reduction field multiplies`, `padded MACs`,
padding factor), the proof sizes, the eleven correctness verdicts, and the compiler's refusal
to instantiate WHIR over `BinaryField128`.

**Cannot:** the timings, as declared everywhere in this benchmark — the machine is shared and
the rows in `RESULTS.md` are single repetitions labelled `SMOKE`. The build-integrity ratios
(0.057 and 5.5) are machine-dependent in their absolute rates and reproducible in their
verdicts.

**Would have to be rebuilt, not reproduced:** anything about T2, T3, the 10-thread cut, or the
upper T1 rungs. None of it was run. See `NOT_EXPRESSIBLE.md` §2 and §3 and `RESULTS.md` §4.
