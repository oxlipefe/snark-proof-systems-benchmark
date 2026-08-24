# jolt-atlas — tasks this system could not run, and why

`bench/README.md` commits to reporting a task a system cannot express as a **result**, not as
a gap. This file is that report for jolt-atlas.

The distinction binius64's and DeepProve's files drew is kept: **not expressible** means the
frontend refuses to build the model; **expressible but not runnable** means the model builds
and the prover then refuses it. Every claim below is a measured error message from a run whose
raw output is in [`bench/data/`](../../data/), not a reading of the source.

| Task | Expressible? | Ran? | Where it stopped |
|---|---|---|---|
| **T1-0** | yes | **yes** | — |
| **T1-a** | yes | **yes** | proved, padded 768→1024 (`EXPRESSION.md` §4) |
| **T1-b** | yes | **yes** | proved, padded — **DeepProve rejects this rung** |
| **T1-c** | yes | **yes** | proved, padded |
| **T1-d** | yes | **yes** | proved, padded |
| **T2** | yes | **no** | einsum registry, at proving time — §1 |
| **T3**, one proof | yes | **no** | same wall as T2 — §1 |
| **T3**, as 8 proofs | yes | **no** | same wall as T2 — §1 |

**jolt-atlas is the first system in this benchmark to run all five rungs of T1**, across three
orders of magnitude, and that is the headline of this file rather than the failures below.
binius64 did not complete the ladder; DeepProve stopped after two rungs.

Two limits cut across everything: jolt-atlas pads every dimension to a power of two and the
switch that appears to disable it does not work (`EXPRESSION.md` §4), and it rebases after
every einsum, which is a deviation from `bench/TASKS.md` Amendment A1 that no configuration
removes (`EXPRESSION.md` §3).

---

## 1. T2 and T3 — a dense layer with a single output

**Both build and both are refused by the einsum layer at proving time**, in 0.06 s and 0.37 s
respectively — before any proving work:

```
thread 'main' panicked at jolt-atlas-core/src/utils/dims.rs:262:13:
Einsum equation (k,k->mn) not supported by Einsum proof system      # T2,  batch 1
Einsum equation (mk,k->mn) not supported by Einsum proof system     # T3,  batch 8
```

Raw: [`cells-jolt-atlas/t2-t1-p1-n5/log.txt`](../../data/cells-jolt-atlas/),
[`t3-t1-p1-n5/log.txt`](../../data/cells-jolt-atlas/).

The panic is `lookup_einsum_config`, which scans a static registry of supported contraction
patterns and panics on a miss:

```rust
pub fn lookup_einsum_config(equation: &str) -> &'static EinsumConfig {
    EINSUM_REGISTRY.iter().find(|(pattern, _)| *pattern == equation)
        .map(|(_, config)| config)
        .unwrap_or_else(|| panic!("Einsum equation ({equation}) not supported by Einsum proof system"))
}
```
`jolt-atlas-core/src/utils/dims.rs:254-264`

### 1.1 The cause, isolated to one variable

"It crashed" is a weaker result than "it crashed because of X", and the authors deserve the
second one for their right of reply. A width ladder varies **only the output width of the final
dense layer** and holds the architecture, the emission, the opset and the seed fixed. Generator:
[`bench/scripts/jolt-atlas/make-probe.py`](../../scripts/jolt-atlas/make-probe.py). **These are
not T2 and produce no benchmark number**; `bench/TASKS.md` is frozen.

| Probe | widths | batch | outcome |
|---|---|---:|---|
| `tw1` | 200-256-128-64 → **1** | 1 | **`Einsum equation (k,k->mn) not supported`** — same panic as T2 |
| `tw2` | 200-256-128-64 → **2** | 1 | **proves and verifies** |
| `tw3` | 200-256-128-64 → **3** | 1 | **proves and verifies** |
| `tw4` | 200-256-128-64 → **4** | 1 | **proves and verifies** |
| `probe-d1` | 64 → **1** (one layer, no MLP around it) | 1 | **`(mk,k->mn) not supported`** |
| `probe-d2` | 64 → **2** | 1 | **proves and verifies** |
| `probe-d4` | 64 → **4** | 1 | **proves and verifies** |

**The result: at this commit, a dense layer whose output is a single element cannot be proved
through jolt-atlas's ONNX frontend. Width 2 works. A single `64→1` layer with no network around
it reproduces it**, so it is the output width and nothing else about T2.

The mechanism is visible in the equations themselves. A weight of shape `[64, 1]` has its
trailing unit dimension folded away by `tract`'s shape inference, so the contraction arrives as
`mk,k->mn` — a rank-1 right operand — and `EINSUM_REGISTRY`
(`jolt-atlas-core/src/utils/dims.rs:54+`) carries `mk,kn->mn`, `amk,kn->amn` and their
relatives, but nothing with a rank-1 operand on the right. At batch 1 the left operand collapses
too and the equation becomes `k,k->mn`.

**T2's final layer is 64 → 1.** `bench/TASKS.md` fixes it: a tabular decision network ending in
a single scalar prediction — the shape credit scoring and fraud detection actually deploy. So the
task is not exotic; it is the common case for the model class T2 represents.

### 1.2 The same wall DeepProve hit, at a different threshold

Worth stating because it is a fact about the class of system rather than about either one:

| | threshold | mechanism |
|---|---|---|
| DeepProve | output width **< 4** | sumcheck has no rounds over a 1- or 2-element tensor; the failure lands in its inserted `Requant` layer (`systems/deepprove/NOT_EXPRESSIBLE.md` §2) |
| jolt-atlas | output width **< 2** | `tract` folds the unit dimension; the resulting contraction is not in the einsum registry |

