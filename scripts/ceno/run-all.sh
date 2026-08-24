#!/usr/bin/env bash
# zk-prover-bench · Ceno · the whole campaign, in order.
#
# Order matters, and it is the same order the other three systems use. The build check runs
# before everything and the correctness control runs before the timings it licenses: a number
# produced by a broken build, or by a system that accepts corrupt proofs, is worse than no
# number.
#
# Ceno-specific ordering notes:
#
#   * The 1-thread cut that binius64 uses as its primary is NOT run, because the prover aborts
#     there on Ceno's own examples (NOT_EXPRESSIBLE.md §1). One 1-thread cell is run anyway, so
#     the failure appears in the published grid instead of as an absence.
#   * The ladder is run in increasing size, so that a campaign cut short by battery or by disk
#     still yields a curve rather than a scatter of the cheap cells.
#   * The shard sweep is run last on an already-proved rung, because it is the finding that
#     needs the rest of the grid to be interpretable.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
readonly HARNESS="${ROOT}/scripts/ceno/harness/target/release"
readonly GUESTS="${CENO_ROOT}/examples/target/riscv32im-ceno-zkvm-elf/release/examples"
readonly TASKS="${ROOT}/tasks/ceno"
readonly DATA="${ROOT}/data"
readonly MIN_FREE_GB="${MIN_FREE_GB:-20}"

free_gb() { df -g /System/Volumes/Data | awk 'NR==2 {print $4}'; }

echo "=== 0. machine state, declared ==="
mkdir -p "${DATA}/repro-ceno"
{
  date -u +%Y-%m-%dT%H:%M:%SZ
  sysctl -n machdep.cpu.brand_string
  sw_vers; uname -a; uptime
  sysctl -n vm.swapusage
  df -g /System/Volumes/Data
  pmset -g ps 2>/dev/null | head -3
} > "${DATA}/repro-ceno/machine-state-start.txt"
cat "${DATA}/repro-ceno/machine-state-start.txt"

echo "=== 1. build integrity (BLOCKING): the authors' own examples must prove AND verify ==="
for ex in fibonacci ceno_rt_alloc ceno_rt_mem ceno_rt_io; do
  extra=()
  [[ "${ex}" == "fibonacci" ]] && extra=(--hints 10 --public-io 4191)
  if ! caffeinate -dimsu "${CENO_ROOT}/target/release/e2e" "${GUESTS}/${ex}" \
        /tmp/bi.proof.bin /tmp/bi.vk.bin --platform=ceno --profiling 1 \
        "${extra[@]:-}" > "/tmp/bi-${ex}.out" 2>&1; then
    echo "BUILD INTEGRITY FAILED on ${ex} — no measurement may proceed"; exit 1
  fi
  echo "  ${ex}: OK"
done

echo "=== 2. cycle counts for EVERY task, including ones too large to prove ==="
mkdir -p "${DATA}/cycles-ceno"
for t in t1-0 t1-a t1-b t1-c t1-d t2 t3; do
  case "${t}" in t1-*) e=bench_t1 ;; *) e=bench_mlp ;; esac
  caffeinate -dimsu "${HARNESS}/ceno_cycles" "${GUESTS}/${e}" "${TASKS}/${t}.hints.bin" \
    > "${DATA}/cycles-ceno/${t}.txt"
  echo "  ${t}: $(grep '^cycles,' "${DATA}/cycles-ceno/${t}.txt")"
done

echo "=== 3. correctness control (BLOCKING for the cells it licenses) ==="
"${ROOT}/scripts/ceno/run-negative.sh" t1-0 t2 || exit 1

echo "=== 4. the 1-thread cell, run so its failure is published rather than absent ==="
REPS=1 WARMUP=0 "${ROOT}/scripts/ceno/run-cell.sh" t1-0:1

echo "=== 5. the ladder, increasing size, primary cut = 10 RAYON threads ==="
REPS=5 WARMUP=1 "${ROOT}/scripts/ceno/run-cell.sh" t1-0:10 t2:10
REPS=3 WARMUP=1 "${ROOT}/scripts/ceno/run-cell.sh" t1-a:10 t3:10
REPS=2 WARMUP=0 "${ROOT}/scripts/ceno/run-cell.sh" t1-b:10

echo "=== 6. secondary thread cut ==="
REPS=3 WARMUP=1 "${ROOT}/scripts/ceno/run-cell.sh" t1-0:2 t2:2

echo "=== 7. the top of the ladder, gated on free disk ==="
# Peak footprint above RAM is swap-backed on the boot volume. A rung large enough to exhaust it
# does not just fail the cell, it destabilises the machine.
if [[ "$(free_gb)" -lt "${MIN_FREE_GB}" ]]; then
  echo "  SKIPPED t1-c: only $(free_gb) GB free, below MIN_FREE_GB=${MIN_FREE_GB}"
else
  REPS=1 WARMUP=0 "${ROOT}/scripts/ceno/run-cell.sh" t1-c:10
fi

echo "=== 8. the shard sweep — peak memory as a function of a flag, on a fixed task ==="
for cap in 536870912 33554432 8388608 2097152; do
  MAX_CYCLE_PER_SHARD="${cap}" REPS=1 WARMUP=0 \
    "${ROOT}/scripts/ceno/run-cell.sh" t1-a:10
done

echo "=== 9. keygen and standalone verify, per proved task ==="
"${ROOT}/scripts/ceno/run-verify.sh" t1-0 t2 t1-a t3

echo "=== 10. reparse and report ==="
python3 "${ROOT}/scripts/ceno/reparse.py"
python3 "${ROOT}/scripts/ceno/report.py"
