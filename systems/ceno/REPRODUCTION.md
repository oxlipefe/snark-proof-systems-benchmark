# ceno — the system's own reference number, and whether we reproduced it

`bench/CHALLENGE.md`: *"We commit to reproducing your own published reference number before
reporting anything about your system. If we could not reproduce it, that discrepancy is
published above our result, not in a footnote."*

**We could not, and the reason is structural rather than a failure of the attempt.** Ceno's only
published performance figure is a GPU figure, and round one of this benchmark is CPU-only — so
it is not a number this machine can produce. We then tried the next best thing, running their
benchmarked example on our CPU to put both measurements on one workload, and **that did not run
either**: it traps before halting on the default CPU build (§3).

So this file reports two things: where the reference number actually lives, since it is in
none of the places the documentation points at (§1), and why no figure in
[`RESULTS.md`](RESULTS.md) is validated against anything of the authors' own (§2, §3).

---

## 1. Finding the reference number, which is not where the documentation says

Nothing in the places one would look publishes a figure:

| Source | Performance figures? |
|---|---|
| ePrint 2024/387, *Ceno: Non-uniform, Segment and Parallel Risc-V Zero-knowledge Virtual Machine* | **No.** Asymptotic cost analysis only; no evaluation table |
| Run Book, [scroll-tech.github.io/ceno](https://scroll-tech.github.io/ceno/) | **No.** `docs/src/profiling.md` explains how to profile your own program |
| Scroll blog, *How Ceno Achieves High Performance ZK Proving* | **No.** Architecture, no numbers |
| `ceno_zkvm/benches/` (criterion) | Benchmarks exist; **no results are committed and no workflow runs them** |
| zkbenchmarks.com · Fenbushi, *Benchmarking zkVMs* (2025-08-29) | **Ceno is not measured by either.** Fenbushi names it once as an example of the GKR family |

But a reference number does exist, and it is machine-generated rather than written down.
`.github/workflows/gpu-integration.yml` records a proving-time baseline on every push to
master via `benchmark-action/github-action-benchmark`, pushing it to the `gh-pages` branch.

**It is not on `gh-pages` today.** The branch head is a book deploy, and `dev/bench/` is gone.
The mechanism is visible in the same repository: `deploy-book.yml` publishes `docs/book` to
`gh-pages` with `peaceiris/actions-gh-pages@v3`, which replaces the branch contents, so the
book deploy erases the benchmark history the other workflow writes.

The data survives in the branch's git history, and that is where we recovered it:

```
$ git log gh-pages --oneline
0811466a deploy: f17c6c18…                                          <- book deploy, wiped dev/bench
c5326c32 add GPU proving time (customSmallerIsBetter) benchmark result for c03ee97f…
e2f0aa5b add GPU proving time (customSmallerIsBetter) benchmark result for e2428402…
a91d1b32 add GPU proving time (customSmallerIsBetter) benchmark result for 722cd60f…
a4dffb4f add GPU proving time (customSmallerIsBetter) benchmark result for 314db14c…
```

Recovered verbatim into
[`bench/data/repro-ceno/gh-pages-data.js`](../../data/repro-ceno/gh-pages-data.js).

### The published series

| Ceno commit | date (UTC) | `keccak_syscall proving time` |
|---|---|---:|
| `def271c2840b` | 2026-07-12 | 0.722 s |
| `314db14cd9ed` | 2026-07-14 | 0.705 s |
| `722cd60f74e6` | 2026-07-14 | 0.732 s |
| `e2428402c233` | 2026-07-15 | 0.685 s |
| **`c03ee97f0b76`** | **2026-07-20** | **0.686 s** |

Conditions, from the workflow that produced them:

```
metric      the `ZKVM_create_proof` tracing span, emitted by `--profiling 1`
            "pure proof generation, excluding emulation/witgen"
binary      cargo run --release --package ceno_zkvm --features gpu --bin e2e
build       RUSTFLAGS="-C opt-level=3"
example     keccak_syscall
machine     self-hosted runner, [self-hosted, Linux, X64, gpu], CUDA
memory      NOT STATED in the workflow; a sibling PR description reports 24 GB device memory
threads     NOT STATED
N           1 — one run per master push, no repetition, no warmup declared
```

**The reference is 9 commits and 25 days older than the tree we measured** (`c03ee97f`,
2026-07-20 → `ac164255`, 2026-08-14). That is far closer than jolt-atlas's reference, which was
3½ months and ~40 commits stale.

## 2. Why it cannot be reproduced here

`bench/README.md` declares round one **CPU only**, "so the comparison is between protocols
rather than between kernel-porting efforts". The published figure is produced by
`--features gpu` on a CUDA runner. There is no CPU baseline published at any commit.

Three further gaps, stated so the number is not over-read even by someone with a GPU:

1. **N = 1.** One run per master push. No dispersion is published, and the series itself spans
   0.685–0.732 s (a 6.9 % range) across commits whose PR descriptions claim much larger
   changes, so run-to-run noise is not separable from real movement.
2. **The GPU backend is not in this repository.** `make enable-gpu` switches to "remote
   implementation, requires private repo access", and the workflow loads
   `secrets.CENO_GPU_DEPLOY_KEY` to clone it. The measured code is therefore not public, and
   the figure is not reproducible by anyone outside Scroll regardless of hardware.
3. **`keccak_syscall` is not one of our tasks.** It exercises the Keccak precompile chip; T1,
   T2 and T3 exercise the base RV32IM opcode chips. A ratio between them would be meaningless.

## 3. We could not even run their benchmarked example on CPU

The honest closest thing to a reproduction would have been their example, their span, their
extraction pipeline, their commit — our CPU. **It does not run.**

```
$ RAYON_NUM_THREADS=10 e2e examples/target/.../keccak_syscall --platform=ceno --profiling 1
thread 'main' panicked at ceno_zkvm/src/e2e.rs:1279:33:
emulator trapped before halt: Trap IllegalInstruction(0xc0001073)
```

`0xc0001073` is a CSR instruction — the Keccak precompile's syscall entry. The emulator does
not accept it in the default CPU build at this commit, so the program traps before halting and
no proof is produced. Measured at 10 and at 2 threads, identically. Raw output in
[`bench/data/repro-ceno/`](../../data/repro-ceno/).

**What we did NOT establish:** why. We did not find a feature flag in `ceno_emul` that enables
this syscall, and we did not search version space for one. The cause is **NOT DETERMINED**, and
we are not asserting that the precompile is unavailable on CPU — only that it did not run for
us on the default build, and that we could not make it run within this campaign's budget. Our
one-word `COMMIT` patch is not implicated: it removes an x86-only assembly feature from a BN256
dependency reached solely by `ceno_emul/src/syscalls/secp256k1.rs`, a different syscall.

**The consequence for this file is the point.** Ceno's single published performance figure is a
GPU measurement of an example that, on this machine's default CPU build, does not execute. So
there is no configuration in which we could have reproduced it here — not merely the wrong
hardware class, but the wrong hardware class for a workload we could not run at all.

**No ratio between any figure of ours and their 0.686 s appears anywhere in this repository.**

## 4. A discrepancy inside their own sources, reported

The PR description merged as `def271c2840b` publishes a second, richer set of figures for a
mainnet Ethereum block (`23817600`) — `E2E elapsed 143.000 → 78.700 s`, `app_prove
66.400 → 57.400 s`, and a per-operation breakdown. Those come from a **different repository**,
`scroll-tech/ceno-reth-benchmark`, and the workflow in this repository
(`regression-reth-benchmark.yml`) only dispatches to it.

We did not attempt them: they require an Ethereum RPC endpoint, a cached block, a CUDA runner
and the private GPU backend. They are noted because a reader comparing our numbers to "Ceno's
published performance" may find those first, and they describe a different workload on
different hardware in a different repository from the one measured here.

## 5. Right of reply

The questions whose answers would change this file
([`CHALLENGE.md`](../../CHALLENGE.md)):

- **Is there a published CPU figure we missed?** We found none at any commit, in the paper,
  the book, the blog, the CI, or the two cross-zkVM aggregators.
- **Is the `gh-pages` erasure intended?** If the benchmark history is meant to persist,
  `deploy-book.yml` and `gpu-integration.yml` currently contend for the same branch, and only
  git history preserved the series we quote.
- **What is the intended aarch64 build configuration?** [`COMMIT`](COMMIT) documents the
  one-word patch we applied to make the tree build at all; if you would do it differently, we
  will re-run.
- **Is the 1-thread panic known?** `RAYON_NUM_THREADS=1` aborts the prover on your own
  examples ([`RESULTS.md`](RESULTS.md) §4). If a supported minimum thread count exists, we will
  say so and re-cut the table.
- **Is the `#[serde(skip)]` on `circuit_index_to_name` intended?** It makes every serialized vk
  unusable ([`BUILD.md`](BUILD.md) §5), so `cargo ceno verify` cannot currently succeed.

If any of this is wrong, or if a configuration we did not use is the one you would have chosen,
open an issue with it and we will re-run all of it and publish both outcomes, with credit and
date. **The old numbers stay in the record next to the new ones; we do not quietly replace
them.**
