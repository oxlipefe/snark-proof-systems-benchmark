#!/usr/bin/env bash
# zk-prover-bench · DeepProve · verify time and proof-artifact size.
#
# The instrument is `deep-prove-cli verify <file>`, DeepProve's own public verifier CLI, timed
# as a whole process. That is a wider bracket than binius64's `verify ms` column, which times
# one function call inside a process that has already loaded everything. What is inside this
# one — file read, base64 decode, postcard deserialization of proof + IO + verifier context,
# and the verification itself — cannot be separated without instrumenting DeepProve's
# internals or reverse engineering its format, and its licence permits neither. The figure is
# published as what it is: a cold, whole-process verify.
#
# WHAT THE ARTIFACT CONTAINS, so its size is not read as a proof size. The file is
#   Output { outputs, proof: Provable { proof, io, ctx } }
# (deep-prove/src/middleware/v1.rs:41-46, v2.rs:14-19) — it carries the VERIFIER CONTEXT as
# well as the proof. It is an upper bound on proof size, not a proof size, and RESULTS.md says
# so wherever the number appears.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly DP_ROOT="${DP_ROOT:?set DP_ROOT to the deep-prove clone outside this repository}"
readonly CLI="${DP_ROOT}/target/release/deep-prove-cli"
readonly NEG="${ROOT}/data/negative-deepprove"
readonly OUT="${ROOT}/data/verify-deepprove"
readonly REPS="${REPS:-6}"       # 1 warmup + 5 timed

mkdir -p "${OUT}"
CSV="${OUT}/verify.csv"
echo "task,rep,is_warmup,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,artifact_b64_bytes,artifact_decoded_bytes,verdict" > "${CSV}"

for task in "$@"; do
  honest="$(ls "${NEG}/${task}"/honest*.postcard 2>/dev/null | head -1)"
  if [[ -z "${honest}" ]]; then
    echo "[verify] ${task}: no honest artifact — run run-negative.sh first" >&2
    continue
  fi
  b64="$(wc -c < "${honest}" | tr -d ' ')"
  dec="$(python3 -c "import base64,sys;print(len(base64.b64decode(open(sys.argv[1],'rb').read())))" "${honest}")"

  for rep in $(seq 0 $((REPS - 1))); do
    log="${OUT}/${task}-rep${rep}.txt"
    caffeinate -dimsu /usr/bin/time -l "${CLI}" verify "${honest}" > "${log}" 2>&1
    rc=$?
    real="$(grep -Eo '^ *[0-9.]+ real' "${log}" | awk '{print $1}' | tail -1)"
    user="$(grep -Eo '[0-9.]+ user' "${log}" | awk '{print $1}' | tail -1)"
    sys="$(grep -Eo '[0-9.]+ sys' "${log}" | awk '{print $1}' | tail -1)"
    rss="$(grep 'maximum resident set size' "${log}" | awk '{print $1}' | tail -1)"
    fp="$(grep 'peak memory footprint' "${log}" | awk '{print $1}' | tail -1)"
    ratio="$(awk -v u="${user:-0}" -v s="${sys:-0}" -v r="${real:-0}" 'BEGIN{ if (r>0) printf "%.4f", (u+s)/r; else print "" }')"
    if [[ ${rc} -eq 0 ]] && grep -q "Proof verified successfully" "${log}"; then
      verdict="VERIFY_ACCEPTED"
    else
      verdict="VERIFY_FAILED_rc${rc}"
    fi
    warm=$([[ ${rep} -eq 0 ]] && echo true || echo false)
    echo "${task},${rep},${warm},${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${fp:-},${b64},${dec},${verdict}" >> "${CSV}"
    echo "[verify] ${task} rep${rep} ${verdict} real=${real:-?}s" >&2
  done
done

echo "[verify] -> ${CSV}" >&2
