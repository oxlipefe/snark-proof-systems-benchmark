# jolt-atlas — results

**Read [`REPRODUCTION.md`](REPRODUCTION.md) first.** `bench/README.md`'s fairness protocol
requires it, and here it is not a formality: jolt-atlas's published nanoGPT figure **reproduces
at the commit that published it and is 5.3× off at the commit measured here**. Every number
below describes the current tree, not the tree the README describes.

Then [`BUILD.md`](BUILD.md) for the build and its checks, [`EXPRESSION.md`](EXPRESSION.md) for
how each task was written and what jolt-atlas does to it before proving, and
[`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) for the two grid rows that did not run.

---

## Conditions line

Applies to every figure below. Where a cell differs, the cell's row says so.

```
system      jolt-atlas (ICME Labs)
commit      434ab990353f0c57b90e89cfd00162282e2150eb, tree pristine
licence     ICME Software License — NOT OSI. Internals NOT instrumented; see COMMIT.
instrument  our harness (bench/scripts/jolt-atlas/harness/), calling jolt-atlas's PUBLIC API
            only; peaks from /usr/bin/time -l around the process
field       BN254 scalar field (a16z arkworks fork, branch dev/twist-shout)
PCS         HyperKZG over BN254; transcript = Blake2b
            Dory also ships in the tree and was NOT measured (BUILD.md §4). Both PCS declare
            REQUIRES_MATERIALIZED_POLYS = true, so neither streams the committed polynomials;
            that is stated here because it conditions every memory figure below.
protocol    sumcheck + lookups ("Just One Lookup Table"), no quotient polynomials
security    NOT DETERMINED — no security-bit, soundness-bit or query-count parameter is
            exposed on this path or stated in the documentation. binius64 publishes
            SECURITY_BITS = 96; jolt-atlas and DeepProve publish nothing comparable, so
            nothing is invented here.
trusted setup   YES. HyperKZG is a pairing-based KZG variant over BN254 and needs a
                structured reference string; `setup_prover` builds it in process, per run,
                inside our `setup` column. binius64 requires no setup at all.
                THIS DIFFERENCE IS NOT NORMALIZABLE AND IS NOT AVERAGED AWAY: a system whose
                soundness rests on discarded toxic waste and a post-quantum hash-based system
                are not comparable on security even when their milliseconds are. It is stated
                here, in the same block as every figure, and not in a footnote.
ZK              no — the `zk` feature (BlindFold) ships and was not built
quantization    i32 fixed point at log2 scale 14 (MODEL_SCALE, the only value the shipped
                lookup tables are built for). The task's INT8 operands are carried as v/128,
                so the committed integers are 128x the task's INT8 values (a 15-bit domain),
                and T1's einsum output is EXACTLY the task's INT32 accumulator. EXPRESSION.md §2.
weights         PREPROCESSED (bench/TASKS.md Amendment A2). The weight matrix is an ONNX
                initializer, not a graph input: EXPRESSION.md §5, "`A` is the graph input — the
                witness — and `B` is an initializer, so the weights are committed at
                preprocessing and the input is what varies per proof", and §2, "The weights are
                ONNX initializers and are quantized by jolt-atlas itself."
weight cost     SETUP. It lands inside the `setup` column — `Model::load` + preprocessing +
                `setup_prover` + verifier preprocessing — which this benchmark reports apart and
                NEVER amortizes into prove time. **So the weight cost is excluded by
                construction from both derived metrics below**, unlike binius64 and Ceno where
                it is inside them. A2 §3: a `bytes/MAC` from a `preprocessed` system and one
                from a `witness` system are not the same quantity. The setup column is in the
                grid and is the same order as prove at 1 thread — 165.5 ms against 204.6 ms at
                T1-a, 8 203.8 ms against 8 533.8 ms at T1-d.
requantization  YES, a floor-rebase by 2^scale fused into EVERY einsum, NOT DISABLEABLE.
                On T1 it is arithmetically the identity (EXPRESSION.md §2); on T2/T3 it would
                break Amendment A1, and those tasks do not run anyway. EXPRESSION.md §3.
padding         every dimension rounded UP to the next power of two. `with_padding(false)` is
                a public setter and the PROVER REJECTS non-powers of two regardless, so it is
                not disableable in practice. T1-a..T1-d's 768 becomes 1024 = 1.778x the task's
                arithmetic. EXPRESSION.md §4.
batching        expressible (batch_size variable, batched einsum patterns) but T3 does not
                reach it. NOT_EXPRESSIBLE.md §2.
threads     RAYON_NUM_THREADS in {1, 4, 10}; NOT full thread control, see BUILD.md §5.
            A nested 2-thread pool per MSM chunk sits below it, by the authors' own account.
machine     Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, NOT dedicated
OS          macOS 26.5.2 (25F84), Darwin 25.5.0, uptime 12 d 20 h, 7.66 GB swap committed
N           5 timed repetitions after 1 warmup, in one process
date        2026-08-23 / 2026-08-24
```

## What is inside each measured quantity

| Column | What it contains | Same bracket as binius64? |
|---|---|---|
| `prove` | `ONNXProof::prove` per repetition. **Includes jolt-atlas's own quantized graph execution** (`Model::trace`), which no public entry point separates | **No — it includes tracing.** Upper bound on proving time |
| `verify` | `ONNXProof::verify`, warm, in the same process | **Yes** — and unlike DeepProve's, which was a cold whole process at 10 ms resolution |
| `proof bytes` | the proof alone, `serialize_compressed` — the call jolt-atlas's own `gpt2_zk_bench` uses | **Yes** — and unlike DeepProve's artifact, which carried its verifier context too |
| `setup` | `Model::load` + preprocessing + `setup_prover` + verifier preprocessing. Reported apart, **never amortized into prove** | Yes |
| `peak RSS` / `peak footprint` | `/usr/bin/time -l` over the whole process: model load, setup, all repetitions, and verification | Yes — binius64's peaks are also whole-process |

## The full grid

Every cell that was run, uncurated, including the ones that failed. Raw per-cell data:
[`cells-jolt-atlas.csv`](../../data/cells-jolt-atlas.csv) and
[`cells-jolt-atlas/`](../../data/cells-jolt-atlas/); derived table:
[`results-jolt-atlas.csv`](../../data/results-jolt-atlas.csv).

| Task | MACs (TASKS.md) | RAYON thr | pad | N | status | prove ms (median) | verify ms | proof B | setup ms | peak RSS MB | peak footprint MB | (u+s)/real | **MAC/s** | **B/MAC fp** | **B/MAC RSS** |
|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 1 | yes | 5 | OK | **64.98** | 5.60 | 21 419 | 49.4 | 25.3 | 18.0 | 1.98 | **1 008 541** | **288.0** | **405.0** |
| T1-a | 589 824 | 1 | yes | 5 | OK | **204.59** | 22.06 | 23 435 | 165.5 | 81.5 | 74.2 | 1.93 | **2 882 928** | **131.9** | **144.9** |
| T1-b | 2 359 296 | 1 | yes | 5 | OK | **584.63** | 22.42 | 25 307 | 571.0 | 208.1 | 201.0 | 2.15 | **4 035 565** | **89.3** | **92.5** |
| T1-c | 9 437 184 | 1 | yes | 5 | OK | **2 163.26** | 22.42 | 27 179 | 2 064.0 | 505.2 | 498.2 | 2.07 | **4 362 485** | **55.4** | **56.1** |
| T1-d | 37 748 736 | 1 | yes | 5 | OK | **8 533.84** | 26.32 | 29 051 | 8 203.8 | 1 864.1 | 1 857.8 | 2.00 | **4 423 416** | **51.6** | **51.8** |
| T1-a | 589 824 | 4 | yes | 5 | OK | **114.18** | 8.95 | 23 435 | 49.5 | 89.6 | 82.4 | 4.92 | **5 165 557** | **146.4** | **159.3** |
| T1-c | 9 437 184 | 4 | yes | 5 | OK | **841.16** | 9.66 | 27 179 | 556.1 | 536.1 | 529.0 | 6.20 | **11 219 210** | **58.8** | **59.6** |
| T1-a | 589 824 | 10 | yes | 5 | OK | **135.54** | 7.81 | 23 435 | 31.8 | 90.7 | 83.5 | 7.53 | **4 351 596** | **148.4** | **161.3** |
| T1-c | 9 437 184 | 10 | yes | 5 | OK | **808.09** | 8.53 | 27 179 | 305.1 | 499.7 | 492.8 | 8.36 | **11 678 353** | **54.8** | **55.5** |
| T1-0 | 65 536 | 1 | **no** | 5 | OK | **63.32** | 5.45 | 21 419 | 72.2 | 25.3 | 18.0 | 0.72 | **1 035 046** | **287.5** | **404.5** |
| T1-a | 589 824 | 1 | **no** | 0 | **FAIL_pow2** | — | — | — | 162.2 | — | — | — | — | — | — |
| T1-b | 2 359 296 | 1 | **no** | 0 | **FAIL_pow2** | — | — | — | 548.7 | — | — | — | — | — | — |
| T1-c | 9 437 184 | 1 | **no** | 0 | **FAIL_pow2** | — | — | — | 2 170.9 | — | — | — | — | — | — |
| T1-d | 37 748 736 | 1 | **no** | 0 | **FAIL_pow2** | — | — | — | 8 453.9 | — | — | — | — | — | — |
| T2 | 92 224 | 1 | yes | 0 | **FAIL_einsum** | — | — | — | 50.8 | — | — | — | — | — | — |
| T3 | 737 792 | 1 | yes | 0 | **FAIL_einsum** | — | — | — | 329.3 | — | — | — | — | — | — |

Status values. `FAIL_pow2` — the prover refuses a non-power-of-two dimension when padding is
switched off (`EXPRESSION.md` §4). `FAIL_einsum` — the einsum registry refuses the contraction
produced by a width-1 output layer (`NOT_EXPRESSIBLE.md` §1). **`FAIL_pow2` is a label derived
here, not a value from the ledger**; the raw ledger records `FAIL_rc101` for those four cells,
because that is all the exit code says, and the split is read off each cell's own `log.txt`.
Both values are in the record.

**Derived columns are empty for every cell that produced no proof.** Those processes still had
a memory peak — the peak of loading a model and then failing — and dividing it by a MAC count
the system never performed would manufacture a number out of a crash.

---

## 1 · The memory curve, which is the thing this repository exists to measure

**`bytes/MAC` is not a constant of jolt-atlas.** It falls by a factor of **5.6** across the
measured ladder and is still falling, more slowly, at the top. Padded, 1 thread, peak footprint:

| Task | MACs | peak footprint | **B/MAC** | vs previous rung |
|---|---:|---:|---:|---|
| T1-0 | 65 536 | 18.0 MB | **288.0** | — |
| T1-a | 589 824 | 74.2 MB | **131.9** | MACs ×9.00, memory ×4.12 |
| T1-b | 2 359 296 | 201.0 MB | **89.3** | MACs ×4.00, memory ×2.71 |
| T1-c | 9 437 184 | 498.2 MB | **55.4** | MACs ×4.00, memory ×2.48 |
| T1-d | 37 748 736 | 1 857.8 MB | **51.6** | MACs ×4.00, memory ×3.73 |

**The curve is the result. No single value on it is a property of the prover**, and quoting one
without its workload would be quoting a property of the pair.

**What the shape says, and where it stops saying it.** Fitting a local exponent
`log(Δfootprint)/log(ΔMACs)` rung by rung gives **0.645, 0.718, 0.655, 0.949**. So over the
first three steps memory grows clearly sublinearly in the task — fixed overhead being amortized
— and **over the last step it is essentially linear (0.949)**. Read plainly: what the falling
`bytes/MAC` shows is a constant being spread thinner, and by the top of the measured range the
marginal cost has flattened out at roughly 50 B/MAC.

**That is a description of five points, not an asymptote.** `bench/CHALLENGE.md` forbids
extrapolating outside the measured range, and it binds here: **nothing above says what
jolt-atlas does past 3.77·10⁷ MACs**, and in particular nothing above supports or refutes a
claim that its memory is bounded. A GPT-2 forward pass is roughly four orders of magnitude
beyond this ladder's top rung.

**And the mechanism does not favour a bounded reading.** Both polynomial commitment schemes in
the tree — HyperKZG and Dory — set `REQUIRES_MATERIALIZED_POLYS = true`
(`hyperkzg/commitment_scheme.rs:33`, `dory/mod.rs:193`), so the committed polynomials are
materialized rather than streamed. A `StreamingCommitmentScheme` trait exists
(`poly/commitment/commitment_scheme.rs:133`) and **the only type implementing it is
`MockCommitScheme`** (`mock.rs:113`), which is test scaffolding. **We report that as context for
the measurement, not as the finding**: what decides is the curve above, and the curve above is
measured to 3.77·10⁷ MACs and no further.

## 2 · Against the other two systems, same task, same machine, same campaign

The only comparisons this repository permits. binius64 at `log_inv_rate = 1`, 1 thread;
DeepProve at `ZKML_BIT_LEN = 8`, 1 thread; jolt-atlas padded, 1 thread.

**These three systems are not comparable on security and this table says so, per
`bench/README.md`: jolt-atlas and DeepProve both require a trusted setup and binius64 does
not, and only binius64 publishes a security parameter at all.** jolt-atlas and DeepProve are
both charged for a requantization/rebase that binius64 was not, and both prove 1.778× the
arithmetic at the 768-wide rungs because of power-of-two padding — which counts against their
`MAC/s` and in favour of their `bytes/MAC`.

| | | binius64 | DeepProve | **jolt-atlas** |
|---|---|---:|---:|---:|
| **T1-0** | prove | 179.0 ms | 977.7 ms | **65.0 ms** |
| 65 536 MACs | **B/MAC** (footprint) | 8 155 | 3 601 | **288.0** |
| | peak footprint | 0.53 GB | 0.24 GB | **0.018 GB** |
| | verify | 6.29 ms | ~20 ms (cold process) | **5.60 ms** |
| | proof | 345 536 B | 59 512 B (+ io + ctx) | **21 419 B** |
| **T1-a** | prove | 2 634.9 ms | 7 826.4 ms | **204.6 ms** |
| 589 824 MACs | **B/MAC** (footprint) | 12 335 | 2 997 | **131.9** |
| | peak footprint | 7.28 GB | 1.77 GB | **0.074 GB** |
| | verify | 68.34 ms | ~20 ms (cold process) | **22.06 ms** |
| | proof | 460 304 B | 116 404 B (+ io + ctx) | **23 435 B** |
| **T1-b** | prove | 19 519.5 ms | **not expressible** | **584.6 ms** |
| 2 359 296 MACs | **B/MAC** (footprint) | 11 684 | — | **89.3** |
| **T1-c** | prove | 396 245.7 ms † | **not expressible** | **2 163.3 ms** |
| 9 437 184 MACs | **B/MAC** (footprint) | 9 854 | — | **55.4** |
| **T1-d** | | **not run** | **not expressible** | **8 533.8 ms**, 51.6 B/MAC |

† **binius64's T1-c cell was swapping and its own file says so**: N = 3 with a
[183 588 – 432 675] ms spread and `(user+sys)/real` = 0.378, i.e. most of the wall clock was
waiting on memory rather than computing. **The T1-c row above is therefore not a like-for-like
speed comparison** and no ratio is quoted from it. It is included because
`bench/README.md` publishes the grid uncurated, and because *that* is the finding: at 9.4·10⁶
MACs binius64 needed 93 GB of footprint on a 32 GiB machine and jolt-atlas needed 0.50 GB.

**The honest summary of this table, with its conditions attached in the same sentence:** on the
two rungs where all three systems have a number, **jolt-atlas is 2.8–12.9× faster than binius64
and 15–38× faster than DeepProve, on 28–94× less memory per MAC than binius64 and 12–23× less
than DeepProve, with a proof 2.8–5.0× smaller than DeepProve's artifact and 16–20× smaller than
binius64's proof** — while requiring a trusted setup binius64 does not, publishing no security
parameter, and proving 1.778× the task's arithmetic at T1-a because of padding.

**These are same-task, same-machine, same-campaign figures.** No number here is compared with a
number from a different task size, and none is compared with any published figure from any
paper.

## 3 · Proof size is logarithmic, and that is the cleanest result in the file

| Task | MACs | proof bytes | Δ vs previous |
|---|---:|---:|---|
| T1-0 | 65 536 | 21 419 | — |
| T1-a | 589 824 | 23 435 | +2 016 B for 9× the MACs |
| T1-b | 2 359 296 | 25 307 | +1 872 B for 4× |
| T1-c | 9 437 184 | 27 179 | +1 872 B for 4× |
| T1-d | 37 748 736 | 29 051 | +1 872 B for 4× |

**Exactly +1 872 bytes per 4× the task, three times in a row.** Over a 576× range of task size
the proof grows 1.36×. binius64's proof over the same range goes 345 536 → 591 200 B and
DeepProve's artifact is not measured past T1-a.

**Part of that proof is not read by the verifier** — §5 measures 5.13 % of it at T1-0 — so the
figures above are the artifact jolt-atlas's own serializer produces, which is the thing a
deployment would transmit.

## 4 · Threads buy time. Threads do not buy memory. Third system, same shape.

`RAYON_NUM_THREADS` 1 → 4 → 10, padded:

| Task | MAC/s @1 | @4 | @10 | best speedup | B/MAC @1 | @4 | @10 | change |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| T1-a | 2 882 928 | 5 165 557 | 4 351 596 | **1.79× at 4** | 131.9 | 146.4 | 148.4 | **+12.5 %** |
| T1-c | 4 362 485 | 11 219 210 | 11 678 353 | **2.68× at 10** | 55.4 | 58.8 | 54.8 | **−1.0 %** |

Ten cores buy 1.8–2.7× of throughput and move peak memory per MAC by at most 12.5 %, in the
wrong direction. **binius64 and DeepProve showed the same asymmetry** — this is the third system
and the second protocol family to show it, which is the finding `bench/README.md` was built
around: wall-clock time responds to hardware, peak memory does not.

**T1-a is also non-monotone in threads**: 10 threads is *slower* than 4. `REPRODUCTION.md` §3.5
measures that effect on nanoGPT across six thread counts and names the mechanism the authors
themselves document.

(`(user+sys)/real` is 1.93–2.15 in the "1 thread" cells. `RAYON_NUM_THREADS=1` does not make
jolt-atlas single-threaded, `BUILD.md` §5, so that column is not the same condition as
binius64's 1-thread column and the speedups above are thread-setting ratios rather than
parallel-efficiency figures.)

## 5 · Correctness control

**It did not pass cleanly, and this section is written the way it is because of what DeepProve
taught us: verify the mechanism before writing the conclusion.**

`bench/README.md`: *"A corrupted trace must make `verify()` fail, in every system, on every
task."* jolt-atlas's licence forbids derivative works, so the control acts from outside, on the
public IO and on the serialized proof, and asks jolt-atlas's own `ONNXProof::verify` to judge.
Harness: [`ja_negative.rs`](../../scripts/jolt-atlas/harness/src/bin/ja_negative.rs). Raw:
[`bench/data/negative-jolt-atlas/`](../../data/negative-jolt-atlas/).

**Two positive controls first**, because a negative test that passes because nothing ever
verifies proves nothing:

| Control | Result |
|---|---|
| the honest proof verifies | **VERIFY_ACCEPTED**, all three tasks |
| serialize → deserialize → verify, unmodified | **VERIFY_ACCEPTED**, all three tasks — so the method itself does not corrupt |

### 5.1 Public input and public output: 21 of 21 rejected

| Family | What changes | T1-0 | T1-a | T1-b |
|---|---|---|---|---|
| `output_word` | one element of the claimed output `io.outputs` | 4/4 rejected | 4/4 | 4/4 |
| `input_word` | one element of the public input `io.inputs` | 3/3 rejected | 3/3 | 3/3 |

**Zero accepted.** The `input_word` family is worth naming: it is the surface jolt-atlas's own
commit `21729b8`, *"fix(soundness): bind public inputs into the transcript (#230)"*, exists to
protect, and at this commit it holds under every corruption we could apply from outside.

### 5.2 The serialized proof: an exhaustive sweep, and 5.13 % of the artifact is not read

Not a sample — **every one of T1-0's 21 419 bytes was flipped and re-verified**, because
`bench/README.md` forbids inferring an absence and because DeepProve's coarse pass had already
missed a whole accepted region once.

| Verdict | Count | % |
|---|---:|---:|
| VERIFY_REJECTED | 17 233 | 80.46 % |
| DESERIALIZE_REJECTED | 1 536 | 7.17 % |
| DESERIALIZE_PANIC | 1 237 | 5.78 % |
| **DESERIALIZE_ABORT** | **314** | **1.47 %** |
| **VERIFY_ACCEPTED** | **1 099** | **5.13 %** |

The accepted bytes are not scattered. They fall in **two sharply defined regions**, both mapped
by measurement:

**Region A — offsets 518–4 444: 20 runs of 50–51 bytes at a period of 204 bytes.** Every bit
pattern tried at these offsets is accepted (`0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
0xff` — 149 of 149 acceptances in the focused re-probe), so these are bytes that are **not read
at all**, not fields checked loosely.

**The mechanism is in jolt-atlas's own source and we did not have to guess it.** The verifier
loads the opening claims and **throws away the opening point that came with each one**:

```rust
for (key, (_, claim)) in &self.opening_claims.0 {
    verifier.accumulator.openings.insert(*key, (OpeningPoint::default(), *claim));
}
```
`jolt-atlas-core/src/onnx_proof/verifier.rs:69-73`

The `_` is the opening point. The prover serializes it, the artifact carries it, and the
verifier substitutes `OpeningPoint::default()`. The measured structure matches exactly: ~20
claim records, ~204 bytes each, of which ~50 bytes are the discarded point and the rest — the
key and the 32-byte claim — are checked.

**So this is a redundancy in the serialized format, not a soundness finding.** The claim values
themselves are read (`verifier.rs:70` inserts them, `verify_output_claim` compares one against
the IO) and every corruption of them was rejected.

**Region B — offsets 18 122–18 249: three runs of exactly 32 bytes, at a period of 48.** Thirty-two
bytes is one BN254 field element. This region sits far into the artifact, past the sumcheck
proofs and the commitments. **Its mechanism is NOT DETERMINED.** `sumcheck_claims` — the obvious
candidate by position — *is* read by the verifier (`verifier.rs:150,166), so a one-line
explanation like Region A's does not fit, and establishing what those three elements are would
mean reverse engineering the byte layout, which the licence forbids (`COMMIT` §3). **Reported
raw**, and it is the reason §3 says part of the proof is not read.

