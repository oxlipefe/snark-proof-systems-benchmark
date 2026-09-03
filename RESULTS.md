# zk-prover-bench — the comparative results

**Five systems, one machine, one campaign, the same tasks: `binius64` · `DeepProve` ·
`jolt-atlas` · `Ceno` · `gnark`.**

This is the document [`README.md`](README.md) promises. Read
[`TASKS.md`](TASKS.md) first — including **Amendments A1, A2 and A3**, which govern
everything below — and then each system's own directory under [`systems/`](systems/), which
holds the conditions line, the build check, the expression, the reproduction attempt and the
uncurated grid for that system. **Nothing here supersedes a system's own file; this file only
puts them side by side, and says where side by side is not allowed.**

Date: 2026-08-24. Machine: Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, macOS
26.5.2 (Darwin 25.5.0), **not dedicated**, per-cell `(user+sys)/real` and swap in every
ledger. No figure in this repository is any system's best achievable performance.

---

## 0 · The rules that make this table valid, and the one that breaks it

Six rules govern every row below. Five are procedural. **The first is substantive, it was
discovered by the last system measured, and it is the reason this file is not the table we set
out to write.**

1. **`bytes/MAC` and `MAC/s` are comparable only within a weight regime** (A2 §3). Five
   systems resolved an unstated part of the specification in four different ways, and the
   weight cost therefore lands in four different reported columns. Where a regime boundary is
   crossed in a column, **the warning is in the column header, never at the foot of the
   table.**
2. **A column for what the proof binds** (A2 §4). Systems differ here in a way that has
   nothing to do with cost, and both answers are correct. §2.
3. **`constraints` is a per-system natural unit and never a cross-system column** (gnark §8.3).
   The same 65 536 MACs are 197 763, 526 592, 1 026 or 67 592 constraints inside gnark alone.
   §11.
4. **`trusted setup` is split `{none | universal | per-circuit}`**, with a separate row for
   whether any of them is ceremony-backed (none is), and a **maturity** row. §8.
5. **A Go process's memory is not a Rust process's memory.** Declared, measured, not
   corrected. §5.4.
6. **The INT8 statement is not the same in the five.** Either the row carries the
   decomposition or the comparison is invalid. §7.

And the rule from [`README.md`](README.md) that outranks all of them:

> **If ours comes last, it is published last.**

The substrate this project bet on is **binius64**. At the one cell where all five systems have
a number it is **fourth of five on `bytes/MAC`** — 12 295 against jolt-atlas's 148.4 — and
second on `MAC/s`. It is published fourth. §3.1.

---

## 1 · THE PARTITION — weight regime, and what it does to the headline metric

`TASKS.md` said only *"Weights: INT8, fixed published seed."* It never said what the weights
**are** to the proof system. Amendment A2 (2026-08-24) fixes that going forward and requires
every system to declare a regime from a fixed vocabulary. Here is the declaration for all
five, each established from that system's own `EXPRESSION.md` and its own raw data, with the
citation:

| system | **weight regime** | **where the weight cost lands** | **inside `bytes/MAC` and `MAC/s`?** | established from |
|---|---|---|---|---|
| **binius64** | `witness` | **prove** — constraints, prove time, peak memory | **YES** | `EXPRESSION.md` §2: *"Both `A` and `B` are private witness wires. The weights are committed, not public"*; §3 |
| **gnark, regime A** | `witness` | **prove** — constraints, prove time, peak memory | **YES** | `EXPRESSION.md` §1, §2; range-checked as witness |
| **DeepProve** | `preprocessed` | **setup** — context generation | **NO — excluded by construction** | `EXPRESSION.md` §3: *"`B` is an initializer, so the weights are committed at setup"* |
| **jolt-atlas** | `preprocessed` | **setup** — `Model::load` + preprocessing + `setup_prover` | **NO — excluded by construction** | `EXPRESSION.md` §5: *"`B` is an initializer, so the weights are committed at preprocessing"*; §2 |
| **Ceno** | `program-data` | **cycles** — inside the proved trace | **YES** | `EXPRESSION.md` §8 (weights are `--hints-file` records), §2 (`LB 131 072 = 2 × 65 536` at T1-0), §3 (one ELF, one vk, five different weight matrices) |
| **gnark, regime B** | `circuit-constant` | **compile / setup** — bound into the verifying key | **NO** | `EXPRESSION.md` §1; Go compile-time constants |

**Ceno's position was never named in its own directory before this campaign.** It is
established here rather than assumed, and the decisive evidence is negative: *one ELF and one
verifying key serve all five T1 rungs* (`ceno/EXPRESSION.md` §3), and those five rungs carry
five different weight matrices. A key that does not change when the weights change cannot be
binding them. That rules out `circuit-constant` and `preprocessed` by measurement, not by
argument.

### 1.1 The consequence, stated as plainly as it deserves

**The five-column `bytes/MAC` table this repository was built to publish does not exist.**
A2 §3 partitions the six measured configurations into four buckets, and only two buckets hold
more than one member:

```
witness            binius64 · gnark regime A          <- comparable to each other
preprocessed       DeepProve · jolt-atlas             <- comparable to each other
program-data       Ceno                               <- comparable to nothing here
circuit-constant   gnark regime B                     <- comparable to nothing here
```

Within a bucket the number means the same thing. Across a bucket boundary it does not, and
**no normalization fixes it**, because the difference is not a scale factor — it is a
difference in what each system decided the weights *are*.

**And the choice is worth 192.8× in constraints, measured inside one system on one machine**
(gnark T1-0, Groth16: 197 763 constraints as witness, 1 026 as circuit constants — 64× on
prove time and 34× on peak memory). No cross-system difference measured anywhere in this
campaign is that large except jolt-atlas's memory advantage.

### 1.2 The two within-regime comparisons that ARE valid

Both at **T1-a, 589 824 MACs, nominal 10 threads** — the only cell in the campaign where all
five systems have a figure (§3.1). Peak footprint basis.

**`witness` bucket — binius64 vs gnark regime A**

| | binius64 (rate 1) | gnark (Groth16 rA) | |
|---|---:|---:|---|
| `B/MAC` | **12 295** | **7 142.2** | gnark **1.72× less** |
| `MAC/s` | **829 984** | **102 204** | binius64 **8.12× faster** |

**Still not clean, and the two reasons run in opposite directions.** gnark proves INT8-ness
and binius64 does not — a **3.006×** constraint tax gnark pays and binius64 omits (§7) — which
flatters binius64. And gnark's memory is Go-GC accounting, **~1.61× looser** than a Rust
process at the default `GOGC` (§5.4), which flatters binius64 again. Neither is corrected.

**`preprocessed` bucket — DeepProve vs jolt-atlas**

| | DeepProve (bit len 8) | jolt-atlas (padded) | |
|---|---:|---:|---|
| `B/MAC` | **2 913.7** | **148.39** | jolt-atlas **19.64× less** |
| `MAC/s` | **221 722** | **4 351 596** | jolt-atlas **19.63× faster** |

**This is the cleanest pair in the benchmark**, and it is worth saying why: same weight
regime, same field (BN254), same PCS family (HyperKZG), same trusted-setup posture, both Rust,
both `security NOT DETERMINED`, both padding 768 → 1024 (**1.778×** the task's arithmetic),
both applying a requantization the task forbids and neither able to switch it off. Almost
every confound that separates the other pairs is shared here. **jolt-atlas is 19.6× better on
both axes**, and the near-identical factor on two independent metrics is itself worth a second
look by anyone re-running this.

### 1.3 The open question A2 leaves open, and that this file will not close

In `circuit-constant` regime the verifier needs only the verifying key. **That the weights are
absent from the verifier's inputs does not establish that the verifying key reveals nothing
about them.** Nobody in this benchmark established that, and no weight-privacy claim may be
built on it. Stated because it is the obvious wrong inference from §2's next row.

---

## 2 · What the proof binds — a required column, and both answers are correct

A2 §4 requires this and it is not a cost column. **binius64 commits the witness and rejects a
change to it. Groth16 with witness weights proves existential satisfiability and accepts an
inert change. Both are correct; they are different theorems.**

| system | regime | what the proof binds about the **weights** | measured, or declared? |
|---|---|---|---|
| **binius64** | `witness` | Weights are private witness wires **committed per proof**. A single-bit flip of a committed private word is **REJECTED** — including `private[190311]` on T2, whose original value was literally `0x0000000000000000`. | **Measured** — but the T2/T3 rows are **weak evidence** under A3 (§10.2). Mechanism: the flip is applied to the `ValueVec` without re-deriving downstream wires, so the constraint system is violated whether or not the output moves. |
| **gnark, regime A** | `witness` | **Nothing.** Existential satisfiability. An alternative satisfying witness verifies: two inert weight perturbations on T2 were **ACCEPTED**, and accepting them is correct behaviour — the network output was bit-identical (`14 623 789 560 139` either way). | **Measured**, four positions for four, mechanism established before the conclusion. |
| **gnark, regime B** | `circuit-constant` | **The weights, to the verifying key**, by Groth16's per-circuit setup. A different `W₀` is a different circuit with a different vk. Model-binding without a commitment scheme. | Structural / declared. Not corruption-tested at the weight level, because there is no weight witness to corrupt. |
| **DeepProve** | `preprocessed` | Weights are an ONNX initializer **committed at setup**; the artifact carries the verifier context. What the commitment binds was **not tested**: the licence forbids the derivative work a witness-level control needs. | **Declared only.** |
| **jolt-atlas** | `preprocessed` | Weights are an ONNX initializer **committed at preprocessing**; verifier preprocessing is derived from prover preprocessing. Same licence bar, same absence of a test. | **Declared only.** |
| **Ceno** | `program-data` | **The program image and the public output digest — not the weights.** The vk is a function of the ELF, and one ELF serves five different weight matrices (`EXPRESSION.md` §3). The output is bound by a Keccak-256 digest the guest commits and the host re-derives (§6), and a mismatched digest **is rejected** — measured, and found by getting it wrong first. | Output binding **measured**; weight binding **absent by construction**. |

**Read the column, not the ranking.** A `preprocessed` system pins the model to an artifact the
verifier holds and pays for it once; a `witness` system re-proves knowledge of the model every
time and pays for it every time; `gnark regime B` pins the model into the verifying key and pays
nothing at proving time; `Ceno` pins the *program* and treats the model as input. **A deployed
fixed-model inference service and a service that updates its model want opposite ends of that
column**, and no figure in this repository decides between them.

**The counter-intuitive measured fact, and it is the most useful single thing this campaign
produced:** in gnark, baking the weights in is *simultaneously* 192.8× cheaper in constraints
**and** binds more. Cost and binding do not trade off in the direction one expects.

---

## 3 · The two cells where the systems actually meet

There is **no thread setting at which all five systems are comparable**, and the reason is in
each system's own file:

- **Ceno's prover aborts at `RAYON_NUM_THREADS=1`** (`FAIL_rc101`, on Ceno's own examples too),
  so there is no 1-thread Ceno row anywhere.
