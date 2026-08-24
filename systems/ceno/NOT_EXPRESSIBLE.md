# ceno — tasks this system could not run, and why

`bench/README.md` commits to reporting a task a system cannot express as a **result**, not as a
gap. This file is that report for Ceno.

The distinction this file keeps, as the other three do:

- **not expressible** — the frontend refuses to build the task;
- **expressible but not runnable** — the task builds and the prover then refuses it, or the
  machine does.

**For Ceno the first column is empty, and that is the headline.** A zkVM has no frontend to
refuse anything: every task in `bench/TASKS.md` is an ordinary RISC-V program, and every one of
them compiled and executed. Where Ceno stopped, it stopped for reasons of *resources* and of
*configuration*, never of expressiveness.

| Task | Expressible? | Executed? | Proved? | Where it stopped |
|---|---|---|---|---|
| **T1-0** | yes | **yes** | **yes** | — |
| **T1-a** | yes | **yes** | see RESULTS.md | — |
| **T1-b** | yes | **yes** | see RESULTS.md | — |
| **T1-c** | yes | **yes** | see RESULTS.md | 403 791 196 cycles — §2 |
| **T1-d** | yes | **yes** | see RESULTS.md | 1 615 020 812 cycles, exceeds the default single-shard cap — §2 |
| **T2** | yes | **yes** | **yes** | — |
| **T3** | yes | **yes** | **yes** | — |

Every cell's exact cycle count, including for rungs that were not proved, is in
[`EXPRESSION.md`](EXPRESSION.md) §1 and in [`bench/data/cycles-ceno/`](../../data/cycles-ceno/).
This is a capability the other systems' entries do not have: Ceno's emulator runs without
proving, so a rung too large to prove still yields an exact measurement of how much work it is.

---

## 1. The wall that is not about task size: `RAYON_NUM_THREADS=1` aborts the prover

**This is the most consequential limitation in this file, because it removes the cut that the
rest of the benchmark is built on.**

binius64's primary cut is 1 thread — the configuration that compares protocols rather than
parallel implementations. Ceno cannot be measured there. At `RAYON_NUM_THREADS=1` the prover
runs for seconds and then aborts:

```
thread 'main' panicked at
  .../gkr-backend-.../crates/sumcheck/src/prover.rs:426:9:
assertion `left != right` failed: Attempt to prove a constant.
```

### 1.1 The cause is Ceno's, not our task's

Isolated to one variable, same binary, same minutes — and then confirmed on **the authors' own
examples**, which is what makes it attributable:

| Program | `RAYON_NUM_THREADS=1` | `=2` |
|---|---|---|
| our `bench_t1`, T1-0 | **panic** after 17.5 s | proves in 9.86 s |
| upstream `fibonacci` | **panic** after 5.27 s | — |
| upstream `ceno_rt_alloc` | **panic** after 4.88 s | — |

Two of Ceno's own examples, unmodified, fail identically. Nothing about our expression is
involved.

### 1.2 What it costs the benchmark

The 1-thread column is **empty for Ceno and cannot be filled**. Any comparison against
binius64's primary figures is therefore between a 1-thread circuit prover and a ≥2-thread
zkVM, and RESULTS.md says so in the same sentence as any such number rather than in a
footnote. The minimum cut we could measure is **2 threads**, and it is reported beside the
10-thread default.

**Right of reply applies here more than anywhere else in this file.** If a supported minimum
thread count exists, or if this is fixed at a later commit, we will re-run the whole ladder at
1 thread and publish both.

## 2. The wall that is about resources: cycles, shards, and a machine at 95 % disk

Ceno segments a trace too large for one shard, so there is no hard task-size ceiling of the
kind binius64 hit at `MAX_VALUES_PER_SEGMENT`. What there is instead is a **budget**, and this
machine's was unusually poor at campaign time (`BUILD.md` §8: 32 GiB RAM with 778 MB of free
swap, and 49 GB free on a boot volume at 95 %).

Two limits, of different kinds:

**(a) The default single-shard cap.** `--max-cycle-per-shard` defaults to `2^29 = 536 870 912`.
T1-d needs **1 615 020 812** cycles, so it cannot be a single shard at the default and is
segmented into four. T1-c, at **403 791 196** cycles, fits in one shard by 25 %.

**(b) The default cell cap is a GPU default.** `--max-cell-per-shard` defaults to
`2^31 = 2 147 483 648`, and the source comment states the reasoning outright:

```rust
// max cycle per shard
// default value: 16GB VRAM, each cell 4 byte, log explosion 2
// => 2^30 * 16 / 4 / 2
#[arg(long, default_value = "2147483648")]
max_cell_per_shard: u64,
```

**The shipped default is sized for a 16 GB GPU, not for a CPU host.** That is not a criticism —
the CI benchmarks on CUDA — but it means the out-of-the-box configuration is not the one a CPU
run should use, and that peak prover memory on this system is **a function of a flag** rather
than a property of the task. RESULTS.md measures that directly rather than asserting it.

Which of T1-a…T1-d completed, at which shard settings, and which were stopped by memory or by
the operator to bound campaign wall time, is reported cell by cell in
[`RESULTS.md`](RESULTS.md), including the cells that produced no proof and why. **No rung's
result is extrapolated from a smaller one** (`bench/CHALLENGE.md`).

## 3. What could NOT be measured because of defects in the tree, not because of the task

Three, each with its mechanism established in [`BUILD.md`](BUILD.md) before being written here:

1. **`cargo ceno`, the documented primary interface, does not build on aarch64** (BUILD.md §2).
   Everything it does was done through `e2e` and our harness with flags copied from the CLI's
   own source, but the CLI itself was never exercised as a command.
2. **`cargo ceno verify` could not have succeeded even if it had built** (BUILD.md §5):
   `ZKVMVerifyingKey::circuit_index_to_name` is `#[serde(skip)]` and the verifier requires it,
   so every deserialized vk rejects every honest proof. Verify timing and the correctness
   control were therefore run against a vk regenerated in process.
3. **The authors' recommended allocator configuration was not measured** (BUILD.md §4). It sets
   `retain:true` with decay disabled — never returning pages to the OS — which would make the
   memory metric measure the allocator instead of the prover. We measured the system allocator
   and declared it.

## 4. What Ceno was NOT asked to do, and so is not reported as unable to do

- **ZK.** Nothing here measures zero-knowledge. The benchmark's distinction between privacy and
  verifiable integrity is strict, and round one measures the latter.
- **GPU.** Declared out of scope for round one (`bench/README.md`). Ceno's only published
  figure is a GPU figure, and [`REPRODUCTION.md`](REPRODUCTION.md) explains what that costs.
- **Recursion / aggregation.** `ceno_recursion_v2` exists and is a large part of the tree's
  recent work. It was not measured, and it is the component that would matter most for a
  multi-shard or multi-proof deployment.
- **Continuations across proofs.** Not asked for by any task.
- **LLM inference.** No task in `bench/TASKS.md` is a language model. T2's MLP is 92 224 MACs;
  a GPT-2 forward pass is roughly four orders of magnitude beyond the top of this ladder.
