# gnark — results

**Read [`REPRODUCTION.md`](REPRODUCTION.md) first.** gnark is the only system in this
benchmark whose authors ship a runnable benchmark, and we ran it. **Their instrument
reproduces; their published number does not** — 2.79–2.86·10⁵ constraints/s here against a
published ">2·10⁶", a gap of about 7.0×, with the hardware differences declared and explicitly
not claimed as the explanation.

Then read **§0 below**, which governs every figure in this file: gnark was measured in **two
regimes**, they prove **different statements**, and they are never averaged or plotted together.

Then [`BUILD.md`](BUILD.md) §3 (the assembly is in the binary and buys 7.5 %), §4 (this
machine gives gnark the element assembly but **not** the vector assembly, and that plays
against gnark), and §8 (machine state, including a boot volume 95 % full that killed a cell).

And [`EXPRESSION.md`](EXPRESSION.md) §1 and §2, which govern what the constraint counts mean.

---

## 0 · THE DECLARATION THAT GOVERNS EVERY FIGURE BELOW

`bench/TASKS.md` fixes shapes, MAC counts, seeds, dtype and — via Amendment A1 —
requantization. **It does not fix whether the weights are public, private, committed, or
constant.** All four previously measured systems resolved that silently and differently (§8.2).
gnark makes the choice visible, because in a circuit both positions are available:

| | **Regime A — the headline** | **Regime B — a declared lever** |
|---|---|---|
| weights are | secret witness, range-checked | Go constants baked into the circuit |
| the proof says | "there exist INT8 `x` **and INT8 `W`** with `out = W·x`" | "there exists INT8 `x` with `out = W₀·x`", where `W₀` is fixed by the verifying key |
| binds the weights? | **no** — an alternative satisfying witness verifies (§6) | **yes** — to the vk, by Groth16's per-circuit setup |
| comparable to the other four systems? | yes, with §8.1's caveat | **no. Never quote it in a cross-system ratio.** |
| T1-0 constraints | 197 763 | 1 026 |

**Regime A is the number that goes in the five-system table. Regime B is reported because it
is `PROTOCOL.md` §2's never-evaluated lever "exploiting fixed weights by precomputation", and
because it is the configuration a deployed fixed-model service would actually ship.** The two
never appear in the same ratio without this paragraph.

## Conditions line

Applies to every figure below. Where a cell differs, the cell says so in its own row.

```
system      gnark
commit      9838556b92c7783cb82971cf37c0d081cc2b6aec (v0.16.2). NO local modifications;
            the module cache was verified byte-identical to the clone, 0 differing paths
            (BUILD.md §2). Unlike Ceno, no patch was needed to build on aarch64.
task        expressed as R1CS (Groth16) and SCS (PLONK) circuits over BN254; MACs = the count
            frozen in bench/TASKS.md, asserted by the builder against the emitted circuit and
            never recomputed. A mismatch refuses to emit.
constraints REPORTED, and NOT a shared axis with any other system in this benchmark. For
            Groth16 a constraint is one R1CS row A·B=C, in which a linear combination with
            CONSTANT coefficients is free; for PLONK it is one fan-in-2 gate, in which it is
            not. The same 65 536 MACs are 197 763 constraints or 1 026 depending only on
            whether the weights are witness or baked in. See §2 and the warning in §8.
field       BN254 scalar field, r ≈ 2.19·10^76 (254 bits). No INT8/INT32 quantity in any task
            can overflow it; the A1 bound is checked against int64 in the reference
            computation, which CAN overflow, not against the field, which cannot.
PCS         Groth16: none (pairing-based, per-circuit SRS). PLONK: KZG.
security    NOT post-quantum. Both backends rest on pairing assumptions over BN254, whose
            conjectured security is ~100 bits and below the 128-bit level often assumed.
            binius64 publishes SECURITY_BITS = 96 by a different accounting and Ceno names an
            enum "Conjecture100bits"; NONE of the three are compared here.
audits      NINE third-party audits, five vendored in the tree (COMMIT). The PLONK prover and
            verifier were audited (OpenZeppelin 2024-06); for Groth16 what was audited is the
            Solidity verifier template, not the Go prover. gnark is the ONLY system in this
            benchmark that is not self-described as research/prototype. We did not read the
            reports and do not restate their findings.
trusted setup   YES — and the two backends differ in a way a y/n column destroys.
                Groth16: PER CIRCUIT. A new weight matrix, a new layer width, one more
                constraint past a power of two — all require a new setup.
                PLONK: UNIVERSAL SRS, one per size bound, reusable across circuits.
                BOTH were run here with gnark's own test utilities, which are NOT an MPC
                ceremony. `test/unsafekzg` says "to be use for test purposes only" in its own
                docstring. NO FIGURE HERE IS A CEREMONY-BACKED SETUP.
ZK              Groth16 default path is NOT statistically zero-knowledge; gnark exposes
                `WithStatisticalZeroKnowledge` as an opt-in that its own docstring says
                "makes the prover more memory costly, as there are 3 more size n
                allocations". We did NOT enable it. So: ZK no, and the kind of no is
                declared rather than shared with the other four.
quantization    signed INT8 in [-128,127] mapped to field elements; INT8-ness is PROVED, not
                structural — `std/rangecheck` with `rc.Check(v+128, 8)`. This is the prime
                field's tax and it has no counterpart in a binary-field system. Cost per
                value is NOT a constant: 4.19 constraints at n=16 falling to 2.01 at
                n=65 792, because it amortizes a shared lookup table (EXPRESSION.md §2).
requantization  NONE, per bench/TASKS.md Amendment A1.
weights         DECLARED, because bench/TASKS.md does not fix it (see §8).
                Regime A (HEADLINE, comparable): weights are secret witness and range-checked.
                Regime B (DECLARED LEVER, never mixed into a cross-system number): weights are
                circuit constants, bound into the verifying key by Groth16's per-circuit setup.
padding         The TASK is never padded — no reshaping, no minimum layer width, no
                power-of-two rung requirement. But the FFT DOMAIN is padded to the next power
                of two, measured from the key actually built, and PEAK MEMORY FOLLOWS THE
                DOMAIN, NOT THE CONSTRAINT COUNT. One constraint past 1024 moves the domain to
                2048 and the proving key from 100 957 to 133 791 bytes (+32.5 %). Measured
                padding ratios across the grid run 1.013 to 1.998 and are in every row.
batching        T3 is 8 inputs in ONE proof, and the weights are SHARED across the 8, which is
                why T3 costs less per MAC than T2.
segmentation    NONE. gnark exposes no shard cap, no streaming prover and no memory limit.
                The only lever that moved peak memory is the Go runtime's GC (§4).
allocator       Go runtime GC, default GOGC=100, no GOMEMLIMIT, and `debug.FreeOSMemory()` is
                NEVER called. Calling it would report a peak the prover did not actually hold.
                This is a different accounting from a Rust jemalloc/system-allocator process
                and it is declared, not normalized away (BUILD.md §4).
threads     GOMAXPROCS. gnark does NOT round it to a power of two, so unlike Ceno the
            nominal count is the count. 1 thread IS available and was measured — Ceno's
            prover aborts at 1 thread, so gnark restores comparability with binius64's
            primary cut.
machine     Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, NOT dedicated,
            ON AC POWER (charging), boot volume 95 % full, swap heavily used
            (~8.3 GiB of 9-10 GiB in use at campaign start). Per-cell loadavg and swap in
            the ledger.
            NOTE: the reproduction runs in REPRODUCTION.md were taken ON BATTERY under load
            averages of 12-30 and say so; the grid below was not.
asm         gnark-crypto v0.21.0 gives arm64 assembly for BN254 element ops (`mul`, `reduce`,
            `Butterfly` — 3 routines, 163 lines) but NOT for vector ops: `vector_purego.go`
            is tagged `purego || !amd64` and there is no `vector_arm64.go`, while amd64 ships
            13 routines / 1862 lines including addVec, mulVec, innerProdVec and AVX-512 IFMA.
            THIS MACHINE THEREFORE RUNS gnark PARTLY DEGRADED, and it plays AGAINST gnark.
OS          macOS 26.5.2 (25F84), Darwin 25.5.0, uptime 13 days
N           per cell, in the table
date        2026-08-24
```

