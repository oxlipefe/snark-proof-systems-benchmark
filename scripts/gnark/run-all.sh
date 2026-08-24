#!/usr/bin/env bash
# zk-prover-bench · gnark · the whole campaign from zero, or a supplied cell list.
#
# ORDER MATTERS. The build check runs before everything and the correctness control runs
# before the timings it licenses: a number produced by a broken build, or by a system that
# accepts corrupt proofs, is worse than no number.
#
# Usage:
#   run-all.sh                       the full campaign in the order below
#   run-all.sh t1-0:groth16:A t2:plonk:B   just those cells, through run-cell.sh
#
# THE HEADLINE IS REGIME A. Regime B is a declared lever and is run in its own pass, written
# to its own labels, and must never be averaged or plotted with regime A. See
# bench/tasks/gnark/spec.go.
set -uo pipefail
# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly S="${ROOT}/scripts/gnark"

if [[ $# -gt 0 ]]; then
  exec "${S}/run-cell.sh" "$@"
fi

echo "=== 0. build + integrity + gadget correctness + gnark's own examples (BLOCKING) ==="
"${S}/build.sh" || exit 1

echo "=== 1. the traps, before any figure is attributed to gnark ==="
# Minimum output-layer width runs FIRST. DeepProve cannot prove a dense layer below 4
# outputs and jolt-atlas cannot below 2; T2 ends in a 64->1 layer, so if gnark had the same
# floor the whole T2/T3 half of the bank would be NOT_EXPRESSIBLE and no timing would matter.
"${ROOT}/tasks/gnark/bin/gnark-probe" minwidth   | tee "${ROOT}/data/probes-gnark-minwidth.txt"
"${ROOT}/tasks/gnark/bin/gnark-probe" padding    | tee "${ROOT}/data/probes-gnark-padding.txt"
"${ROOT}/tasks/gnark/bin/gnark-probe" rangecheck | tee "${ROOT}/data/probes-gnark-rangecheck.txt"
"${ROOT}/tasks/gnark/bin/gnark-probe" relu       | tee "${ROOT}/data/probes-gnark-relu.txt"
"${ROOT}/tasks/gnark/bin/gnark-probe" relubits   | tee "${ROOT}/data/probes-gnark-relubits.txt"
"${ROOT}/tasks/gnark/bin/gnark-probe" maccost    | tee "${ROOT}/data/probes-gnark-maccost.txt"
"${ROOT}/tasks/gnark/bin/gnark-probe" example    | tee "${ROOT}/data/probes-gnark-example.txt"

echo "=== 2. constraint counts for the whole grid, from compilation alone ==="
"${S}/run-compile-grid.sh"

echo "=== 3. correctness control, every task that will be timed (BLOCKING) ==="
GNARK_BACKEND=groth16 GNARK_REGIME=A "${S}/run-negative.sh" t1-0 t2 || exit 1
GNARK_BACKEND=plonk   GNARK_REGIME=A "${S}/run-negative.sh" t1-0    || exit 1
GNARK_BACKEND=groth16 GNARK_REGIME=B "${S}/run-negative.sh" t1-0 t2 || exit 1

echo "=== 4. timing grid, regime A, Groth16 — THE HEADLINE ==="
GNARK_BACKEND=groth16 GNARK_REGIME=A "${S}/run-cell.sh" t1-0 t2
GNARK_BACKEND=groth16 GNARK_REGIME=A GNARK_REPS=3 "${S}/run-cell.sh" t1-a t3
GNARK_BACKEND=groth16 GNARK_REGIME=A GNARK_REPS=1 GNARK_WARMUP=0 "${S}/run-cell-guarded.sh" t1-b
GNARK_BACKEND=groth16 GNARK_REGIME=A GNARK_REPS=1 GNARK_WARMUP=0 "${S}/run-cell-guarded.sh" t1-c
GNARK_BACKEND=groth16 GNARK_REGIME=A GNARK_REPS=1 GNARK_WARMUP=0 "${S}/run-cell-guarded.sh" t1-d

echo "=== 5. timing grid, regime A, PLONK ==="
GNARK_BACKEND=plonk GNARK_REGIME=A "${S}/run-cell.sh" t1-0 t2
GNARK_BACKEND=plonk GNARK_REGIME=A GNARK_REPS=3 "${S}/run-cell.sh" t1-a t3
GNARK_BACKEND=plonk GNARK_REGIME=A GNARK_REPS=1 GNARK_WARMUP=0 "${S}/run-cell-guarded.sh" t1-b

echo "=== 6. timing grid, regime B — A DECLARED LEVER, never mixed into a cross-system number ==="
GNARK_BACKEND=groth16 GNARK_REGIME=B "${S}/run-cell.sh" t1-0 t2 t1-a t3 t1-b t1-c t1-d
GNARK_BACKEND=plonk   GNARK_REGIME=B "${S}/run-cell.sh" t1-0 t2 t1-a t3

echo "=== 7. threads (secondary cut) ==="
for th in 1 2 4; do
  GOMAXPROCS="${th}" GNARK_BACKEND=groth16 GNARK_REGIME=A "${S}/run-cell.sh" t1-0 t2
done

echo "=== 8. the memory-knob question — data, not a verdict ==="
TASK=t1-0 "${S}/run-memory-knob.sh"

echo "=== 9. report ==="
python3 "${S}/report.py"