**Neither region would have been found by a sampled sweep.** The 124-offset coarse walk that
DeepProve was tested with hit Region A once, by luck, at a single offset, and **missed Region B
entirely**.

### 5.3 A robustness finding that is not about soundness

**314 single-bit flips make the process abort rather than return an error**, with

```
memory allocation of 6755399441057472 bytes failed
```

A corrupted length prefix reaches `Vec::with_capacity` and the allocator kills the process.
This is not a panic — `catch_unwind` cannot catch it — and it is why the sweep had to be driven
by a resuming loop
([`sweep-proof-bytes.sh`](../../scripts/jolt-atlas/sweep-proof-bytes.sh)). A further 1 237
flips panic inside arkworks' deserializer.

**In effect these are refusals: no invalid proof is accepted this way.** But a verifier that a
6.75-petabyte allocation can kill is a denial-of-service surface for anyone who verifies
attacker-supplied proofs, and it is reported because a third party should see it from us rather
than discover it.

### 5.4 Amendment A3 — nothing here is re-labelled, and the reason is the licence

`bench/TASKS.md` Amendment A3 (2026-08-24) re-labels witness-level corruptions on T2 and T3 as
weak evidence: up to 52.27 % of T2's weights are inert under ReLU, and a corruption that does
not change the output is not a test.

