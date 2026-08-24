#!/usr/bin/env bash
# zk-prover-bench · Ceno · build the guest ELFs.
#
# This is the repository's own guest build path, not a reconstruction of ours. The `examples`
# package carries a committed `.cargo/config.toml` that pins the custom target
# (`../ceno_rt/riscv32im-ceno-zkvm-elf.json`), the `build-std` set, `panic-immediate-abort`
# and the three linker/codegen rustflags; `examples-builder/build.rs` then invokes exactly
# `cargo build --release --examples --target-dir target` inside `examples/` and
# `include_bytes!`s the result. We run the same command, so the ELF we measure is the ELF
# Ceno's own build system produces.
#
# Two things this deliberately does NOT do, and why:
#
#   1. It does not export RUSTFLAGS. An environment RUSTFLAGS *overrides* `[build] rustflags`
#      in `.cargo/config.toml` rather than adding to it, so exporting the CLI's flag list
#      would silently drop the config's flags. We hit that: the first attempt built the ELFs
#      into the workspace target directory with a different flag set.
#   2. It does not use `cargo ceno build`. `cargo-ceno` cannot be built on this machine
#      (BUILD.md §2), and its flag list differs slightly from the config's — it adds
#      `-Zunstable-options` and `-Cllvm-args=--basic-block-address-map` and passes
#      `-C panic=immediate-abort` directly instead of via the profile. That difference is
#      declared in BUILD.md §3 rather than papered over.
#
# Builds every example in the package, because `examples-builder` requires all of them to
# exist before the host binary will link.
set -euo pipefail

readonly CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
readonly OUT="${CENO_ROOT}/examples/target/riscv32im-ceno-zkvm-elf/release/examples"

cd "${CENO_ROOT}/examples"
caffeinate -dimsu cargo build --release --examples --target-dir target

for g in bench_t1 bench_mlp bench_commit_probe; do
  [[ -f "${OUT}/${g}" ]] || { echo "expected ELF not produced: ${OUT}/${g}" >&2; exit 1; }
  echo "[ceno] ${g} -> ${OUT}/${g} ($(stat -f%z "${OUT}/${g}") bytes)"
done
