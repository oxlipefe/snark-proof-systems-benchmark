#!/usr/bin/env bash
# zk-prover-bench · gnark · one measured cell.
#
# INSTRUMENT. `bench/tasks/gnark/bin/gnark-runner`, OUR code
# (bench/tasks/gnark/), which links gnark v0.16.2 as a normal Go module dependency and calls
# its public API and nothing else. gnark is Apache-2.0, so unlike jolt-atlas there is no
# licence obstacle to instrumenting it — we did not need to, and the fact that we did not is
# what makes `go.sum` a complete statement of what was measured.
#
# ONE PROCESS PER CELL, so `/usr/bin/time -l` attributes one memory peak to one cell. The N
# repetitions run inside that one process, and so do compile and setup — the same convention
# binius64, DeepProve, Ceno and jolt-atlas used. Compile and setup are reported in their own
# columns and are NEVER folded into prove time.
#
# THE LABEL CARRIES EVERY VARYING PARAMETER — task, backend, regime, threads, GOGC,
# GOMEMLIMIT, solver tasks and N. A rerun at a different N writes a different row instead of
# overwriting an earlier cell's data, which is the failure mode that silently deletes the
# evidence for a curve.
#
# Both sleep guards apply: `caffeinate -dimsu` prevents idle sleep and clockprobe.py detects
# it anyway, because a wall-clock figure that includes sleep is garbage that looks like data.
# INVALID_SLEEP wins over every other status, including a non-zero exit.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly RUNNER="${RUNNER:-${ROOT}/tasks/gnark/bin/gnark-runner}"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly DATA="${ROOT}/data"
readonly LEDGER="${DATA}/cells-gnark.csv"

BACKEND="${GNARK_BACKEND:-groth16}"
REGIME="${GNARK_REGIME:-A}"
GADGET="${GNARK_GADGET:-hintedsign}"
THREADS="${GOMAXPROCS:-$(sysctl -n hw.ncpu)}"
REPS="${GNARK_REPS:-5}"
WARMUP="${GNARK_WARMUP:-1}"
NB_TASKS="${GNARK_NB_TASKS:-0}"
GOGC_V="${GOGC:-default}"
GOMEMLIMIT_V="${GOMEMLIMIT:-off}"

mkdir -p "${DATA}/cells-gnark"
[[ -f "${LEDGER}" ]] || echo "label,task,backend,regime,gadget,threads,nb_tasks,gogc,gomemlimit,warmup,reps,status,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,involuntary_ctx_switches,compile_ms,setup_ms,srs_ms,pk_bytes,vk_bytes,constraints,domain_cardinality,internal_vars,secret_vars,public_vars,relus,max_abs_intermediate,prove_ms_median,prove_ms_min,prove_ms_max,verify_ms_median,proof_bytes,proof_bytes_raw,mono_s,wall_s,slept_s,sleep_verdict,loadavg_1m,swap_used_mb,started_utc" > "${LEDGER}"

sanitize() { echo "$1" | tr -c 'A-Za-z0-9._-' '_' | sed 's/_*$//'; }

