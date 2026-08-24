# DeepProve — tasks this system could not run, and why

`bench/README.md` commits to reporting a task a system cannot express as a **result**, not as
a gap. This file is that report for DeepProve. It is long because most of the grid is in it.

The distinction binius64's file drew is kept: **not expressible** means the frontend refuses
to build the model; **expressible but not runnable** means the model is built and the prover
then refuses it. Both are reported, and every claim below is a measured error message from a
cell in `bench/data/cells-deepprove/`, not a reading of the source.

| Task | Expressible? | Ran? | Where it stopped |
|---|---|---|---|
| **T1-0** | yes | **yes** | — |
| **T1-a** | yes | **yes** | proved, but padded 768→1024 (`EXPRESSION.md` §4) |
| **T1-b** | **no** | no | ONNX parser — §1 |
| **T1-c** | **no** | no | ONNX parser — §1 |
| **T1-d** | **no** | no | ONNX parser — §1 |
| **T2** | yes | **no** | sumcheck, at proving time — §2 |
| **T3**, one proof | **no** | no | ONNX parser pins `batch_size = 1` — §3 |
| **T3**, as 8 proofs | yes | **no** | same wall as T2 — §2, §3 |

Two further limits cut across all of them: DeepProve pads every dimension to a power of two
and cannot be told not to (`EXPRESSION.md` §4), and it requantizes after every linear layer
and cannot be told not to (`EXPRESSION.md` §5), which is a deviation from
`bench/TASKS.md` Amendment A1 that no configuration can remove.

---

## 1. T1-b, T1-c, T1-d — any matmul with more than one row

**Not expressible.** DeepProve's ONNX dense parser flattens the input tensor and then
requires the result to be a vector:

```rust
if input_shape.len() != 1 {
    ensure!(input_shape[0] == 1,
        "First dimension of Gemm layer input should be 1. Input shape was: {input_shape:?}");
    input_shape.remove(0);
}
ensure_onnx!(input_shape.len() == 1, "Input shape for Gemm must be a vector, found {:?}", input_shape);
```
`zkml/src/parser/onnx.rs:563-574`

So `[1×K] · [K×N]` is fine and `[M×K] · [K×N]` with `M > 1` is not. The dense layer's einsum
is `A(j)@W(ij)->O(i)` (`zkml/src/layers/einsum/constructor.rs:15-20`) — a matrix-vector
product, with no row dimension to put `M` in.

**The messages, from the cells themselves.** Each was run rather than assumed to fail:

| Task | shape | DeepProve's error |
|---|---|---|
| T1-b | `[4×768]·[768×768]` | `Incompatible shapes found for Gemm node: input shape is [3072], weight shape is [768×768]` |
| T1-c | `[16×768]·[768×768]` | `Incompatible shapes found for Gemm node: input shape is [12288], weight shape is [768×768]` |
| T1-d | `[64×768]·[768×768]` | `Incompatible shapes found for Gemm node: input shape is [49152], weight shape is [768×768]` |

`3072 = 4·768`, `12288 = 16·768`, `49152 = 64·768`: the row dimension has been folded into
the vector length, and the shape check then fails against the weight matrix. All three
rejections happen in **0.01–0.02 s**, before any proving.

Raw output: `bench/data/cells-deepprove/t1-{b,c,d}-q8-t1-n1/log.txt`.

**What this does and does not mean.** It is a limit of the **ONNX frontend**, not of the
protocol. DeepProve's own LLM path has a batched einsum for multi-head attention
(`A(ijm)@B(imk)`, `zkml/src/layers/einsum/mod.rs:114-120`) and evidently multiplies matrices
by matrices there. It is simply not reachable from `FloatOnnxLoader`. A reader should
conclude "DeepProve's ONNX frontend does matrix-vector products", not "DeepProve cannot do
matrix-matrix products".

**Consequence for the benchmark, stated plainly.** The T1 ladder exists to find where each
system breaks across three orders of magnitude. For DeepProve it stops after two rungs:
65 536 and 589 824 MACs. **Everything above 589 824 MACs is unmeasured for this system**, and
`bench/README.md`'s rule against extrapolating outside the measured range applies with full
force — nothing in `RESULTS.md` projects DeepProve's behaviour past T1-a.

## 2. T2 and T3 — expressible, but the prover rejects a narrow output layer

**T2 builds and then fails at proving time.** DeepProve parsed the graph, quantized it into
13 layers, generated the proving context in 1.23 s, started the first proof, and panicked:

```
thread '<unnamed>' panicked at dp-crypto/src/sumcheck/util.rs:31:5:
ceil_log2: x must be positive

Error: generating proof
```

Raw output: `bench/data/cells-deepprove/t2-q8-t1-n6/log.txt`. It reproduces at
`ZKML_BIT_LEN=8` and `12`, and at 1 and 10 threads — four cells, same failure.

### 2.1 The cause, isolated to one variable

"It crashed" is a weaker result than "it crashed because of X", and the authors deserve the
second one for their right of reply. Four diagnostic probes vary **only the width of the
final layer** and nothing else. **These are not T2 and produce no benchmark number**;
`bench/TASKS.md` is frozen. Generator:
[`bench/scripts/deepprove/make-probe.py`](../../scripts/deepprove/make-probe.py).

| Probe | widths | outcome |
|---|---|---|
| `probe-d1` | `64 → 1` | **`ceil_log2: x must be positive`** — same panic as T2 |
| `probe-d2` | `64 → 2` | **`Proving failed: No round evaluations found`**, in `N3: Requant` |
| `probe-w2` | `200-256-128-64 → 2` | **`Proving failed: No round evaluations found`**, in `N12: Requant` |
| `probe-w4` | `200-256-128-64 → 4` | **proves successfully** |

