#!/usr/bin/env bash
# Materialise the harness manifest against a local Plonky3 clone, then build it.
#
# The harness is OUR code and contains no Plonky3 source. It depends on the pinned clone by
# path, so the path has to be supplied:
#
#   PLONKY3_ROOT=/path/to/Plonky3 ./setup.sh
#
# Clone the revision named in ../../../systems/plonky3/COMMIT:
#
#   git clone https://github.com/Plonky3/Plonky3 /path/to/Plonky3
#   git -C /path/to/Plonky3 checkout <commit from systems/plonky3/COMMIT>
#
# RUSTFLAGS matters TWICE here, and both failures are silent:
#
#   * `lto = "thin"` in Cargo.toml.in. Plonky3's own `optimized` profile sets it; the harness
#     is a separate workspace and inherits nothing. Without it the field multiply is measured
#     across a crate boundary with cross-crate inlining off.
#   * `-C target-cpu=native`. p3-binary-field's carryless-multiply backend is compiled only
#     under `target_feature = "aes"`; without it every GF(2^128) multiply is a bit-serial loop.
#
# The gate in step 2 refuses a build with either problem.
set -euo pipefail
readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PLONKY3_ROOT="${PLONKY3_ROOT:?set PLONKY3_ROOT to a Plonky3 clone at the commit in systems/plonky3/COMMIT}"

[ -d "${PLONKY3_ROOT}/sumcheck" ] && [ -d "${PLONKY3_ROOT}/binary-field" ] || {
  echo "[plonky3] ${PLONKY3_ROOT} does not look like a Plonky3 clone (no sumcheck/ or binary-field/)" >&2
  exit 1
}

sed "s|@PLONKY3_ROOT@|${PLONKY3_ROOT}|g" "${HERE}/Cargo.toml.in" > "${HERE}/Cargo.toml"
echo "[plonky3] wrote ${HERE}/Cargo.toml pinned to ${PLONKY3_ROOT}"

cd "${HERE}"
echo "[plonky3] === 1. build ==="
RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}" cargo build --release --bins

echo "[plonky3] === 2. build integrity (BLOCKING) ==="
./target/release/p3-fieldmul-sanity

echo "[plonky3] built: $(pwd)/target/release/{p3-bench,p3-negative}"
