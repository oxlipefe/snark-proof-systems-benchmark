# jolt-atlas — how each task was expressed

`bench/TASKS.md` fixes each task by an **exact MAC count**. That count is the denominator of
both `MAC/s` and `bytes/MAC`, so it is never recomputed here. The generator
[`bench/scripts/jolt-atlas/make-tasks.py`](../../scripts/jolt-atlas/make-tasks.py) asserts
that the graph it emitted performs exactly the published number of multiply-accumulates and
refuses to write a file that does not:

```python
published = PUBLISHED_MACS[task]
if macs != published:
    sys.exit(f"{task}: graph performs {macs} MACs but bench/TASKS.md fixes {published}")
```

Nothing in this directory is jolt-atlas code. The ONNX files are plain ONNX, emitted by our
generator; the harness that feeds them is ours.

---

## 1. Which frontend, and why a harness rather than a binary

jolt-atlas ships **no CLI**. Every documented entry point is a compiled example with its model
path and its input hardcoded (`jolt-atlas-core/examples/*.rs`), so there is no binary that can
be pointed at an arbitrary ONNX file.

What it does ship is a usable **public API**, and the tasks are expressed against it:

```
Model::load(path, &RunArgs)                     atlas-onnx-tracer
AtlasSharedPreprocessing::preprocess(model)     jolt-atlas-core
AtlasProverPreprocessing::<Fr, HyperKZG<Bn254>>::new(pp)
AtlasVerifierPreprocessing::from(&prover_pp)
ONNXProof::<Fr, Blake2bTranscript, HyperKZG<Bn254>>::prove(&prover_pp, &inputs)
proof.verify(&verifier_pp, &io, None)
proof.serialize_compressed(&mut buf)            ark_serialize::CanonicalSerialize
```

That sequence is not our invention: it is the sequence jolt-atlas's own
`examples/nanoGPT.rs` and `examples/gpt2_zk_bench.rs` execute, in that order. The harness
calls it and times it. **No jolt-atlas source is copied, patched or instrumented**
([`COMMIT`](COMMIT)).

**This is a genuinely tighter bracket than DeepProve got**, and the difference is in what
could be measured rather than in the systems:

| | DeepProve | jolt-atlas |
|---|---|---|
| `prove` | includes quantized inference; not separable | `ONNXProof::prove` only — **but see §6**, it still includes graph tracing |
| proof size | artifact = proof **+ io + verifier context** | the proof alone, `serialize_compressed` |
| `verify` | a cold whole process at 10 ms resolution | a warm in-process call |

## 2. The INT8 encoding, declared — and why it is exact on T1

**jolt-atlas is not an INT8 system.** `atlas-onnx-tracer` quantizes a float ONNX graph into
**i32 fixed point at a global log2 scale**, `common::consts::MODEL_SCALE = 14`
(`common/src/consts/general.rs`). That constant carries its own warning:

> *"Several lookup-table shapes in this codebase (activation clamping, softmax's saturating
> clamp) are const-generic on a bound derived from this value rather than the model's runtime
> scale, so changing it requires recompiling."*

`RunArgs::set_scale` exists and would change the runtime scale, but the lookup tables would
not follow it. **Every cell was measured at the default scale 14**, which is the only value
the shipped binaries are built for.

So `bench/TASKS.md`'s INT8 domain is carried like this:

```
INT8 value v   ->   ONNX float  v / 128   ->   tracer integer  round(v/128 · 2^14) = v · 128
```

and for a **single** matmul this is exact rather than approximate. `Einsum::f` accumulates in
i64 and then floor-rebases by `1 << scale` (§3):

```
acc      = Σ_k (a_k · 128) · (w_k · 128)  =  2^14 · Σ_k a_k w_k
rebased  = floor(acc / 2^14)              =         Σ_k a_k w_k        EXACT, because 128·128 = 2^14
```

**So on every T1 rung the integer jolt-atlas proves as the output is bit-for-bit the INT32
accumulator `bench/TASKS.md` specifies**, and no precision is lost to the rebase. The
operands it commits to are `128×` the task's INT8 values — a 15-bit domain, not an 8-bit one —
and that is stated in the conditions line of every figure.

The witness is handed to the prover as **already-quantized integers**, straight from the
generator's `*.inputs.json` into `Tensor::new`, so nothing between the task specification and
the prover reinterprets it. The weights are ONNX initializers and are quantized by jolt-atlas
itself.

**The RNG is not binius64's.** The published seeds from
[`bench/systems/binius64/EXPRESSION.md`](../binius64/EXPRESSION.md) §7 are reused, but this
generator draws with numpy's PCG64 and binius64's harness draws with Rust's `StdRng`. Same
seed, different stream: the systems prove the same shapes and the same MAC counts on
**different instances**. Task-level comparison only, never witness-level. This is the same
declaration DeepProve's file makes, and the two ONNX generators do agree with each other on
neither instance nor stream — each is its own.

## 3. Requantization — the task says no, and jolt-atlas cannot say no

