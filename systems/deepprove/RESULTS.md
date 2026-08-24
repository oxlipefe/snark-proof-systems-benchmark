# DeepProve — results

**Read [`REPRODUCTION.md`](REPRODUCTION.md) first.** `bench/README.md`'s fairness protocol
requires it: we could not reproduce DeepProve's published GPT-2 numbers, and that discrepancy
is published *above* these figures rather than in a footnote.

Then [`BUILD.md`](BUILD.md) for the build and its integrity check,
[`EXPRESSION.md`](EXPRESSION.md) for how each task was written and what DeepProve does to it
before proving, and [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) for the five of seven grid rows
that did not run.

**Scope: this file reports one system, and it reports two rungs of it.** Five of the seven
tasks did not run. What follows is a narrow measurement, and its narrowness is the first
result.

---

## Conditions line

Applies to every figure below. Where a cell differs, the cell's row says so.

```
system      DeepProve (Lagrange Labs)
commit      9d1a53e2ef49ffa2c902b8689cd3c58057a4e662, version 2.0.1, tree pristine
licence     Lagrange License — NOT OSI. Internals NOT instrumented; see COMMIT.
instrument  deep-prove-worker one-shot (their binary), measured from outside the process
field       BN254 scalar field; PCS = HyperKZG; transcript = Blake3
protocol    sumcheck + LogUp-GKR lookups
security    NOT DETERMINED — no security-bit, soundness-bit or query-count parameter is
            exposed on this path or stated in the documentation. binius64 holds
            SECURITY_BITS = 96 constant across its rate sweep and publishes it; there is no
            comparable number to put here, so none is invented.
trusted setup   YES. HyperKZG is a pairing-based KZG variant over BN254 and needs a
                structured reference string: `HyperKZGSRS::setup(&mut rng, max_degree)`
                builds the powers of g from a random tau, in process, per run
                (dp-crypto/src/arkyper/mod.rs:50-78). binius64 requires no setup at all.
                THIS DIFFERENCE IS NOT NORMALIZABLE AND IS NOT AVERAGED AWAY: a system whose
                soundness rests on discarded toxic waste and a post-quantum hash-based system
                are not comparable on security even when their milliseconds are. It is stated
                here, in the same block as every figure, and not in a footnote.
ZK              no
quantization    ZKML_BIT_LEN=8 (= INT8, the task's domain) for the primary cells;
                a control at 12 (DeepProve's default, and the basis of its published numbers)
requantization  YES, after every linear layer, NOT DISABLEABLE — a deviation from
                bench/TASKS.md T1's "not requantized" rule and from Amendment A1.
                EXPRESSION.md §5.
weights         PREPROCESSED (bench/TASKS.md Amendment A2). The weight matrix is an ONNX
                initializer, not a graph input: EXPRESSION.md §3, "`A` is the graph input —
                the witness — and `B` is an initializer, so the weights are committed at setup
                and the input is what varies per proof."
weight cost     SETUP. It lands inside the `setup` column — context generation — which this
                benchmark reports apart and NEVER amortizes into prove time. **So the weight
                cost is excluded by construction from both derived metrics below**, unlike
                binius64 and Ceno where it is inside them. That exclusion is not a correction
                this file applies; it is a difference this file declares. A2 §3: a `bytes/MAC`
                from a `preprocessed` system and one from a `witness` system are not the same
                quantity. The setup column is large and is in §7: 1 190.8 ms at T1-0 and
                18 125.4 ms at T1-a, against proofs of 977.7 ms and 7 826.4 ms.
padding         every dimension rounded UP to the next power of two, not disableable.
                T1-a's 768 becomes 1024, so 1.778x the task's arithmetic. EXPRESSION.md §4.
batching        none — batch_size is pinned to 1. NOT_EXPRESSIBLE.md §3.
threads     RAYON_NUM_THREADS in {1, 10}; NOT full thread control, see BUILD.md §5.
            The sumcheck rounds 10 down to 8 ("thread size 10 is not power of 2").
machine     Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, NOT dedicated
OS          macOS 26.5.2 (25F84), Darwin 25.5.0, uptime 12 days, 7.9-9.4 GB swap committed
N           5 timed repetitions after 1 warmup, in one process, except where stated
date        2026-08-23 / 2026-08-24
```

## What is inside each measured quantity

Stated up front because two of these brackets are **wider than binius64's**, and comparing
them without saying so would be dishonest.