**The result: at this commit, a DeepProve dense layer whose output is narrower than 4
elements cannot be proved through the ONNX frontend.** Width 1 panics one way, width 2 fails
another way, width 4 works. A single dense layer `64→1` reproduces it with no MLP around it,
so it is the output width and nothing else about T2.

Both failures land in the **requantization** layer that DeepProve inserts automatically after
every linear layer (`EXPRESSION.md` §5) — `probe-d2` names it: `proving N3: Requant`. A
sumcheck needs at least one round, and a lookup over a 1- or 2-element tensor does not give
it one.

**T2's final layer is 64 → 1.** `bench/TASKS.md` fixes it: a tabular decision network ending
in a single scalar prediction — the shape credit scoring and fraud detection actually deploy.
So the task is not exotic; it is the common case for the model class T2 represents.

**T3 is the same network**, so it fails identically, at 1 and 10 threads
(`t3-as-8-q8-t{1,10}-n8`).

**Right of reply applies here more than anywhere else in this file.** If the DeepProve
authors consider a width-1 output layer supported through a route we did not find, or fixed
after `9d1a53e2`, we will re-run T2 and T3 and publish both outcomes, per `CHALLENGE.md`.

## 3. T3 — a batch of 8 cannot be one proof

`bench/TASKS.md` asks whether a system can prove 8 independent inputs in **one** proof, and
says explicitly that producing 8 separate proofs is a result about the system rather than a
failure of the task.

**DeepProve produces 8 separate proofs. It cannot produce one.** The ONNX parser pins the
batch symbol to 1 before anything else happens:

```rust
// so far we dont support batching
let mut values = SymbolValues::default();
let symbol = pmodel.sym("batch_size");
values.set(&symbol, 1);
```
`zkml/src/parser/onnx.rs:153-157`

and a graph with a static batch of 8 is rejected, measured rather than assumed:

```
Failed to parse node: MatMul_1: ... outputs: [8,256,F32] :
Incompatible shapes found for Gemm node: input shape is [1600], weight shape is [256×200]
```

`1600 = 8·200`. Raw output: `bench/data/cells-deepprove/t3-batch8-q8-t1-n1/log.txt`.

The worker loops one input at a time, resetting model state between them
(`deep-prove/src/bin/worker/main.rs:146-180`), so 8 inputs are 8 independent proofs with 8
independent proof artifacts.

**So T3's answer for DeepProve is: batching buys nothing, because it does not exist.** That
is the result, and it would have been the result even if T2 had proved — §2 only stops us
from putting a time next to it.

## 4. Requantization cannot be disabled — Amendment A1 cannot be honoured

Covered in full in [`EXPRESSION.md`](EXPRESSION.md) §5, and listed here because it is a task
requirement that no configuration of this system can meet: `bench/TASKS.md` Amendment A1
fixes T2 as **no requantization**, and DeepProve requantizes after every linear layer with no
external switch. On T1 the same applies against T1's "INT32, not requantized" rule.

This is not a bug and not a mis-expression — requantization is how DeepProve keeps
accumulators inside its field, and its LogUp-GKR lookups are a large part of why it is fast.
But it means DeepProve is charged for **strictly more work** than the task specifies, and
every figure carries that in its conditions line.

## 5. The GPT-2 reference could not be run either

Not a benchmark task, but it belongs in the same list: DeepProve's own flagship benchmark,
GPT-2 at sequence 64, does not run on this machine at this commit — in either build. That is
`REPRODUCTION.md` §2, and it is reported there because the fairness protocol requires it to
appear **above** any result rather than in a list of gaps.

## 6. What could not be measured because of the licence, and what that costs

`COMMIT` explains the constraint; this is its bill, so a reader sees what is missing rather
than discovering it.

- **No internal decomposition.** For binius64, `RESULTS.md` §4 splits verification into four
  timed terms and identifies which one is linear. Nothing equivalent exists here: DeepProve's
  licence forbids derivative works, so its internals were not instrumented. Where its cost
  goes inside the prover is **NOT DETERMINED** by this benchmark.
- **No prove/inference split.** Our `prove` bracket includes DeepProve's quantized inference,
  because the ONNX worker emits no marker between them. DeepProve's *own* LLM benchmark
  separates `inference_time` from `prove_full`; that split is unavailable on this path and the
  combined figure is what is published, labelled as such.
- **No proof-only size.** The artifact the public CLI writes is
  `Output { outputs, proof: Provable { proof, io, ctx } }` — it carries the verifier context
  as well as the proof. Separating them would mean reverse engineering the serialization,
  which the licence forbids. `RESULTS.md` publishes the artifact size and says what is in it,
  and a proof-only figure comparable to binius64's is **not available**.
- **No witness-level corruption.** binius64's correctness control mutates a private witness
  word inside the prover — the stronger test. Here the control acts on the serialized
  artifact from outside. What that did and did not catch is in `RESULTS.md`.

## 7. What DeepProve was NOT asked to do, and so is not reported as unable to do

For completeness, so an absence is not read as a failure:

- **GPU.** Round one is CPU-only by design (`bench/README.md`). DeepProve ships `cuda`
  (Linux/NVIDIA) and `wgpu` features; neither was built or measured.
- **ZK.** Not part of round one, which compares integrity-proving cost.
- **Distributed proving.** DeepProve supports chunked/distributed proving
  (`--distributed`, `--num_chunks`). No task asks for it and it was not measured.
- **LLM inference as a task.** T1/T2/T3 are not LLM tasks. GPT-2 appears here only as
  DeepProve's own published reference number, in `REPRODUCTION.md`.
