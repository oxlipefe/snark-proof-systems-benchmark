# gnark — reproducing the authors' own published figure

`bench/README.md` §"Fairness protocol" and `bench/CHALLENGE.md` commit this repository to
reproducing each measured system's **own published reference number before reporting anything
about that system**, and to publishing the discrepancy *above* the result if we cannot.

**We reproduced their benchmark. We did not reproduce their number.** Their own tool, their
own circuit, their own accounting, run on this machine, gives **2.79–2.86·10⁵ constraints per
second** against a published **">2·10⁶"**. That is a gap of about **7.0×**. The largest
declared difference between the two setups — **96 vCPU against 10 cores** — runs in the same
direction. **We do not claim it explains the gap**, because we did not measure their machine
and this repository does not publish scaling arguments it has not measured.

---

## 1. The reference

There is exactly one performance figure published by the gnark authors that names a number,
and it is prose on a documentation page:

> **gnark is fast** […] over 2 million constraints per second

- Source: <https://docs.gnark.consensys.io/overview#gnark-is-fast>
- Scheme: Groth16 over BN254
- Circuit: described only as ~8 million constraints; **what it computes is not stated**
- Hardware: AWS `hpc6a` instance
- Date: November 2022
- **Not published: the commit, the thread count, the circuit source, the command.**

That is a marketing sentence, not an artifact. `bench/README.md` requires a conditions line
before a figure is citable, and this figure has none. **The absence is the first finding of
this file.**

### The one thing they do publish that is runnable

`Consensys/gnark-bench` is the authors' own command-line benchmarking tool.

- Repository: <https://github.com/Consensys/gnark-bench>
- Commit measured: `2dda93f047b12407412493c6d29df6492c71ace2`, dated **2022-08-27**
- **The repository is archived** (read-only, archived 2026-06-01)
- Its README publishes **no results table**. It is a tool, not a number.
- It pins `github.com/consensys/gnark v0.7.2-0.20220819150950-b308ebb1407d` and
  `gnark-crypto v0.8.0` — four years and nine minor versions behind the commit we measure.

Its default circuit, `expo`, is the whole of its `circuit` package:

```go
// benchCircuit is a simple circuit that checks X*X*X*X*X... == Y
func (circuit *benchCircuit) Define(api frontend.API) error {
	for i := 0; i < circuit.n; i++ {
		circuit.X = api.Mul(circuit.X, circuit.X)
	}
	api.AssertIsEqual(circuit.X, circuit.Y)
	return nil
}
```

`--size n` therefore yields exactly `n + 1` constraints. It is the cleanest possible
"N constraints → prove time" instrument, and it is the authors' own.

## 2. What we ran

Two builds, **one machine, one campaign**, so the comparison is internal and not across
papers.

| | `gnark-bench` as pinned | our port of `expo` |
|---|---|---|
| gnark | `v0.7.2-0.20220819150950` (2022) | **v0.16.2** (the commit under measurement) |
| gnark-crypto | v0.8.0 | v0.21.0 |
| built from | the archived tree, unmodified, `go build` | `~/Claude/ext-bench/expo-v0162`, circuit copied verbatim |
| timing | the tool's own `prover done … took=` | `time.Since` around `groth16.Prove` alone |

The port exists because `gnark-bench` cannot be built against v0.16.2 without editing it, and
editing a system's own benchmark to make it produce a number is exactly what
`bench/README.md` forbids. So we ran the tool unmodified at the version it pins, and
separately re-expressed **the same circuit** at the version we measure. Both are published.

### A confound we found rather than assumed

**`gnark-crypto` v0.8.0 has no arm64 assembly.** Its BN254 field directory contains
`element_ops_amd64.s`, `element_mul_adx_amd64.s` and `element_ops_noasm.go` — the last
carrying `//go:build !amd64`. There is no arm64 path. **So the 2022 build runs pure-Go field
arithmetic on this machine**, while the published figure was taken on amd64 hardware where
the assembly and the ADX path both apply.

That is not a defect in their tool; it is what measuring a 2022 artifact on 2026 hardware
means. It is stated here because it makes the v0.7.2 rows a **lower bound on that version's
performance**, and comparing them to the published figure without it would be dishonest.

## 3. The reproduction, and the finding

Apple M1 Max, 10 cores, 32 GiB, macOS 26.5.2, on battery, machine not dedicated. Every run
under `caffeinate -dimsu`. Prove time only; **setup is excluded and never amortised**.

