# binius64 — build configuration and build-integrity check

Build integrity is verified **before** any measurement, and the check is blocking. This
system gets its own check because of a specific, documented failure: in our prior experiment
E-001, a harness compiled without LTO measured this same prover **9.0× slower** and
**inverted the experiment's conclusion**. Nothing in the timing output revealed it — the
phase shares still summed to one. What revealed it was dividing the measured kernel rate by
the throughput of the hardware primitive the kernel is built from.

---

## 1. Build configuration

Binius64's own workspace builds release with `lto = "thin"`. The measurement harness is a
**separate Cargo workspace**, so it does not inherit that profile and must declare it. Both
manifests are reproduced here because their agreement is the thing being checked.

| | `scripts/binius64/harness/Cargo.toml.in` | binius64's own `Cargo.toml` |
|---|---|---|
| `[profile.release]` | `debug = true`, `lto = "thin"` | `lto = "thin"`, `debug = true` |

| Field | Value |
|---|---|
| Toolchain | `rustc 1.97.1 (8bab26f4f 2026-07-14)` · `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Toolchain pin | `scripts/binius64/harness/rust-toolchain.toml`, `channel = "1.97.1"` |
| Flags | `RUSTFLAGS=-C target-cpu=native` |
| Build command | `BINIUS64_ROOT=/path/to/clone scripts/binius64/harness/setup.sh`, which sets that `RUSTFLAGS` and then runs the §2 gate |
| Feature `e001-count` | **not enabled.** It adds an atomic increment per field multiply; no timed run carries counters |
| `rayon` | enabled on `binius-examples`. It is opt-in upstream, and without it the whole prover stack compiles to the single-threaded shim, so a "multi-threaded" cell would be silently serial |
| Threads | set per cell through `RAYON_NUM_THREADS` |

### Where the harness is, and what it contains

The harness lives at [`scripts/binius64/harness/`](../../scripts/binius64/harness/). It is
**our** code; binius64 itself is never vendored into this repository and is obtained as a
clone at the commit in [`COMMIT`](COMMIT), exactly like every other measured prover.

It was carved out of the larger workspace that also served our earlier experiment E-001,
and only the files E-006 compiles were brought across:

```
src/e006/mod.rs           task definitions, INT8 encoding, ReLU, MAC-count assertion
src/e006/matmul.rs        T1
src/e006/mlp.rs           T2, T3
src/stats.rs              order statistics for the repeated measurements
src/sanity/               the build-integrity gate of §2
src/bin/e006_bench.rs     one measured cell
src/bin/e006_negative.rs  the corrupted-trace control
src/bin/e006_verify_split.rs  the verify-time decomposition of §4 of RESULTS.md
src/bin/fieldmul_sanity.rs    the §2 gate's entry point
Cargo.toml.in             the manifest template; setup.sh materialises Cargo.toml from it
```

`src/stats.rs` and `src/sanity/` are **byte-identical to the files that produced E-001's
reference numbers** — the field-multiply probe below in particular — which is what makes the
two campaigns comparable at all. E-006 added new files and touched none of them.

What was deliberately left behind — E-001's own instrument, its roofline and intensity
probes, and the `e001-count` feature that exists only in a locally patched binius64 tree —
is listed with its reason in the harness's own
[`README.md`](../../scripts/binius64/harness/README.md), together with the one figure in
this section that a third party therefore cannot re-run
([`REPRODUCTION.md`](REPRODUCTION.md) §4, a continuity check against E-001).

---

## 2. The build-integrity check: `fieldmul_sanity`

The check does not ask "is this fast?". It asks whether the field multiply runs at the rate
**its own algorithm and this machine's carryless multiplier allow**. Binius64's GF(2^128)
multiply is a schoolbook product plus a reduction — 6 `PMULL` on aarch64. The probe measures
three things in the same process:

1. the machine's raw `PMULL` throughput (a lower bound: the loop round-trips through general
   registers),
2. the same field multiply written directly on the intrinsics — same algorithm, same PMULL
   count, no abstraction,
3. `binius_field`'s own `B128 × B128`.

**The gate criterion is the ratio `binius / hand-written`, with a floor of 0.50.** It is
deliberately not an absolute rate: an absolute threshold would fail on a thermally throttled
machine and pass on a fast broken one. With LTO off, the multiply emits eight non-inlined
calls around its six PMULL and the ratio collapses to ≈0.036.

### Result, 2026-08-23, 7 independent process launches

Raw output in [`bench/data/probe-fieldmul-before.txt`](../../data/probe-fieldmul-before.txt).

| Row | E-001 (2026-08-20) | E-006 (2026-08-23) | E-006 / E-001 |
|---|---|---|---|
| raw PMULL, lower bound | 3161 Mops/s | 3130.2–3226.7 Mops/s | 0.99–1.02 |
| hand-written 6-PMULL | 1249.3 Mmul/s | 1001.8–1014.6 Mmul/s | 0.802–0.812 |
| `binius_field` B128 × B128 | 1238.6 Mmul/s | 996.7–999.6 Mmul/s | 0.805–0.807 |
| **`binius` / hand-written** | **0.991** | **0.983–0.996** | — |

**PASS.** The ratio is 0.983–0.996 against a floor of 0.50, and against 0.991 in E-001. The
broken-build mode the check exists to catch — ≈44 Mmul/s, ratio ≈0.036 — is absent by a
factor of 22 in rate and 27 in ratio. The codegen is correct and the measured build is fit
to produce numbers.



### Post-campaign check

The probe was re-run after the last cell, with 37 GB of swap committed and the machine in its
most degraded state of the day:

| | before campaign | after campaign |
|---|---|---|
| raw PMULL, lower bound | 3130.2–3226.7 Mops/s | 3064.2–3089.4 Mops/s |
| hand-written 6-PMULL | 1001.8–1014.6 Mmul/s | 957.9–974.1 Mmul/s |
| `binius_field` B128 × B128 | 996.7–999.6 Mmul/s | 938.7–945.9 Mmul/s |
| **`binius` / hand-written** | **0.983–0.996** | **0.969–0.987** |

The gate's criterion holds at both ends (floor 0.50; E-001 read 0.991), so **no build drift
occurred across the campaign**. Absolute rates sit ~5% lower after, consistent with the
machine state the cells themselves recorded. Raw output:
[`probe-fieldmul-after.txt`](../../data/probe-fieldmul-after.txt).

### The unexplained part, reported and not interpreted

The **absolute level** of both field-multiply kernels — the library's and the hand-written
one, which shares no code with binius64 — reads **0.805× of its E-001 level**, while the raw
PMULL loop is unchanged at 0.99–1.02×. Our previous experiment E-005 reported the same
anomaly at 0.76–0.80× and did not explain it.

**It persists, at the same magnitude.** E-006 reads 0.805×; E-005 read 0.76–0.80×.
Dispersion across the 7 launches here is 0.3% (996.7–999.6 Mmul/s), tighter than E-005's
5.8%, which is consistent with these launches being sleep-free and `caffeinate`-pinned while
E-005's were not — but that is an observation about the dispersion, not an explanation of the
level. **Cause not established. No number in this repository depends on it**, because the
gate is the ratio and the ratio reproduces. It is recorded here so that a third party sees
it rather than discovers it.

---

## 3. Sleep detection, per cell

**This control exists because the machine idle-slept in the middle of a timed run.**
`pmset -g log` recorded `2026-08-23 13:11:51 Sleep — Entering Sleep state due to 'Idle
Sleep'` with wake at `13:14:49`. Every measurement taken before that point was discarded and
rerun; none of it appears in this repository.

