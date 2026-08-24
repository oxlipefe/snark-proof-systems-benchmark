# DeepProve — how each task was expressed

`bench/TASKS.md` fixes each task by an **exact MAC count**. That count is the denominator of
both `MAC/s` and `bytes/MAC`, so it is never recomputed here. The generator
[`bench/scripts/deepprove/make-tasks.py`](../../scripts/deepprove/make-tasks.py) asserts that
the graph it emitted performs exactly the published number of multiply-accumulates and
refuses to write a file that does not:

```python
published = PUBLISHED_MACS[task]
if macs != published:
    sys.exit(f"{task}: graph performs {macs} MACs but bench/TASKS.md fixes {published}")
```

**But DeepProve does not prove the graph we hand it.** It pads every dimension up to the next
power of two before proving, so on two of the three tasks it proves *more* arithmetic than
the task specifies. That is measured and reported in §4, and it is the single most important
condition attached to every DeepProve figure in `RESULTS.md`.

Nothing in this directory is DeepProve code. The ONNX files are plain ONNX, emitted by our
generator.

---

## 1. Which frontend, and why the ONNX one

DeepProve has two entry points for a non-LLM model, and they are not equivalent.

| | `zkml` binary `bench` (what `zkml/bench.py` drives) | `deep-prove-worker one-shot` |
|---|---|---|
| Input | `--onnx model.onnx --io io.json` | `--model model.onnx --inputs inputs.json` |
| Quantization strategy | `inference` (calibrated) or `maxabs` | `AbsoluteMax`, fixed |
| Runs the float reference model | **yes, unconditionally** | no |
| Runs on this machine | **no** — see [`BUILD.md`](BUILD.md) §3 | yes |

The tasks are therefore expressed against **`deep-prove-worker one-shot`**, DeepProve's own
local proving binary, with `AbsoluteMax` quantization. `BUILD.md` §3 documents why the other
one could not be used and what that costs us.

## 2. The INT8 encoding, declared

`bench/TASKS.md` specifies INT8 operands in `[-128, 127]` — signed. DeepProve does not take
integer models: its frontend takes a **float** ONNX graph and quantizes it itself, and
`zkml/src/inputs.rs:34-45` rejects any input value outside
`QUANTIZATION_RANGE = [-1.0, 1.0]` (`zkml/src/quantization/mod.rs:40-42`).

So every INT8 value `v` is carried as the float `v/128`, landing in `[-1, 0.9921875]`.

DeepProve's quantized domain is set by the `ZKML_BIT_LEN` environment variable
(`zkml/src/quantization/mod.rs:27-38`), which **defaults to 12** — its published evaluation
uses 12-bit. It can be set to 8, and then:

```
MIN = -(1 << (8-1)) = -128        MAX = (1 << (8-1)) - 1 = 127
```

which is exactly the task's INT8 domain. **Every primary cell in `RESULTS.md` was measured
at `ZKML_BIT_LEN=8`**, so the arithmetic proved is the INT8 arithmetic the task specifies and
not DeepProve's wider default. A control cell at `ZKML_BIT_LEN=12` — DeepProve's own default,
and the setting behind its published numbers — is measured beside it, and any comparison
between the two systems carries the bit width in the same sentence.

`ZKML_BIT_LEN` is applied to **both** weights and activations (`zkml/src/quantization/mod.rs`
`ScalingFactor`, used at `zkml/src/layers/einsum/quantise.rs:85` for weights and
`zkml/src/inputs.rs:82-89` for inputs). It is **not validated**: an unparseable value falls
back to 12 silently.

Because DeepProve derives its own scaling factors from the tensors it is given, the integers
it ends up proving are **not bit-identical** to the ones binius64 proved. The two systems
prove the same *shape*, the same *operation count* and the same *task*; they do not prove the
same witness. `bench/README.md` says this benchmark compares tasks rather than circuits, and
this is what that costs.

## 3. The graphs

### Node names carry the dispatch, not op types