- **`RAYON_NUM_THREADS=1` is not one thread** for DeepProve (`(u+s)/real` 1.71–1.83) or
  jolt-atlas (1.93–2.15).
- **`RAYON_NUM_THREADS=10` is not ten threads** for Ceno (*"thread size 10 is not power of 2,
  using 8 threads instead"*) or for DeepProve's sumcheck (same rounding).
- **gnark does not round**, and binius64 does not round. Their nominal count is the count.

So two cuts are published, each labelled with what its thread column actually means.

### 3.1 T1-a · 589 824 MACs · nominal 10 threads — the only five-system cell

> **COLUMN WARNING — `B/MAC` AND `MAC/s` CROSS A WEIGHT-REGIME BOUNDARY IN THIS TABLE.**
> Under A2 §3 they are the same quantity only inside a bucket. The regime is in every row, and
> the two valid within-bucket comparisons are in §1.2. **Reading down the `B/MAC` column and
> calling the smallest number the winner is precisely the error this amendment exists to
> prevent.**

| system | regime | config | prove | **MAC/s** | peak footprint | **B/MAC (fp)** | proof / artifact | verify | setup, reported apart |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| **jolt-atlas** | `preprocessed` | padded, RAYON=10 | **135.54 ms** | **4 351 596** | **0.0875 GB** | **148.39** | 23 435 B | 7.81 ms | 31.8 ms |
| **binius64** | `witness` | `log_inv_rate`=1, 10 thr | 710.64 ms | 829 984 | 7.252 GB | 12 295 | 460 304 B | 24.56 ms (cold) | 618.0 ms + 955.7 ms build |
| **DeepProve** | `preprocessed` | `ZKML_BIT_LEN`=8, RAYON=10 | 2 660.2 ms | 221 722 | 1.719 GB | 2 913.7 | 116 404 B (proof **+ io + ctx**) | ~20 ms (cold process, 10 ms resolution) | 2 410.1 ms |
| **gnark** | `witness` (rA) | Groth16, GOMAXPROCS=10 | 5 771 ms | 102 204 | 4.213 GB | 7 142.2 | **196 B** | **2.753 ms** | **137 285 ms** |
| **Ceno** | `program-data` | RAYON=10 → **8 real**, shard cap 2²⁹, 1 shard | 20 100 ms | 29 344 | 13.221 GB | 22 414.8 | 1 379 317 B | **NOT MEASURED** at 1 shard | keygen 9 483 ms **†** |

**† Ceno's keygen figure is carried, not measured for this cell.** The default single-shard
T1-a cell logs no keygen time — checked in `data/results-ceno.csv` and in the raw
`data/cells-ceno/t1-a-t10-s536870912-n3/` logs. 9 483 ms is the figure Ceno's own §5 measured
for T1-a at its **13-shard** configuration, and it is carried here on the strength of that
file's own claim that **keygen is constant at ~9.4 s and does not vary with the task**
(9.41 / 9.56 / 9.36 s at T1-0 / T2 / T3), because the verifying key depends on the *program
image* and one ELF serves the whole ladder. **That claim is plausible and it is theirs, not a
measurement of this row.** No other figure in this table is carried from a differently
configured cell.

**Conditions line for this table.** Every figure is the median its own system publishes, at the
commit pinned in that system's `COMMIT`, on the machine above, on 2026-08-23/24. N = 5 for
jolt-atlas and DeepProve, 5 for binius64, 3 for gnark, 3 for Ceno. Brackets differ and are
**not** normalized: binius64's `prove` is its whole prover call; **DeepProve's includes
quantized inference** and jolt-atlas's **includes graph tracing**, neither separable, so both
are upper bounds; **Ceno's excludes emulation and witness generation** while its memory
includes them; gnark's is `groth16.Prove` alone with compile and setup inside the memory column
but outside the prove column. Peak footprint is whole-process in all five. Full brackets: each
system's *"What is inside each measured quantity"* section.

**Five things this table says that a ranking would destroy.**

1. **`MAC/s` and `B/MAC` order the systems identically here**, which is unusual and worth
   flagging rather than assuming: jolt-atlas, binius64, DeepProve, gnark, Ceno on speed;
   jolt-atlas, DeepProve, gnark, binius64, Ceno on memory — **binius64 and DeepProve swap.**
   binius64 is 3.74× faster than DeepProve and uses 4.22× more memory per MAC. A prover twice
   as fast on three times the memory is worse, not better, and this is the pair that shows it.
2. **The proof-size spread is 7 037×** — 196 B (gnark Groth16) to 1 379 317 B (Ceno) — and it
   does not track any other column.
3. **Setup spans 4 311×** — 31.8 ms (jolt-atlas) to 137 285 ms (gnark, and it is **per
   circuit**). gnark's setup at this rung is **23.8× its own prove time**.
4. **Two of the five put the weight cost in that setup column** and therefore out of `B/MAC`
   entirely. That is §1, and it is why the `B/MAC` column carries a header warning.
5. **Ceno is last on both, and the reason is not the prover.** A zkVM spends **10.71
   instructions and 42.84 cycles per MAC** at this rung (`ceno/EXPRESSION.md` §1). Per
   `ceno/RESULTS.md` §7 that factor travels with every Ceno ratio in the same sentence, and it
   does here: **Ceno's denominator describes the task, its cost describes the instructions.**

### 3.2 T1-0 · 65 536 MACs · nominal 1 thread — four systems, Ceno absent

> **SAME COLUMN WARNING.** `B/MAC` and `MAC/s` cross a regime boundary: binius64 and gnark are
> `witness`, DeepProve and jolt-atlas are `preprocessed`.

| system | regime | prove | **MAC/s** | peak footprint | **B/MAC (fp)** | proof / artifact | verify | setup | `(u+s)/real` |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| **jolt-atlas** | `preprocessed` | **64.98 ms** | **1 008 541** | **0.0189 GB** | **288.0** | 21 419 B | 5.60 ms | 49.4 ms | **1.98** |
| **binius64** | `witness` | 178.98 ms | 366 160 | 0.535 GB | 8 155 | 345 536 B | 6.29 ms | 66.1 ms + 109.4 build | **0.9925** |
| **DeepProve** | `preprocessed` | 977.7 ms | 67 030 | 0.236 GB | 3 601 | 59 512 B (+io+ctx) | ~20 ms | 1 190.8 ms | **1.83** |
| **gnark** | `witness` (rA) | 4 547.6 ms | 14 411 | 0.559 GB | 8 528.3 | **196 B** | **3.579 ms** | 81 315 ms | **0.9974** |
| **Ceno** | `program-data` | — | — | — | — | — | — | — | — |

**Ceno has no cell here and it is not an omission.** `RAYON_NUM_THREADS=1` aborts its prover
with `assertion left != right failed: Attempt to prove a constant` in the sumcheck prover —
measured on T2 and published as the `FAIL_rc101` row of its own grid, and it fires on Ceno's
own `fibonacci` and `ceno_rt_alloc` examples as well. T1-0 at 1 thread was **not attempted**
after that.

**The `(u+s)/real` column is why this cut needs a warning of its own.** Only binius64 and gnark
were actually single-threaded. DeepProve and jolt-atlas were using roughly two cores under a
setting that names one, so **their rows here are not 1-thread figures** and the comparison to
binius64's and gnark's is not like for like. Each system's file says so; this table repeats it
because the column would otherwise be read as four equal things.

---

## 4 · The complete grid — 7 tasks × 5 systems, including every cell that did not run

**35 cells. 24 produced a figure. 11 did not, and every one of them has its reason here.**
`✓` = proved and verified. Configurations are each system's primary cut.

| | **binius64** (r1) | **DeepProve** (bit 8) | **jolt-atlas** (padded) | **Ceno** (10 thr) | **gnark** (G16 rA) |
|---|---|---|---|---|---|
| **T1-0** 65 536 | ✓ | ✓ | ✓ | ✓ | ✓ |
| **T1-a** 589 824 | ✓ | ✓ | ✓ | ✓ | ✓ |
| **T1-b** 2 359 296 | ✓ | ✗ `FAIL_parse` | ✓ | ✓ (2 shards at default caps) | ✓ (N=1) |
| **T1-c** 9 437 184 | ✓ **swap-dominated** | ✗ `FAIL_parse` | ✓ | ✗ **NOT ATTEMPTED** | ✓ (N=1, **paging**) |
| **T1-d** 37 748 736 | ✗ **setup refuses** | ✗ `FAIL_parse` | ✓ | ✗ **NOT ATTEMPTED** | ✗ **watchdog kill** |
| **T2** 92 224 | ✓ | ✗ `FAIL_prove` | ✗ `FAIL_einsum` | ✓ | ✓ |
| **T3** 737 792 | ✓ **one proof** | ✗ `FAIL_prove` / `FAIL_parse` | ✗ `FAIL_einsum` | ✓ **one proof** | ✓ **one proof** |

**Every empty cell, with its cause and where the evidence is:**

| cell | what happened | class |
|---|---|---|
| **binius64 T1-d** | The circuit **built** in 126.862 s with exactly 37 748 736 IMUL constraints. `setup` then refused it: *"the Private segment declares 118505472 values, over the maximum of 67108864"* — `MAX_VALUES_PER_SEGMENT = 1 << 26`, which its own source calls *"a policy limit rather than a structural one … Raising it is fine."* We did **not** patch it: the fairness protocol measures each system in the configuration its authors document. Ceiling published as a measured interval: proved at 9 437 184 MACs, refused at 37 748 736. | **policy limit** |
| **binius64 T1-c** | Ran, but `(u+s)/real` = 0.378 and three proofs of the same circuit took 183.6 / 396.2 / 432.7 s. Peak footprint 92.99 GB on a 32 GiB machine. **Its own file refuses to publish a throughput point estimate for that rung**, and no ratio in this file quotes it as a speed comparison. | **machine, not prover** |
| **DeepProve T1-b/c/d** | `FAIL_parse` in 0.01–0.02 s. The ONNX dense parser flattens the input and requires it to reduce to a vector, so **only `M = 1` is expressible**. `[4×768]·[768×768]` is rejected at the frontend. | **frontend wall** |
| **DeepProve T2, T3** | `FAIL_prove`. A dense layer whose output is **narrower than 4** cannot be proved at this commit; T2's final layer is 64 → 1. Isolated to that single variable with a width ladder. | **prover wall** |
| **DeepProve T3 as one proof** | `FAIL_parse`. The ONNX parser **pins `batch_size = 1`**, so a batch of 8 cannot be one proof. It would have been 8 separate proofs — which is how `TASKS.md` says such a system reports it — except that the network itself does not prove. | **frontend wall** |
| **jolt-atlas T2, T3** | `FAIL_einsum` in 0.06 s / 0.37 s. The einsum registry refuses the contraction produced by a **width-1 output layer**. Batching *is* expressible here (unlike DeepProve) and T3 never reaches it. | **prover wall** |
| **Ceno T1-c, T1-d** | **No proving cell exists**, in the grid or in `bench/data/cells-ceno/`. Only emulation was run: T1-c is 403 791 196 cycles (fits one default shard by 25 %), T1-d is 1 615 020 812 cycles (4 shards at the default cap). `ceno/NOT_EXPRESSIBLE.md` §2 gives the campaign-budget context — a machine at 95 % disk with 778 MB of free swap — but **names no per-cell reason, and Ceno's `RESULTS.md` publishes no row for either.** Recorded here as a documented gap, not as a limit of Ceno. See §13.7. | **not attempted** |
| **gnark T1-d** | Killed at ~17 minutes by **our own disk watchdog** after the cell grew macOS's swap file from 9 GB to 32 GB and pushed free space below the guard's 20 GiB floor. gnark *compiled* T1-d regime A in 89.13 s, and **regime B proved T1-d in 0.723 s on 1.243 GB**. Published as a measured interval: largest that proved 10 679 708 constraints, smallest that did not 39 175 652. **A property of this machine at 95 % disk, not of gnark.** | **machine, not prover** |

**Of the eleven empty cells: seven are frontend or prover walls** (three DeepProve rungs on the
`M = 1` limit, two DeepProve and two jolt-atlas on a narrow output layer), **one is a policy
limit** (binius64's segment cap), **two were never attempted** (Ceno's top rungs), **and exactly
one was stopped by memory** — gnark's T1-d, killed by our own disk watchdog after the cell drove
macOS's swap file from 9 GB to 32 GB.

That distribution is itself a result, and it is not the one the ladder was built to find:
**T1 spans three orders of magnitude to locate each system's memory ceiling, and in four of the
five systems something else stopped it first.** The two cells that *were* memory-shaped —
binius64's T1-c at 92.99 GB of footprint on a 32 GiB machine, and gnark's T1-c at 2.71× RSS —
both produced figures, and both are contaminated rather than absent.

---

## 5 · The memory curves — the thing this repository exists to measure

**The curve is the result. No single point on it is a property of the prover.** All five
systems confirm `README.md`'s third finding — `bytes/MAC` is not a constant — and **each does
it for a different reason.** Local exponent is `log(Δ peak footprint) / log(Δ MACs)` between
consecutive rungs.

### 5.1 Each system's curve, at the thread count its own file publishes it at

**binius64** — `log_inv_rate` 1, 1 thread. *Exponents derived here from the published
peak-footprint column; binius64's own file does not publish them.*

| task | MACs | peak fp | **B/MAC** | local exponent |
|---|---:|---:|---:|---:|
| T1-0 | 65 536 | 0.535 GB | **8 155** | — |
| T1-a | 589 824 | 7.28 GB | **12 335** | **1.188** |
| T1-b | 2 359 296 | 27.56 GB | **11 684** | **0.961** |
| T1-c | 9 437 184 | 92.99 GB | **9 854** | **0.877** |

**binius64 is the only system in the benchmark whose `B/MAC` rises before it falls**, and the
only one that is superlinear on any step. Its own §6 states the mechanism without hedging: one
`imul` — a 64×64 → 128 integer multiply — per INT8 multiply-accumulate, *"paying for 64-bit
arithmetic to do 8-bit work, on a workload that is ~99 % 8-bit multiply-accumulate."*

**jolt-atlas** — padded, 1 thread. Exponents published in its own §1.

| task | MACs | peak fp | **B/MAC** | local exponent |
|---|---:|---:|---:|---:|
| T1-0 | 65 536 | 18.0 MB | **288.0** | — |
| T1-a | 589 824 | 74.2 MB | **131.9** | **0.645** |
| T1-b | 2 359 296 | 201.0 MB | **89.3** | **0.718** |
| T1-c | 9 437 184 | 498.2 MB | **55.4** | **0.655** |
| T1-d | 37 748 736 | 1 857.8 MB | **51.6** | **0.949** |

Falls 5.6× and **flattens at the top**: a constant being spread thinner, with the marginal cost
settling near 50 B/MAC. The only system that ran the whole ladder.

**gnark** — Groth16 regime A, 10 threads, `GOGC` default. Exponents published in its own §1.

| task | MACs | constraints | peak fp | **B/MAC** | local exponent |
|---|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 197 763 | 619 MB | **9 443.8** | — |
| T1-a | 589 824 | 1 774 726 | 4 213 MB | **7 142.2** | **0.873** |
| T1-b | 2 359 296 | 3 555 722 | 5 411 MB | **2 293.3** | **0.181** |
| T1-c | 9 437 184 | 10 679 708 | 18 868 MB | **1 999.3** | **0.901** |

**The exponent is not stable and the reason is structural.** T1-a through T1-d share one
[768×768] weight matrix, so the ~590 000 **weight range checks are a fixed cost** while the
multiplications grow ×4 a step. Constraints grow ×8.97, ×2.00, ×3.00 — not ×9, ×4, ×4 — and
memory follows the constraints, not the MACs. Against constraints the exponents are 0.874,
0.360, 1.136 and `B/constraint` moves only 1.8× while `B/MAC` falls 4.7×.
**This is the weight regime showing up in the memory curve**: it only happens because the
weights are witness *and* range-checked.

**Ceno** — 10 nominal (8 real) threads, default shard caps. Exponents published in its own §1.
**Note the shard count changes inside this table**, which is declared in the source and matters
under §5.3.

| task | MACs | cycles | shards | peak fp | **B/MAC** | **B/cycle** | local exponent |
|---|---:|---:|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 3 203 656 | 1 | 5 346.0 MB | **81 572.8** | 1 668.7 | — |
| T1-a | 589 824 | 25 265 476 | 1 | 13 220.8 MB | **22 414.8** | 523.3 | **0.412** |
| T1-b | 2 359 296 | 100 983 792 | **2** | 36 568.0 MB | **15 499.5** | 362.1 | **0.734** |

**DeepProve** — `ZKML_BIT_LEN` 8, 1 nominal thread. Two rungs only. *Exponent derived here.*

| task | MACs | peak fp | **B/MAC** | local exponent |
|---|---:|---:|---:|---:|
| T1-0 | 65 536 | 0.236 GB | **3 601** | — |
| T1-a | 589 824 | 1.768 GB | **2 997** | **0.916** |

**Two points is not a curve** and its own file says so. It also proved **1.778× the task's
arithmetic** at T1-a because of padding, which pushes `B/MAC` down and `MAC/s` down.

### 5.2 The cost shapes, contrasted — and what the measured range does and does not show

| system | intercept | slope | shape in one line |
|---|---|---|---|
| **Ceno** | **~5 GB before any task-dependent work** | shallow, ~357 B per marginal cycle | a zkVM pays for every instruction the machine *could* execute, then for the ones it did |
| **gnark** | **negligible** — T1-0 peaks at 0.619 GB, regime B at **0.018 GB** | steep, near-linear in constraints | a circuit prover pays for the circuit it was given |
| **jolt-atlas** | small, amortized over the first three steps | flattens at ~50 B/MAC | a constant being spread thinner, then a marginal rate |
| **binius64** | small | **superlinear then near-linear** | 64-bit primitive doing 8-bit work; no amortization to be had |
| **DeepProve** | not established — two points | — | not enough measured range to say |

**Ceno's ~5 GB floor is the single most transferable number in this benchmark**, because it is
absolute and no denominator is involved in reading it: T1-0 — 65 536 MACs, the smallest thing
here — costs Ceno **5.35 GB**, where every circuit system proves it in a small fraction of
that. *For a product that must prove small things cheaply, this number decides the question on
its own.*

**Do gnark's and Ceno's curves cross? The measured points say: the slopes do, the values do
not.**

| step | gnark local exponent | Ceno local exponent | ordering |
|---|---:|---:|---|
| T1-0 → T1-a | **0.873** | **0.412** | gnark steeper |
| T1-a → T1-b | **0.181** | **0.734** | **Ceno steeper — the ordering inverts** |

But `B/MAC` itself **never crosses inside the measured range**: Ceno is above gnark at all three
common rungs — 8.64× at T1-0, 3.14× at T1-a, 6.76× at T1-b — and the ratio narrows and then
widens again. **`gnark/RESULTS.md` §1 describes the two as *"systems whose `bytes/MAC` curves
cross"*; on the measured points that is true of the local exponents and not of the values, and
extending it to the values would be extrapolation, which `CHALLENGE.md` forbids.** Logged as a
correction in §13.6.

**Nothing in this section is extrapolated.** Each ladder stops where §4 says it stops.

### 5.3 The one system where peak memory is a dial, not a property

**Ceno segments a trace across shards, and no other system here offers anything like it.** Same
task, same instance, same threads — only the flag moves (T1-a, 589 824 MACs, 25 265 476 cycles,
10 nominal threads, N = 1 per row):

| `--max-cycle-per-shard` | shards | prove s | peak fp | proof bytes | **B/MAC** |
|---:|---:|---:|---:|---:|---:|
| 536 870 912 (2²⁹, default) | 1 | 20.10 | 13 221 MB | 1 379 317 | **22 415** |
| 33 554 432 (2²⁵) | 1 | 20.00 | 13 222 MB | 1 379 317 | **22 416** |
| 8 388 608 (2²³) | 4 | 30.02 | 9 162 MB | 4 614 432 | **15 533** |
| 2 097 152 (2²¹) | 13 | 44.04 | 8 822 MB | 13 577 390 | **14 957** |

**Segmentation buys ÷1.50 in memory for ×2.19 in time and ×9.84 in proof bytes** — and a
further ×11 in verify time (0.0503 s single-shard against 0.5627 s at 13 shards). The first two
caps are identical because T1-a already fits inside 2²⁵; the flag only acts once it bites.

**Set against the only other memory lever measured anywhere in this campaign:** gnark has **no
protocol-level lever at all** — no segmentation, no shard cap, no streaming prover, no memory
limit — but the Go runtime has one, and `GOGC` 400 → 25 moves T1-0's peak footprint
**1 230 → 374 MB, ÷3.29, for a prove-time change of 1.005×**, with no change to proof size or
setup.

**These are not the same kind of lever and the difference matters more than the ratio.**
Ceno's shard cap changes *what is proved* — more shards, bigger proof, longer verify. `GOGC`
changes only how much garbage the runtime tolerates; it cannot shrink the proving key or the
QAP, so it bottoms out at the true working set. **One is a knob on the protocol, the other is
a knob on our accounting.**

**Consequence for §3's tables:** a `bytes/MAC` figure for a segmented prover is a property of
(prover, task, **shard size**), so every Ceno row in this file carries its shard cap and shard
count. And `README.md` argues memory is a binary gate because it does not parallelize across
machines. **Segmentation is the one mechanism in this benchmark that argues back.**

### 5.4 The memory column is not one accounting convention

| system | language | allocator | declared distortion |
|---|---|---|---|
| **gnark** | Go | runtime GC, `GOGC=100`, no `GOMEMLIMIT`, **`debug.FreeOSMemory()` never called** | **~1.61× looser** than a non-GC process — the same task holds 602 MB at `GOGC=100` and 374 MB at `GOGC=25`. **Measured, declared, NOT corrected.** |
| **Ceno** | Rust | **system allocator — NOT the authors' documented jemalloc configuration**, which sets `retain:true` with decay disabled and would never return pages to the OS | Their configuration would likely be faster and would certainly report worse memory. |
| **binius64** | Rust | system | — |
| **DeepProve** | Rust | system | — |
| **jolt-atlas** | Rust | system | — |

**A memory column that puts 0.619 GB (gnark, Go, `GOGC=100`) next to a Rust figure is comparing
two accounting conventions as well as two provers.** We do not know the right correction and
did not apply one; the `GOGC` sweep is published so a reader can see its size.

### 5.5 Peak RSS and peak footprint diverge, and above ~16 GB the divergence is the story

| system / cell | peak RSS | peak footprint | ratio |
|---|---:|---:|---:|
| binius64 T1-a | 7.80 GB | 7.28 GB | 0.93 |
| binius64 T1-b | 16.69 GB | 27.56 GB | 1.65 |
| **binius64 T1-c** | 19.15 GB | **92.99 GB** | **4.86** |
| **gnark T1-c** | 6.961 GB | **18.868 GB** | **2.71** |
| Ceno T1-b | 15.05 GB | 34.06 GB | 2.26 |
| every other cell in the campaign | — | — | within ~1 % |

**`bytes/MAC` computed against peak RSS is not a memory-cost metric once RSS saturates** — it
measures how much RAM this machine has. binius64's RSS-based `B/MAC` *falls* from 7 074 at
T1-b to 2 029 at T1-c, not because the prover got frugal but because the rest went to swap.
**The footprint column is the one to read**, and both are published in every system's grid
because publishing only one is how a reader gets misled.

---

## 6 · Batching — T2 against T3, and the answer tracks the weight regime

Three systems have a T2/T3 answer. DeepProve cannot batch (`batch_size` pinned to 1) and its
MLP does not prove anyway; jolt-atlas **can** express a batch of 8 and T3 never reaches it,
stopped by the width-1 output layer.

| | **binius64** (1 thr, r1) | **Ceno** (10 thr) | **gnark** (G16 rA, 10 thr) |
|---|---:|---:|---:|
| MACs | ×8.00 | ×8.00 | ×8.00 |
| the system's own natural unit | constraints **×8.00** (195 975 → 1 567 800) | cycles **×7.89** | constraints **×3.43** (283 408 → 973 058) |
| prove time | **×1.015** (8×T2 = 2 558.80 ms vs T3 = 2 596.94 ms) | **×6.58** | **×1.743** |
| peak footprint | **×7.13 worse** (1.05 → 7.48 GB) | **×2.74** | **×1.730** |
| proof bytes | **÷6.02 better** (2 771 200 → 460 112) | ×1.20 | **×1.000** — 196 B for one inference and 196 B for eight |
| **per request** | proof ÷6.02; **time 1.5 % worse at 1 thread, 1.22× better at 10** | time 1.22×, memory 2.92×, proof 6.67× | **time 4.59×, memory 4.62×, proof 8.00×** |

**The mechanism is the weight regime, and it is measured rather than argued.**

- **gnark (`witness`, range-checked)** shares the weights across the 8 batch items, so the
  ~92 000 **weight range checks are paid once for all eight**. That is why constraints grow
  ×3.43 rather than ×8, and it is the *only* reason: the multiplications still grow ×8.
- **binius64 (`witness`, no range checks)** also shares the weights and holds **21 % fewer
  private values** than 8 × T2 (2 399 408 against 3 044 976) — and gets **nothing** on time,
  because **its constraint count does not fall**: 8 × T2 and T3 hold *the same* 1 567 800
  constraints. There are no range checks to amortize. **gnark's batching win is exactly the
  cost binius64 declines to pay in §7**, seen from the other side.
- **Ceno (`program-data`)** re-reads every weight with an `lb` for every batch item, so there
  is nothing to amortize at all: ×7.89 in cycles against ×8.00 in MACs, and the shortfall is
  only the fixed program prologue being shared. *"A zkVM proving eight inferences executes
  eight times the instructions."*

**And the one place batching buys something in every system that has it: proof size.** ÷6.02
(binius64), ÷6.67 per request (Ceno), and gnark's Groth16 proof is **constant at 196 bytes**.

**One correction inside binius64's own file, carried here because it flips a sign.** Its
grid's T3 verify row reads 9.4 % *worse* than 8 × T2; at steady state it is **8.9 % better**
(8 × 8.26 = 66.08 ms against 60.23 ms). Both figures are cold in the grid, and the mechanism is
exact rather than noisy: 8 × T2 and T3 hold the same constraints, so the linear verify term is
identical and what batching saves is the seven redundant copies of the succinct per-proof term.

**Batching also hardens witness binding.** In T3 one weight matrix serves 8 independent inputs,
so a weight must be inert in all eight at once: **52.27 % of T2's weights are inert against
3.27 % of T3's — a ~16× hardening**, measured exhaustively. §10.

---

## 7 · The INT8 statement — the row carries the decomposition or the comparison is invalid

`TASKS.md` asks systems that cannot express INT8 natively to *"declare their encoding"*. All
five did. **What no single document said before now is that the resulting statements differ in
strength.**

| system | is INT8-ness of the operands **proved**? | mechanism | what it costs |
|---|---|---|---|
| **binius64** | **No, explicitly** | *"witnessed as full 64-bit words with **no range constraint**"* | **zero** — and its own file states the consequence: *"A production deployment would need those range constraints, and they are not in these numbers."* |
| **Ceno** | **Not needed — structural** | one operand per byte of the hint region, read with a sign-extending `lb`; *"every byte trivially is [in range], since a byte is 8 bits"* | **zero**, structurally |
| **DeepProve** | **Not stated** | quantizes a float graph itself; `ZKML_BIT_LEN=8` sets the domain to exactly `[-128, 127]` | **NOT DETERMINED** — no claim either way in any of its documents |
| **jolt-atlas** | **Not stated**, and the committed domain is **15 bits, not 8** | i32 fixed point at `MODEL_SCALE = 14`; operands carried as `v·128`, so the committed integers are **128× the task's INT8 values** | **NOT DETERMINED** |
| **gnark, regime A** | **YES** | `std/rangecheck`, `rc.Check(v+128, 8)`, **every input and every weight** | **3.006×** — see below |
| **gnark, regime B** | inputs yes, **weights no** — they are constants, so there is nothing to range-check | same, applied only to `X` | (not a cross-system figure) |

**The decomposition, measured, because rule 6 requires it in the same row:**

| T1-0, Groth16, regime A | constraints | share |
|---|---:|---:|
| multiplications + output asserts | 65 791 | 33.3 % |
| **range checks on 65 792 INT8 values** | **131 972** | **66.7 %** |
| total | 197 763 | 100 % |

So gnark's headline is **3.0176 constraints/MAC**, and the same circuit **without** the range
checks the other four also omit is **1.0039** — one R1CS constraint per multiply-accumulate,
the floor. Confirmed by two independent routes: decomposing T1-0, and an isolated 256-MAC dot
product giving 257/256 = 1.0039. **The tax is 3.006×.**

**And the per-value cost is not a constant** — it amortizes a shared lookup table: 4.19
constraints per value at n = 16, falling to **2.01 at n = 65 792**, which is T1-0 regime A's own
operand count. Quoting one number for "the cost of a range check" would be false at every n but
one.

**Consequence, and it is the reason this section exists.** A row placing gnark's 3.0176 beside
binius64's IMUL count compares a proof of *"the prover knows INT8 values whose product is the
output"* against a proof of *"the prover knows 64-bit words whose product is the output"*.
**Different theorems.** In §1.2's `witness` comparison, gnark is 1.72× better on memory **while
proving strictly more**, and binius64's advantage on `MAC/s` is measured against a circuit
carrying a cost it does not carry.

**One further asymmetry, in the other direction.** In a binary field or on a byte-addressed
machine, 8-bit-ness is structural rather than proved — **Ceno gets it free and gnark cannot.**
Neither is a virtue of the implementation; both are properties of the substrate.

---

## 8 · Trusted setup split three ways, ceremonies, and maturity

A `trusted setup y/n` column puts gnark-Groth16, gnark-PLONK, DeepProve and jolt-atlas in one
bucket. **Operationally they are not one bucket.**

| system | **trusted setup** | ceremony-backed? | how it behaves | measured cost |
|---|---|---|---|---|
| **binius64** | **none** | n/a | hash-based, no structured reference string | — |
| **Ceno** | **none** | n/a | hash-based. **Keygen is constant at ~9.4 s and does not vary with the task**, because the vk depends on the *program image*: one ELF serves the whole T1 ladder | 9.36–9.56 s, every task |
| **DeepProve** | **universal** | **NO** | HyperKZG SRS built **in process, per run**, from a random tau: `HyperKZGSRS::setup(&mut rng, max_degree)` | inside `setup`: 1 190.8 ms (T1-0) → 18 125.4 ms (T1-a), **2.32× its own prove time at T1-a** |
| **jolt-atlas** | **universal** | **NO** | HyperKZG SRS built in process by `setup_prover` | inside `setup`: 49.4 ms (T1-0) → 8 203.8 ms (T1-d) |
| **gnark, Groth16** | **PER CIRCUIT** | **NO** — gnark's own test utilities | A new weight matrix, a new layer width, **or one constraint past a power of two** forces a fresh setup | **dominates every rung**: 137 s of setup for 5.8 s of proving at T1-a; **355 s at T1-c** |
| **gnark, PLONK** | **universal SRS**, one per size bound, reusable | **NO** — `test/unsafekzg`, *"to be use for test purposes only"* by its own docstring | vk is **34 384 bytes at every task in the grid** | setup 6.7–8.9× cheaper than Groth16's; SRS 14.9–122.0 s, one-off |

**NO FIGURE IN THIS REPOSITORY IS A CEREMONY-BACKED SETUP.** All four systems with a setup
generate their toxic waste inside the measuring process. That is fine for benchmarking and it
is not fine for deployment, and the difference is declared rather than glossed.

**A second axis a y/n column destroys: the verifying key behaves oppositely in gnark's two
backends.** Groth16's vk grows with the number of public inputs — 8 688 B at T1-0 (257 public)
to **393 712 B at T1-c** (12 289), verify time with it, 2.13 → 8.26 ms. **PLONK's vk is 34 384
bytes at every single task.**

### 8.1 Security parameters and maturity — declared, never averaged

| system | security parameter as published by its own tree | post-quantum? | **maturity** |
|---|---|---|---|
| **binius64** | `SECURITY_BITS = 96`, held constant across the rate sweep (FRI query count 232 → 106) | hash-based, no SRS | **PROTOTYPE-RESEARCH** (self-labelled) |
| **Ceno** | `SecurityLevel::Conjecture100bits` — the only variant the enum defines. **We did not audit what it delivers and do not restate it as "100 bits."** | hash-based, no SRS | **PROTOTYPE-RESEARCH** — *"🚧 This project is currently under construction and not suitable for use in production. 🚧"* |
| **DeepProve** | **NOT DETERMINED** — no security-bit, soundness-bit or query-count parameter exposed or documented | no — BN254 pairings | **PROTOTYPE-RESEARCH** — *"This codebase is not audited and not production ready and is provided as is."* |
| **jolt-atlas** | **NOT DETERMINED** — same | no — BN254 pairings | **PROTOTYPE-RESEARCH** — no audit claimed anywhere in the tree |
| **gnark** | BN254 pairing assumptions, conjectured **~100 bits — below the 128-bit level often assumed**; explicitly **NOT post-quantum** | no | **PROD-AUDITED — the only such system here.** **Nine third-party audits**, five vendored in the measured tree |

**gnark's nine audits, with their scope stated precisely rather than rounded up:** Kudelski
2022-10 (gnark-crypto) · Consensys Diligence 2023-06 · Least Authority 2023-08 (Groth16
Solidity verifier template) · OpenZeppelin 2023-11 · Sigma Prime 2024-05 (gnark-crypto KZG) ·
**ZKSecurity 2024-05 (the standard library, where `std/rangecheck` lives)** · **OpenZeppelin
2024-06 (the PLONK prover and verifier)** · Least Authority 2024-09 · Least Authority 2024-11.
**For Groth16 what was audited is the Solidity verifier template, not the Go prover.** We did
not read the reports and do not restate their findings. The authors' own caveat governs the
label: *"provided as-is with no guarantees or warranties … does not guarantee constant-time
implementations or side-channel resistance."*

**The three security parameters — 96, "Conjecture100bits", and ~100 conjectured pairing bits —
are NOT compared here, in any direction.** Three different accountings of three different
things. `README.md`: a system with a trusted setup and a post-quantum system are not comparable
on security even when their milliseconds are.

**Licence is a measurement condition, not a footnote.** binius64 (Apache-2.0/MIT), Ceno
(MIT/Apache-2.0) and gnark (Apache-2.0) could be read, linked, instrumented and have their
mechanisms published. **DeepProve (Lagrange License) and jolt-atlas (ICME License) are not
OSI, forbid derivative works and reverse engineering, and are not vendored anywhere in this
repository.** That is why those two have no internal decomposition, no witness-level
correctness control, and `NOT DETERMINED` where the others have a named field. **The asymmetry
favours nobody's numbers — it means we know less about two of the five.**

---

## 9 · Reproduction — the fairness protocol's primary check, discharged or not

`README.md` commits to reproducing each system's own published reference number **before**
reporting anything about it, and to publishing the discrepancy *above* the result. **It fully
discharged for none of the five.**

| system | verdict | what happened |
|---|---|---|
| **binius64** | **NO TIMING REFERENCE EXISTS** — and the strongest available check **passes** | No results table, no committed benchmark baseline, no captured output anywhere in the tree; CI never runs `cargo bench` and says so. What it does publish is **CI-enforced circuit-size snapshots**, which are machine-independent: **7 of 7 valid examples match exactly.** Continuity with our own E-001/E-005 also reproduces: **3.3 % above** E-001's median, and the *fastest* repetition across three campaigns agrees to **0.003 %**. |
| **DeepProve** | **NOT REPRODUCED** | Half of the paper's Table 1 is BaseFold, and **BaseFold is not in the tree at the pinned commit**. The HyperKZG seq-64 row was attempted in **four `bench-llm` configurations across two build profiles**: the documented build panics in context generation (`Found different MLE for polynomial wte.weight`, 11–12 s, every time); with debug assertions off it produces a proof **its own verifier rejects**. **But the authors' own GPT-2 proving tests all pass here** (11/11, 4 of them proving), so the defect is in the benchmark binary, not the prover. Separately: **their README and their paper disagree by ~1.9–2.2× on prove time, on hardware both declare to be the same.** |
| **jolt-atlas** | **REPRODUCED at the commit that published the number; NOT at the commit measured** | At `53b7c873` (2026-05-06): prove 2.50 s against a published 2.288 s, **1.10–1.12×** — a reproduction. At `434ab99`, the tree measured here: **12.1 s, 5.3× slower**, decomposed by jolt-atlas's own spans (`commit_witness_polynomials` ×9.5, `iop` ×5.9, `prove_reduced_openings` ×3.4). **We did not bisect and we name no cause.** The README's "104× speed-up" claim, computed against the current tree, is **19.6×**. |
| **Ceno** | **NOT REPRODUCIBLE — structurally** | Its only published performance figure is a **GPU** figure, and round one is CPU-only. We recovered the series from `gh-pages` git history after a book deploy erased `dev/bench/`. Then their benchmarked example **would not run on our CPU build at all**: `Trap IllegalInstruction(0xc0001073)`, the Keccak precompile's syscall entry. **Cause NOT DETERMINED.** No ratio between any figure of ours and their 0.686 s appears anywhere. |
| **gnark** | **PARTIALLY REPRODUCED** | **Their instrument reproduces**: `gnark-bench` builds and runs unmodified at the commit it pins, and its `expo` circuit re-expresses cleanly at v0.16.2. **Their number does not**: **2.79–2.86·10⁵ constraints/s here against a published ">2·10⁶"**, a gap of **~7.0×**, at a circuit half the size of theirs. Core count differs 9.6× and runs the same direction; **we do not claim it explains the gap.** The published figure carries no commit, no thread count, no circuit source and no command — under this repository's own rules it would not have been publishable. |

**What this licenses.** It licenses reporting these numbers with the caveats above, because
every build passed its own system's integrity check on the same machine in the same campaign.
**It does not license any sentence of the form "gnark achieves 2M constraints/s" or "jolt-atlas
proves nanoGPT in 2.288 s"** — from us or from a reader of this repository.

---

## 10 · Correctness controls — coverage declared, not inferred

`README.md`: *"A corrupted trace must make `verify()` fail, in every system, on every task.
Systems that do not pass it are not reported."* **Two systems did not pass cleanly and are
reported anyway, with the reason given rather than the rule quietly relaxed.**

### 10.1 Coverage, side by side

| system | what could be corrupted | proof-artifact sweep | **accepted** | verdict |
|---|---|---|---:|---|
| **binius64** | witness (`private_word`), public output (`inout_word`), proof bytes | **sampled** — 3 positions per family, 6 tasks, 43 attempts (42 distinct) | **0** | **PASS.** Every rejection came from the **verifier**, not the prover: `PROVER_ERROR` is zero. |
| **Ceno** | **proof bytes only** — the witness is generated inside the prover from the ELF and hints and is never a mutable artifact | **strided, 1.56 %** — 18 161 of 1 162 285 offsets, stride 64, T1-0 only. Exhaustive would be ~16 hours for one task. | **0** | **PASS**, with coverage declared as a real weakness. **66.99 % of rejections are panics** — a DoS surface, not a soundness finding. |
| **DeepProve** | **artifact only** — licence forbids instrumenting internals | 214 single-bit corruptions over 2 artifacts (107 each): fine head sweep, coarse 5–95 %, every byte of the last 32 | **38** | **Proof body passes; artifact wrapper does not.** Both accepted regions mapped by measurement. |
| **jolt-atlas** | public input, public output, **artifact** — licence forbids instrumenting internals | **EXHAUSTIVE — every one of T1-0's 21 419 bytes**, plus 21/21 on public IO | **1 099 (5.13 %)** | **Proof body passes; part of the artifact is unread and part of the parser is fragile.** |
| **gnark** | witness, public inputs, **artifact** | **EXHAUSTIVE, both backends** — every one of Groth16's 196 bytes and PLONK's 584, on T1-0 and T2, in both regimes; plus `public_input_word` exhaustive over T1-0's 256 outputs | **0** | **PASS.** No corrupted proof accepted at any offset, in any task, in either backend. |

**The two systems that accepted something, and exactly what it was:**

- **DeepProve, 38 acceptances.** Region 1 is a **redundant, unverified copy of the model output
  at the head of the artifact**, sitting in front of the verified one — established from the
  struct declarations, and confirmed by the accepted prefix scaling *exactly* with the output
  element count (448 bytes at 256 elements, 1 472 at 768). `Provable.io.output` **is** an
  argument to the verifier and every probed offset between the head boundary and n−29 was
  rejected; `Output.outputs` is **not**. Region 2 is three bytes at n−29, n−15 and n−1, all
  holding `0x03`, which accept `^0x01` and `^0x02` but reject `^0x08` — **read, not ignored**,
  and **what they are is NOT DETERMINED** because naming them would mean reverse engineering
  the format. **This is an artifact-format defect, not a soundness one.**
- **jolt-atlas, 1 099 acceptances (5.13 %).** Region A — 20 runs of ~50 bytes at a period of
  204 — is the **opening point the verifier throws away**: `for (key, (_, claim)) in
  &self.opening_claims.0 { … OpeningPoint::default() … }`. The `_` is it. Mechanism read from
  the source, structure matches exactly. Region B — three 32-byte runs, one BN254 field element
  each — is **NOT DETERMINED**, and the licence forbids the reverse engineering that would name
  it. Separately, **314 flips make the process abort** on a 6.75-petabyte allocation from a
  corrupted length prefix: a refusal, and a DoS surface.

**Nothing here is a soundness finding against any system.** Every corruption of a proof body, a
public input, a public output, a sumcheck proof, a commitment or a claim value was rejected, in
all five systems, without exception.

**And exhaustiveness is not pedantry — it is load-bearing.** jolt-atlas's exhaustive sweep found
a region that a 124-offset sample had hit **once, by luck**, and a second region a sample
**missed entirely**. That is why gnark swept every byte of a 196-byte proof, and why Ceno's
1.56 % coverage is published as a weakness rather than rounded to "we found nothing".

### 10.2 Amendment A3 — what was re-labelled, and where it does not bite

A3 (2026-08-24): *a witness corruption counts as a test only if the reference output changes.*
Measured exhaustively over all 92 224 weights: **48 208 of T2's — 52.27 % — are inert**, and
3 016 of T3's (3.27 %). They feed neurons whose pre-activation is negative before *and* after
the change, ReLU discards them, and the output is bit-identical. **A perturbed inert witness is
a genuinely satisfying witness for the same true statement, so accepting it is correct.**

| system | witness-level family? | tasks with activations that ran | **effect of A3** |
|---|---|---|---|
| **binius64** | **yes** — `private_word` | T2, T3 | **6 rows re-labelled WEAK EVIDENCE** (3 on T2, 3 on T3). 10 `private_word` rows on T1 unaffected — a matmul has no inert weights. 11 `inout_word` and 16 `proof_byte` unaffected. |
| **gnark** | **yes** — `witness_word` | T2, T3 | Already applied in its own file. Pre-fix run published verbatim; corrected control selects **input** positions, which always propagate, and reports **zero non-control acceptances**. |
| **Ceno** | **no** | T2, T3 | **Does not bite** — every corruption is a `proof_byte` flip, and the sweep ran on T1-0 only. |
| **DeepProve** | **no** (licence) | none — T2/T3 never ran | **Does not bite.** |
| **jolt-atlas** | **no** (licence) | none — T2/T3 never ran | **Does not bite.** |

**NO VERDICT IS WITHDRAWN. No system accepted a corruption that changed the output.**
**Artifact corruption is unaffected and remains the strong control** — and it is the one that
is exhaustive in two systems.

**But read the "does not bite" column the other way.** A3 costs binius64 six rows of evidence
about weight binding and leaves it with the rest; it costs the other three nothing **because
they had no witness-level evidence to lose.** Three of five systems in this benchmark carry
**no evidence, weak or strong, that a perturbed weight is detected** — Ceno because its witness
is never a mutable artifact, DeepProve and jolt-atlas because their licences forbid the
derivative work. That is a gap in the benchmark, and it sits directly under §2's column.

**What no system's control establishes:** that a maliciously *constructed* witness would be
caught. gnark's corrected probe avoids the inert case rather than resolving it, and says so.

---

## 11 · `constraints` — a per-system natural unit, and NEVER a cross-system column

`README.md` mandates `constraints` in every conditions line. **Compliance is 3 of 5, and where
it is populated it means five different things.**

| system | `constraints` field | T1-0's natural unit | what one unit is |
|---|---|---:|---|
| **binius64** | present — *"MACs = IMUL constraints"* | **139 008** (imul + and + zero + bmul) | one 64×64 → 128 integer-multiply, AND, ZERO or BMUL constraint |
| **DeepProve** | **absent, silently** | — | — |
| **jolt-atlas** | **absent, silently** | — | — |
| **Ceno** | present — *"**NOT COMPARABLE** to a circuit's constraint count"* | **3 203 656 cycles** / 800 913 instructions | one proved RISC-V sub-cycle |
| **gnark** | present — a fifth meaning | **197 763** R1CS / 526 592 SCS / 1 026 / 67 592 | one R1CS row `A·B=C` (constant-coefficient linear combinations free) or one fan-in-2 PLONK gate (nothing folds) |

**Two of five silently drop a mandated field**, and that is not house style: both DeepProve and
jolt-atlas explicitly mark the adjacent `security` field `NOT DETERMINED`, three lines from
where `constraints` would have gone. They knew how to declare a gap.

**And inside gnark alone, the same 65 536 MACs are:**

| | R1CS (Groth16) | SCS (PLONK) |
|---|---:|---:|
| weights witness | **197 763** | **526 592** |
| weights baked in | **1 026** | **67 592** |

**A 513× spread inside one system with the arithmetic held fixed.** A quantity that moves 513×
without the task changing is not a property of the task. **`constraints` appears in this file
only inside a single system's rows, never as a column across systems.**

---

## 12 · What was not expressible, per system

| system | deviations from `TASKS.md` | frontend / prover walls hit |
|---|---|---|
| **Ceno** | **NONE. Zero deviation on all seven tasks.** No padding, no minimum layer width, no implicit requantization, no batch-size restriction, no matrix-vector limitation. | **1 thread aborts the prover.** `cargo ceno` does not build on aarch64; `cargo ceno verify` **cannot succeed at this commit** (`circuit_index_to_name` is `#[serde(skip)]` and the verifier requires it), so verify and the control ran against a vk regenerated in process. |
| **gnark** | **NONE.** 28 of 28 compile cells matched the frozen MAC count exactly. No minimum output width (probed first, 20 of 20 width-ladder cells OK), no reshaping, no power-of-two rung requirement, no implicit requantization, no batch restriction. **The FFT domain is padded to a power of two and peak memory follows the domain, not the constraint count** — measured ratios 1.013 to 1.998, in every row. | Only the machine: T1-d's disk watchdog. |
| **binius64** | **INT8 range constraints omitted** (declared, §7). No requantization, per A1. | **T1-d**: `MAX_VALUES_PER_SEGMENT`, a policy limit its authors bless raising. We did not raise it. |
| **DeepProve** | **Requantization after every linear layer, NOT disableable** — a deviation from T1's rule *and* from A1. **Power-of-two padding, not disableable**: 768 → 1024, **1.778× the task's arithmetic** at T1-a. | Matmul with `M > 1` not expressible; dense layer with output width < 4 not provable; **batch of 8 in one proof not expressible** (`batch_size` pinned to 1). |
| **jolt-atlas** | **Floor-rebase fused into every einsum, NOT disableable** — arithmetically the identity on T1 at scale 14, but **A1 cannot be honoured on T2/T3**. **Power-of-two padding**: the switch exists, is public, and **the prover rejects non-powers of two regardless** — 1.778× at the 768-wide rungs. Committed operand domain is **15 bits, not 8**. | Dense layer with output width 1 not provable. Batching expressible and unreachable. |

**Two systems expressed all seven tasks with zero deviation, not one: Ceno and gnark.**

`ceno/RESULTS.md` §7 claims the sweep as unique — *"Every other system in this benchmark hit at
least one frontend wall … Ceno's walls, when it hit them, were resources and defects, never
expressiveness."* **That sentence was written when four systems had been measured, and the
fifth invalidates it.** gnark's own file states the opposite for itself — *"The task
specifications are expressible as written. gnark hit no frontend wall — no minimum layer width,
no power-of-two rung requirement, no implicit requantization"* — and **28 of 28 compile cells
matched the frozen MAC count.** binius64 is a third case the sentence mis-sorts: its T1-d is a
**policy limit at `setup`**, and its own file is explicit that *"the task **is** expressible in
binius64's frontend — the constraint system exists, is correct, and carries exactly the
operations the spec asks for."* **Only DeepProve and jolt-atlas hit true expressiveness walls.**
Logged as a stale claim in §13.7.

**Corrected, the finding is better than the one Ceno claimed, not worse.** Two systems reached
zero deviation by opposite routes — **a RISC-V zkVM and a classical circuit frontend over a
prime field** — and they sit at the two extremes of §5's memory curve: Ceno carries a **~5 GB
floor and the worst `B/MAC` in the campaign**, gnark a **negligible intercept and the steepest
slope**. **Total expressiveness is available at either end of the cost curve, and it is not what
separates these five systems.** What separates them is §1.

---

## 13 · Limitations — including the six defects gnark's §8 found in this very table

`gnark/RESULTS.md` §8 was written before this file existed, expressly to break it. **All six
findings are adopted, and each is answered here rather than dismissed.**

**13.1 — The weight regime was an undeclared free variable (gnark §8.2).** **ADOPTED** as
Amendment A2 and applied to all five systems. §1 and §2 are its consequence. *Residual:* two of
five still exclude the weight cost from `bytes/MAC` by construction, and **no normalization
fixes it**. Declared in the column header, per A2 §3.

**13.2 — The five systems are not proving the same statement about INT8 (gnark §8.1).**
**ADOPTED.** §7 carries the 3.006× decomposition in the same section as the comparison.
*Residual:* two of five leave the strength of their INT8 statement `NOT DETERMINED`, and no
figure resolves it.

**13.3 — `constraints` means five different things (gnark §8.3).** **ADOPTED.** §11 publishes
it as a per-system natural unit only. *Residual:* two of five publish nothing at all where the
conditions line mandates a value.

**13.4 — `trusted setup y/n` is too coarse; there is no maturity row (gnark §8.4).**
**ADOPTED.** §8 splits it `{none | universal | per-circuit}`, adds a ceremony column (**no**,
in all four cases) and a maturity row. *Residual:* `README.md` says this benchmark does not
measure security, yet `security bits` and `trusted setup` were always in the conditions line —
so security *properties* were already in the table. Declaring them is the least bad option, not
a neutral one.

**13.5 — A Go process's memory is not four Rust processes' memory (gnark §8.5).** **ADOPTED
AND NOT CORRECTED.** §5.4. The `GOGC` sweep is published so a reader can size the ~1.61×.
*Residual:* Ceno's allocator is also not its authors' documented one, in the other direction.
**Two of five memory columns carry a declared, unquantified accounting bias.**

**13.6 — The `witness_word` control measures nothing on a ReLU task (gnark §8.6).** **ADOPTED**
as Amendment A3 and applied. §10.2. *Residual:* three of five systems had no witness-level
control to begin with, so A3 exposes a gap rather than fixing one.

**13.7 — Additional limitations this file found while being written.**

- **`gnark/RESULTS.md` §1 calls gnark and Ceno *"two systems whose `bytes/MAC` curves cross."*
  The measured points do not show a crossing of values** — Ceno is above gnark at all three
  common rungs. What crosses is the **local exponent ordering** (§5.2). The sentence is true of
  the slopes and would be extrapolation if read of the values.
- **`ceno/RESULTS.md` §7 claims a uniqueness that the fifth system removed.** *"Every other
  system in this benchmark hit at least one frontend wall"* was written when four systems had
  been measured. **gnark hit none** — its own §8.7: *"The task specifications are expressible as
  written. gnark hit no frontend wall"*, 28 of 28 compile cells matching — and **binius64's
  T1-d is a policy limit at `setup`, not a frontend wall**, its own `NOT_EXPRESSIBLE.md` §1
  being explicit that *"the task **is** expressible in binius64's frontend."* Only DeepProve and
  jolt-atlas hit expressiveness walls. **This is a stale claim rather than a wrong one, and it
  is the failure mode a campaign measured system-by-system invites: a universal quantifier
  written over the systems that existed at the time.** Corrected in §12. **We caught it in our
  own draft, which had quoted it approvingly three lines below a table that contradicted it.**
- **Ceno's T1-c and T1-d have no proving cell at all**, and `ceno/RESULTS.md` publishes no row
  for either — while `ceno/NOT_EXPRESSIBLE.md` §2 states that which rungs completed and why *is*
  reported cell by cell in `RESULTS.md`. **Two documents disagree about whether those cells are
  accounted for.** Recorded in §4 as NOT ATTEMPTED with the emulated cycle counts, which is all
  the evidence supports.
- **No thread setting compares all five systems** (§3), and `RAYON_NUM_THREADS` is not a thread
  count in three of them.
- **Ceno's `verify` at T1-a single-shard was never measured.** The only published T1-a verify is
  the 13-shard configuration (0.5627 s) and it is labelled as such. §3.1 carries the gap rather
  than borrowing the T1-0 figure.
- **The witness instances are not the same across all five.** binius64 and Ceno prove **the same
  instance, value for value** — checked, and the max-accumulator figures reproduce exactly
  (8 955 951 054 519 for T2). DeepProve's and jolt-atlas's generators draw from numpy's PCG64
  and gnark's from its own stream: **same seeds, same shapes, same MAC counts, different
  instances.** Task-level comparison only, never witness-level.
- **N is small on the expensive rungs** — N = 1 at binius64's T1-c rate 4, gnark's T1-b and
  T1-c, and every Ceno shard-sweep row. Where N = 1 no dispersion is published, because none was
  measured.
- **The machine was not dedicated in any campaign**, and was in its worst state during Ceno's
  and gnark's: on battery or with a boot volume 95 % full, 778 MB of free swap, load average
  ~5. `(u+s)/real` is published per cell so contention is visible rather than inferred.
- **`-C target-cpu=native` applied to Ceno** via its own committed `.cargo/config.toml` and did
  **not** apply to DeepProve or jolt-atlas, whose trees do not set it.
- **This machine gives gnark the BN254 element assembly but not the vector assembly**
  (`vector_purego.go` is tagged `purego || !amd64`; amd64 ships 13 routines including AVX-512
  IFMA). **It plays against gnark and we did not quantify it** — that needs an amd64 host and
  this campaign has one machine.
- **Three of our own jolt-atlas expressions were wrong before one was right**, each producing an
  error that looked like a limit of jolt-atlas. The check that caught them was running
  jolt-atlas's own bundled models through our harness. **Any remaining expression error would
  look the same, and we cannot rule one out** — in any system.
- **Nothing anywhere in this file is extrapolated outside a measured range.**

---

## 14 · What is still not comparable, after A2 and A3

The amendments fixed what could be fixed by declaring it. **These do not go away, and no
future amendment fixes them, because they are not omissions — they are differences.**

1. **`bytes/MAC` across a regime boundary.** Four buckets, two of them singletons. DeepProve's
   and jolt-atlas's figures **exclude the weight cost by construction**; binius64's, Ceno's and
   gnark-A's include it. A single five-column ranking of this metric **cannot be made valid**.
2. **What the proof binds.** `witness`, `preprocessed`, `circuit-constant` and `program-data`
   prove **four different theorems** about the model. There is no scalar in which "commits the
   witness" and "binds the weights to the verifying key" are more or less of the same thing.
3. **The INT8 statement.** One system proves it (3.006× tax), one gets it structurally free, one
   explicitly omits it, and two leave it `NOT DETERMINED`. **Two of five cannot be placed on this
   axis at all.**
4. **Security.** 96 bits by one accounting, an enum named `Conjecture100bits`, ~100 conjectured
   pairing bits below the usual 128, and **`NOT DETERMINED` twice**. Post-quantum in two, not in
   three. Trusted setup in three of five and **ceremony-backed in none**.
5. **Maturity.** Nine third-party audits against four self-declared research prototypes. This
   benchmark does not measure security and this row is not a security measurement — it is a
   declaration that one of these five is not the same kind of artifact as the other four.
6. **Memory accounting.** Go GC ~1.61× loose against Rust; Ceno's allocator not its authors'.
   **Two of five columns carry a declared, unquantified bias, in opposite directions.**
7. **Prove-time brackets.** DeepProve's includes quantized inference and jolt-atlas's includes
   graph tracing — **neither separable, both upper bounds.** Ceno's *excludes* emulation and
   witness generation while its memory includes them. gnark's excludes compile and setup while
   its memory includes them. binius64's is the whole prover call. **Five brackets, declared,
   never normalized.**
8. **Verify brackets.** binius64's is a warm in-process call (and its grid figures are the
   **cold** ones — 2.7–3.1× at T1-b); DeepProve's is a whole cold process at 10 ms resolution;
   jolt-atlas's and gnark's are warm in-process; Ceno's excludes a 3.87 s `ZKVMVerifier::new`.