`bench/TASKS.md` Amendment A1 (2026-08-23) resolves T2 explicitly: **no requantization**,
accumulators carry full width from one layer to the next, extending T1's stated rule.

**jolt-atlas rebases after every einsum, unconditionally, and there is no switch.** From its
own source:

```rust
impl Op for Einsum {
    fn f(&self, inputs: Vec<&Tensor<i32>>) -> Tensor<i32> {
        // Fused: i64 accumulate, floor-rescale by `1 << scale`, saturating clamp
        // to i32. Replaces the einsum + its ScalarConstDiv rebase node .
        einsum_i32_with_i64_rebase(&self.equation, &inputs, self.scale).unwrap()
    }
    fn rebase_scale_factor(&self) -> Option<usize> { Some(1) }
}
```
`atlas-onnx-tracer/src/ops/einsum.rs:11-20`

There is no flag, no environment variable and no Cargo feature that turns it off. `RunArgs`
exposes exactly three knobs — `variables`, `scale`, `pad_to_power_of_2`
(`atlas-onnx-tracer/src/model/mod.rs:385-395`) — and none of them is this one.

**What that costs, by task, stated exactly rather than in general:**

- **T1 — no cost.** §2 shows the rebase is arithmetically the identity for a single 8-bit
  matmul at scale 14. jolt-atlas is charged for a right shift and a saturating clamp it did
  not need, and the *value* it proves is the task's value.
- **T2 and T3 — A1 cannot be honoured.** Layer 1's output is already `Σ a w` at the tracer's
  fixed scale; layer 2's rebase divides by `2^14` again, and that division is lossy. **The
  accumulator does not carry full width between layers, and no configuration makes it.**

This is a property of jolt-atlas's design, not a mis-expression: a global fixed scale with a
rebase per multiply *is* how it keeps values inside i32 and inside its lookup tables. Saying so
is not a criticism; hiding it would make the comparison dishonest. It is also **the same wall
DeepProve hit**, reached by a different mechanism — DeepProve inserts a `Requant` layer, jolt-atlas
fuses the rescale into the einsum — which is worth knowing about the class of system rather
than about either one.

## 4. Power-of-two padding — the switch exists and the prover does not honour it

`ModelLoader` pads every dimension up to the next power of two, and unlike DeepProve the call
is **conditional**:

```rust
if run_args.pad_to_power_of_2 {
    loader = loader.pad();
}
```
`atlas-onnx-tracer/src/model/load.rs:35-37`

The default is `true` (`model/mod.rs:403`) and there is a public setter,
`RunArgs::with_padding(bool)` (`model/mod.rs:485-488`), which jolt-atlas's own tests use. Read
from the source, that says padding is optional and this benchmark can measure what it costs.

**Measured, it is not optional.** The whole T1 ladder was run with `with_padding(false)` and
every non-power-of-two rung dies in the prover, not in the loader:

```
thread 'main' panicked at joltworks/src/poly/dense_mlpoly.rs:34:9:
Dense multi-linear polynomials must be made from a power of 2 (not 768)
```

Raw: [`bench/data/cells-jolt-atlas/t1-{a,b,c,d}-t1-p0-n5/log.txt`](../../data/cells-jolt-atlas/).

| rung | dims | `with_padding(false)` |
|---|---|---|
| T1-0 | `[1×256]·[256×256]` | **runs** — already a power of two, so padding was a no-op |
| T1-a | `[1×768]·[768×768]` | **panics** in `dense_mlpoly.rs:34` |
| T1-b, T1-c, T1-d | `768` | **panics**, same line |

**T1-0 is the control that makes this readable**, and it is worth its own line because it
measures the flag rather than assuming it: padded 64.98 ms / 18 874 752 B against unpadded
63.32 ms / 18 841 984 B — **0.17 % apart in memory**. So at a power-of-two shape the flag
genuinely changes nothing, which is what confirms the panics above are about the *dimension*
and not about the flag doing something else.

**Conclusion, stated as measured rather than as read:** jolt-atlas pads to powers of two and
**cannot be told not to** for any shape that needs it. That is the same practical position
DeepProve is in (`systems/deepprove/EXPRESSION.md` §4), reached through a switch that looks
like it should help and does not. `768` is GPT-2's hidden size and pads to `1024`, so **every
768-wide rung of T1 is proved as 1024 wide: 1.778× the task's arithmetic.**

**The denominator stays the published MAC count in every figure**, because `bench/TASKS.md` is
frozen and fixes it. `RESULTS.md` prints the padded-basis rate beside it, clearly labelled as
not the benchmark metric.

This section is also a methodological note against ourselves. An earlier draft of this file,
written from the source, said jolt-atlas "**can** be told not to pad" and treated that as a
point of difference from DeepProve. **The code said where to look; only the measurement said
what it was worth**, and the measurement said the opposite.

## 5. The graphs

### T1 — the INT8 matmul ladder

`C = A[M×K] · B[K×N]`, one `MatMul` node. `A` is the graph input — the witness — and `B` is an
initializer, so the weights are committed at preprocessing and the input is what varies per
proof. That is the shape a real inference proof has.

