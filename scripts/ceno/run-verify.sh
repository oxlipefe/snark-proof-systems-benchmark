#!/usr/bin/env bash
# zk-prover-bench · Ceno · keygen time and standalone verify time, per task.
#
# Both figures have to be taken outside the measured prove process: `e2e` verifies inline and
# reports no separate number, and its keygen is folded into the same run. Neither is ever
# amortised into prove time — bench/README.md reports setup separately, always.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
readonly HARNESS="${ROOT}/scripts/ceno/harness/target/release/ceno_verify"
readonly GUESTS="${CENO_ROOT}/examples/target/riscv32im-ceno-zkvm-elf/release/examples"
readonly TASKS="${ROOT}/tasks/ceno"
readonly DATA="${ROOT}/data/verify-ceno"
readonly LEDGER="${DATA}/verify.csv"

mkdir -p "${DATA}"
[[ -f "${LEDGER}" ]] || echo "task,proof_bytes,shards,keygen_s,verifier_new_s,proof_deserialize_s,verify_s,verdict" > "${LEDGER}"

for task in "$@"; do
  case "${task}" in t1-*) elf=bench_t1 ;; *) elf=bench_mlp ;; esac
  proof="$(ls -t "${ROOT}"/data/cells-ceno/${task}-t10-s*/rep*.proof.bin 2>/dev/null | head -1)"
  [[ -n "${proof}" ]] || { echo "[ceno] ${task}: no proof on disk" >&2; continue; }
  out="${DATA}/${task}.txt"
  caffeinate -dimsu "${HARNESS}" time \
      "${GUESTS}/${elf}" "${TASKS}/${task}.hints.bin" \
      "${TASKS}/${task}.public-io.txt" "${proof}" "${task}" > "${out}" 2>&1
  g() { grep "^$1," "${out}" | cut -d, -f2; }
  echo "${task},$(g proof_bytes),$(g shards),$(g keygen_s),$(g verifier_new_s),$(g proof_deserialize_s),$(g verify_s),$(g verdict)" >> "${LEDGER}"
  echo "[ceno] ${task}: keygen=$(g keygen_s)s verify=$(g verify_s)s verdict=$(g verdict)" >&2
done
