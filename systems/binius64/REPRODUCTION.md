# binius64 — the system's own reference numbers, and whether we reproduced them

`CHALLENGE.md` commits this repository to reproducing each measured system's **own published
reference number before reporting anything about that system**, and to publishing the
discrepancy *above* the result if we cannot. This file discharges that commitment for
binius64.

---

## 1. The finding that comes first: there is no published, hardware-attributed reference number

We searched the pinned tree at `eac2484b1a2e0b68d7b9e9b2e40f3c86ef220d4d` for any performance
figure its authors publish and a third party could reproduce: every `.md` file, every
`benches/` directory, every example, and the CI configuration.

**There is none.** Stated as an absence, because an absence is a result:

- There is **no `docs/` or `book/` directory**. Seven `.md` files exist in total.
- **No results table, no committed benchmark baseline, and no captured benchmark output**
  anywhere in the tree.
- **CI never runs `cargo bench`**, and says so explicitly
  (`.github/workflows/ci.yml`): *"CI never runs them (`cargo bench`), so codegen + linking
  them is pure overhead."*
- The nine `benches/` directories contain criterion benchmarks that measure at runtime and
  ship **no expected values**.

This is not a criticism of the project — it is a research prover, and it publishes its
**circuit sizes** rather than its timings, which is the more reproducible choice. But it
means the fairness protocol's step 3 cannot be discharged the usual way, and that has to be
said out loud rather than papered over with a number of our own.

What the tree *does* offer, in descending order of how reproducible it is:

| # | Reference | Machine stated? | Reproducible? |
|---|---|---|---|
| A | CI-enforced **circuit-size snapshots** (`crates/examples/snapshots/*.snap`) | n/a — machine-independent | **Yes, exactly** |
| B | A sample CLI transcript in `README.md` (SHA-512, 65 536-byte message) | **No** | Only as an order of magnitude |
| C | Two microbenchmark tables in source comments, marked "Apple M1 Pro" | Yes, but they measure BLAKE3 and Rayon task sizing, not the proof system | Not a proof-system figure |

We therefore reproduce **A** as the primary check — it is the only number the project itself
commits to and enforces — and report **B** for what it is worth, labelled unattributed.

---

## 2. Reference A — circuit-size snapshots (the project's own CI gate)

`crates/examples/snapshots/*.snap` records, per example circuit, the exact gate count,
constraint counts by kind, and value-vector layout. Each example binary exposes a
`check-snapshot` subcommand that rebuilds the circuit and compares. These are the numbers
binius64's authors actually stand behind, and they are machine-independent, so a mismatch
would mean our tree or our build is wrong.

**Result: 7 of 7 valid examples match exactly.**

```
sha512:  ✓ Circuit statistics match snapshot
sha256:  ✓ Circuit statistics match snapshot
keccak:  ✓ Circuit statistics match snapshot
blake3:  ✓ Circuit statistics match snapshot
ethsign: ✓ Circuit statistics match snapshot
zklogin: ✓ Circuit statistics match snapshot
blake2b: ✓ Circuit statistics match snapshot
```

Raw output: [`bench/data/repro-binius64/snapshot-check.txt`](../../data/repro-binius64/snapshot-check.txt).

Two notes, so the count is not read as better than it is. `check-snapshot` must be invoked
through `cargo run` — it reads `CARGO_MANIFEST_DIR` to locate the snapshot directory and
fails outright when the compiled binary is run directly. And an eighth file,
`bitcoin_headers.snap`, exists in the snapshot directory but is **not** the name of an
example binary; invoking it prints the list of valid examples instead. We report 7, not 8.

**This is the strongest reproduction available for binius64, and it passes.** It establishes
that our vendored tree builds the same circuits its authors' CI builds. It says nothing about
timing, because it is not a timing reference.

## 3. Reference B — the README SHA-512 transcript

The `README.md` of the pinned tree shows this transcript, with **no machine, no core count
and no thread setting stated**, for `cargo run --release --example sha512 prove
--message-len 65536` at `log_inv_rate = 1`:

