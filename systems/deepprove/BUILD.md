# DeepProve — build configuration and build-integrity check

Build integrity is verified **before** any measurement, and the check is blocking. It exists
because of a specific, documented failure of our own: in experiment E-001 a harness compiled
without LTO measured our prover **9.0× slower** and **inverted the experiment's conclusion**,
and nothing in the timing output revealed it.

For binius64 the check is a field-multiply probe, because binius64 publishes no reference
timings. **DeepProve publishes something better: a test suite its own CI gates on.** That is
the check used here.

---

## 1. Build configuration — the authors' own, unmodified

`zkml/README.md` §Installation documents exactly one CPU build:

```bash
cargo build --release -p zkml --bin bench-llm
```

That is what was used. The whole workspace's binaries were built the same way
(`cargo build --release --bins`) so that `deep-prove-worker` and `deep-prove-cli` — needed for
the tasks and for the correctness control — come from the same configuration.

| Field | Value | Whose choice |
|---|---|---|
| Toolchain | `rustc 1.95.0-nightly (474276961 2026-01-26)` | **theirs** — `rust-toolchain.toml`, `channel = "nightly-2026-01-27"` |
| `[profile.release]` | `debug = 1`, **`debug-assertions = true`**, **`lto = "off"`** | **theirs** — workspace `Cargo.toml` |
| `RUSTFLAGS` | `--cfg tokio_unstable` | **theirs** — `.cargo/config.toml` |
| Features | default = `["cpu"]` (`burn/ndarray`) | **theirs** — no `--features` passed |
| PCS | HyperKZG over BN254, Blake3 transcript | **theirs** — hardcoded, see `REPRODUCTION.md` §1.1 |
| `ZKML_BIT_LEN` | **8** for the primary cells, 12 (their default) for a control | ours, declared per cell |
| Threads | `RAYON_NUM_THREADS` ∈ {1, 10} | ours, declared per cell — **but see §5** |

### Two things about their release profile, declared rather than corrected

```toml
[profile.release]
debug = 1
debug-assertions = true
# LTO, even thin, is **very slow** to compile for marginal gains at best
lto = "off"
```

**`lto = "off"`.** This is the exact class of setting that cost E-001 a 9.0× error, and it
would have been easy to "fix". We did not. `bench/README.md`'s fairness protocol says every
system runs in the best configuration **documented by its own authors**, and the authors
document this one, with a comment explaining why. A `fast` profile with `lto = "fat"` exists
in the same file and is not what any documented command selects. **If the DeepProve authors
consider `--profile fast` the right configuration for benchmarking, we will re-run everything
with it and publish both**, per `CHALLENGE.md`.

**`debug-assertions = true`.** Also unusual for a release profile, and it is not cosmetic: it
is what makes GPT-2 abort in `REPRODUCTION.md` §2.1. A second build with
`CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=false` — an environment override, **no source edit** —
exists solely for that reproduction attempt. **No task figure in `RESULTS.md` comes from it.**

**`-C target-cpu=native` was NOT used.** binius64's build does use it, because binius64's own
documentation specifies it. DeepProve's does not, so it was not applied. That asymmetry
favours neither system by our choice; each is built the way its authors say. **It is declared
because it is a real difference between the two build configurations in this repository**, and
a reader comparing the two numbers should know it. Quantifying what it would be worth to
DeepProve is a measurement we did not make.

---

## 2. The build-integrity check: DeepProve's own test suite

DeepProve's CI (`.github/workflows/tests.yml`) gates on `cargo nextest run`. `cargo test` is
the same tests through a different runner; the substitution is declared. Tests were run
against the **release** profile, i.e. the build actually measured, with `RNG_SEED=1234` as CI
sets.

### Result: 204 of 204 runnable non-LLM tests pass

```
test result: FAILED. 203 passed; 1 failed; 5 ignored; 0 measured; 38 filtered out; in 322.08s
```

and the single failure was **ours, not theirs** — disclosed rather than quietly re-run:

```
Header mismatch in cnn_prover.csv: found [... "witness_commitment"],
                                 expected [...]
```