9. **Proof size.** gnark's 196 B is a proof; DeepProve's 116 404 B **carries its verifier
   context** and is an upper bound on proof size; Ceno's 1 379 317 B is a function of its shard
   count. **Three different objects in one column.**
10. **Thread count.** Not a comparable quantity in three of five systems (§3).
11. **The task actually proved.** DeepProve and jolt-atlas prove **1.778×** the arithmetic at
    every 768-wide rung, and both apply a requantization `TASKS.md` forbids. Their denominators
    are the frozen MAC count regardless — **so their `MAC/s` is a rate for the task, not for
    their work, and their `bytes/MAC` is charged against fewer MACs than they performed.**
12. **The instance.** Only binius64 and Ceno prove the same one.

**None of that makes the campaign worthless. It makes it a map of cost shapes rather than a
ranking**, which is what `README.md` said it would be before any of it was measured:

> *"Not a ranking. It is a map of cost shapes. A system can win T1 and lose T2, and that is
> precisely the result we are looking for."*

**And it delivered exactly that.** jolt-atlas wins T1 on both axes and cannot express T2 or T3
at all. Ceno expresses every task with zero deviation, shares that with gnark, and has the
worst memory floor in the benchmark. gnark is the only audited system, the only one whose
per-circuit setup dominates its own prove time at every rung, and the only one that can bind a
model into a verifying key — cheaply, and while proving strictly more about INT8 than any other
system here. binius64 is second-fastest, **fourth of five on memory**, and — **of the only two
systems in which a witness-level corruption could be tested at all** — the one that rejects a
change which does not move the output, where gnark correctly accepts it. **The other three were
never testable on that axis**, and §10.2 is where that gap is recorded rather than rounded off.
DeepProve is the system whose published `0.686 bytes/MAC` motivated this repository, measured
here at **2 914–3 601 B/MAC** on tasks five to six orders of magnitude smaller — **a difference
of regime, not a contradiction, and one we refuse to extrapolate across.**

