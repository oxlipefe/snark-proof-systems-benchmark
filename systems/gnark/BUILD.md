# gnark — build configuration and the integrity check run before measuring

`bench/README.md` requires this per system, and it requires it for a specific reason: in our
own prior work (E-001 §0) compiling our prover without LTO made it measure **9.0× slower** and
**inverted a verdict**. A benchmark that does not verify its own builds is measuring its build
system.

gnark's check has four parts. Three are structural and BLOCKING; the fourth is the one that
actually matters, because it is the only one that is a measurement.

---

## 1. Toolchain, and what selects what

```
go.mod requires   go 1.25.7
resolved          go version go1.26.4 darwin/arm64
gnark             v0.16.2  (9838556b92c7783cb82971cf37c0d081cc2b6aec)
gnark-crypto      v0.21.0  (the version go.mod requires; NOT overridden)
CGO_ENABLED       0
build tags        NONE — in particular NOT `purego`, which is the whole of §3
GOFLAGS           unset
```

There is no release profile to get wrong. Go has one optimizing compiler, no LTO switch, and
no `--features` surface. **gnark's equivalent of the LTO trap is not a compiler flag — it is a
build tag that silently swaps the field arithmetic**, and §3 is the check for it.

## 2. The pinned tree is the tree that was measured (BLOCKING)

Two checks, both in [`bench/scripts/gnark/build.sh`](../../scripts/gnark/build.sh), both
blocking:

```
=== 1. the clone is at the pinned commit (BLOCKING) ===
clone HEAD = 9838556b92c7783cb82971cf37c0d081cc2b6aec (v0.16.2)
=== 2. the module cache is byte-identical to the clone (BLOCKING) ===
module cache == clone, 0 differing paths
```

This matters more than it looks. Our task circuits depend on gnark as a **normal Go module**
resolved through the proxy and checksummed in `go.sum`, while everything we read and quote in
`EXPRESSION.md` and `RESULTS.md` came from the **clone**. If those two differed, the code we
reasoned about would not be the code we measured. `diff -r` over the two trees reports **0
differing paths**, so `go.sum` is a complete statement of what was measured.

**No local modifications.** `git status` is clean at the pinned commit. Unlike Ceno — which
does not build on aarch64 at all at its pinned commit without dropping a dependency feature —
gnark needed no patch.

## 3. THE INTEGRITY CHECK THAT IS A MEASUREMENT: is the assembly actually in the binary?

**This is the part that corresponds to the LTO failure, and it is the reason this file exists.**

`gnark-crypto` selects its BN254 field implementation by build tag:

```
ecc/bn254/fp/element_arm64.go    //go:build !purego
ecc/bn254/fp/element_purego.go   //go:build purego || (!amd64 && !arm64)
ecc/bn254/fp/element_arm64.s     //go:build !purego
     → #include "../../../field/asm/element_4w/element_4w_arm64.s"
```

So `-tags purego` deselects the assembly and substitutes generic Go. **If a binary built both
ways measures the same, the assembly was never in the binary we measured**, and every timing
in `RESULTS.md` would be a purego timing wearing an assembly label.

[`bench/scripts/gnark/check-asm-purego.sh`](../../scripts/gnark/check-asm-purego.sh) builds
the **same runner from the same source tree twice** — once normally, once with `-tags purego`
— and measures both on the same task back to back. Being a ratio taken back to back on one
machine, it survives ambient load far better than either absolute number; the loadavg is
recorded anyway.

### What it found

**The assembly IS in the binary, and it buys 7.5 %.**

Structural check first — symbols matching `ecc/bn254/fp.(mul|reduce)` / `element_4w` in each
binary, via `go tool nm`:

| build | matching symbols |
|---|---:|
| default | **1** |
| `-tags purego` | **0** |

So the tag does switch the implementation, and the binary we measured is not silently the
purego one.

Then the measurement — T1-0, Groth16, regime A, 197 763 constraints, N = 5, back to back:

| build | prove ms, all 5 repetitions | median | peak footprint |
|---|---|---:|---:|
| default (arm64 asm) | 659.018, 679.193, 677.166, 638.162, 693.395 | **677.166** | 595 575 744 B |
| `-tags purego` | 720.607, 719.733, 745.972, 728.063, 728.143 | **728.063** | 635 569 160 B |