`model::test::test_single_cnn_prover` appends to a `cnn_prover.csv` in the working directory,
and an earlier run of ours had left one behind with a different column set. Deleting the stale
file and re-running that test alone gives `test result: ok. 1 passed; 0 failed`. **So the
honest score is 204/204**, and the contamination was our own bookkeeping, not a DeepProve
defect.

Raw output:
[`bench/data/repro-deepprove/testsuite-nonllm-bigstack.txt`](../../data/repro-deepprove/testsuite-nonllm-bigstack.txt).

**This is the check that matters for the figures in `RESULTS.md`**, because the tasks measured
here are dense/matmul/requant graphs and those paths are covered by these tests. The build is
fit to produce numbers on the paths we measured.

### But the suite does not complete on this machine at default settings

Run without touching the stack size, `cargo test --release -p zkml` **aborts the whole test
binary**:

```
thread 'model::llm::test::test_llm_driver_distributed_prove_gpt2' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

Raw:
[`testsuite-full-defaultstack.txt`](../../data/repro-deepprove/testsuite-full-defaultstack.txt).

And with the LLM tests filtered out it still aborts, on another GPT-2 test:

```
thread 'model::transform::impls::gpt2rmsnorm::tests::test_gpt2_replace_proving' has overflowed its stack
```

Raw: [`testsuite-nonllm.txt`](../../data/repro-deepprove/testsuite-nonllm.txt).

Setting `RUST_MIN_STACK=536870912` clears it and produces the 204/204 above. **The GPT-2 code
path needs more stack than a default macOS thread provides.** Whether DeepProve's CI sets
`RUST_MIN_STACK`, or whether Linux's larger default simply absorbs it, is **NOT DETERMINED**;
their CI workflow does not set it. This is recorded because it is a third, independent way in
which GPT-2 does not work on this machine at this commit — see `REPRODUCTION.md`.

### Two tests the authors ignore, quoted because they bear on our expression

```
test parser::onnx::tests::test_parser_onnx_gpt2 ... ignored,
    this test shows no gpt2 onnx out there are working with tract_onnx
test parser::onnx::tests::test_covid_cnn ... ignored
```

The first is the authors' own statement that the **ONNX** path does not carry GPT-2. It does
not affect T1/T2/T3, which are not transformer graphs, but it is the sort of thing a reader
should see from us rather than discover.

---

## 3. Why the tasks were NOT measured with DeepProve's `bench` binary

`zkml/bench.py` drives a binary called `bench` (`zkml/src/bin/bench/cnn.rs`), and that is the
obvious instrument for T1/T2/T3: it separates proving from verification, times each, and
records proof size and memory into a CSV. **It does not run on this machine at this commit.**

Every ONNX model fails identically, in ~2 s, before any proving:

```
Error: error running bench:
Caused by:
    Tensor is unavailable for a wrapped tensor handler
```

Raw output for T1-0 and T2:
[`bench/data/bench-binary-deepprove/`](../../data/bench-binary-deepprove/). Those logs are
also where `EXPRESSION.md` §5 reads the inserted requantization layers, because this binary
prints the quantized model before it fails.

The path is not conditional and there is no flag that avoids it: `read_model` sets
`.with_keep_float(true)` in **both** of its strategy branches
(`zkml/src/bin/bench/cnn.rs:247,253`), which makes `md.float_model` `Some`, which makes the
binary run the float reference model (`:287-288`) before it proves anything. That call returns
handles of the `TensorHandle::WrappedTensor` variant, whose `.tensor()` accessor is a hard
`bail!` (`zkml/src/tensor/handle.rs:100-101`), and `run_float_model` calls exactly that
(`:222`).

**Cause not established.** `WrappedTensor` is a burn-backed accelerated tensor, and `zkml`'s
manifest gives `burn` the `metal` feature on macOS and `vulkan` elsewhere
(`zkml/Cargo.toml`, the two `[target.'cfg(...)'.dependencies]` blocks) — so a
platform-dependent backend is a candidate. **We did not verify it**, we have no Linux machine
in this campaign to compare against, and we are not going to guess. It is reported so a third
party sees it rather than discovers it.

**The instrument actually used** is `deep-prove-worker one-shot`, DeepProve's own local
proving binary, which does not run the float model. What that costs in measurement precision
— principally that our `prove` bracket includes DeepProve's quantized inference — is stated in
`RESULTS.md` and in `NOT_EXPRESSIBLE.md` §6.

**One further defect of that binary, since every cell hits it.** `one-shot` proves correctly
and then fails writing the proof to disk:

```
Error: writing proofs to file
Caused by:
    key must be a string
