# DeepProve — the system's own reference numbers, and whether we reproduced them

`bench/README.md` §"Fairness protocol" and `bench/CHALLENGE.md` commit this repository to
reproducing each measured system's **own published reference number before reporting anything
about that system**, and to publishing the discrepancy *above* the result if we cannot.

**We could not. This file is that discrepancy, and it comes before every DeepProve figure in
[`RESULTS.md`](RESULTS.md).**

Two separate things went wrong and they should not be blurred together:

1. **The paper's headline configuration is not in the public code at this commit.** Half of
   Table 1 is BaseFold, and BaseFold has been removed from the tree. §1.
2. **The configuration that *is* in the code does not produce a verifying GPT-2 proof on this
   machine, in either build we tried.** §2.

And one thing the authors should want to know regardless: **their README and their paper
disagree by a factor of ~1.9 on the same declared hardware.** §3.

---

## 1. The reference: ePrint 2026/1112, Table 1 — and which half of it the code can reach

DeepProve's paper is *"DeepProve: Verifiable End-to-End Large Language Model Inference"*,
ePrint **2026/1112**, CCS 2026 (received 2026-05-30). Its evaluation runs on a primary machine
of **AMD EPYC 9254, 24 cores / 48 threads, 2.9 GHz, 504 GB RAM**, at **12-bit quantization**.

Table 1 reports GPT-2 under **two** polynomial commitment schemes, as blocks within one table:

| PCS | Seq | Context gen (min) | **RAM (GiB)** | **Prove (min)** | **Verify (s)** | **Proof (MiB)** | TPM |
|---|---:|---:|---:|---:|---:|---:|---:|
| BaseFold | 64 | <1.38 | 18.47 | 0.8 | 1.72 | 20.68 | 80.5 |
| BaseFold | 512 | <1.38 | 41.98 | 4.44 | 2.12 | 24.33 | 115.26 |
| HyperKZG | 64 | <0.86 | **24.41** | **1.09** | **1.81** | **8.98** | 58.68 |
| HyperKZG | 512 | <0.86 | 78.7 | 4.02 | 3.22 | 11.8 | 127.23 |

Source: https://eprint.iacr.org/2026/1112.pdf, Table 1, p.10.

### 1.1 BaseFold is not in the code we are allowed to measure

**At commit `9d1a53e2` there is no BaseFold implementation in the tree.** We searched every
`.rs`, `.toml`, `.md` and `.py` file. BaseFold survives only as prose:

- `zkml/docs/src/commitments.md:5` — *"We make use of the Basefold … polynomial commitment
  scheme"*
- `docs/src/commitments.md:25` — *"At the moment, deep-prove is using Basefold as the
  underlying PCS."*
- `CHANGELOG.md:639` — a `### BaseFold` section

**Every binary at this commit uses HyperKZG over BN254**, with a Blake3 transcript:
`zkml/src/bin/bench/cnn.rs:26`, `zkml/src/bin/bench/llm.rs:37`,
`deep-prove/src/middleware/v2.rs:11` all read `type Pcs = HyperKZG<Bn254>`. The switch is
commit `100ee5b5`, *"feat: hyper-kzg backend + horizontal chunking (#346)"*. `zkml/README.md`
says so plainly: *"The polynomial commitment scheme used in the public binaries is
**HyperKZG**."*

**So the two BaseFold rows of Table 1 are not reproducible from the public tree by anyone,
including us, and that is a fact about the repository rather than about our machine.** The
documentation under `docs/` still describing BaseFold as the current PCS is stale.

**The reproduction target is therefore the HyperKZG seq-64 row** — bolded above. Not the
BaseFold seq-64 row, even though it is the smaller-memory one: reproducing a number the code
cannot produce is not a check on anything.

### 1.2 Why seq 64 and not seq 512

This machine is an Apple M1 Max with **32 GiB**. The paper's HyperKZG seq-512 row needs
**78.7 GiB** and its seq-64 row **24.41 GiB**. Seq 512 was never attempted: it is 2.5× the
machine's RAM and would have measured swap. **Seq 64 is the largest point that could fit**, and
it is the one reported here, whatever the outcome — as `bench/README.md` requires.

