#!/usr/bin/env bash
# zk-prover-bench · gnark · run-cell.sh with a free-disk watchdog.
#
# Same argument the binius64 variant makes, with one gnark-specific twist. On this machine
# peak *footprint* exceeds peak RSS at the large rungs, and the difference is compressed and
# swapped pages backed by the boot volume. A rung large enough to exhaust the boot volume
# does not just fail the cell, it destabilises the machine. So the cell runs under a
# watchdog: if free space on / falls below MIN_FREE_GB the run is killed and the cell is
# recorded as KILLED_DISK, which is a reported result and not a silent gap.
#
# The twist: a Groth16 SETUP for a large circuit allocates a proving key proportional to the
# PADDED domain, not to the constraint count, and it does so before a single proof is
# computed. gnark's ladder therefore dies in setup rather than in prove, and the watchdog has
# to survive a process that is still climbing.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly MIN_FREE_GB="${MIN_FREE_GB:-20}"
readonly POLL_S="${POLL_S:-10}"

"${ROOT}/scripts/gnark/run-cell.sh" "$@" &
readonly RUNNER_PID=$!

while kill -0 "${RUNNER_PID}" 2>/dev/null; do
  free_gb="$(df -g / | awk 'NR==2 {print $4}')"
  if [[ -n "${free_gb}" && "${free_gb}" -lt "${MIN_FREE_GB}" ]]; then
    echo "[watchdog] free space on / is ${free_gb} GiB, below the ${MIN_FREE_GB} GiB floor — killing the cell" >&2
    pkill -P "${RUNNER_PID}" 2>/dev/null
    kill "${RUNNER_PID}" 2>/dev/null
    pkill -f 'gnark-runner' 2>/dev/null
    echo "KILLED_DISK" > "${ROOT}/data/watchdog-last-gnark.txt"
    wait "${RUNNER_PID}" 2>/dev/null
    exit 3
  fi
  sleep "${POLL_S}"
done
wait "${RUNNER_PID}"