**A3 does not bite here.** The families in this control are `output_word`, `input_word` and
`proof_byte` — verified row by row in
[`t1-0.csv`](../../data/negative-jolt-atlas/t1-0.csv) — and there is no witness family, because
the licence forbids the derivative work one would need (`COMMIT` §3, `NOT_EXPRESSIBLE.md` §5).
**A3 states that artifact corruption is unaffected and remains the strong control**, so §5.2's
exhaustive sweep, its two mapped accepted regions and §5.1's 21-of-21 all stand exactly as
written. Nothing in this file's verdict moves.

**And T2 and T3 never ran anyway** (`NOT_EXPRESSIBLE.md` §1), so the two tasks A3 is about have
no jolt-atlas cell of any kind. What ran is five rungs of a matmul ladder, with no activations
and therefore no inert weights.

**The gap A3 exposes by contrast.** binius64 loses six rows of weight-binding evidence and
keeps the rest; jolt-atlas had none to lose, and under A2 its weights are committed at
preprocessing rather than witnessed — a different binding again. **This entry carries no
evidence, weak or strong, that a perturbed weight is detected.** That column belongs in
`bench/RESULTS.md`, not here.

### 5.5 Verdict

**The proof body passes; part of the artifact is unread and part of the parser is fragile.**
Every corruption of the public input, the public output, the sumcheck proofs, the commitments
and the claim values was rejected — 17 233 rejections with no exceptions outside the two mapped
regions. What the sweep also found is a prover-written field the verifier discards, a second
32-byte-granular region we could not identify, and a parser that a length prefix can abort.

