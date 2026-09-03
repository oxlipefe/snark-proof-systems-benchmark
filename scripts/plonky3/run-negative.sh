#!/usr/bin/env bash
# zk-prover-bench · Plonky3 · the correctness control. BLOCKING.
#
# It runs BEFORE the timings it licenses. A number produced by a system that accepts corrupt
# proofs is worse than no number. Exits non-zero if any corruption was ACCEPTED.
set -euo pipefail
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly HARNESS="${ROOT}/scripts/plonky3/harness/target/release/p3-negative"
readonly OUT="${ROOT}/data/negative-plonky3"

[[ -x "${HARNESS}" ]] || { echo "[plonky3] build first: scripts/plonky3/build.sh" >&2; exit 1; }
mkdir -p "${OUT}"
RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-1}" caffeinate -dimsu "${HARNESS}" "$@" --out-dir "${OUT}"
