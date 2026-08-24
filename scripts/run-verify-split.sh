#!/usr/bin/env bash
# zk-prover-bench · diagnostic runner for the verify-time decomposition.
#
# `bench/systems/binius64/RESULTS.md` §4 reported that verify time grows with circuit size
# and did not explain it. This runs `e006-verify-split`, which reproduces
# `binius_verifier::Verifier::verify` call for call and times its four terms in the same
# loop and the same run.
#
# Same two environment controls as `run-cell.sh`, and for the same reasons:
#   caffeinate -dimsu   the machine must not idle-sleep inside a timed run
#   clockprobe.py       detects it anyway; wall - monotonic is time spent asleep
#
# Usage:  run-verify-split.sh <task>:<rate>:<threads>[:<reps>[:drop]] ...
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly HARNESS="${ROOT}/scripts/binius64/harness/target/release/e006-verify-split"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly DATA="${ROOT}/data/verify-split"
readonly LEDGER="${DATA}/cells.csv"

mkdir -p "${DATA}"
[[ -f "${LEDGER}" ]] || echo "label,task,log_inv_rate,threads,reps,drop_prover,status,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,mono_s,wall_s,slept_s,sleep_verdict,loadavg_1m,swap_used_mb,started_utc" > "${LEDGER}"

run_split() {
  local task="$1" rate="$2" threads="$3" reps="$4" drop="$5"
  local label="${task}-r${rate}-t${threads}-n${reps}"
  [[ "${drop}" == "drop" ]] && label="${label}-dropprover"
  local timelog="${DATA}/${label}.time.txt"

  local drop_flag=()
  [[ "${drop}" == "drop" ]] && drop_flag=(--drop-prover)

  local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local load1; load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
  local swap;  swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
  read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

  RAYON_NUM_THREADS="${threads}" caffeinate -dimsu /usr/bin/time -l "${HARNESS}" \
      --task "${task}" \
      --log-inv-rate "${rate}" \
      --warmup 1 \
      --reps "${reps}" \
      ${drop_flag[@]+"${drop_flag[@]}"} \
      --out-dir "${DATA}" \
      --label "${label}" \
    > "${DATA}/${label}.stdout.txt" 2> "${timelog}"
  local rc=$?

  read -r M1 W1 <<< "$(python3 "${CLOCKPROBE}" mark)"
  read -r mono wall slept verdict <<< "$(python3 "${CLOCKPROBE}" diff "${M0}" "${W0}" "${M1}" "${W1}")"

  local real user sys ratio rss footprint status
  real="$(grep -Eo '^ *[0-9.]+ real' "${timelog}" | awk '{print $1}' | tail -1)"
  user="$(grep -Eo '[0-9.]+ user' "${timelog}" | awk '{print $1}' | tail -1)"
  sys="$(grep -Eo '[0-9.]+ sys' "${timelog}" | awk '{print $1}' | tail -1)"
  rss="$(grep 'maximum resident set size' "${timelog}" | awk '{print $1}' | tail -1)"
  footprint="$(grep 'peak memory footprint' "${timelog}" | awk '{print $1}' | tail -1)"
  ratio="$(awk -v u="${user:-0}" -v s="${sys:-0}" -v r="${real:-0}" 'BEGIN{ if (r>0) printf "%.4f", (u+s)/r; else print "" }')"

  if [[ "${verdict}" == "INVALID_SLEEP" ]]; then
    status="INVALID_SLEEP"
  elif [[ ${rc} -eq 0 ]]; then
    status="OK"
  else
    status="FAIL_rc${rc}"
  fi

  echo "${label},${task},${rate},${threads},${reps},${drop},${status},${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${footprint:-},${mono},${wall},${slept},${verdict},${load1},${swap:-},${started}" >> "${LEDGER}"
  echo "[split] ${label} -> ${status} real=${real:-?}s slept=${slept}s" >&2
  tail -n 8 "${timelog}" >&2
  return ${rc}
}

for spec in "$@"; do
  IFS=: read -r task rate threads reps drop <<< "${spec}"
  run_split "${task}" "${rate}" "${threads}" "${reps:-5}" "${drop:-keep}" \
    || echo "[split] ${spec} did not complete — continuing" >&2
done
