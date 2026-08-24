#!/usr/bin/env bash
# zk-prover-bench · DeepProve · the whole grid, in one pass.
#
# Every cell is reported, including the ones that do not run. bench/README.md commits to
# publishing the full grid with the reason for each empty cell, so the tasks DeepProve's
# ONNX frontend rejects are ATTEMPTED here rather than assumed to fail: the rejection is a
# measurement with a message, not an inference from reading the source.
set -uo pipefail

readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly CELL="${HERE}/run-cell.sh"

# 1 · The primary cut: INT8 (ZKML_BIT_LEN=8, matching bench/TASKS.md) at RAYON_NUM_THREADS=1.
for t in 1 10; do
  BITLEN=8 THREADS="${t}" "${CELL}" t1-0:t1-0:6 t1-a:t1-a:6 t2:t2:6
done

# 2 · T3 — DeepProve cannot express a batch of 8 in one proof (see NOT_EXPRESSIBLE.md), so
#     it is run the way bench/TASKS.md says such a system reports it: 8 independent inputs,
#     8 separate proofs, in one process. WARMUP=0 because all 8 belong to the batch.
for t in 1 10; do
  BITLEN=8 THREADS="${t}" WARMUP=0 "${CELL}" t3-as-8:t2:8
done

# 3 · Control: DeepProve's own default quantization (12-bit) on the same task, so the INT8
#     figure above is never compared with their published numbers without this beside it.
BITLEN=12 THREADS=1 "${CELL}" t2:t2:6
BITLEN=12 THREADS=1 "${CELL}" t1-0:t1-0:6

# 4 · The cells that are expected to be rejected. One sample each: the point is the error
#     message and the fact of rejection, not a timing.
BITLEN=8 THREADS=1 "${CELL}" t1-b:t1-b:1 t1-c:t1-c:1 t1-d:t1-d:1 t3-batch8:t3:1
