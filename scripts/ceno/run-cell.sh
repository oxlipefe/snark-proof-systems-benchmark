#!/usr/bin/env bash
# zk-prover-bench · Ceno · one measured cell.
#
# ONE PROCESS PER REPETITION, and one cell is N such processes. Ceno's `e2e` binary proves
# once and exits, so — unlike binius64's and DeepProve's harnesses, which run N repetitions
# inside one process — every repetition here gets its own `/usr/bin/time -l`. That makes the
# memory peak cleaner (it is unambiguously one proof's peak) and it is declared in
# RESULTS.md rather than presented as the same bracket. The per-cell memory reported is the
# maximum over repetitions; the per-repetition values are all in the raw ledger.
#
# Two environment controls wrap every repetition, and both exist because of a specific
# failure documented in this repository:
#
#   caffeinate -dimsu   The machine idle-slept in the middle of a timed run once. Wall-clock
#                       seconds that include sleep are garbage that looks like data.
#   clockprobe.py       Detects it anyway. macOS suspends the monotonic clock during sleep
#                       and keeps the wall clock running, so wall - monotonic is the time
#                       spent asleep. Any repetition with a gap is marked INVALID_SLEEP.
#
# The prove time is NOT the process wall time. It is the `ZKVM_create_proof` tracing span
# emitted by `--profiling 1` — pure proof generation, excluding emulation and witness
# generation. That is the same span, extracted by the same sed/awk pipeline, that Ceno's own
# CI records as its published GPU baseline (.github/workflows/gpu-integration.yml), so our
# number and their number denote the same bracket. Process wall time, emulation time and
# witness-generation time are recorded alongside it and never folded into it.
#
# The peak memory reported covers the whole process: ELF load, emulation, witness
# generation, keygen and proving. That is deliberate and matches the other three systems:
# the question the memory metric answers is whether the task fits on the machine.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
readonly E2E="${CENO_ROOT}/target/release/e2e"
readonly GUESTS="${CENO_ROOT}/examples/target/riscv32im-ceno-zkvm-elf/release/examples"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly TASKS="${ROOT}/tasks/ceno"
readonly DATA="${ROOT}/data"
readonly LEDGER="${DATA}/cells-ceno.csv"

REPS="${REPS:-5}"
WARMUP="${WARMUP:-1}"
STACK="${STACK:-2M}"
HEAP="${HEAP:-2M}"
# Ceno segments a trace that is too large for one shard. The default cap is 2^29 cycles and
# 2^31 cells, and the source comment says the cell default was sized for "16GB VRAM" — i.e.
# for a GPU, which is not what round one measures. Peak prover memory is therefore a
# FUNCTION OF THIS KNOB and not a property of the task, which is the single most important
# thing this system contributes to the benchmark. Every cell records the value it ran with.
MAX_CYCLE_PER_SHARD="${MAX_CYCLE_PER_SHARD:-536870912}"
MAX_CELL_PER_SHARD="${MAX_CELL_PER_SHARD:-2147483648}"

mkdir -p "${DATA}/cells-ceno"
[[ -f "${LEDGER}" ]] || echo "label,task,elf,threads,max_cycle_per_shard,max_cell_per_shard,warmup,reps,rep,is_warmup,status,real_s,user_s,sys_s,cpu_ratio,peak_rss_bytes,peak_footprint_bytes,create_proof_s,instructions,cycles,num_shards,proof_bytes,vk_bytes,mono_s,wall_s,slept_s,sleep_verdict,loadavg_1m,swap_used_mb,started_utc" > "${LEDGER}"

# elf_for <task> -> the guest ELF that expresses it
elf_for() {
  case "$1" in
    t1-*) echo "bench_t1" ;;
    t2|t3) echo "bench_mlp" ;;
    probe-*) echo "bench_commit_probe" ;;
    *) echo "unknown task: $1" >&2; return 1 ;;
  esac
}

