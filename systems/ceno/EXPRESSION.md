# ceno — how each task was expressed

`bench/TASKS.md` fixes each task by an **exact MAC count**. That count is the denominator of
both `MAC/s` and `bytes/MAC`, so it is never recomputed here. The generator
[`bench/tasks/ceno/gen/src/main.rs`](../../tasks/ceno/gen/src/main.rs) counts the
multiply-accumulates of the reference computation and refuses to write a file that disagrees
with the published number:

```rust
assert_eq!(
    e.macs, published,
    "{task}: counted {} MACs, bench/TASKS.md publishes {published}. The published \
     count is frozen; this generator does not get to disagree with it.",
    e.macs
);
```

Nothing in this directory is Ceno code.

---

## 1. THE DECLARATION THAT GOVERNS EVERY NUMBER BELOW

**Ceno is a RISC-V zkVM. It does not prove arithmetic; it proves the execution of
instructions.** T1, T2 and T3 are expressed as RISC-V programs, compiled to a
`riscv32im` ELF, executed by Ceno's emulator, and it is that *execution trace* that is proved.

The other three systems in this benchmark are circuit-based. Given the same task, they build a
constraint system whose size is set by the arithmetic; Ceno builds a trace whose size is set by
the instruction count. **A zkVM proving a matmul and a circuit proving the same matmul are not
doing the same work, even though the task is the same.**

Measured, not asserted — Ceno's own emulator, on the exact instances below:

| Task | MACs (frozen) | RISC-V instructions | cycles | **instructions / MAC** | **cycles / MAC** |
|---|---:|---:|---:|---:|---:|
| T1-0 | 65 536 | 800 913 | 3 203 656 | **12.22** | **48.88** |
| T1-a | 589 824 | 6 316 368 | 25 265 476 | **10.71** | **42.84** |
| T1-b | 2 359 296 | 25 245 947 | 100 983 792 | **10.70** | **42.80** |
| T1-c | 9 437 184 | 100 947 798 | 403 791 196 | **10.70** | **42.79** |
| T1-d | 37 748 736 | 403 755 202 | 1 615 020 812 | **10.70** | **42.78** |
| T2 | 92 224 | 1 124 867 | 4 499 472 | **12.20** | **48.79** |
| T3 | 737 792 | 8 876 478 | 35 505 916 | **12.03** | **48.12** |

Ceno's cycle counter runs at `FullTracer::SUBCYCLES_PER_INSN = 4` sub-cycles per instruction,
which is why the last column is four times the one before it.

**So the ladder that costs a circuit 65 536 multiplications costs Ceno 800 913 proved
instructions.** Every cross-system ratio in [`RESULTS.md`](RESULTS.md) carries that sentence in
the same paragraph, and §7 states our position on whether such a ratio should be quoted at all.

Counts produced by [`bench/scripts/ceno/harness/src/bin/ceno_cycles.rs`](../../scripts/ceno/harness/src/bin/ceno_cycles.rs),
which emulates without proving; raw output in [`bench/data/cycles-ceno/`](../../data/cycles-ceno/).

## 2. The INT8 encoding, declared

`bench/TASKS.md` specifies INT8 operands in `[-128, 127]` and an INT32 accumulator, output not
requantised. RV32IM has native 8-bit loads and a 32-bit multiply, so no encoding trick is
needed and none was used: each operand occupies one byte of the hint region and is read with a
sign-extending `lb`; the product is a native `mul`.

The instruction mix Ceno's own tracer reports for T1-0 confirms the encoding is honest rather
than assumed — one multiply per MAC, two byte-loads per MAC:

```
tracer generated MUL   65 541 records      (65 536 MACs + 5)
tracer generated LB   131 072 records      (exactly 2 × 65 536)
tracer generated ADD  197 687 records
tracer generated ADDI 136 756 records
tracer generated BNE   66 772 records
tracer generated BGEU  65 797 records
```

**`BGEU 65 797` is Rust's slice bounds checking** — about 8 % of the instruction count and
therefore about 8 % of the proved trace. It is a cost of expressing the task in safe Rust, not
a cost Ceno imposes. We left it in: it is what an implementer writes, and removing it with
`get_unchecked` would be us hand-tuning one system's expression in a way we did not do for the
others. It is declared here so nobody has to discover it in a flame graph, and the headroom it
represents belongs to Ceno, not to us.