| Column | What it contains | Same bracket as binius64? |
|---|---|---|
| `prove` | per repetition, between consecutive `Running input` markers: `model.reset()`, tensor-store scoping, `load_input_flat`, **quantized inference**, and `Prover::prove` | **No — it includes inference.** DeepProve's own LLM benchmark separates `inference_time` from `prove_full`; the ONNX worker emits no marker between them, so the split is **NOT DETERMINED** |
| `peak RSS` / `peak footprint` | `/usr/bin/time -l` over the whole process: model load, quantization, context generation, all repetitions | Yes — binius64's peaks are also whole-process |
| `setup` | `Generating proving and verifier contexts` → `Stored generated proving parameters`. Reported apart, **never amortized into prove** | Yes |
| `verify` | whole process, `deep-prove-cli verify <file>`: file read, base64 decode, deserialization of proof + IO + verifier context, and verification | **No — binius64's column times one call in a warm process.** Ours is a cold whole process |
| `artifact` | the bytes the public CLI writes: `Output { outputs, proof: Provable { proof, io, ctx } }` | **No — it carries the verifier context too.** It is an **upper bound on proof size**, not a proof size |

Separating the wider brackets would require instrumenting DeepProve's internals or reverse
engineering its serialization. Its licence permits neither. **The holes are reported, not
estimated** — `NOT_EXPRESSIBLE.md` §6 lists them all.

## The full grid

Every cell that was run, uncurated, including the ones that failed. Raw per-cell data:
[`bench/data/cells-deepprove.csv`](../../data/cells-deepprove.csv) and
[`bench/data/cells-deepprove/`](../../data/cells-deepprove/); derived table:
[`bench/data/results-deepprove.csv`](../../data/results-deepprove.csv).

| Task | MACs (TASKS.md) | bit len | RAYON thr | N | status | prove ms (median) | [min–max] | verify ms | artifact B | setup ms | peak RSS GB | peak footprint GB | (u+s)/real | **MAC/s** | **B/MAC footprint** | **B/MAC RSS** |
|---|---:|---:|---:|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 8 | 1 | 5 | PROVED_NOWRITE | **977.7** | [962.0–994.0] | 20 | 59 512 | 1 190.8 | 0.255 | 0.236 | 1.8292 | **67 030** | **3 601** | **3 886** |
| T1-0 | 65 536 | 8 | 10 | 5 | PROVED_NOWRITE | **398.9** | [388.7–401.4] | 20 | 59 512 | 181.1 | 0.226 | 0.207 | 5.8146 | **164 300** | **3 163** | **3 444** |
| T1-0 | 65 536 | **12** | 1 | 5 | PROVED_NOWRITE | **976.8** | [968.6–986.8] | — | — | 1 209.7 | 0.203 | 0.185 | 1.8205 | **67 095** | **2 822** | **3 105** |
| T1-a | 589 824 | 8 | 1 | 5 | PROVED_NOWRITE | **7 826.4** | [7 709.3–7 891.1] | 20 | 116 404 | 18 125.4 | 1.947 | 1.768 | 1.7053 | **75 363** | **2 997** | **3 301** |
| T1-a | 589 824 | 8 | 10 | 5 | PROVED_NOWRITE | **2 660.2** | [2 611.4–2 719.7] | 20 | 116 404 | 2 410.1 | 1.857 | 1.719 | 5.1149 | **221 722** | **2 914** | **3 149** |
| T1-b | 2 359 296 | 8 | 1 | 0 | **FAIL_parse** | — | — | — | — | — | — | — | — | — | — | — |
| T1-c | 9 437 184 | 8 | 1 | 0 | **FAIL_parse** | — | — | — | — | — | — | — | — | — | — | — |
| T1-d | 37 748 736 | 8 | 1 | 0 | **FAIL_parse** | — | — | — | — | — | — | — | — | — | — | — |
| T2 | 92 224 | 8 | 1 | 0 | **FAIL_prove** | — | — | — | — | 1 225.3 | — | — | — | — | — | — |
| T2 | 92 224 | 8 | 10 | 0 | **FAIL_prove** | — | — | — | — | — | — | — | — | — | — | — |
| T2 | 92 224 | **12** | 1 | 0 | **FAIL_prove** | — | — | — | — | — | — | — | — | — | — | — |
| T3 (as 8 proofs) | 737 792 | 8 | 1 | 0 | **FAIL_prove** | — | — | — | — | — | — | — | — | — | — | — |
| T3 (as 8 proofs) | 737 792 | 8 | 10 | 0 | **FAIL_prove** | — | — | — | — | — | — | — | — | — | — | — |
| T3 (one proof) | 737 792 | 8 | 1 | 0 | **FAIL_parse** | — | — | — | — | — | — | — | — | — | — | — |

