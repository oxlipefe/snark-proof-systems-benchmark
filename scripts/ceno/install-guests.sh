#!/usr/bin/env bash
# zk-prover-bench · Ceno · install the guest programs into the pinned Ceno clone.
#
# Ceno's documented guest build path is its own `examples` package: `cargo ceno build
# --example <name>` compiles `examples/examples/<name>.rs` against `ceno_rt` with the custom
# `riscv32im-ceno-zkvm-elf` target and the two linker scripts. We use that path unmodified
# rather than inventing a package layout of our own, so the guests are built exactly the way
# the authors build theirs.
#
# The consequence is that the guest sources have to be *copied into* the clone. The
# authoritative copies live in this repository, under bench/tasks/ceno/guest/, and this
# script is the only thing that puts them in the tree under test. Copying is declared in
# systems/ceno/COMMIT as a local modification, with this script named as its origin.
#
# CENO_ROOT must point at the pinned clone, which is deliberately outside this repository.
set -euo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
readonly SRC="${ROOT}/tasks/ceno/guest"
readonly DST="${CENO_ROOT}/examples/examples"

[[ -d "${DST}" ]] || { echo "not a Ceno clone: ${CENO_ROOT}" >&2; exit 1; }

for f in "${SRC}"/*.rs; do
  install -m 0644 "${f}" "${DST}/$(basename "${f}")"
  echo "[ceno] installed $(basename "${f}") -> ${DST}"
done

echo "[ceno] guests installed. Build one with:"
echo "  bench/scripts/ceno/build-guest.sh bench_t1"
