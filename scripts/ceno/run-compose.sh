#!/usr/bin/env bash
# zk-prover-bench · Ceno · G-11d, the shard-composition control.
#
# The question this answers: does the verifier BIND the composition of a sharded proof, or does
# it check N unconnected claims and accept any multiset of individually valid shards?
#
# RESULTS.md §5.3 measured proof bytes and verify time as exactly linear in shard count with no
# sign of aggregation. Linearity alone does not distinguish "a dial over the same property"
# from "a discount on the statement". Only this control does.
#
# Mutations: M0 (none) · M1 drop · M2 duplicate · M3 swap · M4 graft from another proof ·
# M5 truncate. M3 and M4 preserve the shard count, so no length check can catch them; they are
# the load-bearing cases.
#
# The vk is regenerated in process by `ceno_verify`, never loaded from disk: a serialized vk
# rejects EVERY proof at this commit (systems/ceno/BUILD.md §5), so a sweep run against one
# would report every mutation as rejected for the wrong reason. Same reasoning as
# run-negative.sh. `M0_LAYOUT` and `M0_ORIGINAL` are the instrument checks; if either fails the
# tool aborts and the sweep is reported as dead rather than as a wall of rejections.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
readonly HARNESS="${ROOT}/scripts/ceno/harness/target/release/ceno_verify"
readonly GUESTS="${CENO_ROOT}/examples/target/riscv32im-ceno-zkvm-elf/release/examples"
readonly TASKS="${ROOT}/tasks/ceno"
readonly DATA="${ROOT}/data/compose-ceno"
readonly LEDGER="${DATA}/compose.csv"

# DONOR is the task whose shards are grafted in by M4. It must share the ELF and the shard cap
# with the subject, otherwise a rejection would only prove the shapes differ.
DONOR_TASK="${DONOR_TASK:-t1-c}"

mkdir -p "${DATA}"
[[ -f "${LEDGER}" ]] || echo "task,mutation,k,shards_in,shards_out,verdict,detail,elapsed_s" > "${LEDGER}"

proof_for() {
  ls -t "${ROOT}"/data/cells-ceno/"$1"-t10-s*/rep*.proof.bin 2>/dev/null | head -1
}

for task in "$@"; do
  case "${task}" in t1-*) elf=bench_t1 ;; *) elf=bench_mlp ;; esac

  proof="$(proof_for "${task}")"
  if [[ -z "${proof}" ]]; then
    echo "[ceno] ${task}: no proof on disk — run the cell first" >&2
    continue
  fi

  # A donor that is the subject itself would make M4_GRAFT a no-op, so refuse it.
  donor=""
  if [[ "${DONOR_TASK}" != "${task}" ]]; then
    donor="$(proof_for "${DONOR_TASK}")"
    [[ -n "${donor}" ]] || echo "[ceno] ${task}: donor ${DONOR_TASK} has no proof — M4_GRAFT skipped" >&2
  else
    echo "[ceno] ${task}: donor task equals subject — M4_GRAFT skipped" >&2
  fi

  echo "[ceno] compose control: ${task} against ${proof} (donor ${DONOR_TASK}: ${donor:-none})" >&2
  caffeinate -dimsu "${HARNESS}" compose \
      "${GUESTS}/${elf}" \
      "${TASKS}/${task}.hints.bin" \
      "${TASKS}/${task}.public-io.txt" \
      "${proof}" \
      "${task}" \
      "${donor}" \
    > "${DATA}/${task}.csv" 2>"${DATA}/${task}.stderr.txt"

  tail -n +2 "${DATA}/${task}.csv" >> "${LEDGER}"
  awk -F, 'NR>1 {printf "  %-12s k=%-4s %s\n", $2, $3, $6}' "${DATA}/${task}.csv"
done