## 2. Two attempts, two failures

### 2.1 Attempt 1 — the build DeepProve's own README documents

`zkml/README.md` documents `cargo build --release -p zkml --bin bench-llm` and then
`bench-llm --model gpt2 --hf openai-community/gpt2 --sequence 64`. That is exactly what was
run, with the GPT-2 SafeTensors weights that ship in the repository's own Git LFS cache.

**It panics during context generation, after 11.57 s:**

```
thread 'main' panicked at zkml/src/iop/context.rs:329:17:
Found different MLE for polynomial wte.weight
```

| | |
|---|---|
| wall time to panic | **11.57 s** (`user` 10.41 s, `sys` 1.28 s) |
| peak RSS | 12.75 GB |
| peak memory footprint | 12.74 GB |
| sleep check | `slept = 0.000 s`, verdict OK |

`wte.weight` is GPT-2's token embedding matrix, which GPT-2 ties to its output head — so it is
registered for commitment twice, and the two registrations produce different multilinear
extensions. The check that catches it is a **`debug_assert!`**, and DeepProve's release profile
switches debug assertions **on**:

```toml
[profile.release]
debug = 1
debug-assertions = true
# LTO, even thin, is **very slow** to compile for marginal gains at best
lto = "off"
```
`Cargo.toml`, workspace root

So the assertion firing is not us building it wrong — **it is the authors' own documented
release configuration, catching an inconsistency in the authors' own flagship benchmark.**

Raw output: [`bench/data/repro-deepprove/gpt2-seq64/`](../../data/repro-deepprove/gpt2-seq64/).

### 2.2 Attempt 2 — the same tree with debug assertions off

Nobody benchmarks with debug assertions on, so the published numbers were presumably not
produced that way. The tree was rebuilt with a **cargo environment override and no source
edit whatsoever**:

```
CARGO_TARGET_DIR=…/target-noassert CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=false \
  cargo build --release -p zkml --bin bench-llm
```

**It gets past context generation, proves, and then its own verifier rejects the proof:**

```
thread 'main' panicked at dp-crypto/src/sumcheck/verifier.rs:151:17:
0th round's prover message is not consistent with the claim.
19387534115087409086436529446446428128650174643557990835957281596030098830415
15622868743010094751920608776837910925421439361640645568627706393121260586774
```

| | |
|---|---|
| wall time to rejection | **112.20 s** (`user` 373.93 s, `sys` 78.15 s, `(u+s)/real` = 4.03) |
| peak RSS | 15.91 GB |
| peak memory footprint | **24.26 GB** |
| sleep check | `slept = 0.000 s`, verdict OK |

**This is the stronger of the two results.** Turning off the debug assertion did not make the
problem go away; it moved the failure from a cheap check to an expensive one. The two
different MLEs for `wte.weight` that attempt 1 flagged are exactly the kind of prover/verifier
disagreement that produces "prover message is not consistent with the claim" at sumcheck round
zero. **The assertion was right.**

Raw output:
[`bench/data/repro-deepprove/gpt2-seq64-noassert/`](../../data/repro-deepprove/gpt2-seq64-noassert/).

### 2.3 The one figure that did land in the paper's regime, and why it is still not a reproduction

The peak memory the failing run reached is close to the paper's:

| | paper, HyperKZG seq 64 | this machine |
|---|---|---|
| RAM / peak footprint | 24.41 GiB | **24.26 GB = 22.59 GiB** (−7.4%) |
| peak RSS | not reported | 15.91 GB |
| prove | 1.09 min, after <0.86 min context generation | — |
| whole process, wall | (not directly comparable) | 112.20 s |

**We do not claim this as a reproduction, and no `RESULTS.md` figure rests on it.** A run that
produces an artifact its own verifier rejects has not done the work that the paper's row
describes — a memory figure for producing an invalid proof is not a memory figure for
producing a proof. It is reported because it is a measurement and this repository publishes
measurements, and because it makes the failure more interesting rather than less: the run was
in the right regime and still did not produce a valid proof.

### 2.4 The failure is in `bench-llm`, not in DeepProve's GPT-2 proving