```

It serializes with `serde_json::to_writer` (`deep-prove/src/bin/worker/immediate.rs:123`) and
the ONNX proof contains a map with non-string keys. The failure is **after** `Proving done.`,
so timings and memory peaks are unaffected — but the process exits non-zero on a run that
proved correctly. The cell ledger records that as `PROVED_NOWRITE` rather than hiding it
behind `OK` or discarding a good measurement as a failure.

---

## 4. Sleep detection, per cell

Identical to binius64's, and it exists for the same reason: **the machine idle-slept in the
middle of a timed run once** (`bench/systems/binius64/BUILD.md` §3). A wall-clock duration
that includes sleep is time the CPU was not running; it inflates `real`, deflates
`(user+sys)/real`, deflates every derived rate, and leaves no trace in the output.

**1 · Prevention.** Every measured command and every long build runs under
`caffeinate -dimsu`.

**2 · Detection.** Every cell is bracketed by `bench/scripts/clockprobe.py`. On macOS
`time.monotonic()` does not advance during sleep while `time.time()` does, so
`wall − monotonic` is the time spent asleep. A gap above 2 s marks the cell `INVALID_SLEEP`
and it is rerun.

**Every DeepProve cell and both GPT-2 attempts recorded `slept = 0.000 s`.** The
`mono_s`, `wall_s`, `slept_s` and `sleep_verdict` columns of
[`bench/data/cells-deepprove.csv`](../../data/cells-deepprove.csv) carry it per cell,
including the cells that passed.

---

## 5. Thread control is NOT fully in our hands, and the cells show it

`RAYON_NUM_THREADS` is what the cell varies, and it is **not** sufficient to make DeepProve
single-threaded. The `t1-0` cell at `RAYON_NUM_THREADS=1` measured
**`(user+sys)/real` = 1.83** — nearly two cores busy in a cell labelled one thread. DeepProve
runs a Tokio multi-threaded runtime and a burn backend alongside rayon.

Second, DeepProve's sumcheck **rounds the thread count down to a power of two** and says so:

```
thread size 10 is not power of 2, using 8 threads instead.
dp-crypto/src/sumcheck/util.rs:75
```

So the 10-thread cells use **8** threads in the sumcheck. Both facts are why `RESULTS.md`
labels those columns `RAYON_NUM_THREADS` rather than "threads", prints the measured
`(user+sys)/real` beside every figure, and **does not** put DeepProve's 1-thread column next
to binius64's 1-thread column as if they meant the same thing.

`deep-prove-worker` exposes no thread flag. `bench-llm` has `--num-threads`; the worker does
not, and the worker is the instrument.

---

## 6. Machine state, declared

The machine is **not dedicated**. It is the same workstation E-001, E-005 and the binius64
half of this benchmark ran on, and using a different one would invalidate the comparison.

| Field | Value |
|---|---|
| Hardware | Apple M1 Max (`MacBookPro18,2`), 10 physical / 10 logical cores, 32 GiB |
| OS / kernel | macOS 26.5.2 (25F84) · Darwin 25.5.0 |
| Uptime at campaign start | 12 days |
| Swap committed at campaign start | **7.88 GB of 9.22 GB**, rising to 9.41 GB during the campaign |
| Free space on `/` | 104 GiB |
| Load average during the grid | 1.9–18.7 (1 min), recorded per cell in `cells-deepprove.csv` |
| Other load | Firefox, Claude, WindowServer and the usual system daemons, running throughout |
| Power | AC. Idle sleep suppressed by `caffeinate` for every measured command |
| `powermetrics` | **not recorded** — requires sudo. Substitutes: the test suite above, plus per-cell `loadavg` and swap in `cells-deepprove.csv` |

**The DeepProve clone lives outside the repository**, in a scratch directory, per the licence
constraint in `COMMIT`. Its two build trees total ~15 GB and its Git LFS model cache 4.7 GB;
none of it is in this repository.