# Extract the ZKVM_create_proof span, in seconds.
# Verbatim from .github/workflows/gpu-integration.yml at the pinned commit, so the bracket
# is theirs and not ours.
extract_create_proof() {
  local log="$1" line
  line="$(grep -F 'ZKVM_create_proof [' "${log}" | head -1 || true)"
  [[ -n "${line}" ]] || { echo ""; return; }
  echo "${line}" \
    | sed -E 's/.*ZKVM_create_proof \[ *([0-9.]+)(ns|µs|us|ms|m|s).*/\1 \2/' \
    | awk '{u=$2;v=$1; if(u=="ns")v/=1e9; else if(u=="µs"||u=="us")v/=1e6; else if(u=="ms")v/=1e3; else if(u=="m")v*=60; printf "%.6f", v}'
}

run_rep() {
  local task="$1" threads="$2" rep="$3" is_warmup="$4"
  local elf_name; elf_name="$(elf_for "${task}")" || return 1
  local label="${task}-t${threads}-s${MAX_CYCLE_PER_SHARD}-n${REPS}"
  local out_dir="${DATA}/cells-ceno/${label}"
  local tag="rep${rep}"; [[ "${is_warmup}" == "1" ]] && tag="warmup"
  local log="${out_dir}/${tag}.log.txt"
  local stdout="${out_dir}/${tag}.stdout.txt"
  local proof="${out_dir}/${tag}.proof.bin"
  local vk="${out_dir}/${tag}.vk.bin"
  mkdir -p "${out_dir}"

  local started; started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  local load1; load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
  local swap;  swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
  read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

  # --public-io carries the task's claimed output. The guest hashes its output bytes with
  # Keccak-256 and emits the digest as the proof's public values; the host derives the
  # expected digest from these words. If they disagree the proof does NOT verify — see
  # EXPRESSION.md §6. It is passed as argv because `e2e` exposes no file form; T1-d's list is
  # 429 067 bytes against a 1 048 576-byte ARG_MAX, and that headroom is declared, not assumed.
  local pio; pio="$(cat "${TASKS}/${task}.public-io.txt")"

  # `--profiling 1` installs a filter that keeps ONLY spans carrying a `profiling_N` field,
  # which is what makes the `ZKVM_create_proof` span readable — and which also suppresses
  # Ceno's own INFO lines `program executed <n> instructions in <m> cycles` and
  # `num_shards: <k>`. So the warmup repetition runs WITHOUT profiling, to capture those two
  # facts from the system itself, and the timed repetitions run WITH it, to capture the span.
  # The warmup is discarded for timing regardless, so this costs nothing.
  # `${arr[@]}` on an empty array trips `set -u` on bash 3.2, which is what macOS ships, so
  # the empty case is spelled out rather than expanded.
  local prof_args=(--profiling 1)
  # The warmup's placeholder restates a default (`--prover-id 0`), so it changes nothing.
  if [[ "${is_warmup}" == "1" ]]; then prof_args=(--prover-id 0); fi

  RAYON_NUM_THREADS="${threads}" RUST_LOG="${RUST_LOG:-info}" \
    caffeinate -dimsu /usr/bin/time -l "${E2E}" \
      "${GUESTS}/${elf_name}" \
      "${proof}" \
      "${vk}" \
      --platform=ceno \
      "${prof_args[@]}" \
      --stack-size "${STACK}" \
      --heap-size "${HEAP}" \
      --max-cycle-per-shard "${MAX_CYCLE_PER_SHARD}" \
      --max-cell-per-shard "${MAX_CELL_PER_SHARD}" \
      --hints-file "${TASKS}/${task}.hints.bin" \
      --public-io "${pio}" \
    > "${stdout}" 2> "${log}"
  local rc=$?

  read -r M1 W1 <<< "$(python3 "${CLOCKPROBE}" mark)"
  read -r mono wall slept verdict <<< "$(python3 "${CLOCKPROBE}" diff "${M0}" "${W0}" "${M1}" "${W1}")"

  local real user sys ratio rss footprint status create insns cycles shards pbytes vbytes
  real="$(grep -Eo '^ *[0-9.]+ real' "${log}" | awk '{print $1}' | tail -1)"
  user="$(grep -Eo '[0-9.]+ user' "${log}" | awk '{print $1}' | tail -1)"
  sys="$(grep -Eo '[0-9.]+ sys' "${log}" | awk '{print $1}' | tail -1)"
  rss="$(grep 'maximum resident set size' "${log}" | awk '{print $1}' | tail -1)"
  footprint="$(grep 'peak memory footprint' "${log}" | awk '{print $1}' | tail -1)"
  ratio="$(awk -v u="${user:-0}" -v s="${sys:-0}" -v r="${real:-0}" 'BEGIN{ if (r>0) printf "%.4f", (u+s)/r; else print "" }')"
  # Ceno's tracing output goes to STDOUT; `/usr/bin/time -l` writes to STDERR. The two are
  # captured to separate files and each field is read from the stream that carries it.
  create="$(extract_create_proof "${stdout}")"
  # `program executed <n> instructions in <m> cycles` and `num_shards: <k>, ...` are Ceno's
  # own log lines (ceno_zkvm::e2e), so the cycle count is the system reporting on itself.
  insns="$(grep -Eo 'program executed [0-9]+ instructions' "${stdout}" | grep -Eo '[0-9]+' | tail -1)"
  cycles="$(grep -Eo 'program executed [0-9]+ instructions in [0-9]+ cycles' "${stdout}" | awk '{print $(NF-1)}' | tail -1)"
  shards="$(grep -Eo 'num_shards: [0-9]+' "${stdout}" | grep -Eo '[0-9]+' | tail -1)"
  pbytes=""; [[ -f "${proof}" ]] && pbytes="$(stat -f%z "${proof}")"
  vbytes="";  [[ -f "${vk}" ]] && vbytes="$(stat -f%z "${vk}")"
  # The vk is ~90 MB and does not vary between repetitions of a cell; keeping one per
  # repetition fills the boot volume, and a full volume destabilises the machine (the reason
  # binius64's campaign needed a disk watchdog). Sizes are recorded before the deletion.
  if [[ "${rep}" != "${REPS}" ]]; then rm -f "${proof}" "${vk}"; fi

  if [[ "${verdict}" == "INVALID_SLEEP" ]]; then
    status="INVALID_SLEEP"
  elif [[ ${rc} -eq 0 ]]; then
    status="OK"
  else
    status="FAIL_rc${rc}"
  fi

  echo "${label},${task},${elf_name},${threads},${MAX_CYCLE_PER_SHARD},${MAX_CELL_PER_SHARD},${WARMUP},${REPS},${rep},${is_warmup},${status},${real:-},${user:-},${sys:-},${ratio:-},${rss:-},${footprint:-},${create:-},${insns:-},${cycles:-},${shards:-},${pbytes:-},${vbytes:-},${mono},${wall},${slept},${verdict},${load1},${swap:-},${started}" >> "${LEDGER}"
  echo "[ceno] ${label} ${tag} -> ${status} real=${real:-?}s create_proof=${create:-?}s cycles=${cycles:-?} shards=${shards:-?} rss=${rss:-?}B fp=${footprint:-?}B slept=${slept}s" >&2
  return ${rc}
}

for spec in "$@"; do
  IFS=: read -r task threads <<< "${spec}"
  threads="${threads:-1}"
  if [[ "${WARMUP}" -gt 0 ]]; then
    run_rep "${task}" "${threads}" 0 1 || echo "[ceno] ${spec} warmup did not complete — continuing" >&2
  fi
  for ((r = 1; r <= REPS; r++)); do
    run_rep "${task}" "${threads}" "${r}" 0 || echo "[ceno] ${spec} rep ${r} did not complete — continuing" >&2
  done
done
