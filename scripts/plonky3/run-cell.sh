#!/usr/bin/env bash
# zk-prover-bench · Plonky3 · one measured cell.
#
# A cell is `task:field:route:threads`, and it gets ONE process, so `/usr/bin/time -l`
# attributes peak RSS and peak memory footprint to that cell alone. A cell that shares a
# process with another cell has an unattributable memory peak, which is why the grid is not
# run in one process.
#
# Two environment controls wrap every cell, and both exist because of a specific failure
# documented in this repository:
#
#   caffeinate -dimsu   The machine idle-slept in the middle of a timed run once. Wall-clock
#                       seconds that include sleep are garbage that looks like data.
#   clockprobe.py       Detects it anyway. macOS suspends the monotonic clock during sleep and
#                       keeps the wall clock running, so wall - monotonic is the time spent
#                       asleep. Any cell with a gap above 2 s is INVALID_SLEEP and is rerun.
#
# The peak memory reported covers the whole process: drawing the instance, embedding it in the
# field, computing the public output, the WHIR setup where there is one, proving AND verifying.
# That is a wider bracket than "the prover" and it is the same bracket the other five systems
# report, because the question the memory metric answers is whether the task fits on the
# machine.
#
# `peak footprint` is the memory column this repository reads; `peak RSS` is recorded beside it
# and is NOT cited on this machine (its own dispersion is 22.9 %).
set -uo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly HARNESS="${ROOT}/scripts/plonky3/harness/target/release/p3-bench"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly DATA="${ROOT}/data"
readonly LEDGER="${DATA}/cells-plonky3.csv"

WARMUP="${WARMUP:-1}"
REPS="${REPS:-5}"

[[ -x "${HARNESS}" ]] || { echo "[plonky3] build first: scripts/plonky3/build.sh" >&2; exit 1; }

mkdir -p "${DATA}/cells-plonky3"
[[ -f "${LEDGER}" ]] || echo "label,task,field,route,threads,warmup,reps,status,smoke,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,padded_macs,padding_factor,sumcheck_rounds,reduction_field_muls,prove_median_nanos,verify_median_nanos,proof_bytes_median,setup_nanos,integer_faithful,mono_s,wall_s,slept_s,sleep_verdict,loadavg_1m,swap_used_mb,started_utc" > "${LEDGER}"

# A cell run with fewer repetitions than the protocol asks for is SMOKE and is labelled so in
# the ledger, so that no smoke row can be read later as a campaign row.
smoke_flag() {
  if [[ "${REPS}" -lt 5 ]]; then echo "SMOKE"; else echo "CAMPAIGN"; fi
}

run_cell() {
  local task="$1" field="$2" route="$3" threads="$4"
  local label="${task}-${field}-${route}-t${threads}-n${REPS}"
  local out_dir="${DATA}/cells-plonky3/${label}"
  local timelog="${out_dir}/time.txt"
  mkdir -p "${out_dir}"

  local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local load1; load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
  local swap;  swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
  read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

  RAYON_NUM_THREADS="${threads}" caffeinate -dimsu /usr/bin/time -l "${HARNESS}" \
      --task "${task}" \
      --field "${field}" \
      --route "${route}" \
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

  # Structural quantities come from the harness's own cell.json, not from this script.
  local pm pf rounds muls prove verify pbytes setup faithful
  read -r pm pf rounds muls prove verify pbytes setup faithful <<< "$(python3 - "${out_dir}/cell.json" <<'PY'
import json, sys, pathlib
p = pathlib.Path(sys.argv[1])
if not p.exists():
    print(" ".join([""] * 9)); raise SystemExit
c = json.loads(p.read_text())
def g(k):
    v = c.get(k)
    return "" if v is None else v
print(g("padded_macs"), g("padding_factor"), g("sumcheck_rounds"), g("reduction_field_muls"),
      g("prove_median_nanos"), g("verify_median_nanos"), g("proof_bytes_median"),
      g("setup_nanos"), g("integer_faithful"))
PY
)"

  if [[ "${verdict}" == "INVALID_SLEEP" ]]; then
    status="INVALID_SLEEP"
  elif [[ ${rc} -eq 0 ]]; then
    status="OK"
  else
    status="FAIL_rc${rc}"
  fi

  echo "${label},${task},${field},${route},${threads},${WARMUP},${REPS},${status},$(smoke_flag),${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${footprint:-},${pm},${pf},${rounds},${muls},${prove},${verify},${pbytes},${setup},${faithful},${mono},${wall},${slept},${verdict},${load1},${swap:-},${started}" >> "${LEDGER}"
  echo "[plonky3] ${label} -> ${status} $(smoke_flag) real=${real:-?}s cpu=${ratio:-?} fp=${footprint:-?}B proof=${pbytes}B slept=${slept}s" >&2
  return ${rc}
}

for spec in "$@"; do
  IFS=: read -r task field route threads <<< "${spec}"
  threads="${threads:-1}"
  run_cell "${task}" "${field}" "${route}" "${threads}" \
    || echo "[plonky3] ${spec} did not complete — continuing with the remaining cells" >&2
done