```
Building circuit [ 2.99s | 100.00% ]
Setup [ 619.81ms | 100.00% ] { log_inv_rate = 1 }
Generating witness [ 14.12ms | 100.00% ]
prove [ 128.58ms | 100.00% ] { ... n_witness_words = 1048576, n_bitand = 1048576, n_intmul = 1 }
```

The transcript is truncated in the README (`...`); no verify time and no proof size are
given. Multithreading is off by default (`README.md`: *"Multithreading using Rayon is
available, but it is disabled by default"*).

**Because no machine is stated, this is not a reproducible reference and no pass/fail can be
declared against it.** We ran it anyway, on the machine declared in `BUILD.md`, and report
both figures side by side so a reader can judge.

| | README (machine **not stated**) | E-006, M1 Max, 1 thread |
|---|---|---|
| Building circuit | 2.99 s | **1.17 s** |
| Setup | 619.81 ms | **360 ms** |
| Generating witness | 14.12 ms | **5.10 ms** |
| **prove** | **128.58 ms** | **162 ms** |
| proof size | not given | 273 KiB |

**No verdict is declared**, because the reference has no machine attached. What can be said
is that the figures do not move together: our circuit build is **2.6× faster**, our setup
**1.7× faster**, our witness generation **2.8× faster**, and our prove **1.26× slower**. A
single machine-speed factor does not explain that pattern, and we do not offer one.

We also ran it with `RAYON_NUM_THREADS=10` and got **161 ms** — indistinguishable from the
1-thread run. That is expected and it confirms the configuration is the documented default:
`rayon` is opt-in on `binius-examples`, so the example is single-threaded either way, which
matches the README's *"Multithreading using Rayon is available, but it is disabled by
default."* Our number is therefore taken in the configuration its authors document.

Raw output: [`sha512-readme.txt`](../../data/repro-binius64/sha512-readme.txt) and
[`sha512-readme-t10.txt`](../../data/repro-binius64/sha512-readme-t10.txt).

## 4. Continuity with our own prior measurement of this prover

Independently of what binius64 publishes, we have measured this prover twice before, on this
machine, with this harness — experiments E-001 and E-005. That gives a reference point whose
conditions we *do* control, and reproducing it is the strongest evidence that this campaign's
build and machine are in the same state.

The overlapping point is the E-001 subject `matmul-int8` at `m = 2048`, `log_inv_rate = 1`,
1 thread:

| Campaign | Date | Median prove wall time |
|---|---|---|
| E-001 | 2026-08-20 | 320 ms |
| E-005 | 2026-08-23 | 319.03 ms [316.70–344.38], N = 5 — **0.3%** from E-001 |
| E-006 (this) | 2026-08-23 | **330.53 ms** [316.69–345.77], N = 5, warmup 1 |



**Reproduced.** E-006 sits **3.3% above** E-001's 320 ms and 3.6% above E-005's median. More
tellingly, the **fastest repetition of the three campaigns is the same number**: E-005's
minimum was 316.70 ms and E-006's is **316.69 ms**, a difference of 0.003%. The floor is
identical and the median has drifted up by a few percent, which is what a busier machine
looks like — E-006's campaign ran with ~9 GB of swap already committed.

Conditions of this run: `e001-harness` unmodified, subject `matmul-int8`, `m = 2048`,
`log_inv_rate = 1`, `RAYON_NUM_THREADS=1`, warmup 1, N = 5, sleep-checked (`slept = 0.000 s`).
Raw output: [`bench/data/repro-e001/`](../../data/repro-e001/).

Note this uses the **E-001 subject**, not an E-006 task: it is the unmodified circuit from
the earlier campaigns, proved with the unmodified `e001-harness` binary, precisely so that
nothing about the new task circuits can enter the comparison.

## 5. What did NOT reproduce, and is reported before any result

The field-multiply build probe reads **0.805× of its E-001 level** in absolute rate, for both
the library kernel and an independent hand-written one, while the machine's raw carryless
multiply is unchanged. E-005 saw the same at 0.76–0.80× and did not explain it; it persists.
Full numbers, and why the build-integrity gate still passes on its actual criterion, are in
[`BUILD.md`](BUILD.md) §2. **Cause not established.**
