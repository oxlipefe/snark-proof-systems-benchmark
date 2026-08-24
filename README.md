# snark-proof-systems-benchmark

**A cross-system benchmark of zero-knowledge proof systems that measures the metric nobody publishes: bytes of prover memory per arithmetic operation.**

> **Status: in progress.** Methodology is frozen; measurements are being collected.
> This file contains no results yet. When it does, every number will carry its full
> conditions line. Nothing here is a marketing claim.

---

## Why this exists

Public comparisons of proof systems measure wall-clock time. Almost none measure memory,
and — as far as we could find — **none measure memory normalized per operation across
systems**.

That omission matters, because memory is not a performance detail. It is a binary gate:

- Slow means you wait longer. More cores, a GPU, or a better implementation fix it.
- **Not fitting means it does not run at all** — not in a day, not in a year, not for more money.

And the two behave differently under scale-out: **wall-clock time parallelizes across
machines, peak memory does not**, unless the protocol was designed to be split.

We found this the expensive way, and the way we found it is itself the argument for this
repository.

Our own prover consumes **6,268–7,932 bytes per MAC**. A published system reports figures
implying **0.686 bytes per MAC** in its own evaluation table. Dividing one by the other gives
a gap of ~10⁴× — and **that division is invalid**, which is exactly the point. The two figures
come from workloads roughly **10⁶ apart in size**, on different machines, under different
rules. We made that error ourselves, in an earlier version of this file, before measuring.

Measured properly — same task, same machine, same campaign — the gap between those two systems
is **2.3–4.2×**, not 10⁴×. And a third finding fell out of it: **`bytes/MAC` is not a constant
of a proof system.** It moves with task size in every system we have measured so far. A single
number quoted without its workload is not a property of the prover; it is a property of the
pair.

That is why cross-system, same-task, same-machine measurement is worth the effort, and why
per-system numbers quoted from separate papers cannot substitute for it — no matter how
carefully each paper reports its own.

**We think these numbers should be public and comparable. That is what this repository is.**

## Prior art, and what we found missing

| Source | What it measures | Memory per operation? |
|---|---|---|
| Fenbushi, *Benchmarking zkVMs* (2025-08-29) | peak RAM per whole task, fixed hardware | No — not normalized |
| ePrint 2026/1729, *A Unified Framework for Contract-Validated Benchmarking* | RAM figures per run | No — not normalized |
| zkbenchmarks.com | wall-clock only | No — memory not reported at all |
| DeepProve (ePrint 2026/1112) | **publishes RAM per run**, in the paper | Not normalized, single system |

DeepProve's own paper states of two competing systems: *"Neither zkGPT [QSL+25] nor
zkTorch [CTK25] reports the actual RAM usage."*

**If a cross-system, per-operation memory benchmark already exists, tell us and we will
link to it instead of building this.** See [CHALLENGE.md](CHALLENGE.md).

## What is compared

Not "the same circuit" — that is impossible across systems with different frontends,
fields and security models, and pretending otherwise is how these benchmarks become
dishonest. **We compare the same task**, expressed natively in each system, with the
expression published for every system. See [TASKS.md](TASKS.md) for exact specifications
and exact MAC counts.

| | Task | Purpose |
|---|---|---|
| **T1** | INT8 matrix multiply, ladder over 3 orders of magnitude | The shape a linear layer has; ~99% of a forward pass |
| **T2** | A complete MLP (92,224 MACs), end to end | A whole model, not a tile |
| **T3** | The same MLP, batch of 8 | Isolates whether batching independent requests buys anything |

If a system cannot express a task, **that is reported as a result**, not hidden.

## Metrics

Measured directly: **prove time**, **peak memory** (both peak footprint *and* peak RSS —
they diverge, and the divergence is itself a finding), **proof size**, **verify time**.

Reported alongside, never folded in: one-off setup/preprocessing time, and
`(user+sys)/real` — the control that tells you whether wall-clock time was computation or
waiting.

**Derived, and always published as a pair:**

> **`MAC/s`** and **`bytes/MAC`**

A prover twice as fast that uses three times the memory is worse, not better. Publishing
either number alone is how the field got here.

## Fairness protocol

Measuring someone else's system badly is easy, and it is the most common failure of
benchmarks like this one.

1. **Every system runs in the best configuration documented by its own authors** — build
   flags, features, release profile. The exact configuration is published.
2. **Build integrity is verified per system, before measuring.** In our own prior work,
   compiling without LTO made our prover measure **9.0× slower** and inverted a conclusion.
   Every system gets its own documented check.
3. **We first reproduce each system's own published reference number.** If we cannot,
   **the discrepancy is published before the result is.**
4. **Right of reply.** Authors can correct us. Corrections are applied with credit and date.

## Correctness control (non-negotiable)

**A corrupted trace must make `verify()` fail, in every system, on every task.** Without
this control you are not benchmarking proofs — you are benchmarking computations that
happen to produce bytes. Systems that do not pass it are not reported.

## Conditions line

Every published figure carries this, or it is not published:

```
system · commit · task · MACs counted · constraints · field · security bits ·
trusted setup y/n · ZK y/n · quantization · rate · threads · machine · OS · N ·
peak footprint · peak RSS · date
```

Differences no normalization can fix are **declared, not averaged**: a system with a
trusted setup and a post-quantum system are not comparable on security even when their
milliseconds are. That goes in the table, not in a footnote.

## What this benchmark does NOT measure

- **Not model quality.** No perplexity, no accuracy.
- **Not security.** Faster says nothing about soundness, and nothing about whether a system
  was audited.
- **Not GPU**, in round one. CPU only, so the comparison is between protocols rather than
  between kernel-porting efforts. GPU is a declared second round.
- **Not a ranking.** It is a map of *cost shapes*. A system can win T1 and lose T2, and that
  is precisely the result we are looking for.

## Honesty rule

This repository does not exist to show that our system wins. **If ours comes last, it is
published last.** Raw data is committed uncurated, including runs that failed and cells
that were never run, with the reason.

## One transformation was applied to the raw data

Captured stdout and stderr contained absolute filesystem paths from the machine the
campaign ran on. Those path prefixes — and the owner column of a handful of `ls -l`
listings — were rewritten before the first commit:

```
/Users/<user>/<project>/bench            -> /repo
/Users/<user>/<project>                  -> /repo/..
/Users/<user>                            -> /home/bench
/private/tmp/.../scratchpad              -> /tmp/bench-scratch
ls -l owner column                       -> bench staff
```

**Nothing else was touched.** This was verified by applying the identical transformation to
the pre-redaction tree and diffing byte-for-byte against the published one: the two are
identical, so the rewrite is provably confined to path strings. No measurement, no verdict,
no log line other than a path was altered, and no file was removed.

It is disclosed here rather than done silently because this repository's argument rests on
its raw data being trustworthy, and a reader has no way to tell a benign path rewrite from a
convenient one unless the transformation is published alongside it.

## Reproducing

Every system directory under `systems/` contains the pinned commit, the build check, the
task expression, and the exact commands. `data/` holds raw uncurated CSVs. `scripts/`
rebuilds everything from zero.

## License

Benchmark code and data: MIT. Each measured system remains under its own license; this
repository measures them and does not redistribute or derive from them.
