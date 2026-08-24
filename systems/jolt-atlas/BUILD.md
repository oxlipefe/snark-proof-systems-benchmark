# jolt-atlas — build configuration and build-integrity check

Build integrity is verified **before** any measurement, and the check is blocking. It exists
because of a specific, documented failure of our own: in experiment E-001 a harness compiled
without LTO measured our prover **9.0× slower** and **inverted the experiment's conclusion**,
and nothing in the timing output revealed it.

---

## 1. Build configuration — the authors' own, unmodified

jolt-atlas's `README.md` documents exactly one way to run anything:

```bash
cargo run --release --package jolt-atlas-core --example <name>
```

so `--release` with the workspace's own `[profile.release]` is the configuration, and it is
what was used. The whole `jolt-atlas-core` example set was built with
`cargo build --release --package jolt-atlas-core --examples`.

| Field | Value | Whose choice |
|---|---|---|
| Toolchain | `rustc 1.88.0 (aarch64-apple-darwin)` | **theirs** — `rust-toolchain.toml`, `channel = "1.88"` |
| `[profile.release]` | `debug = 1`, **`overflow-checks = true`**, **`lto = "fat"`** | **theirs** — root `Cargo.toml` |
| `RUSTFLAGS` | none | **theirs** — no `.cargo/config.toml` in the tree |
| Features | default (no `--features`); the `zk` feature is **off** | **theirs** — the README's commands pass none |
| PCS | HyperKZG over BN254, Blake2b transcript | **theirs** — see §4 |
| Threads | `RAYON_NUM_THREADS` ∈ {1, 4, 10} | ours, declared per cell — **but see §5** |

### Their release profile, declared rather than corrected

```toml
[profile.release]
debug = 1
overflow-checks = true
lto = "fat"
```

Two remarks, and neither is a criticism:

**`lto = "fat"`** is the setting E-001 taught us to check, and jolt-atlas already has it on.
Unlike DeepProve — whose authors document `lto = "off"` and whose profile we therefore left
alone — there is nothing here to be tempted to "fix".

**`overflow-checks = true`** in a release profile is unusual and it is not free: it puts a
check on every arithmetic operation in the prover. A `build-fast` profile
(`inherits = "release"`, `lto = "off"`) exists in the same file and is **not** what any
documented command selects, and it does not turn overflow checks off either. **We did not
measure with overflow checks off**, because no documented configuration does. If the
jolt-atlas authors consider a different profile the right one for benchmarking, we will
re-run everything with it and publish both, per [`CHALLENGE.md`](../../CHALLENGE.md).

**`-C target-cpu=native` was NOT used**, because jolt-atlas's documentation does not specify
it. binius64's does, and binius64 was built with it. That asymmetry is declared here because
it is a real difference between two build configurations in this repository. What it would be
worth to jolt-atlas is a measurement we did not make.

---

## 2. The build-integrity check

jolt-atlas publishes reference timings, which is a stronger check than a probe, and §3 of
[`REPRODUCTION.md`](REPRODUCTION.md) is that check. It gave a result we did not expect and it
is the most important thing in this directory, so it is not repeated here.

Two further checks were run, and both are blocking:

**1 · Every measured cell verifies its own proof.** The harness proves and then calls
jolt-atlas's own `ONNXProof::verify` on every repetition, warmup included, and exits non-zero
if any of them fails. **No figure in [`RESULTS.md`](RESULTS.md) comes from a run whose proof
did not verify.** That is a stronger per-cell guarantee than binius64's or DeepProve's cells
carried, and it is available because jolt-atlas exposes verification through the same API as
proving.

**2 · The correctness control.** [`RESULTS.md`](RESULTS.md) §"Correctness control". It is
reported there rather than here because it found something.

---

## 3. The harness links the same dependency graph as the tree under test

The harness is a separate Cargo workspace ([`COMMIT`](COMMIT) §2), which means Cargo resolves
its dependency versions independently. **Left alone it does not resolve to the same graph**,
and the difference is not cosmetic:

```
error: rustc 1.88.0 is not supported by the following packages:
  tract-onnx@0.23.6-pre requires rustc 1.91
  enum-ordinalize@4.4.2 requires rustc 1.89
  kstring@2.0.4 requires rustc 1.96.0
```

A fresh workspace picks up **tract 0.23.6-pre** where jolt-atlas's own `Cargo.lock` pins
**0.22.1-pre at `c484b3ee`** — a different ONNX frontend, which would mean the harness was not
measuring the code under test. Four packages were pinned back to the versions in jolt-atlas's
own lock file:

```
cargo update tract-onnx        --precise c484b3ee9a22e7d2bfca8394619771397b61c0d6
cargo update enum-ordinalize   --precise 4.3.2
cargo update enum-ordinalize-derive --precise 4.3.2
cargo update kstring           --precise 2.0.2
```

The revisions were **read** from `Cargo.lock`; no part of that file is reproduced here, and a
git revision is a fact rather than code. `bench/scripts/jolt-atlas/harness/setup.sh` prints
these four commands, so the pin is reproducible rather than folklore.

**The harness's `[profile.release]` is `debug = 1`, `overflow-checks = true`, `lto = "fat"` —
copied field for field from jolt-atlas's own**, for the reason E-001 exists.

---

## 4. What the field, PCS and transcript are — and what could not be established