Status values. `PROVED_NOWRITE` — proved correctly, then the binary failed writing the proof
to disk (`serde_json`, non-string map keys, `BUILD.md` §3); the failure is after
`Proving done.`, so timings and peaks are unaffected and the cell is reported rather than
discarded. `FAIL_parse` — the ONNX frontend rejected the model in 0.01–0.02 s;
`NOT_EXPRESSIBLE.md` §1, §3. `FAIL_prove` — the model built and the prover refused it;
`NOT_EXPRESSIBLE.md` §2.

**`FAIL_parse` and `FAIL_prove` are labels derived here, not values from the ledger.** The raw
ledger records `FAIL_rc1` for all six of those cells, because that is all the exit code says.
The split between them is read off each cell's own `log.txt` and is reproducible from it. The
raw value is in [`cells-deepprove.csv`](../../data/cells-deepprove.csv); the derived one is in
this table, and now in both places rather than silently replaced.

**Derived columns are empty for every cell that produced no proof.** Those processes still had
a memory peak — the peak of loading a model and then failing — and dividing it by a MAC count
the system never performed would manufacture a number out of a crash.

**`verify ms` reads 0.02 s at both rungs**, which is the **resolution floor of the
instrument** (`/usr/bin/time` reports hundredths). The only claim it supports is that verify
did **not grow measurably** from T1-0 to T1-a. It is not a claim that verify takes exactly
20 ms. Raw: [`bench/data/verify-deepprove/verify.csv`](../../data/verify-deepprove/verify.csv).

## Correctness control

**It did not pass cleanly, and this is the most consequential thing in this file.**

`bench/README.md`: *"A corrupted trace must make `verify()` fail, in every system, on every
task."* For binius64 that control mutates a private witness word inside the prover. DeepProve's
licence forbids derivative works, so here the control acts on the **serialized artifact** from
outside and asks DeepProve's own `deep-prove-cli verify` to judge it — covering binius64's
`proof_byte` family and not its `private_word` family.

**Two positive controls first**, because a negative test that passes because nothing ever
verifies proves nothing:

| Control | Result |
|---|---|
| the honest artifact verifies | **VERIFY_ACCEPTED**, both tasks |
| an unmodified decode→re-encode round trip verifies | **VERIFY_ACCEPTED**, both tasks — so the method itself does not corrupt. Raw: [`accepted-bits.csv`](../../data/negative-deepprove/accepted-bits.csv), rows `pattern = none (round-trip control)` |

**Then a systematic walk: 107 single-bit corruptions per artifact, 214 in all** — a fine
sweep over the head, a coarse sweep from 5% to 95%, and every byte of the last 32. (An earlier
coarse pass of 11 offsets per artifact, in
[`negative-control.csv`](../../data/negative-deepprove/negative-control.csv), found the same
thing and is what prompted the walk.) Results
([`accepted-region.csv`](../../data/negative-deepprove/accepted-region.csv)):

| Task | artifact bytes | offsets probed | rejected | **accepted** |
|---|---:|---:|---:|---:|
| T1-0 | 59 512 | 107 | 96 | **11** |
| T1-a | 116 404 | 107 | 80 | **27** |

**A corrupted artifact was accepted as valid, 38 times.** The accepted offsets are not
scattered — they fall in two sharply defined places, and both were mapped by measurement.

### Region 1 — a prefix whose size scales exactly with the declared output

**This region is a duplicate copy of the model output that the verifier is never handed.** The
detail is below; the short version is that it is an artifact-format defect, not a soundness
one.

| Task | model output elements | largest accepted head offset | rejected from |
|---|---:|---:|---:|
| T1-0 | 256 | **448** | 512 |
| T1-a | 768 | **1 472** | 1 536 |

`1536 / 512 = 3` and `768 / 256 = 3`. **The accepted prefix scales exactly with the number of
output elements**, at ~2 bytes per element.

**The artifact carries the model output twice, and only one copy is verified.** From the
struct declarations — read from the source, not from a binary format recovered by reverse
engineering:

```rust
pub struct Output { pub outputs: Vec<Tensor<Element>>, pub proof: Provable }   // v1.rs:41-46
pub struct Provable { pub proof: ZkmlProof<F, Pcs>, pub io: IO<F>, pub ctx: VerifierContext<F, Pcs> }
                                                                              // v2.rs:14-19
pub struct IO<F: PrimeField> {
    pub input:  Vec<Tensor<SerializableField<F>>>,
    pub output: Vec<Tensor<SerializableField<F>>>,     // zkml/src/iop/verifier.rs:68-75
    ...
}
impl Provable {
    pub fn verify(self) -> anyhow::Result<()> { verify::<_, T, _>(&self.ctx, self.proof, self.io) }
}                                                                             // v2.rs:20-24
```

- `Provable.io.output` — the output as field elements — **is** an argument to the verifier, and
  the sweep confirms that everything after the head prefix is protected: **every probed offset
  between the head boundary and n−29 was rejected**, in both artifacts, with no exceptions.
- `Output.outputs` — the same values as integers, at the head of the file — is **not** an
  argument to the verifier, and is the region measured above.

**So the finding is narrower than "the outputs are not checked", and narrower than the first
reading of this data suggested.** It is: the artifact contains a **redundant, unverified copy**
of the model output, sitting in front of the verified one, and `deep-prove-cli verify` does
not cross-check the two. A consumer who reads `Output.outputs` as "the answer this proof
attests" gets a value the verifier never looked at, while the same information in
`Provable.io.output` is bound to the proof.

**This is a property of the CLI's artifact format, not of the proof system, and nothing here
supports a soundness claim against DeepProve.** Every corruption inside the proof, the IO and
the context was rejected.

### Region 2 — three offsets at fixed distances from the end

Accepted at **n−29, n−15 and n−1** in *both* artifacts. All three hold the byte `0x03`.

A byte that is never read would accept *every* mutation; a field that is checked loosely
accepts some and rejects others. Those are different findings, so each of the three offsets
was re-probed with five bit patterns
([`probe-accepted-bits.py`](../../scripts/deepprove/probe-accepted-bits.py), raw:
[`accepted-bits.csv`](../../data/negative-deepprove/accepted-bits.csv)). **All six offsets —
three per task, two tasks — behave identically:**

| flip | outcome |
|---|---|
| `^0x01` | **VERIFY_ACCEPTED** |
| `^0x02` | **VERIFY_ACCEPTED** |
| `^0x08` | VERIFY_REJECTED |
| `^0x80` | DESERIALIZE_REJECTED |
| `^0xff` | DESERIALIZE_REJECTED |

So these are **not ignored bytes**: they are small-valued fields that are read, and whose low
two bits do not change the verdict while bit 3 does. **What those bytes are is NOT
DETERMINED.** Establishing it would mean reverse engineering the serialization format, which
the licence forbids. Reported raw.

### Amendment A3 — nothing here is re-labelled, and the reason is the licence

`bench/TASKS.md` Amendment A3 (2026-08-24) re-labels witness-level corruptions on T2 and T3 as
weak evidence: up to 52.27 % of T2's weights are inert under ReLU, and a corruption that does
not change the output is not a test.

**A3 does not bite here.** This control has no witness family to re-label — every one of the
214 corruptions is an offset in the serialized artifact, verified row by row in
[`negative-control.csv`](../../data/negative-deepprove/negative-control.csv) and
[`accepted-region.csv`](../../data/negative-deepprove/accepted-region.csv). **A3 states that
artifact corruption is unaffected and remains the strong control**, so both the pass on the
proof body and the 38 acceptances in the two mapped wrapper regions stand exactly as written
above. Nothing in this file's verdict moves.

**And T2 and T3 never ran anyway** (`NOT_EXPRESSIBLE.md` §2), so the two tasks A3 is about have
no DeepProve cell of any kind. The rungs that did run are T1-0 and T1-a — pure matmuls, no
activations, no inert weights.

**The gap A3 exposes by contrast.** binius64 loses six rows of weight-binding evidence and
keeps the rest; DeepProve had none to lose. Its licence forbids the derivative work that a
witness-level control would require (§ above, `NOT_EXPRESSIBLE.md` §6), so **this entry carries
no evidence, weak or strong, that a perturbed weight is detected** — and under A2 the weights
are committed at setup rather than witnessed, which is a different binding again. That column
belongs in `bench/RESULTS.md`, not here.