This is the part that took the longest and it changes what the result means, so it is stated
before the verdict rather than after it.

**DeepProve proves GPT-2 correctly on this machine. Its own tests say so.** Run against the
same release build, with `RUST_MIN_STACK=536870912` (see [`BUILD.md`](BUILD.md) §2 for why
that is needed), every GPT-2 test passes:

```
test result: ok. 10 passed; 0 failed; 4 ignored; 233 filtered out; finished in 306.55s
```

including the three that actually prove:

| Test | Result |
|---|---|
| `model::llm::test::test_llm_driver_distributed_prove_gpt2` | **ok** |
| `quantization::llm_quant::tests::prove_gpt2` | **ok** |
| `model::transform::impls::gpt2rmsnorm::tests::test_gpt2_replace_proving` | **ok** |

and the **sequential** proving test, which their suite marks `#[ignore]` — *"Sequential case
covered already by distributed prove test, use this test only when checking performance to
ensure sequential proving is used"* — also passes when run explicitly:

```
cargo test --release -p zkml test_llm_driver_prove_gpt2 -- --ignored
test result: ok. 1 passed; 0 failed; finished in 82.81s
```

Raw: [`testsuite-gpt2-bigstack.txt`](../../data/repro-deepprove/testsuite-gpt2-bigstack.txt),
[`testsuite-gpt2-sequential-ignored.txt`](../../data/repro-deepprove/testsuite-gpt2-sequential-ignored.txt).

**So the defect is in `bench-llm` — the binary whose output is the published table — and not
in the prover it calls.** That is a narrower and much more useful statement than "GPT-2 does
not work here", and it is the one the evidence supports.

### 2.5 Everything that was tried, so the verdict is not read as one bad invocation

| # | Configuration | Outcome |
|---|---|---|
| 1 | `bench-llm --sequence 64`, documented build | panic, `Found different MLE for polynomial wte.weight`, 11.57 s |
| 2 | `bench-llm --sequence 64`, `debug-assertions = false` | proof produced, **its own verifier rejects it**, 112.20 s |
| 3 | `bench-llm --sequence 64`, documented build, `RUST_MIN_STACK=512 MiB` | same panic as #1, 12.26 s |
| 4 | `bench-llm --max-context 64 --min-user-len 1`, documented build, big stack | same panic as #1, 11.34 s |
| 5 | the authors' own GPT-2 tests, same build, big stack | **all pass** (§2.4) |

Four `bench-llm` configurations, two build profiles, both of its documented length modes.
**The `wte.weight` panic is reproducible in 11–12 s every time**, and raising the stack — which
is what the tests needed — does not touch it.

Other things held to the authors' own choices, so that nothing of ours is in the failing
configuration:

- **A pristine tree**, `git status` clean at `9d1a53e2`, with the repository's own pinned
  toolchain (`nightly-2026-01-27`).
- **The authors' own weights**, from the Git LFS cache committed to their repository
  (`model_cache/openai-community/gpt2/`), not a separately downloaded copy.
- **The authors' own command line**, from `zkml/README.md` §"Quickstart", with default
  quantization (12-bit) and default thread count.
- **The sleep guard passed** on every run (`slept = 0.000 s`), so no wall-clock figure
  includes suspended time.

**What we did NOT rule out**, stated so it is not discovered later: this machine is
**aarch64 / macOS**, and DeepProve's CI runs on x86-64 Linux. Nothing here establishes that
the `bench-llm` failure is or is not platform-specific, and **the cause is NOT DETERMINED**.
What the tests in §2.4 do establish is that it is not a general inability to prove GPT-2 here.

## 3. The authors' README and the authors' paper disagree, on the same declared hardware

Registered here because the task of reproducing a published number requires knowing which
published number, and because it is exactly what the right of reply exists to settle.

Both of DeepProve's READMEs at this commit publish a GPT-2 table, and both declare the **same
machine as the paper** — `zkml/README.md`: *"Reference numbers on a 24-core / 504 GB CPU
machine (AMD EPYC 9254, 2.9 GHz), HyperKZG PCS, default quantization"*.

