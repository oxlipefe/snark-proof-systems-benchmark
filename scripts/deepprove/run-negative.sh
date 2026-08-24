#!/usr/bin/env bash
# zk-prover-bench · DeepProve · the blocking correctness control.
#
# A corrupted trace must make verify() fail. Without this control you are not benchmarking
# proofs — you are benchmarking computations that happen to produce bytes. If DeepProve
# fails this, no DeepProve number is published.
#
# HOW THIS DIFFERS FROM THE binius64 CONTROL, AND WHY.
#
# For binius64 (Apache-2.0 / MIT) the control corrupts a witness word *inside* the prover:
# it mutates the private ValueVec after an honest witness is built, then proves and
# verifies. That is the stronger test, because it makes the prover produce a proof of a
# false statement and asks the verifier to catch it.
#
# DeepProve's license is not OSI. It permits downloading and internally using the software
# "solely for the purpose of testing and evaluating it" — which is exactly what this is —
# but its §4 forbids creating derivative works and forbids reverse engineering. So:
#
#   * no DeepProve code is modified, linked against, or copied anywhere;
#   * nothing inside the prover is instrumented or reached into;
#   * the corruption is applied to the SERIALIZED PROOF ARTIFACT, from outside, and the
#     system's own public verifier CLI (`deep-prove-cli verify`) is asked to judge it.
#
# That covers binius64's `proof_byte` family. It does NOT cover binius64's `private_word`
# family, and that gap is declared in RESULTS.md rather than papered over.
#
# One further consequence, stated because it is a real limit and not an oversight: we do not
# map byte offsets to fields of the artifact. Working out which bytes are the declared
# output, which are the proof and which are the verifier context would mean reverse
# engineering the serialization format, which the license forbids. Offsets are therefore
# reported as fractions of the artifact, and no claim is made about what each one hit.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly DP_ROOT="${DP_ROOT:?set DP_ROOT to the deep-prove clone outside this repository}"
readonly TASKS="${ROOT}/tasks/deepprove"
readonly DATA="${ROOT}/data/negative-deepprove"
readonly WORKER="${DP_ROOT}/target/release/deep-prove-worker"
readonly CLI="${DP_ROOT}/target/release/deep-prove-cli"
readonly PORT="${PORT:-18080}"
readonly URL="http://localhost:${PORT}"
readonly BITLEN="${BITLEN:-8}"

mkdir -p "${DATA}"
CSV="${DATA}/negative-control.csv"
echo "task,mode,offset_bytes,offset_fraction,outcome,passed,detail" > "${CSV}"

work="$(mktemp -d)"
cleanup() { [[ -n "${WPID:-}" ]] && kill "${WPID}" 2>/dev/null; rm -rf "${work}"; }
trap cleanup EXIT

# One worker for the whole run, in its own scratch directory.
( cd "${work}" && ZKML_BIT_LEN="${BITLEN}" RUST_LOG=info \
    caffeinate -dimsu "${WORKER}" --tensor-store temporary local-api --port "${PORT}" \
    > "${DATA}/worker.log" 2>&1 ) &
WPID=$!
for _ in $(seq 1 60); do
  curl -s -o /dev/null "${URL}/proofs" && break
  sleep 1
done

for task in "$@"; do
  out_dir="${DATA}/${task}"; mkdir -p "${out_dir}"
  python3 - "${TASKS}/${task}.io.json" "${out_dir}/in1.json" <<'PYEOF'
import json, sys
json.dump({"input_data": json.load(open(sys.argv[1]))["input_data"][:1]}, open(sys.argv[2], "w"))
PYEOF

  ( cd "${out_dir}" && rm -f ./*.postcard
    "${CLI}" local-api --worker-url "${URL}" submit --onnx "${TASKS}/${task}.onnx" \
        --inputs "${out_dir}/in1.json" > submit.log 2>&1
    for _ in $(seq 1 600); do
      "${CLI}" local-api --worker-url "${URL}" fetch honest > fetch.log 2>&1
      ls ./honest*.postcard >/dev/null 2>&1 && break
      sleep 1
    done )

  honest="$(ls "${out_dir}"/honest*.postcard 2>/dev/null | head -1)"
  if [[ -z "${honest}" ]]; then
    echo "${task},NO_PROOF,,,PROOF_NOT_PRODUCED,false,\"worker produced no proof for this task\"" >> "${CSV}"
    echo "[neg] ${task}: no proof produced" >&2
    continue
  fi

  # POSITIVE CONTROL FIRST. A negative test that passes because nothing ever verifies proves
  # nothing, so the honest artifact is checked before any corruption is attempted.
  if "${CLI}" verify "${honest}" > "${out_dir}/verify-honest.log" 2>&1 &&
     grep -q "Proof verified successfully" "${out_dir}/verify-honest.log"; then
    echo "${task},honest/positive-control,,,VERIFY_ACCEPTED,true,\"honest artifact verifies\"" >> "${CSV}"
  else
    echo "${task},honest/positive-control,,,HONEST_REJECTED,false,\"honest artifact did NOT verify — control is vacuous, task aborted\"" >> "${CSV}"
    echo "[neg] ${task}: HONEST PROOF DID NOT VERIFY — aborting this task" >&2
    continue
  fi

  # Corruptions: one flipped bit at a spread of offsets through the decoded artifact.
  python3 - "${honest}" "${out_dir}" "${task}" "${CSV}" "${CLI}" <<'PYEOF'
import base64, pathlib, subprocess, sys

src, out_dir, task, csv_path, cli = sys.argv[1:6]
out_dir = pathlib.Path(out_dir)
raw = base64.b64decode(pathlib.Path(src).read_bytes())
n = len(raw)

# Offsets spread across the artifact. The file is `Output { outputs, proof: Provable {
# proof, io, ctx } }` in postcard, so early offsets are nearer the declared model output and
# later ones nearer the verifier context — but WHICH field an offset lands in is not
# determined here, deliberately: establishing that would mean reverse engineering the
# format, which DeepProve's license forbids.
positions = [0, 1, 7, n // 100, n // 10, n // 4, n // 2, (3 * n) // 4, n - 9, n - 2, n - 1]
seen, rows = set(), []
for pos in positions:
    pos = max(0, min(pos, n - 1))
    if pos in seen:
        continue
    seen.add(pos)
    before = raw[pos]
    mutated = bytearray(raw)
    mutated[pos] ^= 1                       # single-bit flip, as in the binius64 control
    dst = out_dir / f"corrupt-{pos}.postcard"
    dst.write_bytes(base64.b64encode(bytes(mutated)))

    proc = subprocess.run([cli, "verify", str(dst)], capture_output=True, text=True)
    blob = (proc.stdout + proc.stderr)
    accepted = proc.returncode == 0 and "Proof verified successfully" in blob
    if accepted:
        outcome = "VERIFY_ACCEPTED"
    elif "failed to deserialize proof" in blob or "decoding base64" in blob:
        outcome = "DESERIALIZE_REJECTED"
    else:
        outcome = "VERIFY_REJECTED"
    detail = " ".join(blob.split())[-160:]
    rows.append(f'{task},proof_byte/offset,{pos},{pos / n:.4f},{outcome},'
                f'{"false" if accepted else "true"},'
                f'"byte[{pos}] 0x{before:02x} -> 0x{mutated[pos]:02x} (low bit flipped); {detail}"')
    print(f"[neg] {task} offset {pos} ({pos / n:.1%}) -> {outcome}", file=sys.stderr)

with open(csv_path, "a") as fh:
    fh.write("\n".join(rows) + "\n")
PYEOF
done

echo "[neg] combined -> ${CSV}" >&2
