# Plonky3 — build configuration and build-integrity check

Build integrity is verified **before** any measurement, and the check is blocking. This
benchmark's binius64 directory explains why: in experiment E-001 a harness compiled without
LTO measured that prover **9.0× slower** and inverted the experiment's conclusion, and nothing
in the timing output revealed it — the phase shares still summed to one. What revealed it was
dividing the measured kernel rate by the throughput of the hardware primitive the kernel is
built from. The same division gates this system, over both of its fields.

---

## 1. Build configuration

Plonky3's own workspace declares two release-like profiles: `release` (the cargo default) and
`optimized`, which sets `lto = "thin"`, `codegen-units = 1`, `opt-level = 3` (its root
`Cargo.toml`, lines 167–171). The measurement harness is a **separate Cargo workspace**, so it
inherits neither and must declare one. It declares the `optimized` one.

| | `scripts/plonky3/harness/Cargo.toml.in` | Plonky3's own `Cargo.toml` |
|---|---|---|
| `[profile.release]` | `debug = true`, `lto = "thin"`, `codegen-units = 1` | `[profile.optimized]`: `lto = "thin"`, `codegen-units = 1`, `opt-level = 3` |

| Field | Value |
|---|---|
| Toolchain | `rustc 1.96.1 (31fca3adb 2026-06-26)` |
| Toolchain pin | `scripts/plonky3/harness/rust-toolchain.toml`, `channel = "1.96.1"` |
| Flags | `RUSTFLAGS=-C target-cpu=native` |
| Build command | `PLONKY3_ROOT=/path/to/clone scripts/plonky3/build.sh`, which sets that `RUSTFLAGS` and then runs the §2 gate |
| Feature `parallel` | **enabled** on `p3-sumcheck`, `p3-multilinear-util`, `p3-whir`, `p3-dft` and `p3-maybe-rayon`. It is opt-in upstream, and without it the whole stack compiles to `p3-maybe-rayon`'s single-threaded shim, so a "10-thread" cell would be silently serial. Same hazard, same fix, as `rayon` on `binius-examples` in this benchmark's binius64 build |
| Feature `probe-binary-pcs` | **never enabled in a measured build.** It is a deliberate compile failure; see §3 |
| Threads | set per cell through `RAYON_NUM_THREADS` |

### Where the harness is, and what it contains

The harness lives at [`scripts/plonky3/harness/`](../../scripts/plonky3/harness/). It is
**our** code; Plonky3 itself is never vendored into this repository and is obtained as a clone
at the commit in [`COMMIT`](COMMIT), exactly like every other measured prover.

```
src/tasks.rs        the published instances, drawn to be binius64's instances (see below)
src/fields.rs       the two field pairs and the one property that separates them
src/mle.rs          multilinear extensions, with ONE index convention, tested against Plonky3's
src/matmul.rs       Thaler's MATMULT on p3-sumcheck, generic over the field pair
src/pcs.rs          the WHIR commitment route — prime field only, and that is the result
src/route.rs        one measured repetition, per route, with the brackets declared
src/sanity/         the build-integrity gate of §2
src/stats.rs        order statistics for the repeated measurements
src/probe_binary_pcs.rs   the deliberate compile failure of §3
src/bin/p3_bench.rs       one measured cell
src/bin/p3_negative.rs    the corrupted-proof control
src/bin/fieldmul_sanity.rs   the §2 gate's entry point
Cargo.toml.in       the manifest template; setup.sh materialises Cargo.toml from it
```

`src/stats.rs` is **byte-identical to the file that produced E-001's and E-006's order
statistics**, and `src/sanity/handmul.rs` is byte-identical to the 6-PMULL reference kernel
those campaigns used. That is what makes the binary row of §2 comparable with
[`../binius64/BUILD.md`](../binius64/BUILD.md) §2 at all.

### The instance is checked against binius64's, not assumed to match