**Unlike DeepProve, every rung is expressible, including `M > 1`.** jolt-atlas's einsum
registry carries `mk,kn->mn` as a first-class pattern
(`jolt-atlas-core/src/utils/dims.rs:54-62`), so `[4×768]·[768×768]`, `[16×768]` and `[64×768]`
are matrix-by-matrix products it proves natively. DeepProve's ONNX frontend rejects all three
(`systems/deepprove/NOT_EXPRESSIBLE.md` §1). **jolt-atlas is the first system in this
benchmark to run all five rungs of T1.**

Node names carry no meaning here: jolt-atlas parses through `tract-onnx` and dispatches on the
real ONNX operator, not on the node's name. (DeepProve dispatches on a substring of the node
*name*; the difference is worth knowing if you are writing graphs for both.)

### T2 and T3 — the MLP, and three of our own mistakes on the way to expressing it

200-256-128-64-1, `Relu` after layers 1–3, linear output, **no biases**
(`bench/TASKS.md` specifies none). T3 is the same network over 8 independent inputs.

The MLP is emitted as **`Gemm` with `transB=1` and the weight stored `[out, in]`**, at
`ir_version = 6`. That is not a stylistic choice and it took three wrong expressions to reach,
each of which *looked* like a limit of jolt-atlas and was a limit of our emission. They are
recorded because the same trap is waiting for anyone else writing graphs for this frontend, and
because a benchmark that reported them as findings would have been wrong three times:

| our emission | what jolt-atlas said | what it actually was |
|---|---|---|
| `MatMul`, static batch 1 | `Einsum equation (k,kn->mn) not supported` | tract collapses the activation's rank across a `MatMul`; **their own MLPs use `Gemm`** |
| `Gemm`, `transB=0`, weight `[in, out]` | same | the `transB=1` form is what `torch.onnx.export` emits and what their models carry |
| symbolic `batch_size`, batch-8 witness | `Input tensor 0 has dims [8, 200], expected [1, 200]` | **ours**: `RunArgs::default()` binds `batch_size` to 1 and our harness had not overridden it |

**The check that caught all three was running jolt-atlas's own bundled models through our
harness unchanged.** `models/perceptron` (4→30→30→30→3, four `Gemm` layers, batch 1) and
`models/mlp_square_4layer` both prove and verify through it. A four-layer batch-1 MLP is
therefore *not* a limit of jolt-atlas, and any report saying so would have been an artefact of
our ONNX.

**With the expression corrected, T2 and T3 still do not prove**, and now for exactly one
reason: the **64→1 output layer**. [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1 isolates it to
that single variable with a width ladder.

## 6. What is inside the `prove` bracket, stated because it is not just the prover

`ONNXProof::prove` calls `Model::trace`, which **executes the quantized graph** before any
proving happens (`ONNXProof::prove:Model::trace:Model::execute_graph:...` appears in every
traced run). So our `prove` column includes jolt-atlas's own quantized inference, exactly as
DeepProve's did.

It is a much smaller share here — on nanoGPT the traced `Model::trace` subtree is a few tens of
milliseconds against a 12 s prove — but it is **not zero**, and no figure in `RESULTS.md`
claims a prover-only time. The three timed stages jolt-atlas's own spans expose inside prove
(`commit_witness_polynomials`, `iop`, `prove_reduced_openings`) are reported in
`RESULTS.md` §7 for the reproduction cells, read from its tracing output rather than from any
instrumentation of ours.

## 7. Witness seeds and the accumulator bound

Seeds are the published ones ([`binius64/EXPRESSION.md`](../binius64/EXPRESSION.md) §7):

| Task | seed | max &#124;accumulator&#124;, INT8 units | headroom under `i64::MAX` |
|---|---|---:|---:|
| T1-0 | `0xE0060100` | 2.46·10⁵ | 3.8·10¹³× |
| T1-a | `0xE00601A0` | 4.91·10⁵ | 1.9·10¹³× |
| T1-b | `0xE00601B0` | 5.63·10⁵ | 1.6·10¹³× |
| T1-c | `0xE00601C0` | 5.95·10⁵ | 1.6·10¹³× |
| T1-d | `0xE00601D0` | 6.27·10⁵ | 1.5·10¹³× |
| T2 | `0xE0060200` | 6.89·10¹² | 1.3·10⁶× |
| T3 | `0xE0060300` | 2.27·10¹³ | 4.1·10⁵× |

Amendment A1 requires an implementation to **assert** this bound rather than rely on the seed
happening to be benign, and the generator does: it computes the whole forward pass in exact
integer arithmetic and refuses to write a task whose maximum lacks a factor-2 margin under
`i64::MAX`. Values are in [`manifest.json`](../../tasks/jolt-atlas/manifest.json).

**The bound is moot for jolt-atlas in practice**, because §3's forced rebase means its
accumulators never carry full width between layers anyway. The assertion is kept because the
task specification requires it of any implementation.