run_cell() {
  # spec is task[:backend[:regime]]; anything omitted falls back to the environment, so a
  # single-task invocation reads exactly like jolt-atlas's.
  local spec="$1"
  local task backend regime
  IFS=':' read -r task backend regime <<< "${spec}"
  backend="${backend:-${BACKEND}}"
  regime="${regime:-${REGIME}}"

  local label="${task}-${backend}-r${regime}-t${THREADS}-g$(sanitize "${GOGC_V}")-m$(sanitize "${GOMEMLIMIT_V}")-nt${NB_TASKS}-n${REPS}"
  local out_dir="${DATA}/cells-gnark/${label}"
  mkdir -p "${out_dir}"

  local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local load1; load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
  local swap;  swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
  read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

  ( GOMAXPROCS="${THREADS}" \
    GNARK_BACKEND="${backend}" GNARK_REGIME="${regime}" GNARK_GADGET="${GADGET}" \
    GNARK_REPS="${REPS}" GNARK_WARMUP="${WARMUP}" GNARK_NB_TASKS="${NB_TASKS}" \
    caffeinate -dimsu /usr/bin/time -l "${RUNNER}" "${task}"
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

  meta() { grep -Eo "(^| )$1=[^ ]+" "${out_dir}/stdout.txt" | head -1 | cut -d= -f2-; }
  local compile setup srs pk vk cons dom iv sv pv relus maxabs pb pbr
  compile="$(meta compile_ms)"; cons="$(meta constraints)"; dom="$(meta domain_cardinality)"
  iv="$(meta internal_vars)"; sv="$(meta secret)"; pv="$(meta public)"
  relus="$(meta relus)"; maxabs="$(meta max_abs_intermediate)"
  setup="$(grep -E '^SETUP ' "${out_dir}/stdout.txt" | grep -Eo ' ms=[0-9.]+' | cut -d= -f2)"
  srs="$(grep -E '^SETUP ' "${out_dir}/stdout.txt" | grep -Eo 'srs_ms=[0-9.]+' | cut -d= -f2)"
  pk="$(grep -E '^SETUP ' "${out_dir}/stdout.txt" | grep -Eo 'pk_bytes=[0-9]+' | cut -d= -f2)"
  vk="$(grep -E '^SETUP ' "${out_dir}/stdout.txt" | grep -Eo 'vk_bytes=[0-9]+' | cut -d= -f2)"
  pb="$(grep -Eo '^DONE proof_bytes=[0-9]+' "${out_dir}/stdout.txt" | cut -d= -f2)"
  pbr="$(grep -E '^REP ' "${out_dir}/stdout.txt" | grep -Eo 'proof_bytes_raw=[0-9]+' | tail -1 | cut -d= -f2)"

  # Only REP lines feed the statistics. The warmup stays in the raw stdout and out of every
  # median, the same rule every other system in this campaign follows.
  local pm pmin pmax vm
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

  # INVALID_SLEEP wins over everything: a cell that spanned a machine sleep has a real-time
  # figure that includes time the CPU was not running, and every rate derived from it is
  # garbage that looks like data.
  if [[ "${verdict}" == "INVALID_SLEEP" ]]; then status="INVALID_SLEEP"
  elif [[ ${rc} -eq 0 ]]; then status="OK"
  elif grep -q 'GNARK_FAIL class=VERIFY' "${out_dir}/log.txt"; then status="FAIL_verify"
  elif grep -q 'GNARK_FAIL class=MAC_ASSERTION' "${out_dir}/log.txt"; then status="FAIL_mac_assertion"
  elif grep -q 'GNARK_FAIL class=A1_ASSERTION' "${out_dir}/log.txt"; then status="FAIL_a1_assertion"
  elif grep -q 'GNARK_FAIL class=COMPILE' "${out_dir}/log.txt"; then status="FAIL_compile"
  elif grep -q 'GNARK_FAIL class=SETUP' "${out_dir}/log.txt"; then status="FAIL_setup"
  elif grep -q 'GNARK_FAIL class=WITNESS' "${out_dir}/log.txt"; then status="FAIL_witness"
  elif grep -q 'GNARK_FAIL class=PROVE' "${out_dir}/log.txt"; then status="FAIL_prove"
  elif grep -qi 'out of memory\|cannot allocate\|signal: killed' "${out_dir}/log.txt"; then status="FAIL_oom"
  else status="FAIL_rc${rc}"; fi

  echo "${label},${task},${backend},${regime},${GADGET},${THREADS},${NB_TASKS},${GOGC_V},${GOMEMLIMIT_V},${WARMUP},${REPS},${status},${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${fp:-},${ics:-},${compile:-},${setup:-},${srs:-},${pk:-},${vk:-},${cons:-},${dom:-},${iv:-},${sv:-},${pv:-},${relus:-},${maxabs:-},${pm:-},${pmin:-},${pmax:-},${vm:-},${pb:-},${pbr:-},${mono},${wall},${slept},${verdict},${load1},${swap:-},${started}" >> "${LEDGER}"
  echo "[gnark] ${label} -> ${status} real=${real:-?}s cpu=${ratio:-?} fp=${fp:-?}B rss=${rss:-?}B cons=${cons:-?} prove=${pm:-?}ms proof=${pb:-?}B slept=${slept}s" >&2
}

for t in "$@"; do run_cell "${t}"; done