A wall-clock duration that includes sleep is time the CPU was not running. It inflates
`real`, deflates `(user+sys)/real`, and deflates every rate derived from them — and it leaves
no trace in the output. It is the same class of hazard as the LTO failure: an environment
condition that can invert a result invisibly.

Two independent guards:

**1 · Prevention.** Every measured command, and every long build, runs under
`caffeinate -dimsu`, which asserts against display, idle, disk and system sleep for the
lifetime of the wrapped process.

**2 · Detection.** Prevention can fail — an assertion can be released, or the machine can be
lidded. So every cell is bracketed by a clock probe. On macOS `time.monotonic()` is
`mach_absolute_time()` (verified through `time.get_clock_info`), which **does not advance
during sleep**, while `time.time()` does. For any interval:

```
wall_elapsed − monotonic_elapsed  ≈  seconds spent asleep
```

`bench/scripts/clockprobe.py` marks both clocks before and after each cell and takes the
difference. A cell with a gap above **2 s** is recorded as `INVALID_SLEEP` and rerun; the
threshold is loose so that ordinary jitter and NTP steps do not flag a healthy cell, while a
real idle sleep on this machine lasts minutes. The `mono_s`, `wall_s`, `slept_s` and
`sleep_verdict` columns of [`bench/data/cells.csv`](../../data/cells.csv) carry the result
for every cell, including the ones that passed.

A third, free cross-check comes from the instrument itself: prove and verify times are taken
with Rust's `Instant`, which on macOS is also monotonic and sleep-excluding, while
`/usr/bin/time -l`'s `real` is wall-clock. A cell that slept shows the two disagreeing.

---

## 4. Machine state, declared

The machine is **not dedicated**. It is the same workstation E-001 and E-005 ran on, and
using a different one would invalidate the comparison with them.

| Field | Value |
|---|---|
| Hardware | Apple M1 Max (`MacBookPro18,2`), 10 physical / 10 logical cores, 32 GiB |
| OS / kernel | macOS 26.5.2 (25F84) · Darwin 25.5.0 |
| Uptime at campaign start | 12 days |
| Swap committed at campaign start | **9.03–9.35 GB of 10.24 GB** |
| Free space on `/` at campaign start | 109 GiB |
| Load average at campaign start | 7.4–8.4 (1 min) |
| Other load | Firefox, Claude, WindowServer and the usual system daemons, running throughout |
| Power | AC, battery 100%. Idle sleep active until `caffeinate` was adopted; see §3 |
| `powermetrics` | **not recorded** — requires sudo. Substitutes: the field-multiply probe before the campaign, plus per-cell `loadavg` and swap in `cells.csv` |

`(user+sys)/real` is recorded per cell precisely so a reader can see how much of each
wall-clock figure was computation rather than waiting.
