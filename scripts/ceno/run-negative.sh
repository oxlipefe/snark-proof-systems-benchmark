#!/usr/bin/env bash
# zk-prover-bench · Ceno · the correctness control.
#
# bench/README.md: "A corrupted trace must make verify() fail, in every system, on every task."
#
# The vk is regenerated in process by `ceno_verify` rather than loaded from disk, and that is
# not a convenience: a serialized vk rejects EVERY proof at this commit (BUILD.md §5), so a
# sweep run against one would report every corruption as rejected for the wrong reason and pass
# while establishing nothing. The honest-proof positive control inside `ceno_verify` is what
# proves this sweep is live.
#
# STRIDE is declared, never inferred. jolt-atlas's sweep was exhaustive because its proof is
# 21 419 bytes; Ceno's T1-0 proof is 1 162 285 bytes — 54x larger — and each verification costs
# ~50 ms, so an exhaustive sweep would run ~16 hours per task. The coverage actually achieved is
# printed by the tool into the CSV as its own row.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
readonly HARNESS="${ROOT}/scripts/ceno/harness/target/release/ceno_verify"
readonly GUESTS="${CENO_ROOT}/examples/target/riscv32im-ceno-zkvm-elf/release/examples"
readonly TASKS="${ROOT}/tasks/ceno"
readonly DATA="${ROOT}/data/negative-ceno"
STRIDE="${STRIDE:-64}"

mkdir -p "${DATA}"

for task in "$@"; do
  case "${task}" in t1-*) elf=bench_t1 ;; *) elf=bench_mlp ;; esac
  # Find a proof this campaign already produced for the task, at the primary cut.
  proof="$(ls -t "${ROOT}"/data/cells-ceno/${task}-t10-s*/rep*.proof.bin 2>/dev/null | head -1)"
  if [[ -z "${proof}" ]]; then
    echo "[ceno] ${task}: no proof on disk — run the cell first" >&2
    echo "${task},none,-,PROOF_NOT_PRODUCED,PROOF_NOT_PRODUCED" > "${DATA}/${task}.csv"
    continue
  fi
  echo "[ceno] negative control: ${task} against ${proof} (stride ${STRIDE})" >&2
  caffeinate -dimsu "${HARNESS}" negative \
      "${GUESTS}/${elf}" \
      "${TASKS}/${task}.hints.bin" \
      "${TASKS}/${task}.public-io.txt" \
      "${proof}" \
      "${task}" \
      "${STRIDE}" \
    > "${DATA}/${task}.csv" 2>"${DATA}/${task}.stderr.txt"
  awk -F, 'NR>1 && $5!="INFO" {c[$5]++} END {for (k in c) printf "  %-22s %d\n", k, c[k]}' \
    "${DATA}/${task}.csv"
done