## What is inside each measured quantity

| Column | What it contains | Same bracket as binius64? |
|---|---|---|
| `prove ms` | `groth16.Prove` / `plonk.Prove` alone, median of N | Yes |
| `setup s` | `groth16.Setup` / `plonk.Setup`, **reported apart and NEVER amortised into prove** | Yes |
| `srs s` | PLONK only: universal SRS generation, **also apart**. A one-off, reusable across circuits | n/a |
| `peak RSS` / `peak fp` | the **whole process**: compile, setup, witness, prove, verify | Yes |
| `proof B` | the serialized artifact, `WriteTo` (compressed) | Yes |
| `verify ms` | `Verify` alone, against a vk held in process | Yes |
| `(u+s)/real` | whether wall-clock was computation or waiting | Yes |

**Compile and setup are inside the memory column but outside the prove-time column.** That is
gnark's own bracket, declared. Process wall time is in the ledger beside prove time.

## The full grid

37 cells, every one that was run, uncurated. Raw:
[`bench/data/cells-gnark.csv`](../../data/cells-gnark.csv). Constraint counts for all 28
task×backend×regime combinations from compilation alone, including the ones too large to prove:
[`compile-grid-gnark.csv`](../../data/compile-grid-gnark.csv). **Cells that produced no figure,
and why:** [`grid-gaps.md`](../../data/repro-gnark/grid-gaps.md).

### Regime A · Groth16 · 10 threads — THE HEADLINE

| Task | MACs | **constraints** | domain | N | prove s | **MAC/s** | peak fp | peak RSS | **B/MAC fp** | **B/MAC rss** | setup s | proof B |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 197 763 | 2¹⁸ | 5 | 0.663 | **98 920** | 0.619 GB | 0.621 GB | **9 443.8** | 9 473.0 | 14.11 | 196 |
| T2 | 92 224 | 283 408 | 2¹⁹ | 5 | 1.114 | **82 795** | 1.041 GB | 1.049 GB | **11 284.4** | 11 372.7 | 21.50 | 196 |
| T3 | 737 792 | 973 058 | 2²⁰ | 3 | 1.941 | **380 048** | 1.801 GB | 1.807 GB | **2 441.5** | 2 449.4 | 38.58 | 196 |
| T1-a | 589 824 | 1 774 726 | 2²¹ | 3 | 5.771 | **102 204** | 4.213 GB | 4.217 GB | **7 142.2** | 7 148.8 | 137.29 | 196 |
| T1-b | 2 359 296 | 3 555 722 | 2²² | 1 | 7.773 | **303 509** | 5.411 GB | 5.414 GB | **2 293.3** | 2 294.7 | 161.89 | 196 |
| T1-c | 9 437 184 | 10 679 708 | 2²⁴ | 1 | 22.668 | **416 330** | **18.868 GB** | **6.961 GB** | **1 999.3** | **737.6** | 355.14 | 196 |
| T1-d | 37 748 736 | 39 175 652 | 2²⁶ | 0 | — | — | — | — | — | — | — | — |

**T1-d produced no proof.** It was killed at ~17 minutes by **our own disk watchdog**, because
the cell's memory demand grew macOS's swap file from 9 GB to 32 GB and pushed free space on `/`
below the guard's 20 GiB floor. **That is a property of this machine at 95 % disk, not of
gnark** — gnark *compiled* T1-d regime A in 89.13 s. The ceiling is published as a measured
interval and nothing inside it is claimed: **largest that proved, T1-c at 10 679 708
constraints; smallest that did not, T1-d at 39 175 652.** Full causal chain in
[`grid-gaps.md`](../../data/repro-gnark/grid-gaps.md).

**The T1-c row carries an asterisk and it is in the table, not a footnote.** Its peak footprint
(18.868 GB) is **2.71× its peak RSS** (6.961 GB), where every other cell in the campaign has
the two within 1 %. That divergence is memory pressure — the process was being compressed and
paged while it ran. So T1-c's wall-clock and its footprint were both taken on a machine already
paging, and **`B/MAC` for T1-c is 1 999.3 or 737.6 depending on which memory you mean.** Both
are published. Neither is gnark's best achievable figure at that size.