**Two independently developed ONNX zkML frontends, two different protocols, and both refuse a
dense layer with one output.** Neither is unable to *compute* it; both are unable to *express*
it through the ONNX path. That is a result about how this class of system is built, and it is
the single most transferable thing this campaign found.

### 1.3 A second, separate wall for batched multi-layer MLPs — NOT DETERMINED

Reported raw because it is real and because we could not settle it. With **batch 8** and the
full four-layer stack at an output width that works at batch 1, the frontend accepts the graph
and **the prover panics**:

```
thread '<unnamed>' panicked at joltworks/src/poly/multilinear_polynomial.rs:244:62:
index out of bounds: the len is 64 but the index is 128
```

It reproduces at widths 2 and 4 (`tw2b8`, `tw4b8`). **But an otherwise identical graph emitted
with `transB=0` and a static batch of 8 proves** (`probe-b8`: 430 ms, 108 776 B, verified). So
the panic is sensitive to the ONNX emission and **we do not claim it is a limit of jolt-atlas**.
Given `EXPRESSION.md` §5 — where three of our own emissions produced errors that looked like
their limits — the honest label is **NOT DETERMINED**, and it is here so a third party sees it
rather than discovers it. Raw:
[`bench/data/probes-jolt-atlas/`](../../data/probes-jolt-atlas/).

**It does not change T3's verdict.** T3's output width is 1, so T3 fails at §1's wall before
this one is reachable.

## 2. T3 — batching, and what we can and cannot say about it

`bench/TASKS.md` asks whether a system can prove 8 independent inputs in **one** proof, and says
that producing 8 separate proofs is a result about the system rather than a failure of the task.

**jolt-atlas can express a batch of 8 in one graph** — unlike DeepProve, whose ONNX parser pins
`batch_size = 1` outright (`systems/deepprove/NOT_EXPRESSIBLE.md` §3). Its `RunArgs` carries a
`batch_size` variable, its einsum registry has batched patterns (`amk,kn->amn`), and a batch-8
four-layer MLP at an expressible output width does prove (`probe-b8`, §1.3).

**But T3 itself does not prove**, because of the width-1 output layer, and the
8-separate-proofs fallback does not either, for the same reason: each of the 8 proofs is T2.

**So T3's answer for jolt-atlas is: the capability exists and the task cannot reach it.** No
number is reported, and in particular **no claim is made about whether jolt-atlas's batching is
sublinear**, which is the question T3 exists to ask. That question is unanswered for this system
and `bench/README.md`'s rule against extrapolation forbids inferring it from `probe-b8`, which is
a different network from T2.

## 3. The unpadded configuration — a grid row that ran and failed

`with_padding(false)` is a documented public setter and it does not survive contact with the
prover for any non-power-of-two dimension: `Dense multi-linear polynomials must be made from a
power of 2 (not 768)`. Four of the five unpadded rungs are `FAIL` rows in
[`cells-jolt-atlas.csv`](../../data/cells-jolt-atlas.csv), reported rather than dropped.
[`EXPRESSION.md`](EXPRESSION.md) §4 has the table and the T1-0 control.

## 4. GPT-2 — their own reference model could not be loaded

Not a benchmark task, but it belongs in the same list: jolt-atlas's own flagship benchmark model
cannot be loaded on this machine today, because the export script it ships pins no exporter
version and today's exporter emits a symbolic dimension its pinned `tract` cannot parse. That is
[`REPRODUCTION.md`](REPRODUCTION.md) §4, and it is reported there because the fairness protocol
requires it to appear **above** any result rather than in a list of gaps.

## 5. What could not be measured, and what that costs

- **No internal decomposition of our own.** For binius64, `RESULTS.md` §4 splits verification
  into four timed terms. Nothing equivalent was done here: jolt-atlas's licence forbids
  derivative works, so its internals were not instrumented. **What partly substitutes is that
  jolt-atlas instruments itself** — its `tracing` spans decompose `prove` into
  `commit_witness_polynomials`, `iop` and `prove_reduced_openings`, and `REPRODUCTION.md` §3.3
  uses exactly those. That decomposition is theirs, read from their output.
- **`prove` includes graph tracing.** `ONNXProof::prove` executes the quantized graph before
  proving and there is no public entry point that separates them (`EXPRESSION.md` §6). The
  figures are an **upper bound** on proving time.
- **Dory was not measured.** A second PCS ships in the tree and no public path on the ONNX
  prover selects it (`BUILD.md` §4).
- **Security bits are NOT DETERMINED** (`BUILD.md` §4). No comparison in `RESULTS.md` claims
  equal soundness.
- **No witness-level corruption.** binius64's control mutates a private witness word inside the
  prover — the stronger test. Here the control acts on the public IO and on the serialized
  proof from outside. What that did and did not catch is in `RESULTS.md`.

## 6. What jolt-atlas was NOT asked to do, and so is not reported as unable to do

- **GPU.** Round one is CPU-only by design (`bench/README.md`).
- **ZK.** jolt-atlas ships a `zk` feature (BlindFold). It was **not** built or measured; round
  one compares integrity-proving cost.
- **LLM inference as a task.** T1/T2/T3 are not transformer graphs. nanoGPT and GPT-2 appear here
  only as jolt-atlas's own published reference numbers, in `REPRODUCTION.md`.
