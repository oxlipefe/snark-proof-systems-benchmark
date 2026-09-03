#!/usr/bin/env bash
# zk-prover-bench · Plonky3 · MEASURE the absence of a binary-field multilinear PCS.
#
# Rule 6 of this project's method: reading the source says where to look, not what a system
# costs — and a claim of ABSENCE is the cheapest kind to assert and the most expensive to
# withdraw. So the absence is not claimed from a grep. This script asks the compiler to
# instantiate `WhirConfig` and `WhirProver` over `BinaryField128` and records its refusal.
#
# The build is EXPECTED TO FAIL. A success here would mean the claim in
# systems/plonky3/NOT_EXPRESSIBLE.md is wrong and must be withdrawn.
set -uo pipefail
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly HARNESS="${ROOT}/scripts/plonky3/harness"
readonly OUT="${ROOT}/data/probe-plonky3-whir-binary.txt"
export PLONKY3_ROOT="${PLONKY3_ROOT:?set PLONKY3_ROOT to a Plonky3 clone at the commit in systems/plonky3/COMMIT}"

sed "s|@PLONKY3_ROOT@|${PLONKY3_ROOT}|g" "${HARNESS}/Cargo.toml.in" > "${HARNESS}/Cargo.toml"
{
  echo "# Deliberate compile failure: p3-whir instantiated over BinaryField128."
  echo "# Produced by scripts/plonky3/run-probe-binary-pcs.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# Plonky3 clone: ${PLONKY3_ROOT} at $(git -C "${PLONKY3_ROOT}" rev-parse HEAD 2>/dev/null || echo UNKNOWN)"
  echo
  ( cd "${HARNESS}" && RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}" \
      cargo build --release --lib --features probe-binary-pcs 2>&1 )
  rc=$?
  echo
  echo "# cargo exit code: ${rc}"
  if [ "${rc}" -eq 0 ]; then
    echo "# ALERT: the probe COMPILED. systems/plonky3/NOT_EXPRESSIBLE.md is wrong."
  else
    echo "# As expected: the build was refused."
  fi
} > "${OUT}" 2>&1
echo "[plonky3] wrote ${OUT}" >&2
grep -E 'not satisfied|error\[' "${OUT}" | head -5 >&2
