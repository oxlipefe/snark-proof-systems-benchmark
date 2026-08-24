#!/usr/bin/env bash
# Exhaustive single-bit sweep over every byte of a serialized jolt-atlas proof.
#
# WHY THIS IS A LOOP AND NOT ONE RUN. Some corrupted length prefixes make the deserializer
# attempt an absurd allocation ("memory allocation of 6755399441057472 bytes failed") and the
# process ABORTS. That is not a panic and `catch_unwind` cannot catch it, so one process
# cannot finish the sweep. The loop restarts after the offset that killed it and records that
# offset's verdict as DESERIALIZE_ABORT, which is a real, reportable behaviour rather than a
# gap in the data.
set -uo pipefail
: "${HARNESS:?}"; : "${JA_ROOT:?}"
task="${1:?task label}"; total="${2:?proof size in bytes}"
# Repository root. Derived from this script's own location so a clone works anywhere.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
out="${ROOT}/data/negative-jolt-atlas/${task}-exhaustive.csv"
work="$(mktemp -d)"; : > "${out}"
next=0
while (( next < total )); do
  seq "${next}" $(( total - 1 )) > "${work}/off.txt"
  ( cd "${JA_ROOT}" && JA_OFFSETS_FILE="${work}/off.txt" JA_PATTERNS=01 \
      caffeinate -dimsu "${HARNESS}" "${task}" \
        "${ROOT}/tasks/jolt-atlas/${task}.onnx" \
        "${ROOT}/tasks/jolt-atlas/${task}.inputs.json" ) 2>/dev/null \
    | grep '^'"${task}"',proof_byte,' >> "${out}"
  last="$(awk -F, 'END{print $3}' "${out}")"
  [[ -z "${last}" ]] && last=$(( next - 1 ))
  if (( last + 1 >= total )); then break; fi
  echo "${task},proof_byte,$(( last + 1 )),xor01,DESERIALIZE_ABORT" >> "${out}"
  next=$(( last + 2 ))
  echo "[sweep] aborted at $(( last + 1 )); resuming at ${next}" >&2
done
rm -rf "${work}"