| gnark | constraints | prove ms (median) | **constraints/s** | peak footprint | setup ms |
|---|---:|---:|---:|---:|---:|
| v0.7.2 (no arm64 asm) | 100 001 | 589.0 | **169 626** | 282 166 088 | — |
| v0.7.2 (no arm64 asm) | 1 000 001 | 5 350.0 | **186 893** | 2 427 652 376 | — |
| **v0.16.2** | 100 001 | 420.952 | **237 559** | 381 338 440 | 5 323.152 |
| **v0.16.2** | 1 000 001 | 3 491.231 | **286 432** | 2 450 082 096 | 52 724.826 |
| **v0.16.2** | 4 000 001 | 14 312.379 | **279 478** | 7 190 763 184 | 197 038.757 |

The v0.7.2 rows are the figures `gnark-bench` prints in **its own CSV, in its own
accounting**; we did not re-derive them. The v0.16.2 rows are ours, `N = 3` at the first two
sizes and `N = 2` at the third.

Raw output: [`bench/data/repro-gnark/`](../../data/repro-gnark/).

### Three things the table establishes

1. **Four years of gnark bought 1.40–1.53× on this machine.** 169 626 → 237 559 at 100 k
   constraints, 186 893 → 286 432 at 1 M. Part of that is the arm64 assembly that v0.8.0 did
   not have; we did not separate the two causes and do not attribute the factor to either.
2. **Throughput is flat from 1 M to 4 M constraints** — 2.86·10⁵ → 2.79·10⁵. So within the
   range we measured, the gap to the published figure is **not** an artifact of our circuit
   being smaller than theirs.
3. **We did not reach their circuit size.** Their figure names ~8 M constraints; the largest
   we proved is **4 000 001**. Setup at that size took 197 s and 7.19 GB, and
   `bench/CHALLENGE.md` forbids extrapolating outside the measured range. **Nothing here says
   what gnark does at 8 M constraints on this machine.**

### The differences we can name, none of which we resolve

| | theirs | ours |
|---|---|---|
| cores | 96 vCPU (`hpc6a`, AMD EPYC Milan) | 10 (Apple M1 Max) |
| ISA / field asm | amd64 with ADX | arm64; element ops in asm, **vector ops not** |
| circuit size | ~8 000 000 constraints | 4 000 001 (largest reached) |
| gnark version | 2022 | v0.16.2 (2026) |
| machine state | not stated | not dedicated, on battery, load average 12–30 |

**The core-count ratio is 9.6× and the throughput gap is 7.0×.** Those two numbers sitting
next to each other is suggestive and it is not evidence. We did not run on `hpc6a`, we did not
measure gnark's parallel scaling, and this repository does not publish inferences of that kind.

## 4. Verdict

**PARTIALLY REPRODUCED.**

- **Their instrument reproduces.** `gnark-bench` builds and runs unmodified at the commit it
  pins, and its circuit re-expresses cleanly at the commit we measure. There is no ambiguity
  about what is being counted: `--size n` gives `n+1` constraints and the tool times the
  prover alone.
- **Their number does not reproduce on this machine**, by about **7.0×**, at a circuit half
  the size of theirs, with the differences above declared.
- **The figure was not citable to begin with.** It carries no commit, no thread count, no
  circuit source and no command. Under `bench/README.md`'s own conditions-line rule we would
  not have published it, and we would not accept it from ourselves.

**What this licenses.** It licenses reporting gnark's numbers in this benchmark with the
normal caveats, because the instrument is sound and the machine is the same one every other
system in this repository ran on. **It does not license any sentence of the form "gnark
achieves 2M constraints/s"**, from us or from a reader of this repository, on this hardware.

## 5. Right of reply

To the gnark authors, per [`CHALLENGE.md`](../../CHALLENGE.md):

1. **What is the ~8 M-constraint circuit** behind the ">2 million constraints per second"
   figure? If it is `expo` at `--size 8000000`, say so and we will run exactly that.
2. **Which commit and how many threads** produced it? An `hpc6a.48xlarge` has 96 physical
   cores; was the run at 96, or at some other count?
3. **Is `gnark-bench` still the tool you would use?** It is archived and pins a 2022 gnark.
   If there is a current benchmark you would rather be measured by, we will use it instead.
4. **Is our port of `expo` to v0.16.2 faithful?** It is a verbatim copy of your `Define`
   body; if the surrounding accounting differs from yours in a way that matters, tell us.
5. **Is there an arm64 configuration we are missing?** We found element-level assembly for
   BN254 but no `vector_arm64.go`, so `Vector.Add`, `Sum`, `InnerProduct` and friends take the
   generic Go path on this machine while amd64 gets AVX-512 IFMA. If that is not the intended
   arm64 configuration, we will re-run.

If any of this is wrong, we will re-run all of it and publish both outcomes, with credit and
date. The old numbers stay in the record next to the new ones; we do not quietly replace them.
