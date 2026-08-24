#!/usr/bin/env bash
# zk-prover-bench · gnark · the blocking correctness control, per task.
#
# bench/README.md: "A corrupted trace must make verify() fail, in every system, on every
# task." If a task fails this, no number from gnark is published for it. Runs BEFORE, and
# independently of, any timing.
#
# TWO POSITIVE CONTROLS RUN FIRST inside the tool — the honest proof verifies, and an
# unmodified serialize→deserialize→verify round trip still verifies. If either fails the
# tool prints "ROUND TRIP FAILED — every other result in this file is meaningless" and exits
# 9, and this script propagates that: a control that passes because nothing ever verifies is
# worse than no control.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly BIN="${BIN:-${ROOT}/tasks/gnark/bin/gnark-negative}"
readonly DATA="${ROOT}/data/negative-gnark"
readonly BACKEND="${GNARK_BACKEND:-groth16}"
readonly REGIME="${GNARK_REGIME:-A}"
readonly SAMPLES="${GNARK_NEG_WITNESS_SAMPLES:-8}"

mkdir -p "${DATA}"
rc_all=0
for task in "$@"; do
  cache="${DATA}/cache/${task}-${BACKEND}-r${REGIME}"
  rm -rf "${cache}"; mkdir -p "${cache}"
  csv="${DATA}/${task}-${BACKEND}-r${REGIME}.csv"

  GNARK_BACKEND="${BACKEND}" GNARK_REGIME="${REGIME}" \
  GNARK_NEG_CACHE="${cache}" GNARK_NEG_WITNESS_SAMPLES="${SAMPLES}" \
    caffeinate -dimsu "${BIN}" prepare "${task}" \
      > "${csv}" 2> "${DATA}/${task}-${BACKEND}-r${REGIME}.stderr.txt"
  rc=$?

  if grep -q '^ROUND TRIP FAILED' "${csv}"; then
    echo "[neg] ${task}/${BACKEND}: ROUND TRIP FAILED — every other result in this file is meaningless" >&2
    rc_all=9
    continue
  fi
  [[ ${rc} -ne 0 ]] && rc_all="${rc}"

  # The exhaustive proof-byte sweep is driven separately, because it has to survive the
  # process dying. See sweep-proof-bytes.sh.
  "${ROOT}/scripts/gnark/sweep-proof-bytes.sh" "${task}" || rc_all=$?

  echo "[neg] ${task}/${BACKEND}/r${REGIME}:" >&2
  awk -F, '{c[$5]++} END {for (k in c) printf "  %-22s %d\n", k, c[k]}' \
    "${csv}" "${DATA}/${task}-${BACKEND}-r${REGIME}-exhaustive.csv" >&2
done
exit "${rc_all}"
