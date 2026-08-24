# Cells that did not produce a figure, and why

`experiments/E-006-prover-bench/PROTOCOL.md` control #8: report the complete grid, including
the cells that did not run and why — no silent caps.

## T1-d · Groth16 · regime A — KILLED BY OUR OWN DISK WATCHDOG

    [watchdog] free space on / is 19 GiB, below the 20 GiB floor — killing the cell
    run-cell-guarded.sh: line 36: 29921 Terminated: 15   run-cell.sh t1-d

Killed after roughly 17 minutes, during setup. No ledger row was written, so the ledger shows
6 rows for phase 4 rather than 7. **This file is that seventh row.**

### The causal chain, measured rather than assumed

| observation | value |
|---|---|
| constraints (from compilation alone) | 39 175 652 |
| FFT domain the key would need | 67 108 864 (2²⁶) |
| process RSS when killed | 453 MB — still early, in setup |
| process CPU when killed | 874 % |
| **system swap before the cell** | total 9 216 MB, used 8 242 MB |
| **system swap during the cell** | **total 32 768 MB, used 31 592 MB** |
| free space on `/` | fell to 19 GiB, tripping the 20 GiB floor |
| swap after the cell died | total 20 480 MB, used 13 109 MB |

macOS grows its swap file on demand. The cell's memory demand grew swap from 9 GB to **32 GB**,
the swap file consumed the boot volume, and **our own disk guard killed the cell to protect the
machine.** The proximate cause of death is the watchdog; the root cause is that the working set
did not fit in 32 GiB of RAM on a volume that was already 95 % full.

### WHAT THIS DOES NOT ESTABLISH

**It is not "gnark cannot prove T1-d."** It is "this machine could not, in this state." The
distinction is the one the jolt-atlas campaign was built to protect: a limit hit by our
harness on our hardware is not a property of someone else's system. gnark **compiled** T1-d
regime A without difficulty (89.13 s, 39 175 652 constraints — `compile-grid-gnark.csv`); what
did not fit is Groth16 setup and proving at a 2²⁶ domain.

**Nothing is interpolated.** Per `bench/CHALLENGE.md` the ceiling is published as a measured
interval and no value inside it is claimed:

> **Largest Groth16 regime-A cell that produced a proof: T1-c — 10 679 708 constraints, 2²⁴
> domain, peak footprint 18.87 GB.**
> **Smallest that did not: T1-d — 39 175 652 constraints, 2²⁶ domain.**

Nothing in this repository says where between those two the boundary lies, or what gnark would
do on a machine with more RAM or a less full disk.

### A caveat that also touches T1-c, the largest cell that DID succeed

T1-c reports **peak RSS 6.96 GB against peak footprint 18.87 GB — a 2.71× divergence**, by far
the largest in this campaign; every smaller cell has the two within 1 %. That divergence is
memory pressure: the process's pages were being compressed and paged while it ran. So **T1-c's
wall-clock figures were taken on a machine already under paging pressure** and are not gnark's
best achievable performance at that size. The figure is published as measured, with this
sentence attached.

## Cells not attempted, and why

- **T1-d, PLONK, regime A** (79 332 096 constraints, 2²⁷ domain). Not attempted after the
  Groth16 regime-A cell at half that constraint count was killed. Declared, not estimated.
- **GOGC = 25 on T1-a** (memory-knob sweep). Launched and killed mid-run when the machine
  dropped to 15 % battery before AC was restored; a timed run interrupted by a sleep is
  contamination, not data, so nothing was banked. The other points of that sweep stand.
- **GPU / ICICLE.** Excluded by declaration: round one of this benchmark is CPU-only, and
  `WithIcicleAcceleration` is deprecated in its own docstring.
