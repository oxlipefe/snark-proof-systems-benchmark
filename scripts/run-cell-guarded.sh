#!/usr/bin/env bash
# zk-prover-bench · run-cell.sh with a free-disk watchdog.
#
# On this machine peak *footprint* exceeds peak RSS by a wide margin at the large rungs
# (T1-b: 27.6 GB footprint against 16.7 GB RSS), and the difference is compressed and
# swapped pages backed by the boot volume. A rung large enough to exhaust the boot volume
# does not just fail the cell, it destabilises the machine. So the cell runs under a
# watchdog: if free space on / falls below MIN_FREE_GB the run is killed and the cell is
# recorded as KILLED_DISK, which is a reported result and not a silent gap.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly MIN_FREE_GB="${MIN_FREE_GB:-20}"
readonly POLL_S="${POLL_S:-10}"

"${ROOT}/scripts/run-cell.sh" "$@" &
readonly RUNNER=$!

while kill -0 "${RUNNER}" 2>/dev/null; do
  free_gb="$(df -g / | awk 'NR==2 {print $4}')"
  if [[ -n "${free_gb}" && "${free_gb}" -lt "${MIN_FREE_GB}" ]]; then
    echo "[watchdog] free space on / is ${free_gb} GiB, below the ${MIN_FREE_GB} GiB floor — killing the cell" >&2
    pkill -P "${RUNNER}" 2>/dev/null
    kill "${RUNNER}" 2>/dev/null
    pkill -f 'e006-bench' 2>/dev/null
    echo "KILLED_DISK" > "${ROOT}/data/watchdog-last.txt"
    wait "${RUNNER}" 2>/dev/null
    exit 3
  fi
  sleep "${POLL_S}"
done
wait "${RUNNER}"
