#!/usr/bin/env bash
# Materialise the harness manifest against a local binius64 clone, then build it.
#
# The harness is OUR code and contains no binius64 source. It depends on the pinned clone
# by path, so the path has to be supplied:
#
#   BINIUS64_ROOT=/path/to/binius64 ./setup.sh
#
# Clone the revision named in ../../../systems/binius64/COMMIT:
#
#   git clone https://github.com/binius-zk/binius64 /path/to/binius64
#   git -C /path/to/binius64 checkout <commit from systems/binius64/COMMIT>
#
# RUSTFLAGS matters. systems/binius64/BUILD.md §1 declares `-C target-cpu=native` for every
# measured build, and the release profile in Cargo.toml.in must not be overridden: a build
# without `lto = "thin"` measures a prover ~28x slower in its field multiply. The gate in
# step 2 below refuses such a build.
set -euo pipefail
readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly BINIUS64_ROOT="${BINIUS64_ROOT:?set BINIUS64_ROOT to a binius64 clone at the commit in systems/binius64/COMMIT}"

[ -d "${BINIUS64_ROOT}/crates/core" ] || {
  echo "[binius64] ${BINIUS64_ROOT} does not look like a binius64 clone (no crates/core)" >&2
  exit 1
}

sed "s|@BINIUS64_ROOT@|${BINIUS64_ROOT}|g" "${HERE}/Cargo.toml.in" > "${HERE}/Cargo.toml"
echo "[binius64] wrote ${HERE}/Cargo.toml pinned to ${BINIUS64_ROOT}"

cd "${HERE}"
echo "[binius64] === 1. build ==="
RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}" cargo build --release --bins

echo "[binius64] === 2. build integrity (BLOCKING) ==="
./target/release/e001-fieldmul-sanity

echo "[binius64] built: $(pwd)/target/release/{e006-bench,e006-negative,e006-verify-split}"