`bench/README.md` says *"Systems that do not pass it are not reported."* We report jolt-atlas
anyway and say why rather than quietly relaxing the rule: the control's purpose is to establish
that the numbers describe real proofs rather than computations that happen to produce bytes,
and on that question it **passes**. The rest is artifact-format and parser robustness, and no
timing figure depends on it. **Both are published.**

**Right of reply applies with priority here** ([`CHALLENGE.md`](../../CHALLENGE.md)), and
specifically on Region B, which we could not identify and would rather be told about than guess
at.

## 6 · What contaminates these numbers, declared

1. **The published reference does not describe the measured tree.** It reproduces at
   `53b7c873` and is 5.3× off at `434ab99` (`REPRODUCTION.md` §3). The fairness check
   discharges for the commit that published it and not for the commit measured.
2. **GPT-2 could not be loaded at all**, because jolt-atlas's own export script pins no
   exporter version (`REPRODUCTION.md` §4). No GPT-2 figure of ours exists.
3. **The machine is not dedicated.** 12 days uptime, 7.66 GB of swap committed at campaign
   start, browser and desktop applications throughout. Same machine as the binius64 and
   DeepProve cells, and using a different one would break comparability.
4. **`prove` includes graph tracing** and cannot be split on this path. The figures are an
   **upper bound** on proving time.
