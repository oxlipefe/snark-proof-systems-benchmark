#!/usr/bin/env bash
# zk-prover-bench · binius64, the whole campaign from zero.
#
# Order matters. The correctness control runs BEFORE the timings it licenses, and the build
# check runs before everything: a number produced by a broken build or by a system that
# accepts corrupt traces is worse than no number.
#
# Every step is wrapped in caffeinate and bracketed by the sleep detector; see BUILD.md.
set -uo pipefail
# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly HARNESS="${ROOT}/scripts/binius64/harness"
# The prover itself is never vendored here. Point this at a clone of the revision named in
# systems/binius64/COMMIT; setup.sh materialises the harness manifest against it.
export BINIUS64_ROOT="${BINIUS64_ROOT:?set BINIUS64_ROOT to a binius64 clone at the commit in systems/binius64/COMMIT}"
cd "${ROOT}"

echo "=== 0. build ==="
caffeinate -dimsu env RUSTFLAGS="-C target-cpu=native" \
  "${HARNESS}/setup.sh" || exit 1

echo "=== 1. build integrity (BLOCKING) ==="
for i in 1 2 3 4 5 6 7; do
  caffeinate -dimsu "${HARNESS}/target/release/e001-fieldmul-sanity" || {
    echo "BUILD INTEGRITY FAILED — no measurement may proceed"; exit 1; }
done

echo "=== 2. correctness control, every task (BLOCKING) ==="
./scripts/run-negative.sh t1-0 t2 t3 t1-a t1-b t1-c || exit 1

echo "=== 3. timing grid, 1 thread (primary cut) ==="
./scripts/run-cell.sh t1-0:1:1 t1-0:4:1 t2:1:1 t2:4:1 t3:1:1 t3:4:1 t1-a:1:1 t1-a:4:1
./scripts/run-cell.sh t1-b:1:1 t1-b:4:1
WARMUP=0 REPS=3 ./scripts/run-cell-guarded.sh t1-c:1:1 t1-c:4:1
WARMUP=0 REPS=1 ./scripts/run-cell-guarded.sh t1-d:1:1

echo "=== 4. timing grid, 10 threads (secondary cut, cheap cells only) ==="
./scripts/run-cell.sh t1-0:1:10 t1-0:4:10 t2:1:10 t2:4:10 t3:1:10 t3:4:10 \
                            t1-a:1:10 t1-a:4:10 t1-b:1:10 t1-b:4:10

echo "=== 5. report ==="
python3 ./scripts/report.py