`bench/TASKS.md` fixes a seed; `systems/binius64/EXPRESSION.md` §7 fixes the RNG and the draw
order. This harness reproduces that draw and then **verifies** it: the largest magnitude any
partial accumulator reaches is a function of every drawn operand, binius64 publishes it per
task (`EXPRESSION.md` §6), and `Instance::assert_matches_binius64` refuses to return an
instance whose value differs.

| task | max &#124;partial accumulator&#124; | binius64 §6 publishes | verdict |
|---|---:|---:|---|
| T1-0 | 270 167 | 270 167 | **same instance** |
| T1-a | 421 915 | 421 915 | **same instance** |

That check is a unit test (`tasks::tests::t1_0_is_the_instance_binius64_measured`) and it runs
on every build.

---

## 2. The build-integrity check: `p3-fieldmul-sanity`

The check does not ask "is this fast?". It asks whether each field's multiply runs at a rate
its own algorithm and this machine's primitives allow. Two failures are in scope, and both are
silent:

1. **LTO off.** The harness is a separate workspace; without `lto = "thin"` the field multiply
   is measured across a crate boundary with cross-crate inlining off.
2. **No carryless multiply.** `p3-binary-field`'s `clmul` backend is compiled only under
   `target_feature = "aes"` (`binary-field/src/clmul/mod.rs:14-31`). Without it every
   `GF(2^128)` multiply falls back to the bit-serial loop of `scalar_clmul_64x64`, roughly two
   orders of magnitude slower.

