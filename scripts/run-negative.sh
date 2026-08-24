#!/usr/bin/env bash
# zk-prover-bench · the blocking correctness control, per task.
#
# A corrupted trace must make verify() fail. If any task fails this, no number from binius64
# is published. Run before, and independently of, any timing.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly BIN="${ROOT}/scripts/binius64/harness/target/release/e006-negative"
readonly DATA="${ROOT}/data"
readonly RATE="${RATE:-1}"

mkdir -p "${DATA}/negative"
for task in "$@"; do
  out_dir="${DATA}/negative/${task}"
  mkdir -p "${out_dir}"
  caffeinate -dimsu "${BIN}" --task "${task}" --log-inv-rate "${RATE}" --out-dir "${out_dir}" \
    > "${out_dir}/report.txt" 2> "${out_dir}/stderr.txt"
  echo "[negative] ${task} rc=$? -> ${out_dir}/report.txt" >&2
done

# One combined, uncurated table.
head -1 "${DATA}/negative"/*/negative-control.csv 2>/dev/null | grep -m1 '^task,' > "${DATA}/negative-control.csv" || true
cat "${DATA}/negative"/*/negative-control.csv 2>/dev/null | grep -v '^task,' >> "${DATA}/negative-control.csv"
echo "[negative] combined -> ${DATA}/negative-control.csv" >&2