### Regime A · PLONK · 10 threads

| Task | MACs | constraints | N | prove s | MAC/s | peak fp | B/MAC fp | setup s | **SRS s** | proof B |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 526 592 | 5 | 9.016 | 7 269 | 2.612 GB | 39 850.8 | 2.11 | 16.06 | 584 |
| T2 | 92 224 | 758 448 | 5 | 9.654 | 9 553 | 2.707 GB | 29 354.8 | 2.40 | 14.94 | 584 |
| T3 | 737 792 | 2 188 800 | 3 | 36.661 | 20 125 | 8.733 GB | 11 836.9 | 7.95 | 64.17 | 584 |
| T1-a | 589 824 | 4 723 968 | 3 | 95.126 | 6 200 | 16.806 GB | 28 493.7 | 15.46 | 119.74 | 584 |
| T1-b | 2 359 296 | 8 276 736 | 1 | 75.843 | 31 108 | 14.529 GB | 6 158.4 | 16.04 | 121.97 | 584 |
| T1-c, T1-d | — | — | 0 | — | — | — | — | — | — | — |

T1-c and T1-d PLONK regime A were **not attempted**, after the Groth16 regime-A cell at less
than half T1-c's PLONK constraint count was killed. Declared, not estimated.

### Regime B · both backends — A DECLARED LEVER, never mixed into a cross-system number

| Task | MACs | constraints | prove s | **MAC/s** | peak fp | **B/MAC** | setup s | proof B |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 1 026 | 0.010 | 6 330 757 | 0.018 GB | 273.0 | 0.10 | 196 |
| T2 | 92 224 | 6 555 | 0.042 | 2 178 280 | 0.035 GB | 383.2 | 0.54 | 196 |
| T3 | 737 792 | 41 724 | 0.200 | 3 685 956 | 0.200 GB | 271.6 | 3.17 | 196 |
| T1-a | 589 824 | 4 099 | 0.027 | 21 778 385 | 0.038 GB | 63.7 | 0.29 | 196 |
| T1-b | 2 359 296 | 15 624 | 0.070 | 33 904 751 | 0.096 GB | 40.5 | 1.10 | 196 |
| T1-c | 9 437 184 | 61 722 | 0.195 | 48 423 876 | 0.334 GB | 35.4 | 3.05 | 196 |
| **T1-d** | **37 748 736** | **246 114** | **0.723** | **52 218 259** | **1.243 GB** | **32.9** | 11.58 | 196 |

PLONK regime B: T1-0 1.119 s / 0.331 GB, T2 1.289 s / 0.344 GB, T1-a 8.416 s / 2.768 GB,
T3 8.616 s / 2.472 GB — all 584-byte proofs, all in the ledger.

**The row that matters to the decision this benchmark was commissioned for is T1-d.** The
largest task in the benchmark — 37 748 736 MACs — proves in **0.72 s using 1.24 GB**, at
**32.9 bytes per MAC**, in the regime where the model is bound to the verifying key. The same
task in regime A could not be proved on this machine at all. **That is the largest single
effect measured anywhere in this campaign**, and §8.2 argues it is also the honest one for a
fixed-model deployment.

## 1 · The memory curve, which is the thing this repository exists to measure

Groth16, regime A, 10 threads, default GC. **The curve is the result; no single point on it is
a property of the prover.**

| Task | MACs | constraints | peak footprint | **B/MAC** | vs previous rung | **local exponent** |
|---|---:|---:|---:|---:|---|---:|
| T1-0 | 65 536 | 197 763 | 619 MB | **9 443.8** | — | — |
| T1-a | 589 824 | 1 774 726 | 4 213 MB | **7 142.2** | MACs ×9.00, memory ×6.807 | **0.873** |
| T1-b | 2 359 296 | 3 555 722 | 5 411 MB | **2 293.3** | MACs ×4.00, memory ×1.284 | **0.181** |
| T1-c | 9 437 184 | 10 679 708 | 18 868 MB | **1 999.3** | MACs ×4.00, memory ×3.487 | **0.901** |

**The exponent against MACs is not stable, and the reason is the whole point of §8.1.** It
collapses to 0.181 at T1-b and returns to 0.901 at T1-c. That is not noise: **T1-a through T1-d
share one [768×768] weight matrix**, so the ~590 000 weight range checks are a *fixed* cost
across those four rungs while the multiplications grow ×4 each step. Constraints therefore grow
×8.97, ×2.00, ×3.00 — not ×9, ×4, ×4 — and memory follows the constraints, not the MACs.

Against the unit memory actually follows:

| step | constraints | memory | **local exponent vs constraints** | B/constraint |
|---|---:|---:|---:|---:|
| T1-0 → T1-a | ×8.97 | ×6.807 | **0.874** | 3 129.5 → 2 373.7 |
| T1-a → T1-b | ×2.00 | ×1.284 | **0.360** | → 1 521.7 |
| T1-b → T1-c | ×3.00 | ×3.487 | **1.136** | → 1 766.7 |

**So `B/MAC` falls 4.7× across the ladder (9 444 → 1 999) while `B/constraint` moves only
1.8× and not monotonically.** `bench/README.md`'s third finding — that `bytes/MAC` is not a
constant of a proof system — holds here for a *different reason* than in Ceno. Ceno's fell
because a ~5 GB fixed floor was being spread thinner. gnark has **no such floor**: T1-0, the
smallest task, peaks at **0.619 GB**, and regime B's T1-0 at **0.018 GB**. gnark's `B/MAC`
falls because the range-check cost that dominates the small rungs is amortised as `M` grows.

**Shape, stated against Ceno's because the contrast is the finding.** Ceno: huge intercept
(~5 GB before any task-dependent work), shallow slope, first local exponent **0.412**. gnark:
negligible intercept, near-linear slope, first local exponent **0.873**. Two systems whose
`B/MAC` curves cross — which is exactly the "map of cost shapes" `PROTOCOL.md` §11 says this
benchmark is for, and it is why a single-point comparison between them would be meaningless in
either direction.

