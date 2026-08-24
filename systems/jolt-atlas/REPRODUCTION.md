# jolt-atlas — the system's own reference numbers, and whether we reproduced them

`bench/README.md` §"Fairness protocol" and [`CHALLENGE.md`](../../CHALLENGE.md) commit this
repository to reproducing each measured system's **own published reference number before
reporting anything about that system**, and to publishing the discrepancy *above* the result
if we cannot.

**We reproduced it — at the commit that published it — and it does not hold at the commit we
measured.** That sentence is the whole of this file and it is why this file comes before every
jolt-atlas figure in [`RESULTS.md`](RESULTS.md).

Nothing here is an accusation. A repository that keeps moving and a README that keeps its old
numbers is the most ordinary situation in this field, and the only reason we can say anything
precise about it is that jolt-atlas's own tracing spans decompose the difference for us.
[Right of reply](#5-right-of-reply) applies with priority.

---

## 1. The reference

jolt-atlas publishes no paper for these figures. Its `README.md` §"Benchmarks" is the source,
and it declares its machine — **"System specs: MacBook Pro M3, 16GB RAM"** — for both tables.

**nanoGPT (~0.25M params, 4 transformer layers)** — which the README calls *"the standard
workload we use for cross-project comparison"*:

| Stage | Published |
|---|---:|
| Verifying key generation (`setup_verifier`) | <0.001 s |
| Proving key generation (`setup_prover`) | 0.263 s |
| Proof time (`ONNXProof::prove`) | **2.288 s** |
| Verify time (`ONNXProof::verify`) | **0.127 s** |
| End-to-end total | 2.678 s |

**GPT-2 (125M params)**:

| Stage | Published |
|---|---:|
| `setup_prover` | 1.003 s |
| `ONNXProof::commit_witness_polynomials` | 0.762 s |
| `ONNXProof::iop` | 5.997 s |
| Reduction opening proof (excl. `HyperKZG::prove`) | 1.899 s |
| `HyperKZG::prove` | 2.392 s |
| **Proof time (`ONNXProof::prove`)** | **14.889 s** |
| **Verify time (`ONNXProof::verify`)** | **1.038 s** |
| End-to-end total | 16.930 s |

On the strength of the nanoGPT row the README claims *"roughly a **104× speed-up** on proof
generation alone"* against ezkl's 237 s.

## 2. Three conditions the README does not declare, measured

`bench/README.md`'s conditions line requires far more than these three, but these three are
the ones a reader of the published table cannot supply for themselves. **This is the most
useful thing this benchmark can contribute about jolt-atlas**, and it is offered as material
for the right of reply, not as a complaint.

| | nanoGPT | source |
|---|---|---|
| **Sequence length** | **64** | not in the README. It is fixed in the code the README's own command runs: `examples/nanoGPT.rs` builds `Tensor::new(…, &[1, 64])`, and the bundled `models/nanoGPT/input.json` carries 64 tokens and a 4 160-element output (= 64 × 65, a 65-symbol character vocabulary) |
| **Peak memory** | **724.4 MB** peak footprint · **734.1 MB** peak RSS | not in the README, and not reported for any workload. Whole process, `/usr/bin/time -l`, 1 warmup + 3 timed proofs in one process, at commit `434ab99` |
| **Proof size** | **3 239 698 B = 3.09 MiB** | not in the README. `ONNXProof::prove` → `serialize_compressed`, the same call jolt-atlas's own `gpt2_zk_bench` example uses to report a size |

The GPT-2 sequence length is **16**, and unlike nanoGPT's it is discoverable from the README
itself if the reader knows what to look for: §"Getting Started" step 4 says the traced output
shape is `[1, 16, 65536]`. It is also hardcoded in the tree —
`examples/gpt2_zk_bench.rs`: `let seq_len: usize = 16;`. **A GPT-2 proof of 16 tokens is what
the 14.889 s figure describes**, and the README does not say so next to the number.

`65536` in that shape is GPT-2's 50 257-symbol vocabulary padded up to the next power of two —
jolt-atlas's padding, discussed in [`EXPRESSION.md`](EXPRESSION.md) §4.

## 3. The reproduction, and the finding

### 3.1 At the pinned commit, the published number does not hold

`434ab99` (2026-08-21), the tree this benchmark measures, built and run exactly as the README
documents: `cargo run --release --package jolt-atlas-core --example nanoGPT`.

| | README | measured at `434ab99` | ratio |
|---|---:|---:|---|
| `setup_prover` | 0.263 s | 0.328 s | 1.25× |
| **`ONNXProof::prove`** | **2.288 s** | **12.1 s** | **5.3× slower** |
| **`ONNXProof::verify`** | **0.127 s** | **0.508 s** | **4.0× slower** |

Timings are jolt-atlas's own `--trace-terminal` spans. **Two independent instruments agree**:
our own harness, calling the same public API on the same model, measures prove
11.78–12.04 s and verify 0.490–0.510 s across three repetitions in one process.

**The gap is not tracing overhead**, which was the first thing checked: the same binary with
no tracing subscriber at all runs the whole process in 12.93 s, against 13.01 s with the
filtered subscriber. **It is not machine sleep**: every run recorded `slept ≈ 0.000 s`.

### 3.2 At the commit that published the number, it reproduces

The README's figures were introduced by commit **`53b7c873`, 2026-05-06**
(*"docs: update performance metrics and descriptions in README (#241)"*); the GPT-2 table by
`497ff796` the day before. So the published numbers are **3½ months and roughly 40 commits
older than the tree they sit in**.

That commit was checked out into a second working tree (`git worktree add`, pristine), built
identically, and run identically:

| | README | `53b7c873` on this machine | ratio |
|---|---:|---:|---|
| `setup_prover` | 0.263 s | 0.337 s | 1.28× |
| **`ONNXProof::prove`** | **2.288 s** | **2.50 / 2.57 / 2.57 s** | **1.10–1.12×** |
| **`ONNXProof::verify`** | **0.127 s** | **0.142 / 0.145 / 0.144 s** | **1.12–1.14×** |

**That is a reproduction.** A 10–14 % spread between an M1 Max and the declared MacBook Pro M3
is exactly the size of difference one expects and cannot be called anything else.

So the fairness protocol's check **discharges**: our build of jolt-atlas behaves the way its
authors' build did, at the commit they measured. What does not hold is the number's currency.

### 3.3 The difference, decomposed by jolt-atlas's own spans

The same three spans the README's GPT-2 table names, on nanoGPT, on one machine, at two
commits:

| Stage | `53b7c873` (2026-05-06) | `434ab99` (2026-08-21) | ratio |
|---|---:|---:|---:|
| `AtlasProverPreprocessing::gen` | 337 ms | 328 ms | 0.97× |
| `ONNXProof::commit_witness_polynomials` | 94.0 ms | 889 ms | **9.5×** |
| `ONNXProof::iop` | 1.17 s | 6.91 s | **5.9×** |
| `ONNXProof::prove_reduced_openings` | 1.23 s | 4.24 s | **3.4×** |
| **`ONNXProof::prove`** | **2.50 s** | **12.1 s** | **4.7×** |
| `ONNXProof::verify` | 144 ms | 508 ms | 3.5× |

**Preprocessing is unchanged and all three proving stages are slower**, the witness-commitment
stage by an order of magnitude. Whatever moved, it moved across the prover rather than in one
place.

**We did not bisect and we do not name a cause.** Between the two commits the log contains,
among about forty others, `feat: Add saturation to prover (#256)`,
`fix(soundness): bind public inputs into the transcript (#230) (#247)`,
`Read-Raf refactor, new PrefixSuffix tables tests (#275)`,
`feat: Clamp generic lookup table and allow various implementations (#250)` and
`feat: dory (#265)`. Several of those are the kind of change that buys correctness with time,
which is a trade a project is entitled to make. **Attributing the 4.7× to any of them would be
guessing, and this file does not guess.** Bisecting is roughly a dozen 13-minute LTO builds and
it was outside this campaign's budget; it is the obvious next measurement and we will run it if
the authors want it.

### 3.4 What that does to the 104× claim

Arithmetic only, on the README's own comparison, with ezkl's 237 s held fixed exactly as the
README holds it:

| | prove | vs ezkl 237 s |
|---|---:|---:|
| README's nanoGPT figure | 2.288 s | **104×** |
| `53b7c873`, this machine | 2.50 s | 94.8× |
| **`434ab99`, this machine** | **12.1 s** | **19.6×** |

**We did not run ezkl**, so 237 s is taken on the README's authority and nothing here is a
measurement of ezkl. The point is narrow: **the same claim computed against the current tree is
19.6×, not 104×**, and a reader of the README has no way to know that.

### 3.5 A second undeclared condition: thread count, which is not monotone

The README declares a machine but no thread count, and jolt-atlas's default is rayon's, i.e.
one worker per core. **On this 10-core machine that is the wrong setting**, measured on
nanoGPT at `434ab99`, whole process:

| `RAYON_NUM_THREADS` | real | `(user+sys)/real` | peak footprint | involuntary ctx switches |
|---:|---:|---:|---:|---:|
| 1 | 20.84 s | 1.15 | 643 MB | 439 573 |
| 2 | 13.19 s | 2.02 | 681 MB | 757 705 |
| **4** | **11.39 s** | 3.16 | 677 MB | 1 151 678 |
| 6 | 12.07 s | 3.98 | 689 MB | 1 822 147 |
| 8 | 13.36 s | 4.98 | 659 MB | 2 408 780 |
| 10 (default) | 13.85 s | 6.10 | 689 MB | 2 722 527 |

**The optimum is 4 threads, and the default is 22 % slower than it.** System time rises from
1.02 s to 43.91 s across that sweep while user time rises only 22.86 → 40.60 s: the extra cores
are buying kernel work, not arithmetic. The involuntary-context-switch column names the
mechanism, and **jolt-atlas's own authors describe it** in a comment in
`examples/gpt2_zk_bench.rs`: the patched arkworks MSM *"builds a fresh nested 2-thread
`rayon::ThreadPoolBuilder` per MSM chunk per call"*.

The declared M3 has 8 cores against this machine's 10, so a reader comparing the two tables
should know that the thread count is both undeclared and non-monotone. **Every figure in
`RESULTS.md` states its `RAYON_NUM_THREADS`.**

## 4. GPT-2

### 4.1 It could not be run, and the reason is not in jolt-atlas

**NOT REPRODUCED, and not attempted as a timing.** jolt-atlas's GPT-2 model is not in the
repository; its README §"Getting Started" instructs the reader to run
`python scripts/download_gpt2.py`, which pip-installs `optimum[exporters]` and exports GPT-2
through Hugging Face Optimum.

**That script, run unmodified, produces an ONNX file the pinned tree cannot load.** The export
itself succeeds (`model.onnx` plus a 498 MB external-data file), and their own tracer-only
check — README step 4, `cargo run --release --package atlas-onnx-tracer --example gpt2` —
then fails:

```
called `Result::unwrap()` on an `Err` value:
  Parsing as TDim: `((batch_size*sequence_length)//sequence_length)'
Caused by:
  Failed to parse "((batch_size*sequence_length)//sequence_length)"
```

Raw: [`gpt2-tracer.stderr.txt`](../../data/repro-jolt-atlas/gpt2-tracer.stderr.txt),
[`gpt2-export.log`](../../data/repro-jolt-atlas/gpt2-export.log).

**The cause is almost certainly the exporter, not the prover.** Today's Optimum/torch exporter
emits a symbolic dimension as the expression `((batch_size*sequence_length)//sequence_length)`
rather than as a plain symbol, and the `tract` version jolt-atlas pins (0.22.1-pre,
`c484b3ee`) has no parser for `//` in a `TDim`. `scripts/download_gpt2.py` pins **no**
version of `optimum`, `transformers` or `torch`, so the file it produces in August 2026 is not
the file it produced in May 2026 when the GPT-2 table was published.

**So this is a reproducibility gap in the published flow rather than a defect in the prover**,
and we say so rather than reporting "GPT-2 does not work". Getting a loadable GPT-2 export
would mean finding the exporter version contemporaneous with `497ff796`, which the repository
does not record. **Four export configurations were tried** — optimum 2.3/2.2 (no
`optimum.exporters.onnx` module at all), optimum 1.24 on Python 3.14 (`NormalizedConfig`
type error), and optimum 1.24 + transformers 4.48.3 on Python 3.13, which is the one that
exported and then failed to load. We stopped there rather than search version space.

### 4.2 What that costs this benchmark

- **No jolt-atlas GPT-2 figure of ours exists**, and none is used anywhere.
- The 14.889 s prove and 1.038 s verify rows remain unchecked by us, at **any** commit.
- The nanoGPT reproduction (§3) is therefore the whole of the fairness check, and it is
  enough for the tasks measured in [`RESULTS.md`](RESULTS.md): T1 is a matmul ladder, not a
  transformer, and every T1 cell verifies its own proof ([`BUILD.md`](BUILD.md) §2).

**The one thing worth carrying forward** is §2's observation, which does not depend on running
GPT-2 at all: the 14.889 s figure describes a **16-token** GPT-2 forward pass, and the README
does not say so.


## 5. Right of reply

[`CHALLENGE.md`](../../CHALLENGE.md) applies, and this is the file where we would most rather
be corrected. Specifically, if the jolt-atlas authors can tell us:

- whether the README's figures are intended to describe `53b7c873` or the current tree, and
  what the current expected numbers are;
- whether the 4.7× prove regression between `53b7c873` and `434ab99` is known, intended
  (a correctness-for-speed trade) or a defect;
- a build configuration or thread count they consider correct for benchmarking — in
  particular whether `overflow-checks = true` in `[profile.release]` is deliberate for
  measured runs ([`BUILD.md`](BUILD.md) §1);
- the sequence length, peak memory and proof size behind each published row —

we will re-run all of it and publish both outcomes, with credit and date. **The old numbers
stay in the record next to the new ones; we do not quietly replace them.**
