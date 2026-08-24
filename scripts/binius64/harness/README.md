# binius64 measurement harness

**This is our code. No binius64 source is vendored here.** The harness is a separate Cargo
workspace that depends on a pinned binius64 clone through path dependencies and calls only
its public API — the same arrangement as `scripts/ceno/harness/` and
`scripts/jolt-atlas/harness/`.

binius64 is the only measured system whose harness we wrote ourselves, which made it the
only one a third party could not rebuild while the harness lived outside this repository.
It now lives here.

## Build

```sh
git clone https://github.com/binius-zk/binius64 /path/to/binius64
git -C /path/to/binius64 checkout <commit from ../../../systems/binius64/COMMIT>

BINIUS64_ROOT=/path/to/binius64 ./setup.sh
```

`setup.sh` substitutes `@BINIUS64_ROOT@` in `Cargo.toml.in`, writes `Cargo.toml`, builds
with `RUSTFLAGS=-C target-cpu=native`, and then runs the blocking build-integrity gate.
**`Cargo.toml` is generated and is not committed** — it would carry a machine-local
absolute path. Edit `Cargo.toml.in`.

Toolchain is pinned in `rust-toolchain.toml` (1.97.1). The release profile must keep
`lto = "thin"`; `systems/binius64/BUILD.md` §1 explains what happens when it does not.

## Binaries

| Binary | What it does |
|---|---|
| `e006-bench` | one measured cell (task × `log_inv_rate` × threads) |
| `e006-negative` | the corrupted-trace correctness control, blocking |
| `e006-verify-split` | diagnostic: decomposes verify time into its four terms |
| `e001-fieldmul-sanity` | the blocking build-integrity gate, `BUILD.md` §2 |

`e001-fieldmul-sanity` keeps its original name because the published raw output in
`data/probe-fieldmul-before.txt` and `data/probe-fieldmul-after.txt` was produced under it.
Its `e001-` prefix is history, not a claim that it belongs to a different experiment: it is
the gate E-006 runs seven times before any cell.

## What was left behind, and why

This harness was carved out of a larger workspace that also served our earlier experiment
E-001. Only the files E-006 actually compiles are here. Left out:

| Left out | Why |
|---|---|
| `src/main.rs` (`e001-harness`), `src/subject/`, `src/buckets.rs`, `src/layer.rs` | E-001's phase-split instrument and its `sha256` / `imul_chain` / `matmul-int8` subjects. No E-006 binary references them. |
| `src/bin/roofline.rs`, `src/roofline/` | E-001's roofline probe. Not part of any E-006 measurement. |
| `src/bin/intensity.rs` | E-001d's arithmetic-intensity counter. Requires the `e001-count` feature (below). |
| `e001-count` Cargo feature | It enables counters that exist **only** in a locally patched binius64 tree, not upstream. Declaring it here would make this manifest fail to resolve against a clean clone. No E-006 measurement enables it; `systems/binius64/COMMIT` says so explicitly. |
| `binius-iop`, `binius-ip-prover`, `binius-math` dependencies | Referenced only by the files left out. |

**One consequence, stated plainly.** `systems/binius64/REPRODUCTION.md` §4 reports a
continuity check against E-001 that is run with the `e001-harness` binary on the E-001
subject `matmul-int8`. That binary is not in this repository, so **§4 is the one figure in
the binius64 section a third party cannot re-run from this tree.** Everything E-006 itself
measured — every cell, the negative control, the verify split and the build gate — is
reproducible from here. §4 is a comparison against a prior experiment of ours, not an E-006
result, and reproducing it would require publishing E-001's instrument as well.