The probe measures five things in the same process: raw `PMULL` throughput, raw 32-bit integer
multiply throughput, the hand-written 6-PMULL GHASH multiply (byte-identical to E-001's),
`p3-binary-field`'s `BinaryField128 × BinaryField128`, and `p3-koala-bear`'s
`KoalaBear × KoalaBear`.

**The gate criteria are two ratios**, never absolute rates: an absolute threshold would fail on
a thermally throttled machine and pass on a fast broken one.

* `p3-B128 / hand-written 6-PMULL ≥ 0.02`
* `raw u32 muls per KoalaBear multiply ≤ 24` (a Montgomery multiply costs 2)

### Result, 2026-09-03, 3 independent process launches

Raw output in [`bench/data/probe-p3-fieldmul.txt`](../../data/probe-p3-fieldmul.txt).

| Row | measured | E-006's figure for the same probe |
|---|---:|---:|
| raw PMULL, lower bound | 3 220.0–3 220.8 Mops/s | 3 130.2–3 226.7 Mops/s |
| raw u32 multiply, lower bound | 9 740.4–9 758.6 Mops/s | — (new row) |
| hand-written 6-PMULL | 1 012.2–1 015.5 Mmul/s | 1 001.8–1 014.6 Mmul/s |
| **`p3-binary-field` B128 × B128** | **57.8–58.1 Mmul/s** | — |
| **`p3-koala-bear` KoalaBear × KoalaBear** | **1 761.6–1 770.5 Mmul/s** | — |
| `p3-B128` / hand-written | **0.057** (floor 0.02) | — |
| u32 muls per KoalaBear multiply | **5.5** (ceiling 24) | — |

**PASS**, on both criteria. And the two raw rows and the hand-written kernel reproduce E-006's
figures to within 1 %, on the same machine four days later, which is the continuity check that
makes this system's numbers comparable with binius64's at all.

### The 17.5× that the gate reports and does NOT fail

`p3-binary-field`'s `GF(2^128)` multiply runs at **0.057× of the hand-written 6-PMULL kernel**
— a factor of **17.5**. That is not a broken build, and the gate's floor is set at 0.02
precisely so that it does not fire on it. **It is an algorithm difference, and it is one of
this campaign's results.** The two multiplies are not the same multiply:

| | binius64 / the hand-written reference | `p3-binary-field` |
|---|---|---|
| representation | GHASH polynomial basis, `GF(2)[x]/(x^128+x^7+x^2+x+1)` | **Wiedemann tower basis**, `GF(2) ⊂ GF(4) ⊂ … ⊂ GF(2^128)` |
| per multiply | 4 schoolbook `PMULL` + 2 reduction `PMULL` | 4 `clmul_64x64` + a **two-way change of basis in and one back out**, byte-at-a-time through lookup tables (`binary-field/src/clmul/basis.rs`), plus a shift/XOR reduction |
| measured | 1 012 Mmul/s | 57.9 Mmul/s |

So *"the binary field"* is not one substrate. **Comparing Plonky3's binary field with
binius64's compares two representations of `GF(2^128)` as much as it compares two provers**,
and any figure in `RESULTS.md` that crosses that line says so in the same sentence.

### What this gate could NOT distinguish on this machine, stated rather than implied

The `aes` hazard of failure mode 2 **does not fire on `aarch64-apple-darwin`**: the target
carries `aes` in its baseline. Measured, not assumed — the probe was rebuilt with `RUSTFLAGS`
empty into a separate target directory and read **57.8–58.1 Mmul/s**, identical to the
`-C target-cpu=native` build. Plonky3's own comment says the same
(`binary-field/src/clmul/mod.rs:22-27`: *"`aarch64-apple-darwin` has `aes`, but generic AArch64
Linux and every `x86_64` target need `-C target-feature=+aes`"*). **A reproduction on Linux or
on x86_64 without those flags would be measuring the bit-serial fallback, and the gate is what
would catch it there.** On this machine the gate's binary criterion is a check on LTO alone.

---

## 3. The absence probe — the binary field has no PCS, and it is MEASURED

This directory claims that no multilinear polynomial commitment scheme in Plonky3 accepts a
binary field. A claim of **absence** is the cheapest kind to assert and the most expensive to
withdraw, so it is not made from a grep. `scripts/plonky3/run-probe-binary-pcs.sh` asks the
compiler to instantiate `WhirConfig` and `WhirProver` over `BinaryField128` and records the
refusal in [`bench/data/probe-plonky3-whir-binary.txt`](../../data/probe-plonky3-whir-binary.txt):

```
error[E0599]: the associated function or constant `new` exists for struct
`WhirConfig<BinaryField128, BinaryField128, BinaryChallenger<..., ...>>`,
but its trait bounds were not satisfied
   --> src/probe_binary_pcs.rs:42:45
   = note: the following trait bounds were not satisfied:
           `BinaryField128: TwoAdicField`
```

The probe is behind a Cargo feature that no measured build enables, and `run-all.sh` rebuilds
the measured binaries after running it so that nothing timed can carry it. **A build that
SUCCEEDS here withdraws the claim in [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1**, and the
script says so in its own output.

---

## 4. Sleep detection, per cell

Identical to this benchmark's other systems, and it exists because the machine idle-slept in
the middle of a timed run once. Every measured command runs under `caffeinate -dimsu`, and
every cell is bracketed by `bench/scripts/clockprobe.py`: on macOS `time.monotonic()` does not
advance during sleep while `time.time()` does, so `wall − monotonic` is the time spent asleep.
A cell with a gap above 2 s is recorded `INVALID_SLEEP` and rerun. The `mono_s`, `wall_s`,
`slept_s` and `sleep_verdict` columns of
[`bench/data/cells-plonky3.csv`](../../data/cells-plonky3.csv) carry the result for every cell,
including the ones that passed.

---

## 5. Machine state, declared

The machine is **not dedicated**. It is the same workstation E-001, E-005 and E-006 ran on, and
using a different one would invalidate the comparison with them.

| Field | Value |
|---|---|
| Hardware | Apple M1 Max (`MacBookPro18,2`), 10 physical / 10 logical cores, 32 GiB |
| OS / kernel | macOS 26.5.2 · Darwin 25.5.0 |
| Other load | the usual desktop session, running throughout |
| `powermetrics` | **not recorded** — requires sudo. Substitutes: the §2 probe before the cells, plus per-cell `loadavg` and swap in `cells-plonky3.csv` |

`(user+sys)/real` is recorded per cell precisely so a reader can see how much of each
wall-clock figure was computation rather than waiting.

**`peak RSS` is recorded and is NOT cited.** On this machine its own dispersion is 22.9 %;
`peak footprint` reproduces to +0.3 % between campaigns, and it is the column `RESULTS.md`
reads.