**Ratio 728.063 / 677.166 = 1.075×.** All five timed repetitions are shown, in the order run.
The warmup (667.355 ms asm, 708.984 ms purego) is excluded from the median, as everywhere else
in this campaign. **The two ranges do not overlap** — the slowest assembly repetition (693.395 ms) is faster
than the fastest purego one (719.733 ms) — so the difference is outside this run's noise, though
it is not large.

Setup was 13 545 ms against 13 619 ms, i.e. unchanged, and `pk_bytes` was byte-identical at
44 740 553 in both. Only the arithmetic path moved.

### What that number means, and what it does not

**It means the check passes**: had the two builds measured the same, every figure in
`RESULTS.md` would have been a purego figure carrying an assembly label, and this file would
say so instead.

**It does NOT mean the arm64 gap is 7.5 %.** This test toggles the *element* assembly, which
arm64 has. It says nothing about the *vector* assembly, which arm64 does not have at all (§4)
and which therefore cannot be toggled on this machine in either direction.

**And it is a smaller factor than we expected**, which is itself worth recording: Groth16's
hot path is multi-scalar multiplication and FFT over curve points, so a faster `Fp::mul` moves
the total less than a naive model predicts. We did not decompose it further. Compare E-001,
where the analogous build error was **9.0×** and inverted a verdict; here the same class of
error would have cost 7.5 % and inverted nothing.

Raw output: [`bench/data/repro-gnark/asmcheck-t1-0-asm.txt`](../../data/repro-gnark/asmcheck-t1-0-asm.txt)
and [`asmcheck-t1-0-purego.txt`](../../data/repro-gnark/asmcheck-t1-0-purego.txt).

## 4. What this machine does NOT give gnark, declared because it plays against gnark

`gnark-crypto` v0.21.0 ships assembly for BN254 on both amd64 and arm64, **but not the same
assembly**. Counted from the tree that was measured:

| | routines in `element_4w` asm | lines |
|---|---:|---:|
| `element_4w_arm64.s` | **3** — `mul`, `reduce`, `Butterfly` | 163 |
| `element_4w_amd64.s` | **13** — the three above plus `addVec`, `subVec`, `sumVec`, `mulVec`, `scalarMulVec`, `innerProdVec`, `fromMont`, `MulBy3`, `MulBy5`, `MulBy13` | 1 862 |

And the vector layer has no arm64 path at all:

```
ecc/bn254/fr/vector_amd64.go     //go:build !purego     ← AVX-512 IFMA, radix-52 Montgomery
ecc/bn254/fr/vector_purego.go    //go:build purego || !amd64
```

There is **no `vector_arm64.go`** — not for BN254, not for any curve in the tree. So on this
machine `Vector.Add`, `Sub`, `ScalarMul`, `Sum` and `InnerProduct` take the generic Go path,
while an amd64 host with AVX-512 gets hand-written IFMA kernels.

**Consequence, stated with its direction.** The single hottest primitive — field multiplication
— IS in assembly here. The vectorized kernels are not. **This machine therefore runs gnark
partly degraded, and the degradation makes gnark's measured times WORSE than they would be on
an AVX-512 amd64 host.** It plays against gnark, not for it. Every cross-system comparison in
`RESULTS.md` carries this sentence or does not get made.

We did not quantify the vector gap. Doing so would require an amd64 machine, and this campaign
has one machine. That is a declared hole, not an estimate.

## 5. Correctness of our own gadgets, before any timing (BLOCKING)

A prover that is fast because its circuit is wrong is not fast. Run with
`-tags=prover_checks`, which makes gnark's own `test.NewAssert` perform a full
setup/prove/verify rather than stopping at the solver:

```
=== 4. correctness of the gadgets (BLOCKING) ===
ok  	github.com/viaas/zk-prover-bench/gnark	2.833s
```

Covered: both ReLU gadgets over positives, negatives, zero and the range boundaries; five
**wrong** witnesses that must fail (the decisive one being `negative_passed_through`, without
which an identity gadget would pass every positive case); the out-of-range witness the
soundness argument rests on; and the A1 assertion driven to its own refusal. Details and the
soundness argument are in [`EXPRESSION.md`](EXPRESSION.md) §4 and §5.

## 6. gnark's own example circuits, through our harness, unchanged (BLOCKING)

**This is the check that stopped the jolt-atlas campaign from publishing three of our own
expression errors as someone else's limits.** If gnark's own circuits do not work through our
harness, the harness is the defect and no limit we hit may be attributed to gnark.