**No range constraints.** As with binius64, nothing establishes that the hint bytes were in
`[-128, 127]` — every byte trivially is, since a byte is 8 bits, and the guest sign-extends it.
This is stronger than binius64's position, where 64-bit words carried 8-bit values with no
constraint; here the 8-bit width is structural.

## 3. T1 — one guest, the whole ladder

[`bench/tasks/ceno/guest/bench_t1.rs`](../../tasks/ceno/guest/bench_t1.rs). The shape `(M, K, N)`
arrives as a hint, so all five rungs share one ELF and therefore **one proving and verifying
key**. That removes per-rung keygen as a confounder — keygen is ~9 s and would otherwise vary
between rungs for reasons having nothing to do with the task. The cost is loop bounds the
compiler cannot constant-fold, which is why T1-0 shows 12.22 instructions/MAC against the
ladder's asymptotic 10.70: the fixed program prologue and the hint-region walk are amortised
away as the rung grows.

The full INT32 output matrix is materialised on the heap — the task's output *is* the matrix,
so the stores that produce it are part of the measured work — and then bound to the proof as
described in §6.

## 4. T2 and T3 — one guest, both batches

[`bench/tasks/ceno/guest/bench_mlp.rs`](../../tasks/ceno/guest/bench_mlp.rs). Batch size is a
hint: 1 for T2, 8 for T3. Same ELF, same vk. T3 proves 8 independent inputs over the same
weights in **one** proof.

## 5. Requantisation: none, and on a 32-bit machine that is expensive

Amendment A1 fixes no requantisation, so accumulators carry full width between layers. Only
layer 1's accumulator provably fits in `i32` (worst case `200 · 128 · 128 = 3 276 800`); from
layer 2 on it needs 64 bits, and **RV32IM has no 64-bit arithmetic**. Every accumulate in
layers 2-4 is lowered to a multi-instruction sequence over register pairs.

That is visible in the table in §1: T2 costs 12.20 instructions per MAC against T1's
asymptotic 10.70, on a network where most MACs are in the 64-bit layers. It is not a defect of
Ceno — it is what "no requantisation" costs on a 32-bit target, and it is the clearest example
in this benchmark of a task rule interacting with a substrate.

Layer 1 is computed in `i32` and widened rather than being done in `i64` throughout. That is
deliberate: the fairness protocol says give the system its best honest expression, and paying
64-bit costs for an accumulator that provably fits in 32 bits would have charged Ceno for work
the task does not require.

**The bound is asserted on the host, in `i128`, with the factor-of-two margin A1 demands** —
the same discipline binius64's builder applies — and the generator refuses to emit otherwise:

| Task | max &#124;accumulator&#124; observed | headroom under `i64::MAX` |
|---|---:|---:|
| T2 | 8 955 951 054 519 (8.96·10¹²) | 1.03·10⁶× |
| T3 | 19 638 755 553 042 (1.96·10¹³) | 4.69·10⁵× |

The guest does not re-derive the bound, because a guest-side check would be paid for in proved
cycles and would not make the emitted trace any safer.

## 6. How the output is bound — and the failure that taught us what it means

A circuit publishes its output on `inout` wires. A zkVM cannot: the guest binds its output by
hashing it into the proof's public values. `ceno_rt::commit(&out)` computes **Keccak-256 in
software, as ordinary RISC-V**, and emits the digest through `syscall_pub_io_commit`. The host
independently derives the expected digest from `--public-io`
(`ceno_zkvm::e2e::public_io_words_to_digest_words`, which Keccak-hashes the little-endian bytes
of those words). The two must agree.

They must agree **or the proof does not verify**, and we learned that the hard way. Our first
T1-0 run proved in 3.42 s and then failed with:

```
VerifyError("0th prod_r != prod_w")
```

which is a RAM grand-product imbalance — a memory-consistency error, giving no hint that the
public output was the problem. We had passed no `--public-io`, so the host expected the
Keccak digest of the empty string while the guest committed the digest of the real matrix.

**It took a four-mode diagnostic guest to attribute it**, because the message points at the
wrong subsystem. The isolating runs, all on the same binary within a few minutes:

| Guest | heap | commit | verdict |
|---|---|---|---|
| commit probe, `L = 0` | no | `commit(&[])` → empty digest, matches host default | **VERIFY_ACCEPTED** |
| diag mode 2 | yes | none | **VERIFY_ACCEPTED** |
| diag mode 0 | no (stack) | 1024 bytes | **VERIFY_REJECTED** |
| diag mode 1 | yes | 4 bytes | **VERIFY_REJECTED** |
| diag mode 3 | yes | 1024 bytes | **VERIFY_REJECTED** |
| upstream `fibonacci` | no | 4 bytes, **matching `--public-io 4191`** | **VERIFY_ACCEPTED** |

Heap allocation was the plausible-looking culprit and was wrong; the variable is whether the
committed digest matches the host's. With the correct `--public-io`, T1-0 verifies.

**This is the system working, not failing.** The public output is genuinely bound, and a proof
of a different output is genuinely rejected — which is exactly what the benchmark's correctness
control needs to be true. It is written up here because the error message actively misdirects,
and an implementer meeting it deserves the two hours back.

Two consequences for the measurements:

- **The Keccak is inside the measured trace.** Part of every T1/T2/T3 cycle count is hashing
  the output rather than multiplying. [`bench_commit_probe.rs`](../../tasks/ceno/guest/bench_commit_probe.rs)
  exists to size that share; it is an instrument and is never proved.
- **`--public-io` is passed as argv**, because `e2e` exposes no file form. T1-d's list is
  429 067 bytes against this machine's `ARG_MAX` of 1 048 576. That headroom is declared, not
  assumed, and the per-task argv size is in
  [`bench/tasks/ceno/manifest.json`](../../tasks/ceno/manifest.json).

## 7. Witness seeds — and the one thing this system got that the others did not

Fixed per task, as `bench/TASKS.md` requires. Same seeds as every other system in this
benchmark:

| Task | seed |
|---|---|
| T1-0 | `0xE0060100` |
| T1-a | `0xE00601A0` |
| T1-b | `0xE00601B0` |
| T1-c | `0xE00601C0` |
| T1-d | `0xE00601D0` |
| T2 | `0xE0060200` |
| T3 | `0xE0060300` |

**The RNG *is* the same one.** DeepProve's and jolt-atlas's generators are Python and draw from
numpy's PCG64, so they share binius64's seeds but not its stream: same shapes, same MAC counts,
**different instances**. Ceno's guest is Rust, so this generator uses the same crate (`rand`
0.10), the same `StdRng::seed_from_u64`, and the same draw order as
`scripts/binius64/harness/src/e006/{matmul,mlp}.rs`.

That claim is checkable rather than asserted, and we checked it before relying on it. binius64
recorded the largest intermediate its instance reaches; this generator, run independently,
reproduces both figures exactly:

| Task | binius64's recorded max &#124;accumulator&#124; | this generator's |
|---|---:|---:|
| T2 | 8 955 951 054 519 | **8 955 951 054 519** |
| T3 | 19 638 755 553 042 | **19 638 755 553 042** |

**So Ceno and binius64 prove the same instance, value for value** — the only such pair in this
benchmark. An order-sensitive FNV-1a digest of the drawn operands is recorded per task in
`manifest.json` so a third party can confirm it without rerunning binius64.

This does not make the two systems' numbers commensurable — §1 is still true, and a zkVM and a
circuit are still not doing the same work. It removes one confound, not the main one.

## 8. Hint layout

`--hints-file` maps a raw file as a memory segment. The file is written in the byte layout
Ceno's own `ceno_host::CenoStdin` defines (`Items::finalise`): a header of `u32` words
`[data_offset, alignment, len_0, …]`, then each record's bytes back to back, padded to the
4-byte alignment. `ceno_rt::read_slice()` walks exactly this, so each record is handed to the
guest as a zero-copy `&[u8]` with no deserialisation cost inside the proved trace.

| Task | records | hint file bytes |
|---|---|---:|
| T1-* | `(M,K,N)`; `A` row-major; `B` row-major | 65 824 – 639 008 |
| T2/T3 | `BATCH`; the four weight matrices `[out][in]`; the inputs | 92 448 / 93 848 |

The hint window is 128 MB (`ceno_emul/src/platform.rs:92`), so no task came near it, and **no
padding of any kind was applied to any task.** Ceno imposes no power-of-two shape requirement
— the trace is padded per chip to the next power of two internally, but the *task* is not
reshaped, so unlike DeepProve (768 → 1024, 1.778× the MACs actually proved) and jolt-atlas
(which cannot express a non-power-of-two rung at all) there is no discrepancy between the MACs
`bench/TASKS.md` publishes and the MACs Ceno performed.
