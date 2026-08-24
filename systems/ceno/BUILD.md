# ceno — build configuration and build-integrity check

Build integrity is verified **before** any measurement, and the check is blocking. It exists
because of a specific, documented failure of our own: in experiment E-001 a harness compiled
without LTO measured our prover **9.0× slower** and **inverted the experiment's conclusion**,
and nothing in the timing output revealed it.

---

## 1. Build configuration — the authors' own, unmodified

| Field | Value | Whose choice |
|---|---|---|
| host binary | `target/release/e2e` (`ceno_zkvm`, `--bin e2e`) | theirs — it is what their own CI benchmarks |
| features | workspace default: `forbid_overflow`, `nightly-features`, `u16limb_circuit`, `parallel`, `bigint-rug` | theirs |
| `jemalloc` | **OFF** | ours, declared — see §4 |
| toolchain | `nightly-2025-11-20` | theirs (`rust-toolchain.toml`) |
| field | BabyBear, degree-4 extension | theirs (`FieldType::default()`) |
| PCS | Jagged(Basefold) | theirs (`PcsKind::default()`) |
| security level | `Conjecture100bits` | theirs — the only variant of the enum |
| `RUSTFLAGS` (host) | none set by us | — |
| `-C target-cpu=native` | **not used** | declared: their `.cargo/config.toml` sets it for `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`, so it applies automatically to workspace builds on this machine |
| guest build | `cargo build --release --examples --target-dir target` inside `examples/` | theirs (`examples-builder/build.rs` runs exactly this) |

Their release profile, declared rather than corrected — the workspace `Cargo.toml` sets:

```toml
[profile.release]
lto = "thin"
```

We left it alone, as we left DeepProve's `lto = "off"` and jolt-atlas's `overflow-checks = true`
alone. Whether `lto = "fat"` would be faster is a measurement we did not make.

### Their `.cargo/config.toml` does apply `target-cpu=native` here

```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "target-cpu=native"]
```

This is the asymmetry binius64 got and DeepProve and jolt-atlas did not — but here it is not
ours to grant or withhold: it is committed configuration in the tree under test, and it fires
on this machine's host triple without us asking. Declared so the comparison is not read as us
having tuned one system and not the others.

---

## 2. The tree does not build on this machine, and that is a result

**`cargo build` fails at `ac16425` on aarch64, for every binary in the workspace.**

```
error: failed to run custom build command for `halo2curves-axiom v0.7.2`
  --- stderr
  Currently feature `asm` can only be enabled on x86_64 arch.
```

`ceno_emul/Cargo.toml:18` requests `halo2curves-axiom` with `features = ["asm"]`
unconditionally; that crate's `build.rs` exits 1 on any non-x86_64 architecture; and every
crate in the workspace depends on `ceno_emul`. The one-word patch that removes the feature,
its justification, and its blast radius are in [`COMMIT`](COMMIT). With it applied, the tree
builds.

**`cargo-ceno` still does not build**, and was not measured. It additionally depends on
`ceno_recursion_v2` → OpenVM → a second copy of the same crate, and the documented install
line in `docs/src/getting-started.md` is:

```sh
JEMALLOC_SYS_WITH_MALLOC_CONF="retain:true,..." \
    cargo install --git https://github.com/scroll-tech/ceno.git --features jemalloc \
    --features nightly-features cargo-ceno
```

which fails here for the same reason. Everything the CLI would have done is done instead by
`e2e` (prove) and by our harness (keygen, verify, correctness control), and every flag we pass
is copied from the CLI's own source and cited at the line. Two consequences are declared
rather than hidden:

- **`cargo ceno verify` was never exercised as a command.** §5 shows it could not have
  succeeded anyway.
- **The guest build path differs slightly from `cargo ceno build`'s.** The CLI's
  `get_rust_flags` adds `-Zunstable-options` and `-Cllvm-args=--basic-block-address-map` and
  passes `-C panic=immediate-abort` directly, where the `examples` package's committed
  `.cargo/config.toml` gets the same abort behaviour from `[profile.release] panic =
  "immediate-abort"` plus the `panic-immediate-abort` build-std feature. We used the config —
  it is what `examples-builder/build.rs` invokes, so it is the path the repository's own build
  system takes. We did not measure what the difference is worth.

