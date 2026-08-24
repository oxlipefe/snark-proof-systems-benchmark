#!/usr/bin/env bash
# zk-prover-bench · jolt-atlas · one measured cell.
#
# INSTRUMENT. `ja-harness/ja_bench`, OUR code (bench/scripts/jolt-atlas/harness/), which calls
# jolt-atlas's public API and nothing else. No jolt-atlas source is copied, patched or
# instrumented: its licence forbids derivative works, and §2(i) permits exactly this — internal
# use for testing and evaluation.
#
# ONE PROCESS PER CELL, so `/usr/bin/time -l` attributes one memory peak to one cell. The N
# repetitions run inside that one process, the same convention binius64 and DeepProve used.
#
# Both sleep guards apply: `caffeinate -dimsu` prevents idle sleep and clockprobe.py detects
# it anyway, because a wall-clock figure that includes sleep is garbage that looks like data.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly JA_ROOT="${JA_ROOT:?set JA_ROOT to the jolt-atlas clone outside this repository}"
readonly HARNESS="${HARNESS:?set HARNESS to the built ja_bench binary}"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly TASKS="${ROOT}/tasks/jolt-atlas"
readonly DATA="${ROOT}/data"
readonly LEDGER="${DATA}/cells-jolt-atlas.csv"

THREADS="${THREADS:-1}"
REPS="${REPS:-5}"
WARMUP="${WARMUP:-1}"
PADDING="${PADDING:-1}"

mkdir -p "${DATA}/cells-jolt-atlas"
[[ -f "${LEDGER}" ]] || echo "label,task,threads,padding,warmup,reps,status,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,involuntary_ctx_switches,setup_ms,prove_ms_median,prove_ms_min,prove_ms_max,verify_ms_median,proof_bytes,max_num_vars,mono_s,wall_s,slept_s,sleep_verdict,loadavg_1m,swap_used_mb,started_utc" > "${LEDGER}"

run_cell() {
  local task="$1"
  local label="${task}-t${THREADS}-p${PADDING}-n${REPS}"
  local out_dir="${DATA}/cells-jolt-atlas/${label}"
  mkdir -p "${out_dir}"

  local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local load1; load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
  local swap;  swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
  read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

  ( cd "${JA_ROOT}" &&
    RAYON_NUM_THREADS="${THREADS}" JA_REPS="${REPS}" JA_WARMUP="${WARMUP}" JA_PADDING="${PADDING}" \
    caffeinate -dimsu /usr/bin/time -l "${HARNESS}" \
      "${task}" "${TASKS}/${task}.onnx" "${TASKS}/${task}.inputs.json"
  ) > "${out_dir}/stdout.txt" 2> "${out_dir}/log.txt"
  local rc=$?

  read -r M1 W1 <<< "$(python3 "${CLOCKPROBE}" mark)"
  read -r mono wall slept verdict <<< "$(python3 "${CLOCKPROBE}" diff "${M0}" "${W0}" "${M1}" "${W1}")"

  local real user sys ratio rss fp ics status
  real="$(grep -Eo '^ *[0-9.]+ real' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  user="$(grep -Eo '[0-9.]+ user' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  sys="$(grep -Eo '[0-9.]+ sys' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  rss="$(grep 'maximum resident set size' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  fp="$(grep 'peak memory footprint' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  ics="$(grep 'involuntary context switches' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  ratio="$(awk -v u="${user:-0}" -v s="${sys:-0}" -v r="${real:-0}" 'BEGIN{ if (r>0) printf "%.4f",(u+s)/r; else print "" }')"

  local setup pm pmin pmax vm pb mnv
  setup="$(grep -Eo '^SETUP ms=[0-9.]+' "${out_dir}/stdout.txt" | cut -d= -f2)"
  mnv="$(grep -Eo 'max_num_vars=[0-9]+' "${out_dir}/stdout.txt" | cut -d= -f2 | head -1)"
  pb="$(grep -Eo '^DONE proof_bytes=[0-9]+' "${out_dir}/stdout.txt" | cut -d= -f2)"
  read -r pm pmin pmax vm <<< "$(python3 - "${out_dir}/stdout.txt" <<'PYEOF'
import sys, statistics as st
p, v = [], []
for line in open(sys.argv[1]):
    if not line.startswith("REP "):
        continue
    kv = dict(t.split("=", 1) for t in line.split()[1:])
    p.append(float(kv["prove_ms"])); v.append(float(kv["verify_ms"]))
if p:
    print(f"{st.median(p):.3f} {min(p):.3f} {max(p):.3f} {st.median(v):.3f}")
else:
    print("   ")
PYEOF
)"

  if [[ "${verdict}" == "INVALID_SLEEP" ]]; then status="INVALID_SLEEP"
  elif [[ ${rc} -eq 0 ]]; then status="OK"
  elif grep -q "not supported by Einsum proof system" "${out_dir}/log.txt"; then status="FAIL_einsum_unsupported"
  else status="FAIL_rc${rc}"; fi

  echo "${label},${task},${THREADS},${PADDING},${WARMUP},${REPS},${status},${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${fp:-},${ics:-},${setup:-},${pm:-},${pmin:-},${pmax:-},${vm:-},${pb:-},${mnv:-},${mono},${wall},${slept},${verdict},${load1},${swap:-},${started}" >> "${LEDGER}"
  echo "[ja] ${label} -> ${status} real=${real:-?}s cpu=${ratio:-?} fp=${fp:-?}B prove=${pm:-?}ms proof=${pb:-?}B slept=${slept}s" >&2
}

for t in "$@"; do run_cell "${t}"; done