**Nothing here is extrapolated.** The largest measured Groth16 regime-A point is T1-c;
`bench/CHALLENGE.md` forbids saying what happens past it, and T1-d's failure (a disk watchdog,
on this machine) is not evidence about gnark's asymptote.

## 2 · Batching: what T3 buys, measured

T2 and T3 at the same backend, regime, threads and GC — one comparison, not two campaigns.

| | T2 | T3 | ratio |
|---|---:|---:|---:|
| MACs | 92 224 | 737 792 | ×8.00 |
| constraints | 283 408 | 973 058 | **×3.43** |
| prove s | 1.114 | 1.941 | **×1.743** |
| peak footprint | 1 041 MB | 1 801 MB | **×1.730** |
| proof bytes | 196 | 196 | **×1.000** |

**Per request, batching 8 inferences into one proof buys 4.59× on prove time, 4.62× on peak
memory, and 8.00× on proof size.**

**And this is a protocol-shape result, not an implementation detail.** The weights are shared
across the 8 batch items, so the weight range checks — the dominant cost at this size — are
paid **once for all eight**. That is why constraints grow ×3.43 rather than ×8.

The contrast with the zkVM is sharp and it runs the other way: **Ceno's T2→T3 prove time was
×6.58**, essentially linear, because a zkVM proving eight inferences executes eight times the
instructions and there is no protocol-level saving available. Ceno's proof also grew ×1.20;
gnark's Groth16 proof is **196 bytes for one inference and 196 bytes for eight**.

## 3 · Two backends, one system: the trade a `trusted setup y/n` column destroys

Regime A, 10 threads, same tasks, same machine.

| Task | | prove s | peak fp | proof B | **per-circuit setup s** | **universal SRS s** | verify ms | vk bytes |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | Groth16 | **0.663** | **0.619 GB** | **196** | 14.11 | — | 2.13 | 8 688 |
| T1-0 | PLONK | 9.016 | 2.612 GB | 584 | **2.11** | 16.06 | 1.73 | 34 384 |
| T1-a | Groth16 | **5.771** | **4.213 GB** | **196** | 137.29 | — | 2.75 | 25 072 |
| T1-a | PLONK | 95.126 | 16.806 GB | 584 | **15.46** | 119.74 | 3.35 | 34 384 |

**Groth16 proves 13.6–16.5× faster, on 4.0–4.2× less memory, with a 2.98× smaller proof.
PLONK's per-circuit setup is 6.7–8.9× cheaper, and its SRS is universal.** Part of the prove
gap is structural: the same task is **2.66× more constraints** in SCS than in R1CS (§8.3),
because a PLONK gate is fan-in-2 and folds nothing while an R1CS row absorbs a whole
constant-coefficient linear combination for free.

**Two things this makes visible that a y/n column cannot.**

1. **Groth16's setup is per circuit.** Change a weight, a layer width, or cross a power-of-two
   boundary and it must be redone. Here it is the dominant cost at every rung — **137 s of
   setup for 5.8 s of proving at T1-a**, and 355 s at T1-c. A new model version means a new
   ceremony. PLONK's SRS is generated once and reused.
2. **The verifying key behaves oppositely in the two backends.** Groth16's vk grows with the
   number of public inputs — 8 688 B at T1-0 (257 public) to **393 712 B at T1-c** — and
   verify time with it (2.13 → 8.26 ms). **PLONK's vk is 34 384 bytes at every single task in
   the grid**, and verify stays 1.6–3.3 ms.

**Neither setup here is ceremony-backed.** PLONK's SRS comes from `test/unsafekzg`, whose own
docstring says "to be use for test purposes only". No figure in this file is a ceremony-backed
setup, and none should be read as one.

## 4 · The memory knob: gnark has none; the Go runtime has one, and it is nearly free

**gnark exposes no protocol-level memory lever.** No segmentation, no shard cap, no streaming
prover, no memory limit. `backend.ProverOption` offers nothing of the kind;
`solver.WithNbTasks` caps solver goroutines; `WithStatisticalZeroKnowledge` moves memory the
*wrong* way by its own docstring ("3 more size n allocations"). **For the classical circuit
anchor, "no lever" is the answer**, and it is the answer Ceno's shard sweep makes worth asking.

What does move it is the Go garbage collector. T1-0, Groth16, regime A, 10 threads, N=3, only
`GOGC` moving:

| `GOGC` | peak footprint | prove ms | proof B | setup s |
|---:|---:|---:|---:|---:|
| 400 | **1 230 MB** | 679.0 | 196 | 13.41 |
| 100 (default) | 602 MB | 672.4 | 196 | 13.40 |
| 50 | 441 MB | 683.1 | 196 | 13.58 |
| 25 | **374 MB** | 682.2 | 196 | 13.47 |

**Peak footprint falls 3.29× (1 230 → 374 MB) for a prove-time change of 1.005× — within this
run's noise — and no change at all to proof size or setup.**

Set against the only other memory lever in this benchmark: **Ceno's segmentation bought ÷1.50
in memory for ×2.20 in time and ×9.84 in proof bytes.** gnark's GC knob buys **÷3.29 for
essentially nothing.**

**But it is a different kind of lever and the difference matters more than the ratio.** Ceno's
shard cap changes *what is proved* — more shards, bigger proof, longer verify. `GOGC` changes
only how much garbage the Go runtime tolerates before collecting; it cannot shrink the proving
key or the QAP, so it bottoms out at the true working set. It is a knob on *our accounting*,
not on the protocol.

**And that has a consequence for the memory column of the five-system table.** gnark's default
figures are taken at `GOGC=100`, where the same task holds **602 MB** against **374 MB** at
`GOGC=25` — so **the default-GC number overstates the working set by ~1.61×** relative to what
a non-GC language would report. The other four systems are Rust processes with no equivalent
slack. This is declared in `BUILD.md` §7 and is **not** corrected for.

Two more axes, both negative results, published because they were run:

- **`GOMEMLIMIT=2GiB`**: peak fp 618 MB against 619 MB at default — no effect, because the task
  never approached the limit.
- **`solver.WithNbTasks` ∈ {1, 2, 4, 10}**: peak fp 551 / 664 / 607 / 639 MB, prove 675.3 /
  666.3 / 673.0 / 665.2 ms. **No systematic effect on either axis** at this size.