5. **jolt-atlas does more work than the task asks** at the 768-wide rungs: 1.778× the
   arithmetic from padding that cannot be switched off. That makes its `MAC/s` look worse and
   its `bytes/MAC` look better than a like-for-like expression would. On the padded basis T1-a
   (1 048 576 MACs actually proved) reads 5 125 205 MAC/s and 74.22 B/MAC — **not benchmark figures**, printed so the size of the
   effect is visible rather than hidden inside the first row.
6. **`RAYON_NUM_THREADS=1` is not one thread** — measured `(user+sys)/real` of 1.93–2.15 — and
   the thread response is non-monotone.
7. **Two of seven grid rows did not run**, both on a dense layer with a single output
   (`NOT_EXPRESSIBLE.md` §1). **T2 and T3 are the only two tasks in this benchmark that
   represent a whole model rather than a tile**, so what is measured here is five rungs of a
   matmul ladder and nothing else.
8. **Security bits are NOT DETERMINED**, and a trusted setup is required. Neither is
   normalizable against binius64.
9. **Dory was not measured**, and the streaming commitment interface in the tree has only a
   mock implementation. Neither fact is a measurement of memory; §1's curve is.
10. **Three of our own expressions were wrong before this one was right**
    (`EXPRESSION.md` §5), each producing an error that looked like a limit of jolt-atlas. The
    check that caught them was running jolt-atlas's own bundled models through our harness. Any
    remaining expression error would look the same, and we cannot rule one out.
