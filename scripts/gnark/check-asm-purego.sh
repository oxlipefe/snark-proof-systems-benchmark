#!/usr/bin/env bash
# zk-prover-bench · gnark · THE BUILD-INTEGRITY CHECK, done by measurement.
#
# bench/README.md requires a per-system build-integrity check because in E-001 compiling our
# own prover without LTO made it measure 9.0x slower and inverted a verdict. Arguing "gnark
# exposes no tunables" is not that check. This is: build the SAME runner twice, once normally
# and once with `-tags purego`, and measure both on the same task back to back.
#
# gnark-crypto selects its BN254 field implementation by build tag:
#     ecc/bn254/fp/element_arm64.go   //go:build !purego
#     ecc/bn254/fp/element_purego.go  //go:build purego || (!amd64 && !arm64)
# so `-tags purego` deselects the assembly. If the two builds measure the SAME, the assembly
# was never in the binary we measured and every timing in RESULTS.md is a purego timing.
#
# This is a RATIO taken back to back on one machine, so it survives ambient load far better
# than either absolute number does. The loadavg is recorded anyway.
set -uo pipefail
# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly PKG="${ROOT}/tasks/gnark"
readonly OUT="${ROOT}/data/repro-gnark"
readonly TASK="${1:-t1-0}"
readonly REPS="${GNARK_REPS:-5}"

mkdir -p "${OUT}"
cd "${PKG}"

echo "=== building both variants from the same source tree ==="
caffeinate -dimsu go build -o /tmp/gnark-runner-asm    ./runner || exit 1
caffeinate -dimsu go build -tags purego -o /tmp/gnark-runner-purego ./runner || exit 1
ls -la /tmp/gnark-runner-asm /tmp/gnark-runner-purego

echo
echo "=== which object files each binary actually contains ==="
# The asm build must reference the arm64 assembly symbols; the purego build must not.
for b in asm purego; do
  n=$(go tool nm "/tmp/gnark-runner-${b}" 2>/dev/null | grep -cE 'ecc/bn254/fp\.(mul|reduce)|element_4w' || true)
  echo "  ${b}: ${n} matching symbol(s)"
done

echo
for v in asm purego; do
  la=$(uptime | sed 's/.*load averages: //')
  echo "=== ${v}  task=${TASK}  reps=${REPS}  loadavg=${la} ==="
  GNARK_BACKEND=groth16 GNARK_REGIME=A GNARK_REPS="${REPS}" GNARK_WARMUP=1 \
    caffeinate -dimsu /usr/bin/time -l "/tmp/gnark-runner-${v}" "${TASK}" \
      > "${OUT}/asmcheck-${TASK}-${v}.txt" 2> "${OUT}/asmcheck-${TASK}-${v}.log"
  echo "  exit=$?"
  grep -E '^META|^SETUP|^DONE' "${OUT}/asmcheck-${TASK}-${v}.txt" | head -3
  grep -E 'prove_ms=' "${OUT}/asmcheck-${TASK}-${v}.txt" | tail -3
  grep -E ' real| maximum resident| peak memory' "${OUT}/asmcheck-${TASK}-${v}.log"
  echo
done
echo "Compare the median prove_ms. If they are within noise, the assembly is NOT being used."