---

## Reproducing any figure in this file

Every number above is read from a system's own `RESULTS.md` or from `bench/data/`, and every
one of them is traceable to a row:

- `data/cells.csv`, `data/negative-control.csv` — binius64
- `data/cells-deepprove.csv`, `data/results-deepprove.csv`, `data/negative-deepprove/`
- `data/cells-jolt-atlas.csv`, `data/results-jolt-atlas.csv`, `data/negative-jolt-atlas/`
- `data/cells-ceno.csv`, `data/results-ceno.csv`, `data/cycles-ceno/`, `data/negative-ceno/`
- `data/cells-gnark.csv`, `data/results-gnark.csv`, `data/compile-grid-gnark.csv`,
  `data/negative-gnark/`
- `data/repro-*/` — one directory per reproduction attempt, including the ones that failed

**Two figures in this file are derived rather than read**, and both are marked at their point of
use: binius64's local memory exponents (1.188 / 0.961 / 0.877) and DeepProve's (0.916), computed
from those systems' own published peak-footprint columns because neither file publishes an
exponent. Every other system publishes its own.

**If any of this is wrong, [`CHALLENGE.md`](CHALLENGE.md) applies to every row.** We would
rather be corrected in public than be wrong in private, and the old numbers stay in the record
next to the new ones.

