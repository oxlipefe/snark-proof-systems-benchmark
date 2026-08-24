#!/usr/bin/env python3
"""zk-prover-bench · DeepProve · map the region of the proof artifact where a corrupted byte
is still accepted by `deep-prove-cli verify`.

WHY THIS EXISTS. The correctness control (run-negative.sh) found that some single-bit
corruptions of the serialized artifact are ACCEPTED. That is the most consequential
observation in this campaign, so it is measured rather than argued: this script walks the
artifact and records, byte by byte over a chosen range, whether the system's own verifier
CLI accepts the mutated file.

WHAT IS AND IS NOT CLAIMED. The artifact is
`Output { outputs: Vec<Tensor<Element>>, proof: Provable { proof, io, ctx } }`
(deep-prove/src/middleware/v1.rs:41-46, v2.rs:14-19) — those are Rust struct declarations,
read from the source, not a binary format recovered by reverse engineering. `Provable::verify`
is `verify(&self.ctx, self.proof, self.io)` (v2.rs:20-24): the `outputs` field is **not an
argument to it**. Everything beyond that — exactly which byte belongs to which field — is NOT
determined here and is not claimed, because establishing it would mean reverse engineering
the serialization, which DeepProve's license forbids.

Output is a raw CSV. No interpretation is written into it.
"""

import base64
import csv
import pathlib
import subprocess
import sys


def verdict(cli, path):
    proc = subprocess.run([cli, "verify", str(path)], capture_output=True, text=True)
    blob = proc.stdout + proc.stderr
    if proc.returncode == 0 and "Proof verified successfully" in blob:
        return "VERIFY_ACCEPTED", ""
    if "failed to deserialize proof" in blob or "decoding base64" in blob:
        return "DESERIALIZE_REJECTED", " ".join(blob.split())[-120:]
    return "VERIFY_REJECTED", " ".join(blob.split())[-120:]


def main():
    cli, honest, out_csv = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
    task = sys.argv[4]
    raw = base64.b64decode(honest.read_bytes())
    n = len(raw)

    # Offsets: a fine walk over the head (where the declared outputs live), a coarse walk
    # over the body, and every byte of the tail.
    head = list(range(0, min(n, 4096), 64))
    body = [round(n * f) for f in (0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95)]
    tail = list(range(max(0, n - 32), n))
    offsets = sorted({min(max(o, 0), n - 1) for o in head + body + tail})

    scratch = out_csv.parent / f"scan-{task}"
    scratch.mkdir(parents=True, exist_ok=True)
    rows = []
    for pos in offsets:
        mutated = bytearray(raw)
        before = mutated[pos]
        mutated[pos] ^= 1
        probe = scratch / "probe.postcard"
        probe.write_bytes(base64.b64encode(bytes(mutated)))
        outcome, detail = verdict(cli, probe)
        rows.append({
            "task": task, "artifact_bytes": n, "offset": pos,
            "offset_fraction": f"{pos / n:.6f}",
            "byte_before": f"0x{before:02x}", "byte_after": f"0x{mutated[pos]:02x}",
            "outcome": outcome, "detail": detail,
        })
        print(f"{task} {pos:>8} ({pos / n:6.2%})  {outcome}", file=sys.stderr)
    probe.unlink(missing_ok=True)

    write_header = not out_csv.exists()
    with out_csv.open("a", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        if write_header:
            w.writeheader()
        w.writerows(rows)
    accepted = [r for r in rows if r["outcome"] == "VERIFY_ACCEPTED"]
    print(f"\n{task}: {len(accepted)}/{len(rows)} offsets ACCEPTED; artifact {n} bytes",
          file=sys.stderr)
    if accepted:
        print(f"  accepted offsets: {[r['offset'] for r in accepted]}", file=sys.stderr)


if __name__ == "__main__":
    main()