```
EXAMPLE gnark-example-cubic groth16  constraints=3   prove_ms=1.429  verify_ms=1.597  proof=164 B
EXAMPLE gnark-example-cubic plonk    constraints=4   prove_ms=4.115  verify_ms=2.485  proof=520 B
EXAMPLE gnark-example-mimc  groth16  constraints=331 prove_ms=7.519  verify_ms=1.336  proof=164 B
EXAMPLE gnark-example-mimc  plonk    constraints=442 prove_ms=13.214 verify_ms=1.550  proof=520 B
```

`cubic` is `x³+x+5=y` at `x=3` from `examples/cubic/`; `mimc` is taken verbatim from
`examples/mimc/mimc_test.go`. Both at v0.16.2.

**And this produced a result, not just a pass.** gnark's own circuits use no range checks, so
they carry no Pedersen commitment, and their Groth16 proof is **164 bytes**. Ours is **196**.
The 32-byte difference is exactly one compressed G1 point: the commitment that
`std/rangecheck` adds via `api.Commit`. That is how `RESULTS.md` §6 can attribute proof bytes
to named fields instead of guessing at them.

## 7. Memory accounting, declared rather than normalised away

**The allocator is the Go runtime's garbage collector**, at its defaults: `GOGC=100`, no
`GOMEMLIMIT`, `GOMAXPROCS` as stated per cell.

**`debug.FreeOSMemory()` is never called.** It would hand pages back to the OS on demand and
report a peak the prover did not actually hold — a smaller number that describes our
instrumentation rather than the prover.

This is a genuinely different accounting from the other four systems, all of which are Rust
processes using a system or jemalloc allocator, and Ceno's `BUILD.md` §4 already established
that the allocator choice moves the memory column. **A Go process's peak footprint includes GC
headroom that a Rust process's does not.** We did not correct for it and we do not know its
size. It is declared here because `bench/README.md` requires differences no normalization can
fix to be declared rather than averaged.

What follows from it: **the Go-runtime GC is also the only thing in this system that moved
peak memory at all** (`RESULTS.md` §4), because gnark exposes no segmentation, no streaming
prover and no memory cap.

## 8. Machine state at campaign start, uncomfortable parts included

```
Apple M1 Max (MacBookPro18,2), 10 physical / 10 logical cores, 32 GiB
macOS 26.5.2 (25F84), Darwin 25.5.0, uptime 13 days
NOT a dedicated machine — Firefox, Teams and WindowServer resident throughout
boot volume 95 % full, ~50 GiB free
swap: 8.26 GiB of 9-10 GiB in use at campaign start
```

Two honest qualifications:

1. **The reproduction runs in [`REPRODUCTION.md`](REPRODUCTION.md) were taken ON BATTERY**,
   under load averages of 12–30, and say so in that file. **The timing grid in `RESULTS.md`
   was not** — the machine was on AC power, with load averages around 5 at grid start.
   Per-cell `loadavg` and swap are in [`bench/data/cells-gnark.csv`](../../data/cells-gnark.csv)
   so contention is visible rather than inferred.
2. **The swap figure is bad and it matters at the top of the ladder.** With under 1 GiB of
   free swap, the largest rungs are running on a machine that has very little room to page.
   Where a cell failed, `RESULTS.md` reports the failure and its message and does **not**
   attribute it to gnark without saying that this machine was in this state.

Every cell ran under `caffeinate -dimsu`, and `bench/scripts/clockprobe.py` brackets each cell
independently. **`INVALID_SLEEP` wins over every other status, including a successful exit** —
a wall-clock figure that includes a sleep is garbage that looks like data.

## 9. What was NOT built, and why

- **GPU / ICICLE.** `backend.WithIcicleAcceleration` is marked **DEPRECATED in its own
  docstring** ("we don't switch to ICICLE automatically anymore… It will error at runtime
  instead"), and round one of this benchmark is CPU-only by declaration
  (`bench/README.md`). Not built, not measured, not estimated.
- **`WithStatisticalZeroKnowledge`.** Not enabled. Its own docstring says it "makes the prover
  more memory costly, as there are 3 more size n allocations". So the Groth16 figures here are
  from the **non-statistically-ZK path**, which is declared in the conditions line rather than
  folded into a shared "ZK: no" column.
- **A ceremony-backed SRS.** PLONK's setup uses `test/unsafekzg`, which says "to be use for
  test purposes only" in its own docstring. **No figure in this directory is a ceremony-backed
  setup.**