---

# Amendments

This file was frozen when the campaign closed. Frozen does not mean immutable — it means every
change is logged here with its date, its reason, and its effect on figures already published.
**Silent edits are the failure mode this log exists to prevent.** Amendments A1–A3 amend the
task specification and live in [TASKS.md](TASKS.md#amendments); A4 amends
[README.md](README.md#amendments). The series is shared across the frozen documents.

## A5 · Ceno's top rungs were run, and four claims in this file are now wrong (2026-09-02)

**What changed.** Ceno T1-c and T1-d — reported in §4 as `✗ NOT ATTEMPTED` — were run at a
**reduced shard cap** (`--max-cycle-per-shard` = 2²³ = 8 388 608), together with a new T1-b cell
at the same cap and a thread sweep. All proved and **all verified**. T1-b was also re-run at 4
threads, and T1-a was re-measured at the same cap with N = 3.

```
task  shards  threads  prove s    peak fp   proof      verify s
T1-a       4       10    30.02*   9.19 GB   4.61 MB      —
T1-b      13       10   129.97   11.29 GB  15.02 MB   0.6066
T1-b      13        4   315.76    9.89 GB  15.02 MB      —
T1-c      49       10   539.29   11.43 GB  56.50 MB   2.2893
T1-d     193       10  2107.82   11.75 GB 222.40 MB   8.8756
```
\* prove figures were taken on an uncontended machine except T1-a's re-runs; see "Corrections" below.

**These cells use a configuration the authors do not document as their default.** The fairness
protocol measures each system in the configuration its own authors document, and the default caps
(2²⁹ cycles / 2³¹ cells) are one of them. **These rows are published as what they are — a
declared deviation, with the cap and the shard count in the same row — and they do NOT fill the
default-configuration gap.** At default caps, T1-c and T1-d remain unmeasured on this machine.

**What is now wrong in this file, item by item:**

1. **§4, the grid.** Ceno T1-c and T1-d read `✗ NOT ATTEMPTED`. They are now proved and verified
   at cap 2²³. The default-cap cells remain unattempted, and the grid does not distinguish the
   two. **The cause of the original absence is also now known and it was ours:**
   `scripts/ceno/run-all.sh:82` skipped T1-c behind a pre-flight disk guard, and **T1-d had no
   invocation in the script at all.**
2. **§13.7.** *"Ceno's T1-c and T1-d have no proving cell at all"* — obsolete.
3. **§5.1, Ceno's curve.** Its exponents (0.412, 0.734) are measured with **the shard count
   changing inside the table** (1, 1, 2). The table's own note declares this without correcting
   it. **It is not a fixed-configuration curve**, and the cells above are the first one in this
   benchmark that is.
4. **§5.5 is falsified in both its coverage and its mechanism.** It states *"every other cell in
   the campaign — within ~1 %"* and attributes divergence to RSS saturating *"above ~16 GB"*.
   The new cells diverge **1.238 / 1.341 / 1.459 with peak RSS of 8.0–9.2 GB** — nowhere near
   saturation. The coverage claim is false and the stated mechanism does not apply. **A
   replacement mechanism is NOT established.**
5. **§14, item 10.** It says thread count is *"not a comparable quantity in three of five
   systems"*, treating it as a time variable. **In Ceno, threads are a memory knob**: the same
   task at the same cap and the same 13 shards peaks at **9.89 GB on 4 threads and 11.29 GB on
   10** (×1.142). **No memory column in this file declares its thread count as a condition of
   the memory figure.** Every Ceno memory figure should be read as a property of
   (task, shard cap, **threads**).

**What this campaign adds, stated as a bound and not as an exponent.** Once shard count exceeds
thread count, peak footprint stops tracking task size: T1-b/c/d at 13, 49 and 193 shards on 10
threads peak at 11.29 / 11.43 / 11.75 GB — **×1.040 across ×16 in MACs**, where linear scaling
predicts +1500 %. Two cells with the **same number of shards in flight** but tasks 4× apart in
size land **7.6 %** apart. Meanwhile the cost is exactly linear elsewhere: **1.153 MB of proof
per shard** (spread 0.25 %) and **0.0466 s of verify per shard** (1.59 %), with throughput
constant at 17 500–18 150 MAC/s (3.74 %). **A quantitative model is NOT established: 0.73 GB of
the difference between the two same-in-flight cells is unexplained.**

**A composition control was run, and it passed.** Because proof bytes and verify time are exactly
linear in shard count with no sign of aggregation, we tested whether the verifier binds the shards
into one statement. Six mutations on the T1-d (193-shard) and T1-c (49-shard) proofs — drop,
duplicate, transpose, graft a shard from the other proof, truncate: **26 of 26 rejected by
semantic `VerifyError`, zero panics, zero deserialization errors**, against three controls that
accept. The binding mechanisms observed are `init_pc` chaining, a **global `shard_ec_sum`
elliptic-curve accumulator**, hint continuation, and a halt-position check. **This establishes
that honestly-produced shards cannot be recomposed. It says nothing about an adversarial prover**,
which is a soundness claim this benchmark does not make. Raw data: `data/compose-ceno/`.

**Corrections to method, disclosed:**

- **A harness bug is fixed in the derived ledger, not in the raw data.** `scripts/ceno/run-cell.sh`
  recorded `create_proof_s` from **one** `ZKVM_create_proof` span out of the 49 or 193 emitted —
  18.80 s instead of 539.29 s for T1-c, which would have made segmented cells look **28× faster**
  than they are. `scripts/ceno/reparse.py` sums them, and its own comment documents the same class
  of bug caught once before. It also writes rows one field short of the header. **Re-deriving the
  whole Ceno ledger from the raw logs reproduces all 31 previously published rows byte for byte**,
  so no published figure changes.
- **One measurement was contaminated by us and is flagged rather than dropped.** T1-a's three
  repetitions ran while another process was working the disk: prove time rose **25.5 %** against
  the 2026-08-24 figure and `(u+s)/real` fell to 3.77. Peak footprint was unaffected
  (**+0.3 %, spread 0.1 %, across a 3× range of load average**), which is itself the finding:
  **footprint is robust to contention on this machine and prove time is not.**
- **Peak RSS is not measurement-grade here.** Across four measurements of the same cell it varies
  **22.9 %** between repetitions minutes apart. §5.5 already said *"the footprint column is the
  one to read"*; these numbers say why.

**Effect on previously published figures.** None. No figure, no verdict and no raw data file from
the original campaign is altered by this amendment. Every claim above rests on cells added after
it, committed uncurated alongside the originals.