## 5 · Threads buy time. Threads do not buy memory.

Groth16, regime A, `GOMAXPROCS`. **gnark does not round the thread count to a power of two**,
so unlike Ceno the nominal count is the count.

| Task | threads | prove ms | speedup | (u+s)/real | peak fp | memory change |
|---|---:|---:|---:|---:|---:|---:|
| T1-0 | 1 | 4 547.6 | — | 0.997 | 559 MB | — |
| T1-0 | 2 | 2 336.7 | ×1.946 | 1.921 | 576 MB | +3.0 % |
| T1-0 | 4 | 1 221.2 | ×3.724 | 3.558 | 619 MB | +10.7 % |
| T1-0 | 10 | 662.5 | **×6.863** | 6.345 | 619 MB | **+10.7 %** |
| T2 | 1 | 7 037.9 | — | 0.998 | 906 MB | — |
| T2 | 10 | 1 113.9 | **×6.318** | 6.455 | 1 041 MB | **+14.9 %** |

**7× the threads buys 6.86× the speed and costs 10.7 % more memory** — the same shape all five
systems in this benchmark show, and the reason `bench/README.md` calls memory a binary gate
rather than a performance detail. `(u+s)/real` tracks the nominal thread count closely (0.997
at 1, 6.345 at 10), so these cells were computing rather than waiting.

**And the 1-thread row exists.** binius64's primary cut is 1 thread; **Ceno's prover aborts
there** and has no such row. gnark restores that comparison — with §8.1's caveat that gnark
regime A is proving more than binius64 is.

Setup parallelises too, and harder than proving: 81.32 s → 14.11 s from 1 to 10 threads
(×5.76).

## 6 · Correctness control

`bench/README.md`: *"A corrupted trace must make `verify()` fail, in every system, on every
task."*

**Two positive controls first**, because a negative test that passes because nothing ever
verifies proves nothing:

| Control | Result |
|---|---|
| the honest proof verifies | **VERIFY_ACCEPTED** |
| serialize → deserialize → verify, unmodified | **VERIFY_ACCEPTED** — the method itself does not corrupt |

### The sweep is EXHAUSTIVE, in both backends

The Groth16 proof is 196 bytes and the PLONK proof is 584, so there is no excuse to sample —
and Ceno's entry is the reason it matters: its sweep covered **1.56 %** of offsets and it says
so, because jolt-atlas's exhaustive sweep had found an accepted region a 124-offset sample hit
only once by luck.

| Sweep | proof B | offsets | DESERIALIZE_REJECTED | VERIFY_REJECTED | **VERIFY_ACCEPTED** |
|---|---:|---:|---:|---:|---:|
| T1-0 Groth16 rA | 196 | **196 — every byte** | 132 | 64 | **0** |
| T1-0 Groth16 rB | 196 | **196** | 133 | 63 | **0** |
| T2 Groth16 rA | 196 | **196** | 135 | 61 | **0** |
| T2 Groth16 rB | 196 | **196** | 128 | 68 | **0** |
| T1-0 PLONK rA | 584 | **584 — every byte** | 162 | 422 | **0** |

**Verdict: PASS. No corrupted proof was accepted at any offset, in any task, in either
backend.** Plus `public_input_word` exhaustive over T1-0's 256 public outputs — **256/256
rejected** — and every `witness_word` position caught at proving time.

**The deserialize/verify split is NOT a stable figure and is not quoted as one.** Repeated
sweeps of the same task gave 132/64 and 125/71, and PLONK gave 422/162 and 410/174, because
the Groth16 prover is randomised and each sweep corrupts a different artifact. **The zero is
stable; the split is not.**

### The byte map, which the licence lets us publish

Derived from `backend/groth16/bn254/marshal.go:33-57` and **verified against the artifact**
(derived 196 = measured 196):

| offsets | field | rejections |
|---|---|---|
| 0–31 | `Ar`, G1 compressed | 19 deser / 13 verify |
| 32–95 | `Bs`, G2 compressed | **64 deser / 0 verify** — no flip of a G2 byte reaches the verifier |
| 96–127 | `Krs`, G1 compressed | 16 / 16 |
| 128–131 | `Commitments` slice length prefix | 3 / 1 |
| 132–163 | `Commitments[0]`, G1 compressed | 13 / 19 |
| 164–195 | `CommitmentPok`, G1 compressed | 17 / 15 |

**The per-region split is recomputed from the committed CSV
([`t1-0-groth16-rA-exhaustive.csv`](../../data/negative-gnark/t1-0-groth16-rA-exhaustive.csv)),
and like the totals it is run-specific.** An earlier draft of this table carried per-region
counts from a *different* sweep; the totals matched (132/64 either way), which is precisely why
the discrepancy survived a first reading and was caught only by recounting the file offset by
offset. **What is stable across every sweep is the zero**, and the structural facts below it:
`Bs` never reaches the verifier, and every region rejects everything.

**Proof body separated from envelope, measured rather than assumed.** gnark's own example
circuits use no range checks, carry no commitment, and produce a **164-byte** Groth16 proof
(`BUILD.md` §6). Ours is 196. **196 − 164 = 32 = one compressed G1 point** — the Pedersen
commitment `std/rangecheck` adds via `api.Commit`. So gnark's proof size in this benchmark is
**coupled to the INT8 encoding**: the prime field's range checks are why the artifact is 196
bytes and not 164.

jolt-atlas and DeepProve could not do this: their licences forbid the reverse engineering that
would name a region, and both left offsets as `NOT DETERMINED`. **That is a licence
difference, not an opacity difference.**

### The finding that is about the systems, not about our harness

The first run of the `witness_word` family reported two **accepted** corruptions on T2 regime A:

    t2,witness_word,W[46112],plus1,VERIFY_ACCEPTED
    t2,witness_word,W[69168],plus1,VERIFY_ACCEPTED

**It is not unsoundness, and the mechanism was established before the conclusion was written**
([`MECHANISM-t2-accepted-witness.md`](../../data/negative-gnark/MECHANISM-t2-accepted-witness.md)),
four positions for four:

| index | neuron | pre-activation, before → after `w+1` | ReLU output | verdict |
|---|---|---:|---|---|
| W[0] | L0 n0 | 6 310 → 6 379 | changes | REJECTED |
| W[23056] | L0 n16 | 13 447 → 13 429 | changes | REJECTED |
| **W[46112]** | L0 n32 | **−11 129 → −11 257** | **0 → 0** | **ACCEPTED** |
| **W[69168]** | L1 n48 | **−87 033 032 → −86 997 218** | **0 → 0** | **ACCEPTED** |

**A ReLU is not injective.** A weight feeding a neuron whose pre-activation is negative before
*and* after the change is discarded by the activation, so the network output is bit-identical
(14 623 789 560 139 either way) and the perturbed witness is a **genuinely satisfying witness
for the same true statement**. Accepting it is correct; rejecting it would mean rejecting a
true statement.

**And here is why this belongs in the comparison and not only in our errata.** On the **same
task and the same corruption class**, binius64 returns **VERIFY_REJECTED** where gnark returns
**VERIFY_ACCEPTED** — including on a witness word whose original value was literally zero:

    T2,1,private_word/middle,190311,VERIFY_REJECTED,true,
      "private[190311] 0x0000000000000000 -> 0x0000000000000001 (low bit flipped)"

**Both are correct. They bind different things.** binius64 **commits to the witness**, so
altering any committed word breaks the commitment whether or not the output moves. Groth16
with witness weights proves **existential satisfiability** — *"there exists a witness
satisfying this circuit for these public inputs"* — and an inert weight yields another valid
witness for the same statement. Neither system is better here; they answer different questions,
and **the five-system table needs a row for it: what the proof binds** (§8.2).

How much of the witness is inert is a property of the instance, and it is large:

| task | weights | **inert** (dead-and-stays-dead for every batch item) |
|---|---:|---:|
| T2 | 92 224 | **48 208 — 52.27 %** |
| T3 | 92 224 | **3 016 — 3.27 %** |