### Verdict on the control

**The proof body passes; the artifact wrapper does not.** Every corruption between the head
prefix and the last 29 bytes was rejected, in both artifacts, with no exceptions — that range
covers the proof, the verified IO and the verifier context. The failures are confined to the
**unverified duplicate** of the model output at the head and to three bytes at the tail.

`bench/README.md` says *"Systems that do not pass it are not reported."* We report DeepProve
anyway, and say why rather than quietly relaxing the rule: the control's purpose is to
establish that the numbers describe real proofs rather than computations that happen to
produce bytes, and on that question it **passes** — the proofs verify, and mutating them makes
verification fail. What it also found is a separate defect in the artifact format that the
control was not designed to look for and that no timing figure depends on. **Both are
published.** If the benchmark's own maintainers judge that this should have excluded DeepProve
from reporting, the raw data to make that call is in `bench/data/negative-deepprove/`.

**Right of reply applies with priority here** ([`CHALLENGE.md`](../../CHALLENGE.md)).

## What the numbers say

### 1 · The ladder stops after two rungs, and that is the headline

T1 spans three orders of magnitude to find where each system breaks. **DeepProve's ONNX
frontend breaks at the second rung**, not on memory or time but on shape: it does
matrix-**vector** products, so `[4×768]·[768×768]` is rejected in 0.01 s
(`NOT_EXPRESSIBLE.md` §1). T2 and T3 break on a different wall — a final layer narrower than
4 outputs cannot be proved (`NOT_EXPRESSIBLE.md` §2).

**Measured range: 65 536 to 589 824 MACs.** [`bench/CHALLENGE.md`](../../CHALLENGE.md)
forbids extrapolating outside the measured range, and that rule binds hard here: **nothing below is projected past
589 824 MACs**, which is 1.6% of the ladder's top rung.

### 2 · Against binius64, on the same machine and the same task

The only two cells where both systems have a number. binius64 at `log_inv_rate = 1`;
DeepProve at `ZKML_BIT_LEN=8`.

**These two systems are not comparable on security and the table says so, per
`bench/README.md`: DeepProve requires a trusted setup (HyperKZG) and binius64 does not.**
DeepProve is also charged with a requantization lookup per linear layer that binius64 was not,
and proves 1.778× the arithmetic at T1-a because of power-of-two padding — both in its favour
on `bytes/MAC` and against it on `MAC/s`.

| | | binius64 | DeepProve | ratio |
|---|---|---:|---:|---|
| **T1-0**, 1 thread | prove | 179.0 ms | 977.7 ms | DeepProve **5.46× slower** |
| | **B/MAC** (footprint) | 8 155 | **3 601** | DeepProve **2.26× less** |
| | peak footprint | 0.53 GB | 0.24 GB | |
| | verify | 6.29 ms | ~20 ms (whole process) | brackets differ |
| **T1-a**, 1 thread | prove | 2 634.9 ms | 7 826.4 ms | DeepProve **2.97× slower** |
| | **B/MAC** (footprint) | 12 335 | **2 997** | DeepProve **4.12× less** |
| | peak footprint | 7.28 GB | 1.77 GB | |
| | verify | 68.34 ms | ~20 ms (whole process) | |
| | proof / artifact | 460 304 B (proof only) | 116 404 B (proof **+ io + ctx**) | DeepProve **3.95× smaller** |
| **T1-a**, 10 threads | prove | 710.6 ms | 2 660.2 ms | DeepProve **3.74× slower** |
| | **B/MAC** (footprint) | 12 295 | **2 914** | DeepProve **4.22× less** |

**The cost shapes are genuinely different, and in opposite directions.** DeepProve is 3–5×
slower per MAC and uses 2–4× less memory per MAC, and it ships an artifact that includes its
verifier context and is still 4× smaller than binius64's proof alone. That is exactly the kind
of result `bench/README.md` says the benchmark exists to produce: a map of cost shapes, not a
ranking.

### 3 · Verify does not grow. binius64's does.

| Task | constraints/MACs | binius64 verify | DeepProve verify |
|---|---|---:|---:|
| T1-0 | 65 536 MACs | 6.29 ms | 0.02 s |
| T1-a | 589 824 MACs | 68.34 ms | 0.02 s |
| growth | 9× the MACs | **10.9×** | **not measurable** |

binius64's verifier is linear in constraint count — its own `RESULTS.md` §4 establishes that
by decomposition, and its authors document the missing feature that would fix it. DeepProve's
did not move across a 9× circuit.

