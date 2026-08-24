#!/usr/bin/env bash
# zk-prover-bench · DeepProve · reproduction of the system's own published reference number.
#
# bench/README.md "Fairness protocol" step 3 commits this repository to reproducing each
# measured system's own published reference number BEFORE reporting anything about that
# system, and to publishing the discrepancy above the result if we cannot.
#
# For DeepProve the reference is Table 1 of ePrint 2026/1112 (GPT-2, 12-bit quantization),
# on the paper's primary machine: AMD EPYC 9254, 24 cores / 48 threads, 504 GB RAM.
# We run it here on an Apple M1 Max, 10 cores, 32 GiB. The hardware is not comparable and
# no pass/fail is declared from the ratio alone; what is being checked is whether the
# figures land in the regime the paper describes and whether the run completes at all.
#
# DeepProve's source tree is NOT vendored into this repository. Its license (Lagrange
# License, not OSI) permits downloading and internally using the software "solely for the
# purpose of testing and evaluating it" and forbids reproducing or distributing it. So the
# tree is cloned outside the repository and only the pinned commit, the commands and OUR
# measurements are kept here. DP_ROOT must point at that clone.
#
# Everything runs under `caffeinate -dimsu` and is bracketed by the clock probe, for the
# reasons documented in bench/systems/binius64/BUILD.md §3: the machine idle-slept in the
# middle of a timed run once, and wall-clock seconds that include sleep are garbage that
# looks like data.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly DP_ROOT="${DP_ROOT:?set DP_ROOT to the deep-prove clone outside this repository}"
readonly CLOCKPROBE="${ROOT}/scripts/clockprobe.py"
readonly DATA="${ROOT}/data/repro-deepprove"

SEQ="${SEQ:-64}"
THREADS="${THREADS:-}"          # empty => bench-llm default (all logical cores)
LABEL="${LABEL:-gpt2-seq${SEQ}}"

mkdir -p "${DATA}/${LABEL}"
run_dir="${DATA}/${LABEL}"

# bench-llm resolves the Hugging Face cache as ./model_cache relative to the working
# directory (zkml/src/parser/mod.rs:157, zkml/src/parser/safe.rs:115), so the run happens
# in a scratch directory holding a symlink to the clone's cache. Running inside the clone
# itself would write our CSV output into their tree.
work="$(mktemp -d)"
ln -s "${DP_ROOT}/model_cache" "${work}/model_cache"

started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
load1="$(sysctl -n vm.loadavg | awk '{print $2}')"
swap="$(sysctl -n vm.swapusage | sed -E 's/.*used = ([0-9.]+)M.*/\1/')"
read -r M0 W0 <<< "$(python3 "${CLOCKPROBE}" mark)"

# BIN selects which build is under test. The default is the configuration DeepProve's own
# README documents (`cargo build --release`). DP_BIN can point at the debug-assertions-off
# build instead; see REPRODUCTION.md for why that second build exists.
BIN="${DP_BIN:-${DP_ROOT}/target/release/bench-llm}"

args=(--model gpt2 --hf openai-community/gpt2 --sequence "${SEQ}")
[[ -n "${THREADS}" ]] && args+=(--num-threads "${THREADS}")

( cd "${work}" && caffeinate -dimsu /usr/bin/time -l \
    "${BIN}" "${args[@]}" ) \
  > "${run_dir}/stdout.txt" 2> "${run_dir}/time.txt"
rc=$?

read -r M1 W1 <<< "$(python3 "${CLOCKPROBE}" mark)"
read -r mono wall slept verdict <<< "$(python3 "${CLOCKPROBE}" diff "${M0}" "${W0}" "${M1}" "${W1}")"

# DeepProve's own CSV output, kept verbatim: bench.csv is its per-trial row (including its
# own prove_full_memory_peak column) and bench-llm-deprecated.csv its per-method profile.
for f in bench.csv bench-llm-deprecated.csv bench_distributed.csv; do
  [[ -f "${work}/${f}" ]] && cp "${work}/${f}" "${run_dir}/${f}"
done

{
  echo "label=${LABEL} seq=${SEQ} threads=${THREADS:-default} rc=${rc}"
  echo "binary=${BIN}"
  echo "started_utc=${started} loadavg_1m=${load1} swap_used_mb=${swap}"
  echo "mono_s=${mono} wall_s=${wall} slept_s=${slept} sleep_verdict=${verdict}"
} > "${run_dir}/cell.txt"

echo "[repro] ${LABEL} rc=${rc} slept=${slept}s -> ${run_dir}" >&2
rm -rf "${work}"
exit ${rc}