Measured exhaustively over all 92 224 weights at
[`inert-weights.txt`](../../data/repro-gnark/inert-weights.txt). Criterion, stated because the
number depends on it: a weight is inert iff the neuron it feeds has pre-activation ≤ 0 both
before and after the `+1`, **for every batch item**. An earlier estimate from the probe's 256
sampled positions gave 29.3 % for T2 — **off by 23 points, far outside sampling error for
n = 256**, so that sampler's positions are not independent of the layer structure. The
exhaustive figure is the one cited; the sampled one is kept in
[`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §4.2 as the record of how it was first
mis-measured. **The T2/T3 gap is structural and worth keeping:**
in T3 one weight matrix serves 8 independent inputs, so a weight must be dead in all eight at
once. **Batching hardens the witness-binding property by ~16×.**

**Both runs are published.** The pre-fix CSVs are preserved verbatim under
[`negative-gnark/prefix-run-2026-08-24/`](../../data/negative-gnark/prefix-run-2026-08-24/)
with a README saying which is which; the corrected control, which checks that a corruption
actually changes the statement before counting the position as a test, is at the top level and
reports **zero non-control acceptances**. `bench/CHALLENGE.md` promises we do not remove an
unflattering number, including our own, and regenerating silently would have broken that
promise while we hold five other teams to it.

**What the control does NOT establish.** That a maliciously *constructed* witness would be
caught. And the corrected probe now selects input positions, which always propagate — so it
avoids the inert case rather than resolving it. The inert case is documented separately, above.

## 7 · Setup and verification, reported separately and never amortised

| Task | backend | constraints | **setup s** | SRS s | pk bytes | vk bytes | **verify ms** | proof B |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | Groth16 | 197 763 | 14.11 | — | 44 740 553 | 8 688 | **2.13** | 196 |
| T2 | Groth16 | 283 408 | 21.50 | — | 68 915 955 | 528 | **1.92** | 196 |
| T3 | Groth16 | 973 058 | 38.58 | — | 136 454 535 | 752 | **1.88** | 196 |
| T1-a | Groth16 | 1 774 726 | 137.29 | — | 393 253 967 | 25 072 | **2.75** | 196 |
| T1-b | Groth16 | 3 555 722 | 161.89 | — | 578 425 015 | 98 800 | **3.97** | 196 |
| T1-c | Groth16 | 10 679 708 | 355.14 | — | **1 453 343 515** | 393 712 | **8.26** | 196 |
| T1-0 | PLONK | 526 592 | 2.11 | 16.06 | 67 143 352 | 34 384 | **1.73** | 584 |
| T1-a | PLONK | 4 723 968 | 15.46 | 119.74 | 536 905 400 | 34 384 | **3.35** | 584 |

**Setup dominates Groth16 at every rung** — 137 s of setup against 5.8 s of proving at T1-a,
355 s against 22.7 s at T1-c — and it is **per circuit**. It is reported here and never folded
into prove time, per `bench/README.md`.

**The proving key is the reason the ladder ends.** 1.45 GB at T1-c, and T1-d's 2²⁶ domain is
what drove this machine into 32 GB of swap.

**Verification is milliseconds and effectively flat**, but the two backends' verifying keys
scale oppositely: Groth16's vk and verify time grow with the **public input count** (8 688 B /
2.13 ms at T1-0's 257 public values; 393 712 B / 8.26 ms at T1-c's 12 289), while **PLONK's vk
is 34 384 bytes at every task** and verify stays 1.6–3.3 ms. T2 and T3 have tiny Groth16 vks
(528 B, 752 B) because the MLP has a single public output.

## 8 · What would make the five-system comparative table incorrect

gnark is the last system measured, so this is the last chance to break the bank before it is
published. Six findings, in descending order of damage. Each is evidenced from the other
systems' own committed documents, quoted rather than characterised. **We would rather write
this than have an author of a measured system write it for us.**

### 8.1 The five systems are not proving the same statement about INT8, and gnark proves the most

`bench/TASKS.md` asks systems that cannot express INT8 natively to "declare their encoding".
All five did. What no document says in the same place is that **the resulting statements differ
in strength**:

| system | is INT8-ness of the operands proved? | mechanism |
|---|---|---|
| binius64 | **No**, explicitly | "witnessed as full 64-bit words with **no range constraint**" |
| Ceno | **Not needed** — structural | a byte read with `lb` "trivially is" 8 bits |
| DeepProve | **Not stated** | no claim either way in any of its documents |
| jolt-atlas | **Not stated**, and its committed domain is **15 bits, not 8** | "operands it commits to are 128× the task's INT8 values" |
| **gnark, regime A** | **Yes** | `std/rangecheck`, every input and every weight |

binius64 states the consequence honestly and it binds the whole bank: *"A production deployment
would need those range constraints, and they are not in these numbers."*

**gnark regime A put them in the numbers**, and they are two thirds of the circuit:

| T1-0, Groth16, regime A | constraints | share |
|---|---:|---:|
| multiplications + output asserts | 65 791 | 33.3 % |
| **range checks on 65 792 INT8 values** | **131 972** | **66.7 %** |
| total | 197 763 | 100 % |

So gnark's headline is **3.0176 constraints/MAC**, and the same circuit without the range checks
the other four also omit is **1.0039** — one R1CS constraint per multiply-accumulate, the floor.
Confirmed by two independent routes: decomposing T1-0, and an isolated 256-MAC dot product that
gives 257/256 = 1.0039. **The tax is 3.006×.**

**Consequence.** A row placing gnark's 3.0176 beside binius64's IMUL count compares a proof of
*"the prover knows INT8 values whose product is the output"* against a proof of *"the prover
knows 64-bit words whose product is the output"*. Different theorems. **Either the table
carries the 3.006× decomposition in the same row, or the comparison is invalid.**

### 8.2 THE BIG ONE — weight status is an undeclared free variable, and the table has no column for what the proof binds

`bench/TASKS.md` fixes shapes, MAC counts, seeds, dtype and — via A1 — requantization. The
whole of what it says about weights is one line: `**Weights:** INT8, fixed published seed.`
**It never fixes whether they are public, private, committed, or constant.** There is no
amendment A2; A1 is the only amendment in the file.

The four measured systems resolved it three different ways, and **the weight-handling cost
lands in three different reported columns**:

| system | weight status | where the weight cost lands |
|---|---|---|
| binius64 | private witness wires, **committed per proof** | **inside** peak memory and prove time |
| DeepProve | ONNX initializer, "committed at setup" | **inside `setup`** — reported apart, never amortised |
| jolt-atlas | ONNX initializer, "committed at preprocessing" | **inside `setup`** — same exclusion |
| Ceno | hint bytes, re-read with `lb` once per operand per MAC | **inside the proved trace** (`LB 131 072 = 2 × 65 536`) |
| gnark regime A | secret witness, range-checked | **inside** constraints and prove time |
| gnark regime B | circuit constants bound into the vk | **inside `setup`**, and nowhere else |

**This is the finding that damages the table most.** For DeepProve and jolt-atlas the weight
cost is *excluded by construction* from the two derived metrics this repository exists to
publish, because setup is never folded into prove. For binius64 and gnark regime A it is inside
them. For Ceno it is inside the cycle count. **`bytes/MAC` and `MAC/s` do not cover the same
envelope in the five columns**, and no normalization fixes it, because the difference is in what
each system decided the weights *are*. Only binius64 names its position; **Ceno never names one
anywhere in its directory.**

**And the choice is worth ~600×, measured within one system on one machine:**

| T1-0, Groth16 | constraints | constraints/MAC | prove | peak fp | ratio |
|---|---:|---:|---:|---:|---:|
| regime A — weights are witness | 197 763 | 3.0176 | 0.663 s | 0.619 GB | — |
| regime B — weights baked in | 1 026 | 0.0157 | 0.010 s | 0.018 GB | **192.8× constraints, 64× prove, 34× memory** |

**Now the part that makes this a table column and not a caveat.** The two positions differ not
only in cost but in **what the proof binds**, and the two properties move *together*:

| | regime A (witness) | regime B (baked) |
|---|---|---|
| cost per MAC | **3.006× more expensive** | cheapest in the bank |
| does the proof bind the weights? | **No.** An alternative satisfying witness verifies — §6 measured it | **Yes.** Bound into the verifying key by the per-circuit setup |

**Baking is simultaneously cheaper and binds more.** That is counter-intuitive and it is the
most useful single thing this campaign produced, because a deployed fixed-model inference
service wants exactly that pairing: the model pinned to the artifact the verifier already
holds, and the arithmetic nearly free.

**Two reservations, both mandatory.**

1. **Regime B is not comparable to the other four systems** and its numbers never enter a
   cross-system ratio. It proves a weaker statement about `W` (fixed, not witnessed) and a
   circuit specialised to one weight set.
2. **That the verifier needs only the vk does NOT establish that the vk hides the weights.**
   Whether a verifying key that embeds `W₀` leaks information about `W₀` is an **open question**
   we did not investigate and do not assert either way. Any weight-privacy claim built on
   regime B is unsupported by anything in this repository.

**Recommendation, amendment-level.** `bench/TASKS.md` needs an **A2** fixing weight status the
way A1 fixed requantization, with A1's own reasoning: *"the spec was silent, and silence is not
a specification."* Until it exists, the five-system table must carry **two** extra columns —
*weight status* and *what the proof binds* — in the table, not a footnote, which is what
`bench/README.md` already demands for differences no normalization can fix.

### 8.3 The `constraints` column exists for two systems and means something different in each

`bench/README.md` mandates `constraints` in every published conditions line. Compliance:

| system | `constraints` field |
|---|---|
| binius64 | present — "MACs = IMUL constraints" |
| DeepProve | **absent**, silently |
| jolt-atlas | **absent**, silently |
| Ceno | present — **"NOT COMPARABLE to a circuit's constraint count"** |
| gnark | present — a fifth meaning |

**Two of five silently drop a mandated field**, and that is not house style: both DeepProve and
jolt-atlas explicitly mark the adjacent `security` field `NOT DETERMINED`, three lines from
where `constraints` would have gone. They knew how to declare a gap.

And where populated it does not mean one thing. Within gnark alone, the same 65 536 MACs are:

| | R1CS (Groth16) | SCS (PLONK) |
|---|---:|---:|
| weights witness | 197 763 | 526 592 |
| weights baked in | 1 026 | 67 592 |

**A 513× spread inside one system with the arithmetic held fixed**, because an R1CS row absorbs
a whole constant-coefficient linear combination for free and a PLONK gate is fan-in-2 and folds
nothing. A quantity that moves 513× without the task changing is not a property of the task.

**Recommendation.** Publish `constraints` as a per-system natural unit — as Ceno publishes
cycles — and **never as a cross-system column**.

### 8.4 `trusted setup y/n` is too coarse, and there is no maturity row

**Setup.** A y/n column puts gnark-Groth16, gnark-PLONK, DeepProve and jolt-atlas in one
bucket. Operationally they are not one bucket: **Groth16 is per circuit** (a new weight matrix,
a new layer width, or one constraint past a power of two forces a fresh setup — and §7 shows
setup dominating every rung); **PLONK is a universal SRS**, reusable; DeepProve and jolt-atlas
build a HyperKZG SRS in process; **binius64 and Ceno have none**. Split it into
`{none | universal | per-circuit}`.

**Maturity.** All four previously measured systems self-label `PROTOTYPE-RESEARCH`, three
quoting their own upstream warnings ("not audited and not production ready", "under construction
and not suitable for use in production"). **gnark lists nine third-party audits**, five vendored
in the measured tree, including the PLONK prover and verifier (OpenZeppelin, 2024-06) and the
standard library that `std/rangecheck` lives in (ZKSecurity, 2024-05). `bench/README.md` says
this benchmark does not measure security — but `security bits` and `trusted setup` are already
in the conditions line, so security *properties* are already in the table. "The only measured
system with third-party audits" is exactly the kind of difference the README says must be
declared rather than averaged. **Add a `maturity` row.**

### 8.5 The memory column is not comparable between a Go process and four Rust ones

§4 measured it: the same task, same everything, holds **602 MB at `GOGC=100`** and **374 MB at
`GOGC=25`**. So **gnark's default-GC memory figures overstate its working set by ~1.61×**
relative to what a language without a tracing GC would report. The other four systems are Rust
processes with a system or jemalloc allocator and have no equivalent slack — and Ceno's
`BUILD.md` §4 already established that the allocator choice moves this column.

We did not correct for it and we do not know the right correction. **It is declared, and the
`GOGC` sweep is published so a reader can see its size.** A memory column that puts 0.619 GB
(gnark, Go, GOGC=100) next to a Rust figure is comparing two accounting conventions as well as
two provers.

### 8.6 The `witness_word` control measures nothing on any ReLU task, in any system

§6 established the mechanism. The generalisation is what belongs here: **a witness-level
corruption is a valid negative control only if it changes the public output**, and on any task
with an activation a large fraction of witness values are discarded by that activation —
**52.27 % of T2's weights, by our criterion.** Every ReLU task in this benchmark, T2 and T3, in
all five systems, has this property.

That does not mean any system failed. binius64 rejects these corruptions because it commits the
witness (§6) — a stronger binding, honestly earned. It means **the control as specified cannot
distinguish "the system caught it" from "the position mattered"**, so its pass on a ReLU task
carries less information than it appears to.

**Recommendation.** `bench/README.md`'s correctness-control rule should require that a
witness-level corruption be shown to change the reference output before the position counts as
a test. Cheap — the reference forward pass is already computed. Proof-artifact corruption is
unaffected and remains the strong control: **exhaustive, every byte, both backends, zero
accepted.**

### 8.7 What does NOT break the table

Stated so the list above is not read as longer than it is.

- **The MAC counts are right.** gnark's builder asserts the emitted MAC count against
  `bench/TASKS.md` and refuses to emit on a mismatch. **All 28 compile cells matched exactly.**
- **A1's numerical bound is right.** Its stated 1.44·10¹⁹-against-9.22·10¹⁸ overflow reproduces
  here exactly: our static worst case exceeds int64 by **1.562×**, and the published instances
  clear it by **630 710×** (T2) and **478 224×** (T3).
- **The task specifications are expressible as written.** gnark hit no frontend wall — no
  minimum layer width, no power-of-two rung requirement, no implicit requantization.
- **The correctness control on the proof artifact is sound**, exhaustive, and gnark passes it.
- **The no-extrapolation rule held.** Every ceiling here is a measured interval.

## What contaminates these numbers, declared

1. **The machine was not dedicated.** Firefox, Teams and WindowServer resident throughout;
   load averages around 5 at grid start. `(u+s)/real` is published per cell so contention is
   visible rather than inferred. **No figure here is gnark's best achievable performance.**
2. **The boot volume was 95 % full, and it killed a cell.** T1-d regime A died to our own disk
   watchdog after the cell grew macOS's swap file to 32 GB. That is this machine's limit, not
   gnark's.
3. **T1-c ran under paging pressure.** Peak footprint 2.71× peak RSS, where every other cell
   agrees within 1 %. Both numbers are published; neither is clean.
4. **The memory column is Go-GC accounting** (§8.5, `BUILD.md` §7), ~1.61× looser than a
   non-GC process at the default `GOGC`.
5. **This machine gives gnark the element assembly but not the vector assembly** (`BUILD.md`
   §4). It plays **against** gnark, and we did not quantify the vector gap — that needs an
   amd64 host and this campaign has one machine.
6. **The `purego` A/B measured 7.5 %**, so the build is genuinely the assembly build — but that
   toggles element ops only and says nothing about the missing vector kernels.
7. **N is small on the expensive rungs** (N=1 at T1-b, T1-c and T1-b PLONK) and is stated per
   cell. Where N = 1 no dispersion is published, because none was measured.
8. **The reproduction runs were taken on battery** under load averages of 12–30 and say so in
   `REPRODUCTION.md`; the grid was on AC.
9. **The witness instance differs from binius64's** — same seeds, same shapes, same MAC counts,
   different RNG stream. **Task-level comparison only, never witness-level.**
10. **PLONK's SRS is `test/unsafekzg`**, "for test purposes only" by its own docstring. Not a
    ceremony.
11. **Nothing is extrapolated outside the measured range.**