**This is a weak measurement stated as one.** DeepProve's figure is a whole cold process at
10 ms resolution, so a real growth of a few milliseconds would be invisible. Over 9× it says
only: **no growth large enough for this instrument to see.** Two rungs is not a trend, and a
decomposition like the one that settled the binius64 case is not available here because the
licence forbids instrumenting the internals.

### 4 · Threads buy time. Threads do not buy memory. Again.

`RAYON_NUM_THREADS` 1 → 10, at `ZKML_BIT_LEN=8`:

| Task | MAC/s, 1 | MAC/s, 10 | speedup | B/MAC, 1 | B/MAC, 10 | change |
|---|---:|---:|---:|---:|---:|---|
| T1-0 | 67 030 | 164 300 | 2.45× | 3 601 | 3 163 | **−12.2%** |
| T1-a | 75 363 | 221 722 | 2.94× | 2 997 | 2 914 | **−2.8%** |

Same asymmetry binius64 showed: ten cores buy 2.45–2.94× of throughput and move peak memory per
MAC by ≤12%. **Wall-clock time responds to hardware; peak memory does not.** Two systems, two
protocol families, same shape — which is the finding `bench/README.md` was built around.

(`(user+sys)/real` is 5.1–5.8 in the 10-thread cells and **1.7–1.8 in the 1-thread cells** —
`RAYON_NUM_THREADS=1` does not make DeepProve single-threaded, `BUILD.md` §5. So the "1
thread" column is not the same condition as binius64's, and the speedups above are
thread-setting ratios rather than parallel-efficiency figures.)

### 5 · Quantization width barely moved prove time, and moved memory the wrong way

T1-0, 1 thread, `ZKML_BIT_LEN` 8 vs 12 — DeepProve's default and the basis of its published
numbers:

| | 8-bit | 12-bit | change |
|---|---:|---:|---|
| prove | 977.7 ms | 976.8 ms | **−0.1%** |
| peak footprint | 0.236 GB | 0.185 GB | **−21.6%** |
| B/MAC | 3 601 | 2 822 | −21.6% |

**Widening the quantization from 8 to 12 bits cost no measurable time and used 22% *less*
peak memory.** That is the opposite of the naive expectation and we do not explain it. The
lookup tables DeepProve builds are sized by bit width, so table sizing is a candidate; **cause
NOT ESTABLISHED**, and no figure here depends on it. It is reported so a third party sees it
rather than discovers it. N = 5 on both cells, dispersion under 2%.

### 6 · Padding: what DeepProve actually proved at T1-a

`bench/TASKS.md` fixes the denominator, so every figure above is charged against the published
MAC count. DeepProve padded 768 → 1024 and therefore proved **1 048 576** MACs, 1.778× the task.

| basis | MAC/s | B/MAC footprint |
|---|---:|---:|
| **published MACs (the benchmark metric)** | **75 363** | **2 997** |
| padded MACs actually proved | 133 979 | 1 686 |

The second row is **not a benchmark figure** and is not used in any comparison. It is here so
that a reader can see the size of the effect rather than have it hidden inside the first row.

### 7 · Setup is large, and it is reported apart

Never amortized into prove time, per `bench/README.md`.

| Task | setup (context generation) | prove, 1 rep | setup / prove |
|---|---:|---:|---:|
| T1-0 | 1 190.8 ms | 977.7 ms | 1.22× |
| T1-a | 18 125.4 ms | 7 826.4 ms | **2.32×** |

Context generation costs more than a proof, and grows faster: **15.2× for 9× the MACs**, while
prove grows 8.0×. It is a one-off per model — DeepProve is explicitly designed to amortize it
(`zkml/README.md` §"Pre-process the model once", `--save_params` / `--load_params`) — so this
is not a per-proof cost. It is reported because a deployment pays it once per model and
`bench/README.md` requires setup to be visible rather than folded in.

There is a further **0.95–15.5 s** between the end of context generation and the first proof,
which is the worker storing the contexts and reading them back through its parameter store
(`deep-prove/src/bin/worker/main.rs:112-140`). It is in
`results-deepprove.csv` as `ctx_store_roundtrip_s`, and it is an artifact of the worker's
store round-trip rather than a property of the proof system.

## The `0.686 bytes/MAC` figure that motivates this repository is not what we measured

