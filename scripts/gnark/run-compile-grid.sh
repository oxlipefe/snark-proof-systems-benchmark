#!/usr/bin/env bash
# zk-prover-bench · gnark · constraint counts for every task x regime x backend, FROM
# COMPILATION ALONE.
#
# Constraint count is gnark's natural unit the way cycles were Ceno's, and compiling is far
# cheaper than a Groth16 setup. The grid therefore climbs several rungs past where a measured
# cell fits in memory, and the rung where COMPILING ITSELF dies is a reported result.
#
# ONE PROCESS PER CELL so that a compile that dies of memory does not take the rest of the
# grid with it.
set -uo pipefail
# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly BIN="${BIN:-${ROOT}/tasks/gnark/bin/gnark-compile-grid}"
readonly OUT="${ROOT}/data/compile-grid-gnark.csv"
readonly TASKS="${TASKS:-t1-0 t2 t1-a t3 t1-b t1-c t1-d}"
readonly BACKENDS="${BACKENDS:-groth16 plonk}"
readonly REGIMES="${REGIMES:-A B}"

echo "label,backend,regime,gadget,status,macs,macs_emitted,relus,max_abs_intermediate,static_worst_case,relu_bits,constraints,internal_vars,secret,public,coefficients,instructions,domain_derived,domain_derived_from,padding_ratio,constraints_per_mac,compile_ms,go_heap_alloc_bytes,go_sys_bytes,msg" > "${OUT}"

for t in ${TASKS}; do
  for b in ${BACKENDS}; do
    for r in ${REGIMES}; do
      line="$(caffeinate -dimsu "${BIN}" "${t}" "${b}" "${r}" 2>/dev/null | grep '^CELL ' | head -1)"
      if [[ -z "${line}" ]]; then
        # No CELL line at all means the process died before it could print one — an OOM kill
        # or a runtime abort. That is data: the rung where compiling itself dies.
        echo "${t},${b},${r},,COMPILE_DIED_NO_OUTPUT,,,,,,,,,,,,,,,,,,,,process_produced_no_CELL_line" >> "${OUT}"
        echo "[grid] ${t}/${b}/${r} -> COMPILE_DIED_NO_OUTPUT" >&2
        continue
      fi
      python3 - "${line}" >> "${OUT}" <<'PYEOF'
import sys
kv = dict(p.split("=", 1) for p in sys.argv[1].split()[1:] if "=" in p)
cols = ["label","backend","regime","gadget","status","macs","macs_emitted","relus",
        "max_abs_intermediate","static_worst_case","relu_bits","constraints",
        "internal_vars","secret","public","coefficients","instructions","domain_derived",
        "domain_derived_from","padding_ratio","constraints_per_mac","compile_ms",
        "go_heap_alloc_bytes","go_sys_bytes","msg"]
print(",".join(kv.get(c, "") for c in cols))
PYEOF
      echo "[grid] ${line}" >&2
    done
  done
done
echo "[grid] -> ${OUT}" >&2