### One thing we got wrong first, and how it was caught

The first guest build exported `RUSTFLAGS` reconstructed from the CLI source. An environment
`RUSTFLAGS` **overrides** `[build] rustflags` in `.cargo/config.toml` rather than adding to
it, so that build silently dropped the config's linker-script flags and wrote its ELFs into a
different target directory. It was caught because the expected ELF path was checked for
existence rather than assumed. `bench/scripts/ceno/build-guest.sh` no longer sets `RUSTFLAGS`
at all, and says why in its header.

---

## 3. The build-integrity check: the authors' own examples

Ceno publishes no LTO-style knob whose misconfiguration silently changes a rate, so the check
here is different in kind from binius64's: it is a **functional** gate, not a rate ratio.
Before any measurement, four of the tree's own examples must prove **and verify** end to end.
They are the same examples the repository's `integration.yml` runs.

**Verdict: PASS.** Run before the campaign, on the patched tree:

| Example | `ZKVM_create_proof` | verified |
|---|---|---|
| `fibonacci` (`--hints 10 --public-io 4191`) | 1.07 s | yes |
| `ceno_rt_alloc` | 807 ms | yes |
| `ceno_rt_mem` | 819 ms | yes |
| `ceno_rt_io` | 811 ms | yes |

This gate earned its place immediately. Our first T1-0 run failed verification, and the
question "is Ceno broken on aarch64, or is our task wrong?" is not answerable from the T1-0
run alone. `fibonacci` passing on the same binary, in the same minute, is what made the
failure attributable to our expression — and §5 of [`EXPRESSION.md`](EXPRESSION.md) is what it
turned out to be.

**What this check does not cover.** It establishes that the prover and verifier work; it does
not establish that they are as fast as the authors would get. There is no rate ratio here
against a hand-written kernel because there is no single hot primitive to compare against —
Ceno's prover is a multi-chip GKR pipeline, not one field-multiply loop.

---

## 4. `jemalloc` is OFF, and that decision is about the memory metric

The authors' documented install line enables `jemalloc` and sets:

```
JEMALLOC_SYS_WITH_MALLOC_CONF="retain:true,metadata_thp:always,thp:always,dirty_decay_ms:-1,muzzy_decay_ms:-1,abort_conf:true"
```

`retain:true` with `dirty_decay_ms:-1` and `muzzy_decay_ms:-1` instructs the allocator **never
to return freed pages to the operating system**. That is a sound choice for throughput and a
fatal one for this benchmark: `peak memory footprint` and `maximum resident set size` are both
measured by the OS, so an allocator configured never to release would inflate both by an amount
that has nothing to do with how much memory the proof needs.

This is a genuine tension with the fairness protocol, which says to run each system in the
best configuration its authors document. We resolved it toward the metric and declared it:
**the memory figures in [`RESULTS.md`](RESULTS.md) are for the system allocator, not for the
authors' recommended allocator configuration.** Anyone reproducing their *timing* guidance
should expect different — and probably better — prove times, and worse memory figures, than
ours. We did not measure the difference; `cargo-ceno` is the only target the documented line
builds, and it does not build here at all (§2).

---

## 5. `cargo ceno verify` cannot succeed at this commit, and it changes the correctness control

This is the single most consequential thing in this file, because it decides whether the
correctness control means anything.

A vk that has been through `bincode` **always fails to verify any proof**:

```
VerifyError / VKNotFound("0th shard circuit index 0 missing from vk index map")
```

Mechanism, read from the source and then confirmed by measurement:

| Where | What |
|---|---|
| `ceno_zkvm/src/structs.rs:1081` | `#[serde(skip)] pub circuit_index_to_name: BTreeMap<usize, String>`, commented "mainly used for debugging" |
| `ceno_zkvm/src/scheme/verifier.rs` | `ZKVMVerifier::new` computes a digest and does **not** rebuild that map |
| `ceno_zkvm/src/scheme/verifier.rs:577-583` | the main verification path looks up every chip-proof index in it, and returns `VKNotFound` when absent |

So the map is empty in any deserialized vk, and the verifier rejects. Both `e2e --out-vk` and
`cargo ceno prove --out-vk` write vks through this path, and `cargo ceno verify --proof --vk`
reads them through it.

