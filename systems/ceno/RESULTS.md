# ceno — results

**Read [`REPRODUCTION.md`](REPRODUCTION.md) first.** Ceno publishes no CPU reference figure at
any commit; the only published number is a GPU one, and round one of this benchmark is CPU-only.
Nothing below was validated against a figure of the authors' own.

Then read [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1: **the 1-thread cut that is binius64's
primary cannot be measured for Ceno**, because the prover aborts there on Ceno's own examples.
Every cross-system comparison in this file is between a 1-thread circuit prover and a
≥2-thread zkVM.

Then [`BUILD.md`](BUILD.md) §2 (the tree does not build on aarch64 without a one-word patch),
§5 (`cargo ceno verify` cannot succeed at this commit), and §8 (**this machine was in the worst
state of any campaign in this repository**: not dedicated, on battery, 778 MB of free swap,
boot volume 95 % full).

And [`EXPRESSION.md`](EXPRESSION.md) §1, which governs every number here.

---

## THE DECLARATION THAT GOVERNS EVERY FIGURE BELOW

**Ceno is a RISC-V zkVM. It proves the execution of instructions, not the arithmetic those
instructions compute.** The other three systems in this benchmark are circuit-based: given a
task, their cost is set by its arithmetic. Ceno's cost is set by its instruction count.

Measured on Ceno's own emulator, exactly, for the instances proved here:

| Task | MACs (frozen) | RISC-V instructions | cycles | instructions/MAC | cycles/MAC |
|---|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 800 913 | 3 203 656 | 12.22 | 48.88 |
| T1-a | 589 824 | 6 316 368 | 25 265 476 | 10.71 | 42.84 |
| T1-b | 2 359 296 | 25 245 947 | 100 983 792 | 10.70 | 42.80 |
| T1-c | 9 437 184 | 100 947 798 | 403 791 196 | 10.70 | 42.79 |
| T1-d | 37 748 736 | 403 755 202 | 1 615 020 812 | 10.70 | 42.78 |
| T2 | 92 224 | 1 124 867 | 4 499 472 | 12.20 | 48.79 |
| T3 | 737 792 | 8 876 478 | 35 505 916 | 12.03 | 48.12 |

**So a task the benchmark denominates at 65 536 multiply-accumulates costs Ceno 800 913 proved
instructions and 3 203 656 proved cycles.** `bench/TASKS.md` freezes the MAC count as the
denominator and it is not recomputed — but `B/MAC` and `MAC/s` for a zkVM are therefore ratios
against a denominator that describes the *task*, not the *work*. Both a per-MAC and a per-cycle
column are published below for exactly this reason, and **any ratio taken against a
circuit-based system in this benchmark carries this paragraph in the same breath.** §7 states
our position on whether such a ratio should be quoted at all.

## Conditions line

Applies to every figure below. Where a cell differs, the cell says so in its own row.

```
system      ceno
commit      ac164255081d0b4dc58d3559c4c7331afd7af7e6 (+ a one-word dependency-feature patch
            required to build on aarch64 at all; COMMIT)
task        expressed as RISC-V programs; MACs = the count frozen in bench/TASKS.md, asserted
            by the generator against the reference computation, never recomputed
constraints NOT COMPARABLE to a circuit's constraint count. Ceno's proved object is a
            multi-chip execution trace; the per-opcode instance counts are in each cell's
            stdout.txt. Instructions and cycles are reported instead, per task, above
field       BabyBear (p = 2^31 - 2^27 + 1), degree-4 extension BabyBearExt4 — the tree's
            default (FieldType::default())
PCS         Jagged(Basefold) — the tree's default (PcsKind::default())
security    SecurityLevel::Conjecture100bits — the only variant the enum defines. Named
            "Conjecture100bits" by its authors; we did not audit what it delivers and do not
            restate it as "100 bits". binius64 publishes SECURITY_BITS = 96 by a different
            accounting; the two are NOT compared here
trusted setup   no — hash-based, no structured reference string. Same as binius64 on this
                axis; UNLIKE DeepProve and jolt-atlas, whose HyperKZG needs one
ZK              no — this is the non-ZK proving path
quantization    signed INT8 in [-128,127], one operand per byte in the hint region, read with
                a sign-extending `lb`; INT32 accumulator (i64 from MLP layer 2 on)
requantization  NONE, per bench/TASKS.md Amendment A1
weights         PROGRAM-DATA (bench/TASKS.md Amendment A2). Established here from this
                directory's own evidence, because no earlier document named a position.
                The weight matrices are records in the `--hints-file` memory segment
                (EXPRESSION.md §8: "T2/T3 | BATCH; the four weight matrices [out][in]; the
                inputs"), and the guest re-reads them with a sign-extending `lb` once per
                operand per MAC (EXPRESSION.md §2: "tracer generated LB 131 072 records
                (exactly 2 x 65 536)" at T1-0). They are NOT baked into the program image:
                one ELF and one vk serve the whole T1 ladder with five different weight
                matrices (EXPRESSION.md §3), which is the measurement that rules out
                `circuit-constant` and `preprocessed` rather than an argument that does.
weight cost     CYCLES. Two `lb` per MAC plus the address arithmetic around them, inside the
                proved trace, and therefore inside prove time, inside peak memory and inside
                both derived metrics. Nothing about the weights lands in keygen: keygen is
                constant at ~9.4 s across every task (§5) precisely because it depends on the
                program image and not on the data.
padding         NONE. No task was reshaped. Ceno pads each chip's trace to a power of two
                internally, but the task is not padded, so the MACs proved equal the MACs
                published — unlike DeepProve (768 -> 1024, 1.778x) and jolt-atlas
batching        T3 is 8 inputs in ONE proof
segmentation    --max-cycle-per-shard, default 2^29; --max-cell-per-shard, default 2^31,
                whose source comment says it was sized for "16GB VRAM". PEAK MEMORY IS A
                FUNCTION OF THESE FLAGS — see §3
allocator       system allocator. NOT the authors' documented jemalloc configuration, which
                sets retain:true with decay disabled and would never return pages to the OS;
                BUILD.md §4
threads     10 (primary) and 2 (secondary), via RAYON_NUM_THREADS. 1 thread ABORTS THE
            PROVER and is reported as a failed cell, not omitted. Ceno additionally rounds
            down internally: "thread size 10 is not power of 2, using 8 threads instead"
machine     Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, NOT dedicated,
            ON BATTERY, load average 5.13 at campaign start, 778 MB free swap,
            boot volume 95% full
OS          macOS 26.5.2 (25F84), Darwin 25.5.0, uptime 12 days
N           per cell, in the table
date        2026-08-24
```

## What is inside each measured quantity

| Column | What it contains | Same bracket as binius64? |
|---|---|---|
| `prove s` | the `ZKVM_create_proof` tracing span only — "pure proof generation, excluding emulation/witgen". This is the same span, extracted by the same sed/awk pipeline, that Ceno's own CI records as its GPU baseline | **No.** binius64's prove time is its whole prover call |
| `peak RSS` / `peak fp` | the **whole process**: ELF load, emulation, witness generation, keygen and proving | Yes — deliberately. The question is whether the task fits on the machine |
| `proof B` | the serialized `Vec<ZKVMProof>`, bincode | Yes |
| `verify s` | `ceno_zkvm::e2e::verify` alone, against a vk **regenerated in process** | **No.** A vk loaded from disk rejects every proof (BUILD.md §5) |
| `keygen s` | one-off setup, reported separately in §5 and **never amortised into prove time** | Yes |
| `(u+s)/real` | the control that says whether wall-clock was computation or waiting | Yes |

**Emulation and witness generation are inside the memory column but outside the prove-time
column.** That asymmetry is Ceno's own bracket, not ours, and it flatters prove time relative to
a system whose prove call includes witness population. Process wall time is published beside
prove time so the gap is visible.

## The full grid

Every cell that was run, uncurated, including the ones that failed. Raw per-repetition data in
[`bench/data/cells-ceno.csv`](../../data/cells-ceno.csv), per-repetition logs under
[`bench/data/cells-ceno/`](../../data/cells-ceno/), derived table in
[`bench/data/results-ceno.csv`](../../data/results-ceno.csv).

| Task | MACs | cycles | RAYON thr | shard cap | shards | N | status | prove s (median) | [min-max] | proof B | peak RSS GB | peak fp GB | (u+s)/real | **MAC/s** | **cycles/s** | **B/MAC fp** | **B/cycle fp** |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| T1-0 | 65 536 | 3 203 656 | 10 | 536 870 912 | 1 | 5 | OK | 3.67 | [3.31-3.88] | 1 162 285 | 5.03 | 4.98 | 2.1895 | **17 857** | **872 931** | **81 572.8** | **1 668.7** |
| T1-0 | 65 536 | 3 203 656 | 2 | 536 870 912 | 1 | 2 | OK | 9.60 | [9.60-9.60] | 1 162 285 | 4.96 | 4.92 | 1.3521 | **6 827** | **333 714** | **80 621.8** | **1 649.3** |
| T1-A | 589 824 | 25 265 476 | 10 | 2 097 152 | 13 | 1 | OK | 44.04 | [44.04-44.04] | 13 577 390 | 8.27 | 8.22 | 5.4788 | **13 393** | **573 694** | **14 956.5** | **349.2** |
| T1-A | 589 824 | 25 265 476 | 10 | 33 554 432 | 1 | 1 | OK | 20.00 | [20.00-20.00] | 1 379 317 | 12.37 | 12.31 | 4.2251 | **29 491** | **1 263 274** | **22 415.7** | **523.3** |
| T1-A | 589 824 | 25 265 476 | 10 | 536 870 912 | 1 | 3 | OK | 20.10 | [20.00-20.30] | 1 379 317 | 12.34 | 12.31 | 4.2196 | **29 344** | **1 256 989** | **22 414.8** | **523.3** |
| T1-A | 589 824 | 25 265 476 | 10 | 8 388 608 | 4 | 1 | OK | 30.02 | [30.02-30.02] | 4 614 432 | 8.57 | 8.53 | 5.1389 | **19 648** | **841 621** | **15 533.3** | **362.6** |
| T1-B | 2 359 296 | 100 983 792 | 10 | 536 870 912 | 2 | 2 | OK | 104.50 | [103.80-105.20] | 2 844 740 | 15.05 | 34.06 | 5.3078 | **22 577** | **966 352** | **15 499.5** | **362.1** |
| T2 | 92 224 | 4 499 472 | 1 | 536 870 912 | 1 | 0 | **FAIL_rc101** | — | — | — | — | — | — | — | — | — | — |
| T2 | 92 224 | 4 499 472 | 10 | 536 870 912 | 1 | 5 | OK | 4.00 | [3.98-4.27] | 1 188 781 | 5.18 | 5.15 | 2.4350 | **23 056** | **1 124 868** | **59 944.9** | **1 228.7** |
| T3 | 737 792 | 35 505 916 | 10 | 536 870 912 | 1 | 3 | OK | 26.30 | [25.70-27.80] | 1 424 885 | 13.08 | 14.12 | 4.5675 | **28 053** | **1 350 035** | **20 546.7** | **426.9** |

`FAIL_rc101` is a Rust panic. For the 1-thread cells it is
`assertion left != right failed: Attempt to prove a constant` in the sumcheck prover, which
also fires on Ceno's own `fibonacci` and `ceno_rt_alloc` — see
[`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1. **Derived columns are empty for every cell that
produced no proof:** those processes still had a memory peak — the peak of failing — and
dividing it by a MAC count the system never completed would manufacture a number out of a
crash.

## 1 · The memory curve, which is the thing this repository exists to measure

Memory curve, RAYON thr = 10, shard cap = 536 870 912:
| Task | MACs | cycles | shards | peak footprint | **B/MAC** | vs previous rung | local exponent |
|---|---:|---:|---:|---:|---:|---|---:|
| T1-0 | 65 536 | 3 203 656 | 1 | 5 346.0 MB | **81 572.8** | — | — |
| T1-A | 589 824 | 25 265 476 | 1 | 13 220.8 MB | **22 414.8** | MACs x9.00, memory x2.47 | 0.412 |
| T1-B | 2 359 296 | 100 983 792 | 2 | 36 568.0 MB | **15 499.5** | MACs x4.00, memory x2.77 | 0.734 |

**The curve is the result. No single value on it is a property of the prover**, and quoting one
without its workload would be quoting a property of the pair. `bench/README.md`'s third finding
— that `bytes/MAC` is not a constant of a proof system — holds here more violently than
anywhere else in the benchmark.

**What the shape says.** `bytes/MAC` falls steeply across the measured rungs. It falls almost
entirely because a very large constant is being spread thinner, not because the prover becomes
more frugal. The evidence is the marginal cost, which is an order of magnitude below the
average:

| | peak footprint | cycles | |
|---|---:|---:|---|
| T1-0 | 5 346 MB | 3 203 656 | average **1 669 B/cycle** |
| T1-a | 13 221 MB | 25 265 476 | average **523 B/cycle** |
| **marginal, T1-0 → T1-a** | **+7 875 MB** | **+22 061 820** | **357 B/cycle** |

So the intercept is roughly **5 GB before any task-dependent work at all**, and the slope is
roughly **357 bytes per proved cycle**. Both numbers are absolute and neither depends on the
MAC denominator, which is why §7 argues they are the transferable part of this entry.

**That 5 GB intercept is the zkVM's signature.** T1-0 is the smallest task in the benchmark —
65 536 MACs, a `[1×256]·[256×256]` matmul — and proving it costs 5.35 GB, because instantiating
the RV32IM chip set costs that much whatever the program does. A circuit prover pays for the
circuit it was given; a zkVM pays for every instruction the machine *could* execute, and then
for the ones it did.

**That is a description of the measured points, not an asymptote.** `bench/CHALLENGE.md`
forbids extrapolating outside the measured range and it binds here: nothing above says what
Ceno does past the largest rung actually proved, and §3 shows that past that point the question
is not even well posed without naming a shard size.


## 2 · Batching: what T3 buys, measured

T3 is T2 eight times over, in **one** proof. Same weights, eight independent inputs. Both cells
ran at the same threads and the same shard cap, so this is one comparison, not two campaigns.

| | T2 | T3 | ratio |
|---|---:|---:|---:|
| MACs | 92 224 | 737 792 | ×8.00 |
| cycles | 4 499 472 | 35 505 916 | ×7.89 |
| prove s (median) | 4.00 | 26.30 | **×6.58** |
| peak footprint | 5 528 MB | 15 159 MB | **×2.74** |
| proof bytes | 1 188 781 | 1 424 885 | **×1.20** |

**Batching eight requests into one proof is very close to linear in time and strongly
sublinear in memory and proof size** — and both sublinearities come from the same place as §1,
the constant being amortised. Per request, batching buys **1.22× on prove time, 2.92× on peak
memory, and 6.67× on proof size.**

The time result is the honest disappointment and the expected one: a zkVM proving eight
inferences executes eight times the instructions, so there is no protocol-level saving to be
had. The ×7.89 in cycles against ×8.00 in MACs is just the fixed program prologue being shared.
**Whatever batching buys here, it is amortisation of overhead, not sublinearity of the
protocol.** The proof-size figure is the one worth taking away: eight answers for 1.20× the
bytes.


## 3 · Peak memory is a flag, not a property of the task

**This is the most transferable thing this entry found, and it has no counterpart in the other
three systems.**

Ceno segments a trace across shards. So for a fixed task, on a fixed machine, at fixed threads,
peak prover memory is set by `--max-cycle-per-shard` and `--max-cell-per-shard`. Same task, same
instance, same everything else — only the flag moves:

| `--max-cycle-per-shard` | shards | prove s (sum over shards) | peak footprint | proof bytes | **B/MAC** |
|---:|---:|---:|---:|---:|---:|
| 536 870 912 (2²⁹, default) | 1 | 20.10 | 13 221 MB | 1 379 317 | **22 415** |
| 33 554 432 (2²⁵) | 1 | 20.00 | 13 222 MB | 1 379 317 | **22 416** |
| 8 388 608 (2²³) | 4 | 30.02 | 9 162 MB | 4 614 432 | **15 533** |
| 2 097 152 (2²¹) | 13 | 44.04 | 8 822 MB | 13 577 390 | **14 957** |

T1-a, 589 824 MACs, 25 265 476 cycles, 10 RAYON threads, N = 1 per sweep row.

**Segmenting buys memory and pays in time and in proof size.** From the default to 13 shards:
peak footprint falls **1.50×**, prove time rises **2.19×**, and the proof grows **9.84×**. The
first two caps are identical because T1-a's 25.27 M cycles already fit inside 2²⁵, so nothing
was segmented — the flag only acts once it bites.

Three consequences, and they are why `bytes/MAC` needs an asterisk for this system specifically:

1. **A `bytes/MAC` figure for a segmented prover is not a property of the prover.** It is a
   property of (prover, task, shard size). Every row in the grid therefore carries its shard cap
   and its shard count, and the memory curve in §1 is cut at one configuration.
2. **The shipped default is a GPU default.** `--max-cell-per-shard`'s source comment says it was
   derived for "16GB VRAM". On a CPU host it is not the configuration a careful operator would
   choose, and T1-b demonstrates it: at the default caps T1-b splits into **2 shards anyway** —
   not because of the cycle cap, which it is four times under, but because of the *cell* cap.
3. **"Does it fit?" becomes a different question.** For the three circuit systems here, peak
   memory is what it is and the answer is yes or no. For Ceno it is a dial, and the honest
   answer is "at what shard size, and are you willing to pay 2.2× the time and 9.8× the bytes
   for it?" `bench/README.md` argues memory is a binary gate because it does not parallelise
   across machines. Segmentation is the one mechanism in this benchmark that argues back.

**What this does not show.** Nothing here says the trade stays this cheap further down. The
sweep is four points on one rung; `bench/CHALLENGE.md` forbids extrapolating past it, and the
memory floor of ~5 GB (§1) is not something segmentation was observed to go below.


## 4 · Threads buy time. Threads do not buy memory.

| Task | RAYON thr | prove s (median) | speedup | peak footprint | change |
|---|---:|---:|---:|---:|---:|
| T1-0 | 2 | 9.60 | — | 5 284 MB | — |
| T1-0 | 10 | 3.67 | **2.62×** | 5 346 MB | **+1.1 %** |

**Wall-clock time responds to hardware; peak memory does not.** 5× the nominal threads buys
2.62× the speed and costs 1.1 % more memory — the same shape all four systems in this benchmark
show, and the reason `bench/README.md` calls memory a binary gate rather than a performance
detail.

Two qualifications, so the speedup is not read as better than it is:

- **`RAYON_NUM_THREADS` is not the thread count.** Ceno's own log says
  `thread size 10 is not power of 2, using 8 threads instead`, so the "10-thread" cell is an
  8-thread cell. Against 8 real threads, 2.62× is a parallel efficiency of about 0.65 — on a
  machine carrying a load average of 5 from other work.
- **`(u+s)/real` is 1.35 at 2 threads and 2.19 at 10.** Neither approaches the nominal thread
  count, which is what contention on a non-dedicated machine looks like.

**And the row that is missing is the important one.** binius64's primary cut is 1 thread. Ceno
has no 1-thread row because the prover aborts there — on Ceno's own examples, not only on ours
(`NOT_EXPRESSIBLE.md` §1). The `FAIL_rc101` cell in the grid is that failure, published rather
than omitted.


## 5 · Setup and verification, reported separately

Setup is never amortised into prove time. Both figures are taken outside the measured prove
process, because `e2e` verifies inline and reports no separate number and folds its keygen into
the same run.

| Task | shards | proof bytes | keygen s | `ZKVMVerifier::new` s | proof deserialize s | **verify s** | verdict |
|---|---:|---:|---:|---:|---:|---:|---|
| T1-0 | 1 | 1 162 285 | 9.41 | 3.88 | 0.0010 | **0.0503** | VERIFY_ACCEPTED |
| T2 | 1 | 1 188 781 | 9.56 | 3.87 | 0.0010 | **0.0510** | VERIFY_ACCEPTED |
| T3 | 1 | 1 424 885 | 9.36 | 3.85 | 0.0012 | **0.0596** | VERIFY_ACCEPTED |
| T1-a | **13** | 13 577 390 | 9.48 | 3.87 | 0.0100 | **0.5627** | VERIFY_ACCEPTED |

**Keygen is constant at ~9.4 s and does not vary with the task.** That is not noise — it is
structural, and it is one of the few places a zkVM is unambiguously better shaped than a
circuit prover. The verifying key depends on the *program image*, and one ELF serves the whole
T1 ladder (EXPRESSION.md §3), so the same key proves a 65 536-MAC matmul and a 37 748 736-MAC
one. A circuit system pays setup per circuit; Ceno pays it once per program.

**`ZKVMVerifier::new` costs 3.87 s and is excluded from the verify figure.** It computes the vk
digest. It is reported here rather than folded in, because folding it in would multiply the
verify column by roughly 78× and would be reporting a one-off, not a per-proof cost.

**Verification scales with shard count.** The T1-a row is the 13-shard proof from the §3 sweep:
0.5627 s against 0.0503 s for a single-shard proof, about 11×, with a proof 9.8× larger. So
segmentation's bill is paid three times — in prove time, in proof size, and in verify time.
That row is the segmented configuration and is labelled as such; it is not T1-a's
single-shard verify figure.


## 6 · Correctness control

`bench/README.md`: *"A corrupted trace must make `verify()` fail, in every system, on every
task."*

**Two positive controls first**, because a negative test that passes because nothing ever
verifies proves nothing — and here that risk was not hypothetical (BUILD.md §5):

| Control | Result |
|---|---|
| the honest proof verifies | **VERIFY_ACCEPTED** |
| serialize → deserialize → verify, unmodified | **VERIFY_ACCEPTED** — so the method itself does not corrupt |

### The sweep

**18 161 single-bit corruptions of the T1-0 proof. 18 161 rejected. 0 accepted.**

| Verdict | Count | % |
|---|---:|---:|
| VERIFY_PANIC | **12 166** | **66.99 %** |
| VERIFY_REJECTED | 4 716 | 25.97 % |
| DESERIALIZE_REJECTED | 1 279 | 7.04 % |
| **VERIFY_ACCEPTED** | **0** | **0.00 %** |

**Verdict: PASS.** No corrupted proof was accepted at any probed offset.

### Coverage, declared rather than inferred

**This is a strided sweep, not an exhaustive one, and that is a real weakness.** Stride 64 over
1 162 285 bytes — 18 161 of 1 162 285 offsets, **1.56 %**. jolt-atlas's sweep was exhaustive
because its proof is 21 419 bytes; Ceno's T1-0 proof is **54× larger** and each verification
costs ~50 ms, so an exhaustive sweep would run about **16 hours** for one task.

jolt-atlas's campaign is the reason this matters: an exhaustive sweep there found an accepted
region that a 124-offset sample had hit only once by luck, and a second region a sample missed
entirely. **A strided sweep here could miss an accepted region shorter than 64 bytes.** We did
not find one; we also did not look everywhere, and those are different statements. Raw data,
one row per probed offset, in
[`bench/data/negative-ceno/t1-0.csv`](../../data/negative-ceno/t1-0.csv).

### Two-thirds of rejections are panics, and that is reported raw

`VERIFY_PANIC` means the corruption aborted the verifier or its deserializer rather than
producing a returned error. **It is still a rejection** — no such proof was accepted — and
nothing about soundness follows from it. What does follow is a robustness surface: a verifier
that panics on malformed input is a denial-of-service target in a deployment that verifies
untrusted proofs in-process, and 67 % is a large fraction of the corruption space. jolt-atlas
showed the same class of behaviour (1 237 panics and 314 hard aborts). We did **not** establish
which fields these offsets belong to, and do not speculate.

### Amendment A3 — nothing here is re-labelled, and the reason is a gap, not a pass

`bench/TASKS.md` Amendment A3 (2026-08-24) re-labels witness-level corruptions on T2 and T3 as
weak evidence, because up to 52.27 % of T2's weights are inert under ReLU and a corruption that
does not change the output is not a test.

**A3 does not bite here, because this system has no witness-level corruption to re-label.**
Every one of the 18 161 corruptions above is a `proof_byte` flip — verified row by row in
[`t1-0.csv`](../../data/negative-ceno/t1-0.csv), whose only families are `proof_byte`, `none`
(the positive controls) and `coverage`. **Proof-artifact corruption is explicitly unaffected by
A3 and remains the strong control**, so the PASS verdict above stands in full and unqualified.

**But the absence cuts the other way too, and that is the honest reading.** A3's re-labelling
costs binius64 six rows of evidence about weight binding and leaves it with the rest; it costs
Ceno nothing because Ceno had none to begin with. The reason is in the section below: the
witness is generated inside the prover from the ELF and the hints and is never exposed as a
mutable artifact. **So this entry carries no evidence, weak or strong, that a perturbed weight
is detected** — and the sweep was run only on T1-0, which has no ReLU anyway.

### What this control does NOT establish

**It is weaker than binius64's, and the difference is structural.** binius64's control flips
words of the *witness* — the trace itself — through `ValueVec::word_mut`. Ceno's witness is
generated inside the prover from the ELF and the hints and is never exposed as a mutable
artifact, so **no witness-level flip was available**, and every corruption here is a flip of the
serialized proof. What the control establishes is that the bytes being measured are a proof
whose mutation is detected, and not a computation that happens to produce bytes. It does not
establish that a maliciously *constructed* trace would be caught.

The one place a wrong-witness proof was tested is not this sweep but EXPRESSION.md §6: a proof
whose committed output did not match the claimed public IO was **rejected**, which is the
property that matters most and which we found by getting it wrong first.

## 7 · Does comparing a zkVM to a circuit inform, or mislead? Our position, argued

The brief that commissioned this measurement asked for a judgement and said a reasoned "this
cell does not belong in the table" would be preferable to a misleading number. Here it is.

**The comparison informs, and the ratio may be published — on one condition, which is just
rule F.7 applied: every cross-system ratio involving Ceno carries the instructions-per-MAC
factor in the same sentence.** Without it the number is not wrong, it is unattributable, and
a reader will charge to Ceno's prover a cost that belongs to the encoding.

### Why we did not conclude "this cell does not aggregate"

The tempting position is that `bytes/MAC` is meaningless for a zkVM, because the MAC count
describes the task while the cost is driven by cycles, and Ceno spends **10.70 instructions per
MAC** at the top of the ladder. On that view the comparison is a category error and the cell
should be blank.

We rejected it, because it answers the wrong question. E-006 does not ask "whose prover
implementation is better." It asks **which family of protocol has the right shape for proving
an LLM forward pass.** For that question, "this system needs 10.7 instructions to do one
multiply" is not a confound to be normalised away — **it is the finding.** A buyer who wants a
matmul proved pays for the instructions, not for the MACs. Blanking the cell would suppress
the single most decision-relevant fact about the family.

So the ratio stays, and the factor stays with it.

### The three things this system contributes that are NOT denominator-dependent

These are the reason the entry earns its place regardless of what one thinks of `bytes/MAC`.
Each is an absolute measurement, and none of them changes if you re-denominate the task.

1. **A fixed floor of about 5 GB, before any task-dependent work.** T1-0 — 65 536 MACs, the
   smallest thing in the benchmark — peaks at **5.35 GB**. That is not a per-MAC quantity and
   does not shrink with a smaller task: it is what instantiating the RV32IM chip set costs.
   Every circuit system in this benchmark proves T1-0 in a small fraction of that. For a
   product that must prove *small* things cheaply, this number decides the question on its own,
   and no denominator is involved in reading it.
2. **A marginal cost per cycle that is far below the average.** Between T1-0 and T1-a the
   footprint goes 5.35 → 13.22 GB for 3.20 → 25.27 M cycles: about **355 bytes per marginal
   cycle**, against an average of 1 669 B/cycle at T1-0. So the curve is dominated by the
   constant, and `bytes/MAC` falls steeply (81 573 → 22 415) for that reason and not because
   the prover got frugal. This is the same "a constant being spread thinner" shape jolt-atlas
   showed, one order of magnitude larger.
3. **Peak memory is a flag, not a property** (§3). No circuit system in this benchmark offers
   that, and it changes the *kind* of answer available to "does it fit": for the other three,
   peak memory is what it is; for Ceno you choose it and pay in proofs. That is the most
   transferable thing this entry found.

### And one that is a genuine, denominator-free win

**Ceno expressed all seven tasks with zero deviation from `bench/TASKS.md`.** No padding, no
minimum layer width, no implicit requantisation, no batch-size restriction, no matrix-vector
limitation. Every other system in this benchmark hit at least one frontend wall — DeepProve
pads 768 → 1024 and cannot do `M > 1` or a width-1 output; jolt-atlas cannot express a
non-power-of-two rung at all. Ceno's walls, when it hit them, were resources and defects
(NOT_EXPRESSIBLE.md), never expressiveness.

**That is what the generality buys, and this benchmark is the first place where its price and
its purchase are measured on the same tasks, on the same machine, in the same campaign.**

### What we will not publish

- **No "Ceno is N× worse than X" sentence**, anywhere, in any form, without the
  instructions-per-MAC factor in the same sentence.
- **No cross-system `MAC/s` ranking that places Ceno in the same column as the three circuit
  systems without a separating rule**, because a column implies commensurability the
  measurement does not support.
- **No ratio against Ceno's own published GPU figure** (REPRODUCTION.md §3).

## What contaminates these numbers, declared

1. **The machine was in the worst state of any campaign here.** Not dedicated (load 5.13 on 10
   cores), on battery, 778 MB of free swap, boot volume 95 % full. `(u+s)/real` is published per
   cell so contention is visible rather than inferred. **No figure here is Ceno's best
   achievable performance.**
2. **`RAYON_NUM_THREADS` is not the thread count.** Ceno rounds it down to a power of two
   internally and says so in its own log. `(u+s)/real` above 2 on nominally 10-thread cells
   shows how many cores were actually busy.
3. **1 thread is unavailable**, so no figure here can be compared to binius64's primary cut
   without that being stated in the same sentence.
4. **The allocator is not the authors' documented one** (BUILD.md §4). Their configuration
   would likely be faster and would certainly report worse memory.
5. **`-C target-cpu=native` applies to Ceno** via its own committed `.cargo/config.toml`, and
   did not apply to DeepProve or jolt-atlas, whose trees do not set it.
6. **Prove time excludes emulation and witness generation; memory includes them.** Ceno's
   bracket, declared.
7. **The Keccak that binds the output is inside the proved trace** (EXPRESSION.md §6), so a
   share of every cycle count is hashing rather than multiplying.
8. **Rust bounds checks are inside the proved trace** — `BGEU` is ~8 % of T1-0's instructions
   (EXPRESSION.md §2). That headroom belongs to Ceno; we did not take it, and we did not take
   the equivalent for the other systems either.
9. **N is small on the expensive rungs** and is stated per cell. Where N = 1 no dispersion is
   published, because none was measured.
10. **Nothing is extrapolated outside the measured range.** `bench/CHALLENGE.md` forbids it, and
    it binds here: nothing in this file says what Ceno does past the largest rung actually
    proved.