Read from the type aliases the public API exposes and corroborated by every cell running
through them:

| | |
|---|---|
| field | BN254 scalar field (`Fr`), via a16z's `arkworks-algebra` fork, branch `dev/twist-shout` |
| PCS | `HyperKZG<Bn254>` |
| transcript | `Blake2bTranscript` |
| trusted setup | **YES** — HyperKZG is a pairing-based KZG variant and needs a structured reference string. `setup_prover` runs in process, per run; it is inside our `setup` column |
| security bits | **NOT DETERMINED** |

**Security bits could not be established, and no number is invented.** No parameter named for
security bits, soundness bits or a query count is exposed on this path or stated in the
documentation. binius64 holds `SECURITY_BITS = 96` across its rate sweep and publishes it;
DeepProve exposes nothing comparable either. So of the three systems in this benchmark, one
publishes its security parameter and two do not.

**A second PCS exists in the tree.** `joltworks/src/poly/commitment/dory/` implements Dory, and
`REQUIRES_MATERIALIZED_POLYS = true` for **both** it and HyperKZG
(`dory/mod.rs:193`, `hyperkzg/commitment_scheme.rs:33`). **Every figure in `RESULTS.md` is
HyperKZG**, because that is what the public `AtlasProverPreprocessing` path and every example
in the README use. Dory was not measured, and that is a declared gap rather than a judgement.

---

## 5. Thread control is NOT fully in our hands, and the cells show it

`RAYON_NUM_THREADS` is what the cell varies, and it is **not** sufficient to make jolt-atlas
single-threaded. Every `RAYON_NUM_THREADS=1` cell in
[`cells-jolt-atlas.csv`](../../data/cells-jolt-atlas.csv) measured `(user+sys)/real` between
**1.93 and 2.15** — about two cores busy in a cell labelled one thread.

The reason is documented by jolt-atlas's own authors, in a comment in
`jolt-atlas-core/examples/gpt2_zk_bench.rs`: the patched arkworks MSM *"builds a fresh nested
2-thread `rayon::ThreadPoolBuilder` per MSM chunk per call, spawning `current_num_threads / 2`
chunks each time"*, and they note that with default rayon this hits macOS's per-process
pthread limit. So a second, nested pool exists below the one `RAYON_NUM_THREADS` controls.

`RESULTS.md` therefore labels those columns `RAYON_NUM_THREADS` rather than "threads", prints
the measured `(user+sys)/real` beside every figure, and **does not** put jolt-atlas's 1-thread
column next to binius64's 1-thread column as if they meant the same thing.

**This is measurable in a second way, and §6 of `RESULTS.md` reports it: involuntary context
switches.** A nanoGPT run at `RAYON_NUM_THREADS=10` recorded **2 722 527** of them in 13.85 s
of wall clock, against **439 573** at 1 thread. That is the thread-creation storm the comment
predicts, and it is why jolt-atlas gets *slower* above 4 threads on this machine.

---

## 6. Sleep detection, per cell

Identical to binius64's and DeepProve's, and it exists for the same reason: **the machine
idle-slept in the middle of a timed run once** (`bench/systems/binius64/BUILD.md` §3).

**1 · Prevention.** Every measured command and every long build runs under `caffeinate -dimsu`.

**2 · Detection.** Every cell is bracketed by `bench/scripts/clockprobe.py`. On macOS
`time.monotonic()` does not advance during sleep while `time.time()` does, so
`wall − monotonic` is the time spent asleep. A gap above 2 s marks the cell `INVALID_SLEEP`
and it is rerun.

**Every jolt-atlas cell recorded `slept ≤ 0.003 s`**, against a 2 s threshold. The `mono_s`,
`wall_s`, `slept_s` and `sleep_verdict` columns of
[`cells-jolt-atlas.csv`](../../data/cells-jolt-atlas.csv) carry it per cell, including the
cells that passed.

---

## 7. Machine state, declared

The machine is **not dedicated**. It is the same workstation E-001, E-005, and the binius64
and DeepProve halves of this benchmark ran on, and using a different one would invalidate the
comparison.

| Field | Value |
|---|---|
| Hardware | Apple M1 Max (`MacBookPro18,2`), 10 physical / 10 logical cores, 32 GiB |
| OS / kernel | macOS 26.5.2 (25F84) · Darwin 25.5.0 |
| Uptime at campaign start | 12 days 20 h |
| Swap committed at campaign start | **7.66 GB of 9.22 GB** |
| Free space on `/` | 68 GiB |
| Load average at campaign start | 5.65 (1 min); recorded per cell in `cells-jolt-atlas.csv` |
| Other load | Firefox, Claude, WindowServer and the usual system daemons, running throughout |
| Power | AC. Idle sleep suppressed by `caffeinate` for every measured command |
| `powermetrics` | **not recorded** — requires sudo. Substitutes: per-cell `loadavg`, swap, and involuntary context switches in `cells-jolt-atlas.csv` |

Raw: [`bench/data/repro-jolt-atlas/machine-state-start.txt`](../../data/repro-jolt-atlas/machine-state-start.txt).

**The jolt-atlas clone lives outside the repository**, per the licence constraint in
[`COMMIT`](COMMIT). Its build tree and the GPT-2 ONNX export its own script downloads are not
in this repository.