**It is a completeness defect, not a soundness one.** It makes valid proofs fail; it never
makes invalid proofs pass. Nothing in `RESULTS.md` claims otherwise.

**Why it decides the correctness control.** A corruption sweep run against a round-tripped vk
would report every single corruption as rejected — and would be reporting the vk defect, not
the corruption. It would pass while establishing nothing, which is exactly the vacuous control
this repository's rules forbid. So `bench/scripts/ceno/harness/src/bin/ceno_verify.rs`
regenerates the vk in process by running keygen from the ELF, and the honest-proof positive
control is what proves the sweep is live. Measured both ways, on the same T1-0 proof:

| vk source | honest proof | time to verdict |
|---|---|---|
| loaded from `e2e --out-vk` via bincode | **VERIFY_REJECTED** | 0.000425 s |
| regenerated in process by keygen | **VERIFY_ACCEPTED** | 0.049859 s |

Right of reply applies with priority here ([`CHALLENGE.md`](../../CHALLENGE.md)).

---

## 6. Thread control is NOT fully in our hands, and the cells show it

`RAYON_NUM_THREADS` is set per cell, and it is not the whole story. Ceno's own log emits, on
every run:

```
thread size 10 is not power of 2, using 8 threads instead.
```

from `multilinear_extensions::util` — the same rounding-down DeepProve exhibited. The column
in `RESULTS.md` is therefore labelled **`RAYON thr`**, never "threads", and `(u+s)/real` is
published beside every figure so a reader can see how many cores were actually busy.

---

## 7. Sleep detection, per cell

**1 · Prevention.** Every measured process is wrapped in `caffeinate -dimsu`, which asserts
against display, idle, disk and system sleep for the lifetime of the wrapped process.

**2 · Detection.** `bench/scripts/clockprobe.py` brackets every repetition. On macOS
`time.monotonic()` does not advance during sleep and `time.time()` does, so
`wall_elapsed − monotonic_elapsed` is the time spent asleep. Threshold 2 s, deliberately loose
so ordinary clock jitter does not flag a healthy cell. Any repetition over it is recorded
`INVALID_SLEEP`.

Both guards exist because this machine idle-slept in the middle of a timed run during the
binius64 campaign, and every measurement taken before that point was discarded and rerun.

---

## 8. Machine state, declared

Captured at campaign start into
[`bench/data/repro-ceno/machine-state-start.txt`](../../data/repro-ceno/machine-state-start.txt).

| Field | Value |
|---|---|
| Hardware | Apple M1 Max, 10 physical / 10 logical cores, 32 GiB |
| OS | macOS 26.5.2 (25F84), Darwin 25.5.0, arm64 |
| Uptime at campaign start | 12 days, 22:17 |
| Load average at campaign start | **5.13 / 5.75 / 6.46 — the machine was NOT dedicated** |
| Swap committed at campaign start | **11 510 MB used of 12 288 MB — 778 MB free** |
| Free space on `/System/Volumes/Data` | **49 GB of 926 GB — 95 % full** |
| Power | **Battery, 87 %, discharging, 5:49 remaining — NOT on AC** |
| Other load | Firefox, Microsoft Teams, GitHub Desktop, Claude — not quiesced |
| `powermetrics` | **not recorded** — requires sudo. Substitutes: per-repetition `loadavg` and swap in `cells-ceno.csv`, and `(u+s)/real` per cell |

**These conditions are worse than any previous campaign in this repository, and they are stated
here rather than in a footnote.** binius64 declared "NOT dedicated"; this one is that and also
on battery, with swap nearly exhausted and a boot volume at 95 %. Three consequences are
carried into `RESULTS.md` and are not averaged away:

1. Load of ~5 on a 10-core machine means roughly half the cores were contended. Absolute times
   are therefore upper bounds, and `(u+s)/real` is published per cell so the contention is
   visible rather than inferred.
2. With 778 MB of free swap and 49 GB of disk, a rung whose footprint exceeds RAM cannot be
   measured safely — it would page to a nearly full volume. This is why the top of the ladder
   is gated rather than attempted blind; see `RESULTS.md` and
   [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md).
3. Running on battery, Apple Silicon may apply power-management policies we did not measure and
   cannot rule out.

**No figure in this campaign should be quoted as Ceno's best achievable performance.** They are
what this system did on this machine in this state, and the state is part of the number.