DeepProve's ONNX loader does **not** dispatch on the ONNX operator. It lowercases the node's
*name* and looks for a parser key as a substring (`zkml/src/parser/onnx.rs:257-264`, keys at
`:230-238`: `Conv`, `Gemm.ab`, `MatMul`, `Relu`, `Flatten`, `Pool`, `Reshape`). A node whose
name does not contain one of those is rejected as `Unknown node type`, whatever its op is.
Our nodes are named `MatMul_0`, `MatMul_1`, `Relu_1`, … so each hits exactly one key.

This also rules out an `Identity` node to rename the final tensor: there is no `Identity`
parser. The last `MatMul` writes the graph output directly.

### T1 — the INT8 matmul ladder

`C = A[M×K] · B[K×N]`, one `MatMul` node. `A` is the graph input — the witness — and `B` is
an initializer, so the weights are committed at setup and the input is what varies per proof.
That is the shape a real inference proof has.

```python
node = helper.make_node("MatMul", ["input", "W"], ["output"], name="MatMul_0")
```

**Only `M = 1` is expressible.** DeepProve's dense parser flattens the input and then
requires it to reduce to a vector (`zkml/src/parser/onnx.rs:563-574`), so T1-b, T1-c and T1-d
are rejected. See [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1 for the measured error.

### T2 and T3 — the MLP

200-256-128-64-1, `Relu` after layers 1–3, linear output, **no biases** (`bench/TASKS.md`
specifies none; DeepProve's dense parser treats bias as optional,
`zkml/src/parser/onnx.rs:613-655`, so none is emitted).

T3 is the same network over 8 independent inputs. DeepProve cannot put them in one proof —
its ONNX parser pins `batch_size = 1` (`zkml/src/parser/onnx.rs:151-160`) — so T3 is
expressed the way `bench/TASKS.md` says such a system reports it: **8 separate proofs**.

**Neither T2 nor T3 could be proved.** The 64→1 output layer is rejected inside DeepProve's
sumcheck. `NOT_EXPRESSIBLE.md` §2 isolates the cause to a single variable.

## 4. Power-of-two padding — DeepProve proves more than the task asks

`FloatOnnxLoader::build` calls `pad_model` unconditionally (`zkml/src/parser/onnx.rs:110`),
and `pad_model` rounds every dimension up to the next power of two
(`zkml/src/padding.rs:110-123`, `:346`). This is not configurable.

**Read off the models DeepProve itself printed**, in each cell's own log, not derived:

| Task | task shape | DeepProve's padded input | published MACs | MACs actually proved | inflation |
|---|---|---|---|---|---|
| T1-0 | `[256] · [256×256]` | `[256]` | 65 536 | 65 536 | **1.000×** |
| T1-a | `[768] · [768×768]` | **`[1024]`** | 589 824 | 1 048 576 | **1.778×** |
| T2 | `[200] → … → [1]` | `[256]` | 92 224 | — (not proved) | — |

`768` is GPT-2's hidden size, so this is not a synthetic corner: **a transformer's natural
linear layer costs DeepProve 1.78× the arithmetic it contains.** The ladder in
`bench/TASKS.md` was built around 768 for exactly that reason.

**The denominator stays the published MAC count**, in every figure, because `bench/TASKS.md`
is frozen and fixes it. So T1-a's `MAC/s` is a rate for the *task*, not for DeepProve's
internal work, and its `bytes/MAC` is charged against the task's MACs rather than the padded
ones. `RESULTS.md` prints the padded-basis rate beside it so a reader can have both, clearly
labelled as not the benchmark metric.

## 5. Requantization — the task says no, and DeepProve cannot say no

`bench/TASKS.md` Amendment A1 (2026-08-23) resolves T2 explicitly: **no requantization**,
accumulators carry full width from one layer to the next, extending T1's stated rule.

**DeepProve inserts a requantization layer after every linear layer, and there is no way to
turn it off from outside the library.** The flag exists but is private:

- `EinSum` is constructed with `requantise: true` (`zkml/src/layers/einsum/mod.rs:223`).
- Quantization emits a `Requant` whenever that flag is set
  (`zkml/src/layers/einsum/quantise.rs:87-96`).
- The layer enum's own comment: *"Since we always do a requant layer after each dense…"*
  (`zkml/src/layers/mod.rs:80-82`).
- `no_requant()` / `disable_requantisation()` exist
  (`zkml/src/layers/einsum/mod.rs:288-300`) but every caller is on the LLM path; the ONNX
  loader never calls them.

We verified the absence of an external switch directly rather than taking it on trust: the
**only** quantization-related environment variable in the whole `zkml` crate is
`ZKML_BIT_LEN`, and no Cargo feature (`cpu`, `cuda`, `wgpu`, `gpu`, `otel`, `mem-track`,
`capture-layers-quant`) and no flag of `bench` or `deep-prove-worker` touches requantization.

**It is visible in the measurement, not just in the source.** DeepProve prints the layers it
inserted. Raw logs:
[`bench/data/bench-binary-deepprove/`](../../data/bench-binary-deepprove/) — captured from the
`bench` binary, which prints the quantized model before it fails for the unrelated reason in
[`BUILD.md`](BUILD.md) §3.

T1-0 — a **one-node** ONNX graph becomes a two-layer proved model:

```
- N1: EinSum(A(j)@W(ij)->O(i))
- N3: Requant: right shift: 25, scale: 17799
```

T2 — **four** linear layers, **four** inserted requantizations:

```
- N9:  Requant: right shift: 25, scale: 18500
- N10: Requant: right shift: 24, scale: 25809
- N11: Requant: right shift: 24, scale: 26667
- N12: Requant: right shift: 22, scale: 26045
```

The worker cells corroborate it without printing the layers: T1-0's one ONNX node becomes
`Quantized model with 4 layers`, and T2's seven ONNX nodes become
`Quantized model with 13 layers` (`bench/data/cells-deepprove/*/stdout.txt`).

**Consequence, and it is not small.** On T1 this is a deviation from `bench/TASKS.md`'s
"INT32, not requantized" rule, and on T2 it is a deviation from Amendment A1. DeepProve is
doing **strictly more work** than the task specifies — a lookup argument per linear layer that
binius64 was not charged for. Every DeepProve figure in `RESULTS.md` therefore carries
`requantized: yes (not disableable)` in the same line as the number, and no DeepProve figure
is compared with a binius64 figure without it.

This is a property of DeepProve's design, not a mis-expression: requantization *is* how it
keeps accumulators inside its field, and its LogUp-GKR lookups are what make it fast. Saying
so is not a criticism; hiding it would make the comparison dishonest.

## 6. Witness seeds

The seeds published for these tasks in
[`bench/systems/binius64/EXPRESSION.md`](../binius64/EXPRESSION.md) §7 are reused, so the
instances are drawn from the same declared seed:

| Task | seed |
|---|---|
| T1-0 | `0xE0060100` |
| T1-a | `0xE00601A0` |
| T1-b | `0xE00601B0` |
| T1-c | `0xE00601C0` |
| T1-d | `0xE00601D0` |
| T2 | `0xE0060200` |
| T3 | `0xE0060300` |

**The RNG is not the same one.** binius64's harness draws with Rust's
`StdRng::seed_from_u64`; this generator draws with numpy's PCG64. Same seed, different
stream, so the two systems prove the same shapes and the same MAC counts on **different
instances**. Stated rather than glossed: it means no witness-level comparison between the two
systems is available, only a task-level one.

Operands are drawn as integers over the full `[-128, 127]` and carried as `v/128`.

## 7. The accumulator bound, asserted

Amendment A1 requires an implementation to assert the no-requantization bound rather than
rely on the published seed happening to be benign. The generator computes the whole forward
pass in float, converts each accumulator back to integer units, and refuses to write a task
whose maximum lacks a factor-2 margin under `i64::MAX`:

| Task | max &#124;accumulator&#124;, INT8 units | headroom under `i64::MAX` |
|---|---|---|
| T1-0 | 3.44·10⁵ | 2.7·10¹³× |
| T1-a | 5.30·10⁵ | 1.7·10¹³× |
| T2 | 3.96·10¹³ | 2.3·10⁵× |
| T3 | 2.92·10¹³ | 3.2·10⁵× |

These are our instance's values, from `bench/tasks/deepprove/manifest.json`. They differ from
binius64's (8.96·10¹² for T2) because the RNG differs, per §6.

**The bound is moot for DeepProve in practice**, because §5's forced requantization means its
accumulators never carry full width between layers anyway. The assertion is kept because the
task specification requires it of any implementation.
