#!/usr/bin/env bash
# Exhaustive single-bit sweep over every byte of a serialized gnark proof.
#
# WHY THIS IS A LOOP AND NOT ONE RUN. jolt-atlas taught the campaign that a corrupted length
# prefix can reach an allocator and ABORT the process — not panic, abort, which no recover()
# can catch — so one process cannot be assumed to finish the sweep. gnark's decoder has not
# been observed to do that at v0.16.2, and the loop is kept anyway: the cost is one extra
# process when nothing goes wrong, and the alternative is a silent hole in an exhaustive
# claim. An offset that kills the process is recorded as DESERIALIZE_ABORT and the sweep
# resumes after it.
#
# THE SWEEP DOES NOT RE-PROVE. Groth16's prover is randomized, so a restarted process would
# produce a DIFFERENT proof and every offset recorded before the restart would refer to bytes
# that no longer exist. `negative prepare` persists the verifying key, the public witness and
# the proof; `negative sweep` only reads them.
#
# EVERY BYTE IS SWEPT. A Groth16 proof here is 196 bytes and a PLONK proof 584; there is no
# excuse to sample, and DeepProve's coarse pass had already missed a whole accepted region
# once. Any byte whose 0x01 flip is ACCEPTED is re-probed with eight further patterns, which
# is what separates "this byte is not read at all" from "this field is read loosely".
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly BIN="${BIN:-${ROOT}/tasks/gnark/bin/gnark-negative}"
readonly DATA="${ROOT}/data/negative-gnark"
readonly BACKEND="${GNARK_BACKEND:-groth16}"
readonly REGIME="${GNARK_REGIME:-A}"
readonly REPROBE_PATTERNS="${REPROBE_PATTERNS:-01,02,04,08,10,20,40,80,ff}"

task="${1:?task label}"
cache="${DATA}/cache/${task}-${BACKEND}-r${REGIME}"
out="${DATA}/${task}-${BACKEND}-r${REGIME}-exhaustive.csv"
[[ -f "${cache}/proof.bin" ]] || { echo "[sweep] no cached proof at ${cache}; run negative prepare first" >&2; exit 1; }

total="$(wc -c < "${cache}/proof.bin" | tr -d ' ')"
work="$(mktemp -d)"; : > "${out}"
echo "[sweep] ${task}/${BACKEND}: ${total} bytes, one 0x01 flip each" >&2

next=0
while (( next < total )); do
  seq "${next}" $(( total - 1 )) > "${work}/off.txt"
  GNARK_BACKEND="${BACKEND}" GNARK_NEG_CACHE="${cache}" \
  GNARK_NEG_OFFSETS_FILE="${work}/off.txt" GNARK_NEG_PATTERNS=01 \
    caffeinate -dimsu "${BIN}" sweep "${task}" 2>/dev/null \
    | grep "^${task},proof_byte," >> "${out}"
  last="$(awk -F, 'END{print $3}' "${out}")"
  [[ -z "${last}" ]] && last=$(( next - 1 ))
  if (( last + 1 >= total )); then break; fi
  echo "${task},proof_byte,$(( last + 1 )),xor01,DESERIALIZE_ABORT" >> "${out}"
  next=$(( last + 2 ))
  echo "[sweep] aborted at $(( last + 1 )); resuming at ${next}" >&2
done

# Re-probe every ACCEPTED offset with the full pattern set. An offset that accepts all nine
# patterns is a byte the verifier does not read at all; one that accepts some and rejects
# others is a field read loosely, and the two are different findings.
accepted="$(awk -F, '$5=="VERIFY_ACCEPTED" {print $3}' "${out}" | sort -n -u)"
if [[ -n "${accepted}" ]]; then
  echo "${accepted}" > "${work}/acc.txt"
  echo "[sweep] $(wc -l < "${work}/acc.txt" | tr -d ' ') accepted offset(s); re-probing with ${REPROBE_PATTERNS}" >&2
  GNARK_BACKEND="${BACKEND}" GNARK_NEG_CACHE="${cache}" \
  GNARK_NEG_OFFSETS_FILE="${work}/acc.txt" GNARK_NEG_PATTERNS="${REPROBE_PATTERNS}" \
    caffeinate -dimsu "${BIN}" sweep "${task}" 2>/dev/null \
    | grep "^${task},proof_byte," > "${DATA}/${task}-${BACKEND}-r${REGIME}-reprobe.csv"
else
  : > "${DATA}/${task}-${BACKEND}-r${REGIME}-reprobe.csv"
  echo "[sweep] no accepted offsets; nothing to re-probe" >&2
fi

rm -rf "${work}"
awk -F, '{c[$5]++} END {for (k in c) printf "  %-22s %d\n", k, c[k]}' "${out}" >&2