| GPT-2, seq 512 | README (`zkml/README.md`) | Paper Table 1, HyperKZG | Paper Table 1, BaseFold |
|---|---|---|---|
| Prove | **7.64 min** | 4.02 min | 4.44 min |
| Verify | **1.33 s** | 3.22 s | 2.12 s |
| Proof size | **10.71 MiB** | 11.8 MiB | 24.33 MiB |
| Throughput | 1.12 tok/s = **67 TPM** | 127.23 TPM | 115.26 TPM |
| RAM | **not reported** | 78.7 GiB | 41.98 GiB |

| GPT-2, seq 64 | README | Paper, HyperKZG | Paper, BaseFold |
|---|---|---|---|
| Prove | **2.35 min** | 1.09 min | 0.8 min |
| Verify | **1.25 s** | 1.81 s | 1.72 s |
| Proof size | **7.95 MiB** | 8.98 MiB | 20.68 MiB |

Three observations, and no explanation offered:

1. **Each source is internally consistent.** The README's 7.64 min for 512 tokens *is* 67 TPM;
   the paper's 4.02 min for 512 tokens *is* 127 TPM. Neither table contradicts itself.
2. **The README's proof sizes match the HyperKZG block, not BaseFold** (10.71 vs 11.8 MiB;
   7.95 vs 8.98 MiB), consistent with the README describing the shipped HyperKZG code.
3. **But its prove times are ~1.9–2.2× the paper's, on hardware it declares to be the same.**
   The README also reports verify times *below* the paper's while reporting prove times far
   above them, so a single machine-speed factor does not explain the pattern, and we do not
   offer one.

**The paper governs, by this project's source hierarchy** (ePrint/arXiv > team blog/README).
Every reference figure quoted in this file is the paper's. The README figures are recorded so
that a reader who finds them first knows they disagree, and so that the authors can tell us
which is current.

## 4. Verdict

**NOT REPRODUCED — but the failure is narrower than that phrase suggests.**

- The BaseFold rows of Table 1 **cannot** be reproduced from the public tree at `9d1a53e2`,
  because BaseFold is not in it. That is a property of the repository, not of our machine.
- The HyperKZG seq-64 row was attempted in four `bench-llm` configurations across two build
  profiles. The documented build panics in context generation on a tied-weight inconsistency
  (`wte.weight`); with debug assertions off it produces a proof that **DeepProve's own
  verifier rejects**.
- **But DeepProve's own GPT-2 proving tests all pass on this machine** (§2.4). So this is not
  "GPT-2 does not work here". It is: **the benchmark binary that produces DeepProve's
  published GPT-2 table does not produce a verifying proof on this machine, while the prover
  underneath it does.**
- Consequently **no DeepProve GPT-2 figure of ours exists**, and none is used anywhere.

**What this does and does not do to the task measurements in [`RESULTS.md`](RESULTS.md).**

It does **not** invalidate them, and §2.4 is why. The fairness protocol's purpose is to
establish that our build of someone else's system behaves the way its authors' build does.
We could not check that against the published *number*, but we did check it against the
authors' own *tests*, and it passes: 204/204 non-LLM tests and 11/11 GPT-2-related tests, of
which **4 invoke the prover**
([`BUILD.md`](BUILD.md) §2). The T1 figures rest on that, on the fact that T1-0's and T1-a's
proofs verify, and on the correctness control in `RESULTS.md`.

What remains unreproduced is a **published number**, and the honest reading is that the gap
sits in `bench-llm` rather than in the prover the tasks exercise. That is still a real hole in
the fairness protocol and it is why this file comes before the results.

## 5. Right of reply

`bench/CHALLENGE.md` applies, and we would rather be corrected here than anywhere else in the
repository. Specifically, if the DeepProve authors can tell us:

- a commit, branch or build configuration at which GPT-2 proves **and verifies**;
- whether the `wte.weight` MLE inconsistency is known, platform-specific, or already fixed;
- which of the README table and the paper table is current for the shipped code;
- whether BaseFold is expected to return to the public tree, since the documentation still
  describes it as the PCS in use —

we will re-run all of it and publish both outcomes, with credit and date. The old numbers stay
in the record next to the new ones; we do not quietly replace them.
