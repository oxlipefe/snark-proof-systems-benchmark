#!/usr/bin/env bash
# zk-prover-bench · DeepProve · one measured cell.
#
# INSTRUMENT. The cell is `deep-prove-worker one-shot`, DeepProve's own local proving
# binary. It is used instead of DeepProve's `bench` binary (the one `zkml/bench.py` drives)
# because `bench` does not run on this machine at this commit: it unconditionally runs the
# float reference model (zkml/src/bin/bench/cnn.rs:247,253 set `with_keep_float(true)`;
# :287-288 run it) and that call fails with "Tensor is unavailable for a wrapped tensor
# handler" (zkml/src/tensor/handle.rs:100-101) before any proving happens. There is no CLI
# flag that skips it. See BUILD.md.
#
# WHAT IS MEASURED, AND FROM WHERE. Everything here is observable from outside the process.
# DeepProve's license is not OSI and forbids derivative works; its internals are NOT
# instrumented, and no part of its source is copied into this repository.
#
#   /usr/bin/time -l   peak RSS, peak memory footprint, real/user/sys -> (user+sys)/real
#                      (written to log.txt, which is the process's stderr)
#   its own tracing log  setup and prove boundaries, by timestamp; DeepProve writes it to
#                      STDOUT, so it lands in stdout.txt and is parsed by parse-cells.py
#
# ONE PROCESS PER CELL, so `/usr/bin/time -l` attributes the memory peak to that cell alone.
# The N repetitions run inside that one process, exactly as binius64's cells do, so the peak
# covers the warmup and every repetition. That convention is stated in RESULTS.md.
#
# Both sleep guards from bench/systems/binius64/BUILD.md §3 apply: `caffeinate -dimsu`
# prevents idle sleep, and the clock probe detects it anyway, because a wall-clock figure
# that includes sleep is garbage that looks like data.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly DP_ROOT="${DP_ROOT:?set DP_ROOT to the deep-prove clone outside this repository}"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly TASKS="${ROOT}/tasks/deepprove"
readonly DATA="${ROOT}/data"
readonly LEDGER="${DATA}/cells-deepprove.csv"
readonly PY="${PY:-python3}"

BITLEN="${BITLEN:-8}"
THREADS="${THREADS:-1}"
SAMPLES="${SAMPLES:-6}"      # 1 warmup + 5 timed repetitions
WARMUP="${WARMUP:-1}"

mkdir -p "${DATA}/cells-deepprove"
[[ -f "${LEDGER}" ]] || echo "label,task,graph,bitlen,threads,warmup,samples,status,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,mono_s,wall_s,slept_s,sleep_verdict,loadavg_1m,swap_used_mb,started_utc" > "${LEDGER}"

# run_cell <task-label> <graph> <samples>
#   task-label  the cell name (t2, t3, ...)
#   graph       the ONNX/io basename under bench/tasks/deepprove
run_cell() {
  local task="$1" graph="$2" samples="$3"
  local label="${task}-q${BITLEN}-t${THREADS}-n${samples}"
  local out_dir="${DATA}/cells-deepprove/${label}"
  mkdir -p "${out_dir}"

  # The inputs file the worker reads is `zkml::inputs::Input` (input_data only,
  # zkml/src/inputs.rs:12-15). The task's io.json carries the extra fields DeepProve's own
  # `bench` binary wants; serde ignores them, but the sample count has to be cut here
  # because the worker proves every input it is given.
  "${PY}" - "${TASKS}/${graph}.io.json" "${samples}" "${out_dir}/inputs.json" <<'PYEOF'
import json, sys
src, n, dst = sys.argv[1], int(sys.argv[2]), sys.argv[3]
data = json.load(open(src))
rows = data["input_data"]
# T3 is 8 independent inputs proved one after another; if the task asks for more samples
# than the file holds, cycle it rather than silently proving fewer.
rows = [rows[i % len(rows)] for i in range(n)]
json.dump({"input_data": rows}, open(dst, "w"))
PYEOF

  local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local load1; load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
  local swap;  swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
  read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

  local work; work="$(mktemp -d)"
  (
    cd "${work}" &&
    RAYON_NUM_THREADS="${THREADS}" \
    ZKML_BIT_LEN="${BITLEN}" \
    RUST_LOG="deep_prove_worker=debug,info" \
    caffeinate -dimsu /usr/bin/time -l \
      "${DP_ROOT}/target/release/deep-prove-worker" \
        --tensor-store temporary \
        one-shot \
        --model "${TASKS}/${graph}.onnx" \
        --model-format onnx \
        --inputs "${out_dir}/inputs.json"
  ) > "${out_dir}/stdout.txt" 2> "${out_dir}/log.txt"
  local rc=$?
  rm -rf "${work}"

  read -r M1 W1 <<< "$(python3 "${CLOCKPROBE}" mark)"
  read -r mono wall slept verdict <<< "$(python3 "${CLOCKPROBE}" diff "${M0}" "${W0}" "${M1}" "${W1}")"

  local real user sys ratio rss footprint status
  real="$(grep -Eo '^ *[0-9.]+ real' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  user="$(grep -Eo '[0-9.]+ user' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  sys="$(grep -Eo '[0-9.]+ sys' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  rss="$(grep 'maximum resident set size' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  footprint="$(grep 'peak memory footprint' "${out_dir}/log.txt" | awk '{print $1}' | tail -1)"
  ratio="$(awk -v u="${user:-0}" -v s="${sys:-0}" -v r="${real:-0}" 'BEGIN{ if (r>0) printf "%.4f", (u+s)/r; else print "" }')"

  # `one-shot` proves and then fails writing the proof to disk, because it serializes with
  # serde_json and the ONNX proof contains a map with non-string keys
  # (deep-prove/src/bin/worker/immediate.rs:123). The failure is AFTER "Proving done.", so
  # the timings and the memory peak are unaffected — but it means rc is non-zero on a run
  # that proved correctly. PROVED_NOWRITE records exactly that, rather than hiding it
  # behind OK or discarding a good measurement as a failure.
  if [[ "${verdict}" == "INVALID_SLEEP" ]]; then
    status="INVALID_SLEEP"
  elif [[ ${rc} -eq 0 ]]; then
    status="OK"
  elif grep -q "Proving done." "${out_dir}/stdout.txt"; then
    status="PROVED_NOWRITE"
  else
    status="FAIL_rc${rc}"
  fi

  echo "${label},${task},${graph},${BITLEN},${THREADS},${WARMUP},${samples},${status},${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${footprint:-},${mono},${wall},${slept},${verdict},${load1},${swap:-},${started}" >> "${LEDGER}"
  echo "[dp] ${label} -> ${status} real=${real:-?}s cpu=${ratio:-?} rss=${rss:-?}B footprint=${footprint:-?}B slept=${slept}s" >&2
}

# Each spec is task:graph:samples
for spec in "$@"; do
  IFS=: read -r task graph samples <<< "${spec}"
  run_cell "${task}" "${graph}" "${samples:-${SAMPLES}}"
done
