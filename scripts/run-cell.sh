#!/usr/bin/env bash
# zk-prover-bench · one measured cell of the binius64 system.
#
# One process per cell (task x rate x threads), so `/usr/bin/time -l` attributes peak RSS
# and peak memory footprint to that cell alone. A cell that shares a process with another
# cell has an unattributable memory peak, which is why the ladder is not run in one process.
#
# Two environment controls wrap every cell, and both exist because of a specific failure:
#
#   caffeinate -dimsu   The machine idle-slept in the middle of a timed run once. Wall-clock
#                       seconds that include sleep are garbage that looks like data.
#   clockprobe.py       Detects it anyway. macOS suspends the monotonic clock during sleep
#                       and keeps the wall clock running, so wall - monotonic is the time
#                       spent asleep. Any cell with a gap is marked INVALID_SLEEP and rerun.
#
# The peak RSS reported includes circuit construction and witness generation, not just the
# prover. That is deliberate: the question the memory metric answers is whether the task
# fits on the machine, not whether the prover would fit given a free witness.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly HARNESS="${ROOT}/scripts/binius64/harness/target/release/e006-bench"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly DATA="${ROOT}/data"
readonly LEDGER="${DATA}/cells.csv"

WARMUP="${WARMUP:-1}"
REPS="${REPS:-5}"

mkdir -p "${DATA}/cells"
[[ -f "${LEDGER}" ]] || echo "label,task,log_inv_rate,threads,warmup,reps,status,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,mono_s,wall_s,slept_s,sleep_verdict,loadavg_1m,swap_used_mb,started_utc" > "${LEDGER}"

# run_cell <task> <rate> <threads>
run_cell() {
  local task="$1" rate="$2" threads="$3"
  # Reps are part of the label: rerunning a cell at a different N must not
  # overwrite the per-repetition data of the earlier run.
  local label="${task}-r${rate}-t${threads}-n${REPS}"
  local out_dir="${DATA}/cells/${label}"
  local timelog="${DATA}/cells/${label}.time.txt"
  mkdir -p "${out_dir}"

  local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local load1; load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
  local swap;  swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
  read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

  RAYON_NUM_THREADS="${threads}" caffeinate -dimsu /usr/bin/time -l "${HARNESS}" \
      --task "${task}" \
      --log-inv-rate "${rate}" \
      --warmup "${WARMUP}" \
      --reps "${REPS}" \
      --out-dir "${out_dir}" \
      --label "${label}" \
    > "${out_dir}/stdout.txt" 2> "${timelog}"
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

  echo "${label},${task},${rate},${threads},${WARMUP},${REPS},${status},${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${footprint:-},${mono},${wall},${slept},${verdict},${load1},${swap:-},${started}" >> "${LEDGER}"
  echo "[bench] ${label} -> ${status} real=${real:-?}s cpu=${ratio:-?} rss=${rss:-?}B footprint=${footprint:-?}B slept=${slept}s" >&2
  return ${rc}
}

for spec in "$@"; do
  IFS=: read -r task rate threads <<< "${spec}"
  run_cell "${task}" "${rate}" "${threads}" || echo "[bench] ${spec} did not complete — continuing with the remaining cells" >&2
done
