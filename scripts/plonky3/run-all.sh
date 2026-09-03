#!/usr/bin/env bash
# zk-prover-bench · Plonky3 · the whole campaign from zero.
#
# Order matters. The build-integrity gate runs before everything and the correctness control
# runs before the timings it licenses: a number produced by a broken build, or by a system that
# accepts corrupt proofs, is worse than no number.
#
# THIS SCRIPT RUNS A FULL CAMPAIGN AND TAKES THE MACHINE. For a smoke check use
#   REPS=1 WARMUP=1 ./run-cell.sh t1-0:koala-bear:sumcheck:1
# which the ledger labels SMOKE.
set -uo pipefail
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly HERE="${ROOT}/scripts/plonky3"
export PLONKY3_ROOT="${PLONKY3_ROOT:?set PLONKY3_ROOT to a Plonky3 clone at the commit in systems/plonky3/COMMIT}"
cd "${ROOT}"

echo "=== 0. build + build integrity (BLOCKING) ==="
"${HERE}/build.sh" || exit 1
for i in 1 2 3; do
  caffeinate -dimsu "${HERE}/harness/target/release/p3-fieldmul-sanity" || {
    echo "BUILD INTEGRITY FAILED — no measurement may proceed"; exit 1; }
done

echo "=== 1. the absence probe (records, never blocks) ==="
"${HERE}/run-probe-binary-pcs.sh" || true
# The probe rewrote Cargo.toml identically but left the feature OFF in the manifest; rebuild
# the measured binaries so nothing timed can carry the probe's feature.
"${HERE}/build.sh" || exit 1

echo "=== 2. correctness control, every task (BLOCKING) ==="
"${HERE}/run-negative.sh" t1-0 t1-a || exit 1

echo "=== 3. the cross-field cut: same task, same machine, same codebase, two fields ==="
"${HERE}/run-cell.sh" t1-0:koala-bear:sumcheck:1 t1-0:binary128:sumcheck:1
"${HERE}/run-cell.sh" t1-a:koala-bear:sumcheck:1 t1-a:binary128:sumcheck:1
"${HERE}/run-cell.sh" t1-0:koala-bear:sumcheck:10 t1-0:binary128:sumcheck:10
"${HERE}/run-cell.sh" t1-a:koala-bear:sumcheck:10 t1-a:binary128:sumcheck:10

echo "=== 4. the committed route — prime field only, and that is the result ==="
"${HERE}/run-cell.sh" t1-0:koala-bear:sumcheck-whir:1 t1-a:koala-bear:sumcheck-whir:1
"${HERE}/run-cell.sh" t1-0:koala-bear:sumcheck-whir:10 t1-a:koala-bear:sumcheck-whir:10

echo "=== 5. report ==="
python3 "${HERE}/report.py"
