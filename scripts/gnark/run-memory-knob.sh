#!/usr/bin/env bash
# zk-prover-bench · gnark · the memory-knob sweep.
#
# gnark exposes NO protocol-level memory knob. backend.ProverConfig at v0.16.2 has five
# fields and none of them segments, streams or caps a shard; there is nothing with the shape
# of Ceno's --max-cycle-per-shard or of a streaming prover's window.
#
# What DOES exist is the Go runtime. GOGC changes how much garbage the heap is allowed to
# accumulate before a collection; GOMEMLIMIT is a soft ceiling the collector works to stay
# under. Both move peak RSS and peak footprint, and neither is a property of the proof
# system.
#
# THIS SCRIPT DOES NOT DECIDE WHETHER THAT COUNTS AS A MEMORY KNOB. It produces the data:
# same task, same backend, same regime, same threads, one variable moving at a time.
#
#   axis 1   GOGC in {400, 100, 50, 25}, GOMEMLIMIT off
#   axis 2   GOMEMLIMIT at one setting, GOGC at its default
#   axis 3   solver.WithNbTasks, which is a PARALLELISM knob that happens to move memory and
#            is swept separately for exactly that reason
set -uo pipefail
# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly CELL="${ROOT}/scripts/gnark/run-cell.sh"
TASK="${TASK:-t1-0}"
BACKEND="${GNARK_BACKEND:-groth16}"
REGIME="${GNARK_REGIME:-A}"
REPS="${GNARK_REPS:-3}"
MEMLIMIT="${MEMLIMIT:-2GiB}"

echo "=== axis 1: GOGC ===" >&2
for g in 400 100 50 25; do
  GOGC="${g}" GNARK_BACKEND="${BACKEND}" GNARK_REGIME="${REGIME}" GNARK_REPS="${REPS}" \
    "${CELL}" "${TASK}"
done

echo "=== axis 2: GOMEMLIMIT ===" >&2
GOMEMLIMIT="${MEMLIMIT}" GNARK_BACKEND="${BACKEND}" GNARK_REGIME="${REGIME}" GNARK_REPS="${REPS}" \
  "${CELL}" "${TASK}"

echo "=== axis 3: solver.WithNbTasks (a parallelism knob, swept separately) ===" >&2
for n in 1 2 4 10; do
  GNARK_NB_TASKS="${n}" GNARK_BACKEND="${BACKEND}" GNARK_REGIME="${REGIME}" GNARK_REPS="${REPS}" \
    "${CELL}" "${TASK}"
done
