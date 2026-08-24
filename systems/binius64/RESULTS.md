# binius64 — results

**Scope: this file reports one system.** It is not a comparison and it is not a ranking;
no other system has been measured yet. Read [`BUILD.md`](BUILD.md) before any number here,
[`REPRODUCTION.md`](REPRODUCTION.md) for what of binius64's own published figures we could and
could not reproduce, [`EXPRESSION.md`](EXPRESSION.md) for how each task was written, and
[`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) for the rung that did not run.

---

## Conditions line

Applies to every figure below. Where a cell differs, the cell says so in its own row.

```
system      binius64
commit      eac2484b1a2e0b68d7b9e9b2e40f3c86ef220d4d (+ spans-only instrumentation patch)
constraints reported per task in EXPRESSION.md §6; MACs = IMUL constraints, checked against
            the count frozen in bench/TASKS.md
field       GF(2^128), BinaryField128bGhash
hash suite  StdHashSuite = SHA-256
security    SECURITY_BITS = 96, held constant across the rate sweep; FRI query count derived
            from the rate (232 at log_inv_rate 1, 106 at 4), so rates are equal-soundness
trusted setup   no
ZK              no  — the non-ZK prover; binius64 has a ZK prover, out of scope for round one
quantization    signed INT8 in [-128,127], sign-extended into 64-bit two's-complement words,
                no range constraint (EXPRESSION.md §5)
weights         WITNESS (bench/TASKS.md Amendment A2). Both operands, weights included, are
                private witness wires committed per proof; in T3 one committed weight set is
                shared across the 8 batch items. EXPRESSION.md §2 — "Both A and B are private
                witness wires. The weights are committed, not public" — and §3.
weight cost     PROVE. It lands inside the constraint count, inside prove time and inside peak
                memory, and therefore inside both derived metrics below. There is no setup
                column for it to hide in: binius64 has no trusted setup at all.
threads     1 (primary cut) / 10 (secondary), via RAYON_NUM_THREADS
machine     Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, NOT dedicated
OS          macOS 26.5.2 (25F84), Darwin 25.5.0, uptime 12 days
N           per cell, in the table; warmup 1 except where the table says 0
date        2026-08-23
```

## How to read the memory columns — this decides the headline metric

Both peaks are **per process** and cover circuit construction, witness generation, setup,
proving **and verification**, because this harness verifies in the same process that proves.
That is a wider bracket than "the prover"; it is chosen deliberately, because the question
the memory metric answers is whether the task fits on the machine.

**Peak RSS and peak footprint diverge, and above ~16 GB the divergence is the whole story.**
This machine has 32 GiB of RAM. Once a cell's footprint exceeds what is resident, RSS
saturates while footprint keeps growing:

| Task | peak RSS | peak footprint | ratio |
|---|---|---|---|
| T1-a | 7.80 GB | 7.28 GB | 0.93 |
| T1-b | 16.69 GB | 27.56 GB | 1.65 |
| T1-c | 19.15 GB | 92.99 GB | **4.86** |

So **`bytes/MAC` computed against peak RSS is not a memory-cost metric at T1-b and above** —
it is a measurement of how much RAM this machine has. It falls from 7 074 B/MAC at T1-b to
2 029 B/MAC at T1-c *not* because the prover became five times more frugal but because RSS
hit the ceiling and the rest went to swap. **The footprint column is the one to read.** Both
are published because publishing only one is how a reader gets misled.

## How the verify column is measured — what is inside it

The `verify ms` column times `binius_examples::check_proof(&verifier, &witness, proof)` and
nothing else. That call constructs a `StdChallenger`, wraps the proof bytes in a
`VerifierTranscript`, calls `Verifier::verify`, and calls `finalize`. It does **not** rebuild
the circuit, re-derive the witness, or validate the constraint system: those happen once per
cell and are reported in the `build ms` and `setup ms` columns.

**It is the route binius64's own authors time.** Their Criterion benchmark
`{prefix}_proof_verification` (vendor `crates/examples/benches/utils/runner.rs:210-227`) runs
exactly the same two calls, and additionally clones the proof bytes inside its timed closure,
which this harness does not.

**Only the public statement reaches the verifier.** `check_proof` is handed the whole
`ValueVec`, but it passes on only `witness.inout()` (vendor `crates/examples/src/lib.rs:180`),
and `Verifier::verify` rejects any statement whose length is not `n_inout`. The private
segment is never read. Taking that slice is 1–103 µs across the whole ladder, measured
separately in §4.

**There is no separable deserialization term.** `VerifierTranscript::new` is
`Bytes::from(vec)` (vendor `crates/transcript/src/transcript.rs:46-53`) and parses nothing;
the transcript is read lazily by the protocol as it runs. `finalize` only asserts the tape was
fully consumed.

**Verify runs in the same process, immediately after the proof it checks.** That is a
measurement condition, not a neutral choice, and §4 shows it is worth up to 3× at T1-b. Both
the figure it produces and the steady-state figure are published there.

## The full grid

Every cell that was run, uncurated, including the ones that failed and the ones superseded by
a rerun. Raw per-repetition data is in [`bench/data/`](../../data/).

| Task | rate | thr | N | status | prove ms (median) | [min–max] | verify ms | proof B | setup ms | build ms | peak RSS GB | peak footprint GB | (u+s)/real | **MAC/s** | **B/MAC footprint** | **B/MAC RSS** |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| T1-0 | 1 | 1 | 5 | OK | 178.98 | [177.83–186.78] | 6.29 | 345 536 | 66.1 | 109.4 | 0.60 | 0.53 | 0.9925 | **366 160** | **8 155** | **9 149** |
| T1-0 | 4 | 1 | 5 | OK | 269.31 | [268.54–272.27] | 6.29 | 258 016 | 66.6 | 107.5 | 0.73 | 0.67 | 1.0000 | **243 349** | **10 173** | **11 189** |
| T2 | 1 | 1 | 5 | OK | 319.85 | [316.41–321.79] | 8.38 | 346 400 | 93.6 | 144.2 | 1.11 | 1.05 | 0.9956 | **288 333** | **11 335** | **12 062** |
| T2 | 4 | 1 | 5 | OK | 408.30 | [407.02–410.98] | 8.20 | 258 880 | 94.6 | 144.2 | 1.19 | 1.13 | 0.9965 | **225 875** | **12 228** | **12 951** |
| T3 | 1 | 1 | 5 | OK | 2 596.94 | [2 572.69–2 632.37] | 73.34 | 460 112 | 746.7 | 1 242.8 | 7.98 | 7.48 | 0.9979 | **284 101** | **10 138** | **10 822** |
| T3 | 4 | 1 | 5 | OK | 3 317.26 | [3 234.81–3 350.14] | 73.43 | 324 112 | 743.1 | 1 213.1 | 8.71 | 8.19 | 0.9982 | **222 410** | **11 104** | **11 803** |
| T1-a | 1 | 1 | 5 | OK | 2 634.93 | [2 579.18–2 679.44] | 68.34 | 460 304 | 626.3 | 967.0 | 7.80 | 7.28 | 0.9967 | **223 848** | **12 335** | **13 231** |
| T1-a | 4 | 1 | 5 | OK | 3 227.78 | [3 197.93–3 272.40] | 67.38 | 324 304 | 632.5 | 988.3 | 8.46 | 7.93 | 0.9995 | **182 734** | **13 446** | **14 343** |
| T1-b | 1 | 1 | 5 | OK | 19 519.46 | [18 972.12–19 728.60] | 593.00 | 505 952 | 2 490.5 | 4 295.8 | 16.69 | 27.56 | 0.8729 | **120 869** | **11 684** | **7 074** |
| T1-b | 4 | 1 | 5 | OK | 22 827.26 | [21 572.09–25 041.22] | 628.67 | 349 280 | 2 364.2 | 4 000.7 | 16.64 | 28.00 | 0.8564 | **103 354** | **11 866** | **7 054** |
| T1-C | 1 | 1 | 1 | **SUPERSEDED** | — | — | — | — | — | — | 16.49 | 87.18 | 0.6337 | — | — | — |
| T1-c | 1 | 1 | 3 | OK | 396 245.67 | [183 587.87–432 674.74] | 8 738.27 | 591 200 | 11 177.3 | 18 179.2 | 19.15 | 92.99 | 0.3783 | **23 816** | **9 854** | **2 029** |
| T1-C | 4 | 1 | 3 | **FAIL_rc143** | — | — | — | — | — | — | — | — | — | — | — | — |
| T1-c | 4 | 1 | 1 | OK | 186 422.46 | [186 422.46–186 422.46] | 7 842.15 | 396 896 | 11 349.6 | 16 900.8 | 17.90 | 91.13 | 0.6094 | **50 623** | **9 657** | **1 896** |
| T1-D | 1 | 1 | 1 | **FAIL_rc1** | — | — | — | — | — | — | 21.60 | 52.03 | 0.8416 | — | — | — |
| T1-0 | 1 | 10 | 5 | OK | 72.58 | [71.03–75.22] | 4.33 | 345 536 | 63.6 | 103.1 | 0.61 | 0.54 | 4.2537 | **902 994** | **8 285** | **9 271** |
| T1-0 | 4 | 10 | 5 | OK | 86.40 | [84.53–90.04] | 4.22 | 258 016 | 64.0 | 104.3 | 0.70 | 0.64 | 4.8421 | **758 549** | **9 721** | **10 722** |
| T2 | 1 | 10 | 5 | OK | 109.82 | [105.88–121.61] | 5.51 | 346 400 | 89.5 | 140.2 | 1.04 | 0.98 | 4.4600 | **839 790** | **10 587** | **11 312** |
| T2 | 4 | 10 | 5 | OK | 124.30 | [120.97–127.26] | 5.32 | 258 880 | 89.8 | 140.1 | 1.13 | 1.07 | 4.8519 | **741 941** | **11 571** | **12 296** |
| T3 | 1 | 10 | 5 | OK | 720.38 | [713.22–756.06] | 28.17 | 460 112 | 723.9 | 1 182.7 | 8.02 | 7.50 | 4.8198 | **1 024 164** | **10 168** | **10 868** |
| T3 | 4 | 10 | 5 | OK | 819.40 | [805.59–856.56] | 28.69 | 324 112 | 721.1 | 1 205.8 | 8.71 | 8.19 | 5.0535 | **900 401** | **11 104** | **11 804** |
| T1-a | 1 | 10 | 5 | OK | 710.64 | [696.48–758.03] | 24.56 | 460 304 | 618.0 | 955.7 | 7.78 | 7.25 | 4.9767 | **829 984** | **12 295** | **13 192** |
| T1-a | 4 | 10 | 5 | OK | 814.51 | [804.70–856.30] | 27.34 | 324 304 | 611.9 | 954.4 | 8.51 | 7.98 | 5.3755 | **724 144** | **13 526** | **14 423** |
| T1-b | 1 | 10 | 5 | OK | 9 732.16 | [9 386.76–10 465.79] | 136.03 | 505 952 | 2 351.0 | 3 991.7 | 20.78 | 28.14 | 3.3749 | **242 423** | **11 927** | **8 809** |
| T1-b | 4 | 10 | 5 | OK | 11 148.32 | [10 135.61–11 628.97] | 309.00 | 349 280 | 2 341.2 | 3 941.8 | 20.03 | 29.40 | 3.3974 | **211 628** | **12 459** | **8 490** |

Status values: `OK` — measured. `SUPERSEDED` — a later rerun of the same label overwrote this
row's per-repetition file; its memory figures survive in the ledger but no rate is derived
from it, because the numerator would come from one run and the denominator from another.
`FAIL_rc143` — **killed by the operator** to bound campaign wall time, not a system failure.

**The `verify ms` column is the cold figure.** Every measured verify in this grid follows a
fresh proof in the same process, so each one pays first-touch page cost on the memory the
verifier allocates. §4 measures that term, publishes the steady-state figure beside it, and
explains which question each answers. **No figure in this grid was changed.**

## What the numbers say

### 1 · T3 answers its question: batching buys proof size always, and prove time only when cores are idle

T3 exists to isolate whether folding 8 independent requests into one proof is sublinear in
the number of requests. binius64 **can** do it in a single proof (`NOT_EXPRESSIBLE.md` §2),
so the comparison is clean: one T3 proof against eight sequential T2 proofs, both at
`log_inv_rate = 1`, 1 thread, same machine, same campaign.

| | 8 × T2, sequential | T3, one proof | batching |
|---|---|---|---|
| prove time | 2 558.80 ms | 2 596.94 ms | **1.5% worse** |
| verify time | 67.04 ms | 73.34 ms | 9.4% worse |
| total proof bytes | 2 771 200 B | 460 112 B | **6.02× better** |
| peak footprint | 1.05 GB | 7.48 GB | **7.13× worse** |

The weights are committed **once** in T3 and eight times across the eight T2 proofs, and
T3 accordingly holds 21% fewer private values than 8 × T2 (2 399 408 against 3 044 976). It
still does not make proving cheaper. **In this system the fixed per-proof overhead is not
what dominates; the per-MAC work is** — which is why amortising the commitment across a batch
returns nothing on time and costs 7× on peak memory.

**The verify row above is superseded by §4 and flips sign.** Both figures in it are cold
(§4), and at steady state the comparison reads 8 × 8.26 = 66.08 ms against T3's 60.23 ms —
batching is **8.9% better** on verify, not 9.4% worse. The reason is exact rather than
noisy: 8 × T2 and T3 hold **the same 1 567 800 constraints**, so the linear verify term is
identical (8 × 7.39 = 59.15 ms against 59.10 ms), and what batching saves is the seven
redundant copies of the succinct per-proof term, 7 × 0.85 = 5.95 ms against the 5.85 ms
observed. **Batching buys nothing on the part of verification that dominates**, for the same
reason it buys nothing on prove time: the cost is per-constraint, and the batch has the same
constraints.


**But the answer flips with threads, and that is the interesting part.** At 10 threads the
same comparison reads:

| | 8 × T2, sequential | T3, one proof | batching |
|---|---|---|---|
| prove time | 878.40 ms | 720.38 ms | **1.22× better** |

At 1 thread the batch is 1.5% *worse*; at 10 threads it is 22% *better*. The batch is not
cheaper in work — it is **larger, and a larger circuit fills more cores**. So what batching
buys in this system is proof size always, and prove time only when there is parallelism
going unused. It never buys peak memory, which is 7.13× worse either way.

That is a result about a cost *shape*, and it is the kind T3 was designed to produce.

### 2 · Rate is a straight trade, in the same direction in every cell

`log_inv_rate` 1 → 4, at equal soundness (96 bits; the FRI query count drops 232 → 106):

| Task | prove time | proof size |
|---|---|---|
| T1-0 | 1.50× slower | 1.34× smaller |
| T1-a | 1.22× slower | 1.42× smaller |
| T1-b | 1.17× slower | 1.45× smaller |
| T2 | 1.28× slower | 1.34× smaller |
| T3 | 1.28× slower | 1.42× smaller |

No cell breaks the pattern. Both directions are in the grid; neither is reported alone.

### 3 · Proof size is the metric where this system looks strong

345 536 B at 65 536 MACs, 591 200 B at 9 437 184 MACs: **144× the circuit for 1.71× the
proof.** That is the expected shape for a FRI-based system and it is worth stating plainly,
because the rest of this file is less flattering.

### 4 · Verify time is linear in constraint count — and the linear term is not the FRI verifier

**This section replaces an earlier one titled "Verify time is NOT flat, and we do not explain
it."** That text reported the growth, named our use of `check_proof` as an untested candidate,
and left the cause open. The cause is now established by measurement. The earlier text's own
guess was also wrong, and is corrected here: `check_proof` does **not** hand the verifier the
full value vector. No figure in the grid was changed; the figures the diagnosis adds are
published beside the originals below.

The question this had to answer before anything was published: **is the growth an artifact of
how we call binius64's verification API, or a property of the system?** Both answers are
publishable. Publishing the wrong one would break the fairness protocol of
[`bench/README.md`](../../README.md) — the one the remaining five systems will be held to.

**Verdict: property of the system, at the pinned commit.** Attributed below in binius64's own
words, because the system's authors document it as a known gap rather than deny it.

#### How it was measured

`Verifier::verify` (vendor `crates/verifier/src/verify.rs:316-344`) is four steps. A separate
binary, `e006-verify-split`, reproduces those four calls in the same order and times each one,
**in the same loop and the same run**, so the terms sum to the whole rather than being
collected from separate campaigns:

| | Term | What it is |
|---|---|---|
| **A** | statement | build the BaseFold channel over the transcript, `observe_words(witness.inout())` |
| **B** | reduction | `IOPVerifier::verify` — trace-oracle commitment, the constraint reduction (zerocheck / shift / IntMul sumchecks), ring-switching. It **queues** the polynomial-commitment opening rather than performing it |
| **C** | wiring | `WiringEvalClaim::check_native` — evaluate the wiring ("monster") multilinear from the constraint system and compare it against the value the prover claimed |
| **D** | FRI opening | `channel.finish()` — where the batched BaseFold/FRI opening is actually verified: masking, the batched sumcheck, the combined FRI opening and its Merkle paths (vendor `crates/iop/src/basefold/channel.rs:103-127`), then `finalize` |

A+B+C+D is `check_proof` minus only a `StdChallenger::default()` construction. **D, not B, is
the FRI verifier proper.**

#### The decomposition

`log_inv_rate` 1, 1 thread, median of N = 5 (N = 3 at T1-c), one proof verified repeatedly.
Constraints are `imul + and + zero + bmul`, the full set the wiring multilinear ranges over —
not the MAC count.

| Task | constraints | verify ms | A statement (µs) | B reduction (ms) | C wiring (ms) | D FRI opening (ms) | **C share** | **C ns/constraint** |
|---|---|---|---|---|---|---|---|---|
| T1-0 | 139 008 | 6.31 | 2.4 | 0.288 | 5.39 | 0.592 | **85.4%** | **38.8** |
| T2 | 195 975 | 8.26 | 0.3 | 0.252 | 7.39 | 0.598 | **89.6%** | **37.7** |
| T1-a | 1 252 608 | 54.82 | 6.8 | 0.457 | 53.55 | 0.745 | **97.7%** | **42.8** |
| T3 | 1 567 800 | 60.23 | 0.2 | 0.272 | 59.10 | 0.826 | **98.1%** | **37.7** |
| T1-b | 5 010 432 | 198.43 | 26.6 | 0.933 | 195.87 | 0.970 | **98.7%** | **39.1** |
| T1-c | 20 041 728 | 934.04 | 102.7 | 2.912 | 929.93 | 1.067 | **99.6%** | **46.4** |

Three things fall out of that table, and only the first one was in question.

**1 · The growth is one term, and it is `check_native`.** C is 85% of verify at the bottom of
the ladder and 99.6% at the top. Its cost per constraint is **37.7–39.1 ns across a 36× range
of circuit size** (T1-0 through T1-b), which is what a linear term looks like when you divide
it by its own variable. T1-a reads 42.8 and T1-c reads 46.4; T1-c ran with the machine in
swap and its three repetitions fell monotonically, 1 050 → 930 → 804 ms, the last of which is
40.1 ns/constraint and back inside the band.

**2 · The FRI verifier is succinct, and is not the problem.** B + D — the constraint reduction
and the BaseFold/FRI opening together — goes **0.880 ms at T1-0 to 3.979 ms at T1-c, 4.52× for
144× the circuit**. That is the shape a FRI verifier is expected to have. The earlier text
said the measured growth was "not the shape a FRI verifier is expected to have"; that was
right about the total and wrong about the cause. The FRI part has the expected shape. A
different term sits next to it.

**3 · It is not our API call.** A — the only step that touches anything we supply — is 0.2 µs
to 103 µs, at most 0.03% of verify. `check_proof` is the authors' own route (see *How the
verify column is measured*). Dropping the prover before verifying, to rule out its retained
memory, moved T1-b by 0.9% (198.43 → 196.57 ms); that cell is in the grid below as
`t1-b-r1-t1-n5-dropprover`.

#### Why `check_native` is linear, and what its authors say about it

The wiring multilinear is evaluated by walking the constraint list. Vendor
`crates/verifier/src/protocols/shift/monster.rs:202-204`, in the native path's own comment:

> One unreduced wide product per constraint. The constraints partition cleanly across rayon:
> each produces a single wide element and they are summed […]

and at `:162`, in the generic path: *"One contribution per constraint."* The loop is
`self.constraints.par_iter()` over `imul`, `and`, `zero` and `bmul` in turn, with an inner
loop over each constraint's operands.

**binius64's authors document this as a gap they intend to close.** Vendor
`crates/iop/src/channel/mod.rs:94-96`:

> a non-witness-dependent oracle (e.g. **a pre-indexed commitment to the wiring matrix for
> succinctness, a planned feature**) is never masked

A pre-indexed commitment to the wiring matrix is precisely what would replace this Θ(constraints)
evaluation with an opening. It does not exist at the pinned commit, and the crate does not
claim it does. `crates/verifier/src/lib.rs:24-26` and `ARCHITECTURE.md:102-106` are also
explicit that verifier crates "optimize for readability over performance" and "avoid
parallelization" — so the linear term is not being presented as tuned.

This corroborates `R-004` in the project ledger, which held that verification complexity is
not yet succinct in Binius64. It was verified here against the pinned code, not carried over
from the ledger entry.

**One condition of this build, declared.** This harness enables `binius-examples/rayon`, which
unifies to `binius-utils/rayon` across the whole graph, so `monster.rs`'s `par_iter()` is real
rayon here rather than the single-threaded shim. The linear term is therefore the **only**
parallelizable part of the verifier — which is what the grid's 10-thread verify column shows:
1 → 10 threads buys 1.45× at T1-0, 2.78× at T1-a and 4.36× at T1-b, growing exactly as C's
share of the total grows.

#### The two rows, and which question each answers

The grid's `verify ms` is measured immediately after the proof it checks, in the same process.
The diagnostic verifies one proof repeatedly. At the bottom of the ladder these agree; above
it they diverge, and the divergence is first-touch page cost on the ~10²–10³ MB of tensors the
wiring evaluation allocates per call.

**That is measured, not argued.** Re-running the diagnostic with `--warmup 0` isolates the
first verify after a proof from the ones that follow it:

| Task | constraints | **published (grid)** | **cold: first verify after a proof** | **warm: steady state** | cold/warm |
|---|---|---|---|---|---|
| T1-0 | 139 008 | 6.29 | 7.12 | 6.31 | 1.13× |
| T2 | 195 975 | 8.38 | 9.77 | 8.26 | 1.18× |
| T1-a | 1 252 608 | 68.34 | 66.71 | 54.82 | 1.22× |
| T3 | 1 567 800 | 73.34 | 72.39 | 60.23 | 1.20× |
| T1-b | 5 010 432 | 593.00 | 528.93 / 553.73 / 613.97 | 198.43 | 2.7–3.1× |
| T1-c | 20 041 728 | 8 738.27 | **not measured** | 934.04 | — |

**The cold column reproduces the published column.** 66.71 against 68.34 at T1-a, 72.39
against 73.34 at T3, and three independent cold observations of T1-b — 528.93, 553.73 and
613.97 ms — bracketing the published 593.00. That is the mechanism identified, not inferred: in
`e006-bench` every measured verify follows a fresh proof, including the warmup, so every
measured verify is a cold one.

**Both rows are real, and they answer different questions.** The cold figure is what a
prove-then-verify pipeline in one process actually pays. The warm figure is what the verifier
costs when the pages are already resident — the figure to use for a verifier running as a
service. **Neither changes the shape**: both are linear in constraint count.

**T1-c's published 8 738.27 ms is the one figure the diagnosis materially revises.** It is
9.4× the steady-state 934.04 ms, far beyond the 2.7–3.1× cold penalty at T1-b, because that
cell was measured with the machine paging to disk — its `(user+sys)/real` of **0.3783** is in
the grid, and contaminant #3 below already refused to publish a throughput point estimate for
that rung. The steady-state figure carries its own caveat: N = 3, monotonically falling
across repetitions, taken on a machine still in swap.

#### The headline number, corrected

The earlier text led with **1 389× the verify time for 144× the circuit** and called the shape
unexplained. Against constraint count and at steady state:

**144.2× the constraints, 148.0× the verify time.** The growth is linear, with a coefficient
of roughly 38–40 ns per constraint on this machine at 1 thread. The 1 389× figure was that
linear growth multiplied by a cold-start penalty and, at T1-c, by swap.

This is still the metric on which this system looks weakest, and it is the one an author of
the system is most invited to correct us on; see [`CHALLENGE.md`](CHALLENGE.md).

#### What was not measured

- **T1-c cold.** The cold column has no T1-c entry. Producing one costs another ~3.5-minute
  proof at 93 GB peak footprint, and the rung's published figure is swap-dominated anyway, so
  the cold penalty could not be separated from swap there even if measured. Reported as a gap
  rather than estimated.
- **T1-d.** Never proved; see [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md).
- **Rate 4, and the 10-thread cells,** were not decomposed. The decomposition was run at
  `log_inv_rate` 1, 1 thread only. The 10-thread claim above is read off the existing grid, not
  off a decomposition.
- **Inside C.** The split stops at `check_native`. It does not separate the per-constraint loop
  from the word-index tensor expansion that precedes it, both of which are linear in a size
  that grows with the circuit. Which of the two dominates is **NOT DETERMINED**.

Raw per-repetition data: [`bench/data/verify-split/`](../../data/verify-split/). Reproduce
with [`bench/scripts/run-verify-split.sh`](../../scripts/run-verify-split.sh).

### 5 · Threads buy time. Threads do not buy memory.

This is the asymmetry `bench/README.md` was built around, and it shows up cleanly here.
1 thread against 10, `log_inv_rate = 1`:

| Task | MAC/s, 1 thr | MAC/s, 10 thr | speedup | B/MAC footprint, 1 thr | 10 thr | change |
|---|---|---|---|---|---|---|
| T1-0 | 366 160 | 902 994 | 2.47× | 8 155 | 8 285 | +1.6% |
| T1-a | 223 848 | 829 984 | 3.71× | 12 335 | 12 295 | −0.3% |
| T1-b | 120 869 | 242 423 | 2.01× | 11 684 | 11 927 | +2.1% |
| T2 | 288 333 | 839 790 | 2.91× | 11 335 | 10 587 | −6.6% |
| T3 | 284 101 | 1 024 164 | 3.60× | 10 138 | 10 168 | +0.3% |

**Ten cores buy 2.0–3.7× of throughput and change peak memory per MAC by less than 7% in
either direction.** Wall-clock time responds to hardware; peak memory does not. A machine
that cannot hold the circuit does not run the task faster by having more cores, and that is
the reason this benchmark reports `bytes/MAC` at all.

(Effective parallel efficiency is 20–37% of ten cores, consistent with the measured
`(user+sys)/real` of 3.4–5.4 in those cells; the cells are in the grid.)

### 6 · The cost shape, said plainly

**binius64 charges one 64×64 → 128 integer-multiply constraint per INT8 multiply-accumulate.**
The task needs an 8×8 → 16 product. This is not a mis-expression — `imul` is the system's
native multiply and using it is the configuration its authors document — but it means the
protocol is paying for 64-bit arithmetic to do 8-bit work, on a workload that is ~99%
8-bit multiply-accumulate.

The consequence is the memory column: **8 155 to 13 446 bytes of peak footprint per MAC**,
across three orders of magnitude of circuit size, with no sign of falling. Whether that is
good or bad is a question this file cannot answer, because no second system has been measured
yet. It is the number the comparison exists to put next to others.

## What contaminates these numbers, declared

1. **The machine is not dedicated.** 12 days uptime, ~9 GB of swap already committed at
   campaign start, browser and desktop applications running throughout. It is the machine
   E-001 and E-005 used, and using a different one would break comparability with them.
2. **T1-b and above swap heavily**, and `(user+sys)/real` shows exactly how much: 0.99 for
   the small cells, 0.87 at T1-b, **0.38–0.63 at T1-c**. At T1-c *more than half the CPU time
   was kernel time*, and 62% of wall-clock was waiting rather than computing.
3. **T1-c is dominated by machine state, not by the prover.** Three consecutive proofs of the
   *same circuit in the same process* took **183.6 s, 396.2 s, 432.7 s** — a 2.36× spread,
   monotonically increasing as swap degraded. An earlier N=1 run of the same cell measured
   169.3 s. We report the range and refuse to publish a point estimate for that rung's
   throughput.
4. **Memory figures include the verifier**, which runs in the same process. It is small next
   to the prover, but the number is not prover-only.
5. **N < 5 at T1-c** (N = 3 at rate 1, N = 1 at rate 4, warmup 0 in both), declared per cell
   in the grid rather than in a footnote. A single discarded warmup proof at that rung costs
   three minutes.
6. **No extrapolation.** Nothing here is projected outside the rungs actually measured. The
   ladder stops where it stops and `NOT_EXPRESSIBLE.md` says why.

## Correctness control

**Blocking, and it passed.** A corrupted trace must make `verify()` fail, or none of the
above may be published. See `bench/data/negative-control.csv` for the raw table and
`EXPRESSION.md` for what each mode corrupts.

**43 corruption attempts across 6 tasks. 43 rejected by `verify()`. 0 accepted.**

| Task | attempts | VERIFY_REJECTED | PROVER_ERROR | VERIFY_ACCEPTED | verdict |
|---|---|---|---|---|---|
| T1-0 | 8 | 8 | 0 | 0 | PASS |
| T1-a | 8 | 8 | 0 | 0 | PASS |
| T1-b | 8 | 8 | 0 | 0 | PASS |
| T1-c | 3 | 3 | 0 | 0 | PASS |
| T2 | 8 | 8 | 0 | 0 | PASS |
| T3 | 8 | 8 | 0 | 0 | PASS |

Three things about this table matter more than the totals.

**Every rejection came from the verifier, not from the prover.** `PROVER_ERROR` is zero: in
every case binius64 *produced* a proof from the corrupted trace and the verifier then refused
it. That is the stronger of the two ways to pass — a system that merely crashed on bad input
would also score 43/43 here and would tell us much less.

**The control is not vacuous.** Each task first proves and verifies an *honest* witness, and
the run aborts if that fails. A negative test that passes because nothing ever verifies
proves nothing, so the honest proof is checked first, every time.

**Two caveats on the count, so it is not read as better than it is.** T2 has exactly one
`inout` word (the model's scalar prediction), so its `inout_word/first` and
`inout_word/last` are the same corruption run twice — T2 exercises **7 distinct**
corruptions, not 8, and the honest total is 42 distinct across the six tasks. And **T1-c was
run in reduced mode** (`--quick`: one corruption per family instead of three) because each
attempt there costs a full ~3-minute proof; it exercises all three families, at one position
each.

### Amendment A3 — the `private_word` family on T2 and T3 is WEAK EVIDENCE

`bench/TASKS.md` Amendment A3 (2026-08-24): *a corruption counts as a test only if it changes
the output.* **No verdict above is withdrawn — binius64 accepted nothing — but six rows are
re-labelled**, and the label is in the table, not a footnote.

| task | family | positions | status under A3 |
|---|---|---:|---|
| T2 | `private_word` (first, middle, last) | 3 | **WEAK EVIDENCE** |
| T3 | `private_word` (first, middle, last) | 3 | **WEAK EVIDENCE** |
| T1-0, T1-a, T1-b, T1-c | `private_word` | 10 | unaffected — a matmul has no inert weights |
| all six tasks | `inout_word` | 11 (10 distinct) | unaffected — corrupting a public output changes the statement by construction |
| all six tasks | `proof_byte` | 16 | **unaffected — this remains the strong control** |

Counts read off [`negative-control.csv`](../../data/negative-control.csv) row by row; they sum
to the 43 attempts above (10 + 6 + 11 + 16), and the "10 distinct" is T2's single `inout` word
being corrupted twice, already declared.

**Why.** T2 and T3 carry ReLUs, and a witness value discarded by a ReLU can be perturbed
without moving the output. Measured exhaustively by the gnark campaign on the same task
specification: **48 208 of T2's 92 224 weights — 52.27 % — are inert**, and 3 016 of T3's
(3.27 %). A perturbed inert value is still a valid witness for the same true statement, so
accepting it would be correct behaviour and rejecting it proves nothing about detection. **We
never recomputed the reference forward pass for these six positions**, so we cannot say whether
they changed the output, and the rows therefore carry less information than they appear to.

**What does not change, and why it is not nothing.** binius64 *rejected* all six, and A3's
mechanism does not explain that away: this control mutates one word of the `ValueVec` **after**
an honest witness was built and does **not** re-derive the wires downstream of it, so the
constraint system is violated whether or not the output moves. That is a different failure
mode from a re-solved circuit, and it is why gnark — which re-solves — accepted two inert
positions on the same task where binius64 rejected one whose original value was literally
`0x0000000000000000` (`private[190311]`, T2, in the CSV above). **The two systems bind
different things; neither is better.** See `bench/RESULTS.md` for the column.

**What would discharge this properly**: re-run the `private_word` family with the reference
forward pass recomputed per position, reporting `WITNESS_INERT` for positions that leave the
output unchanged. Not run in this campaign, and reported as a gap rather than estimated.

What is corrupted, and why those three things:

| Mode | What changes | What it tests |
|---|---|---|
| `private_word` | one committed private word — an INT8 weight or an internal wire, mutated directly in the `ValueVec` after an honest witness was built | **This is "a corrupted trace"**: the prover's secret data no longer satisfies the constraint system while the public claim is untouched |
| `inout_word` | one public output word | the prover claims a different result for the same model and input |
| `proof_byte` | one byte of the serialized transcript, after an honest proof | the proof artifact itself is tampered with |

The corruption is a single-bit flip (`XOR 1`) applied through `ValueVec::word_mut`, which
bypasses the frontend's own witness-population checks — otherwise the frontend would catch
the inconsistency before the prover ever ran, and the control would be testing the wrong
layer. Exact before/after values for every attempt are in
[`bench/data/negative-control.csv`](../../data/negative-control.csv).

## Build integrity held for the whole campaign

The field-multiply probe was run before and after. Before: `binius / hand-written` =
0.983–0.996. After, with 37 GB of swap committed: **0.969–0.987**. The gate's floor is 0.50
and E-001 measured 0.991. No build drift across the campaign. Raw output in
[`probe-fieldmul-before.txt`](../../data/probe-fieldmul-before.txt) and
[`probe-fieldmul-after.txt`](../../data/probe-fieldmul-after.txt); the unexplained absolute-level
anomaly is in [`BUILD.md`](BUILD.md) §2.