`bench/README.md` opens by contrasting our prover's 6 268–7 932 B/MAC with *"a published
system solving the same task on the same model"* at **0.686 B/MAC**, a gap of roughly **10⁴×**,
and says that gap *"decides more about what is buildable than any throughput number"*. That
published system is DeepProve, and the figure is derived from its paper's GPT-2 evaluation.

**Measured, on the same machine and the same tasks, DeepProve reads 2 914–3 601 B/MAC.**
Against binius64's 8 155–12 335 on those same two rungs, the measured gap is **2.3–4.2×, not
10⁴×.**

The two numbers are not in contradiction, and this file does not claim the 0.686 figure is
wrong. They are **different regimes**:

- 0.686 B/MAC comes from a **GPT-2 forward pass**, on the order of 10¹¹ MACs.
- These cells are 6.6·10⁴ and 5.9·10⁵ MACs — **five to six orders of magnitude smaller.**
- And DeepProve's `bytes/MAC` is **not flat**: it falls from 3 601 to 2 997 (−17%) as the task
  grows 9×. Fixed per-proof overhead is being amortized, in the direction that would keep
  reducing it at larger sizes.

**We refuse to extrapolate across five orders of magnitude**, and
[`bench/CHALLENGE.md`](../../CHALLENGE.md)'s own rule forbids it: *"We did that once, it cost us a strategic decision, and the rule now is
absolute."* So this file does **not** say whether 0.686 B/MAC is reachable. It says:

> At 10⁴–10⁵ MACs, DeepProve's measured memory per MAC is ~3·10³ bytes, and its advantage over
> binius64 on the same tasks is a factor of 2–4, not a factor of 10⁴.

**The 10⁴ framing in `bench/README.md` compares figures from workloads 10⁶ apart in size**, and
whoever maintains that file should decide what to do about it. This file does not edit it; it
records the measurement that bears on it. **The gate that would settle it — measuring DeepProve
at 10⁸ MACs or more — was not reachable here**, because its ONNX frontend stops at
589 824 MACs (`NOT_EXPRESSIBLE.md` §1) and its GPT-2 benchmark path does not produce a
verifying proof on this machine (`REPRODUCTION.md`).

## What contaminates these numbers, declared

1. **The reproduction failed.** DeepProve's own published GPT-2 numbers could not be
   reproduced in four configurations (`REPRODUCTION.md`). The build passes the authors' own
   204/204 non-LLM tests and 11/11 GPT-2-related tests (4 of them proving tests), which is why
   these figures are published at all — but the fairness protocol's primary check did not discharge.
2. **The machine is not dedicated.** 12 days uptime, 7.9–9.4 GB of swap committed at cell
   start, browser and desktop applications throughout. Same machine as binius64's cells, and
   using a different one would break comparability.
3. **`prove` includes quantized inference** and cannot be split on this path. The figures are
   therefore an **upper bound** on DeepProve's proving time.
4. **`artifact` includes the verifier context** and is an **upper bound** on proof size. A
   proof-only figure comparable to binius64's is not obtainable through the public CLI.
5. **`verify` is a whole cold process at 10 ms resolution**, not a warm in-process call like
   binius64's column.
6. **DeepProve does more work than the task asks**: a requantization lookup per linear layer
   that no configuration removes, and 1.778× the arithmetic at T1-a from power-of-two padding.
   Both make its `MAC/s` look worse and its `bytes/MAC` look better than a like-for-like
   expression would.
7. **`RAYON_NUM_THREADS=1` is not one thread** — measured `(user+sys)/real` of 1.7–1.8.
8. **Two rungs.** Five of seven grid rows did not run. Nothing is extrapolated past
   589 824 MACs.
9. **No internal decomposition.** Where DeepProve's cost goes inside the prover is NOT
   DETERMINED; its licence forbids instrumenting it. binius64 got that treatment and DeepProve
   did not, and **this asymmetry favours neither system's numbers — it just means we know less
   about one of them.**

## Build integrity held for the campaign

The build passed DeepProve's own test suite before the figures were taken: **204 of 204
runnable non-LLM tests**, plus **11 of 11 GPT-2-related tests — 4 of which invoke the prover** —
with `RUST_MIN_STACK` raised.
The single initial failure was a stale CSV fixture left by our own earlier run, disclosed in
[`BUILD.md`](BUILD.md) §2. Every cell passed the sleep check; the largest reading in the campaign was `slept = 0.001 s`, against a 2 s threshold.
