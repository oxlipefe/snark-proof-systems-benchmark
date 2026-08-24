#!/usr/bin/env python3
"""zk-prover-bench · DeepProve · two follow-ups to the correctness control.

`scan-accepted-region.py` flips ONE bit (`^0x01`) per offset. Two questions it cannot answer,
both of which bear on how the accepted-corruption finding should be read:

1. **Is the method itself sound?** If decoding the artifact and re-encoding it unchanged did
   not verify, every "rejected" result would be an artefact of our own round trip rather than
   the verifier catching anything. This is the control for that, and it must run first.

2. **Are the accepted tail offsets ignored, or only partly checked?** A byte that is never
   read would accept every mutation. A small-valued field that is checked loosely would accept
   some and reject others. Those are different findings, and only a multi-pattern probe
   separates them.

Raw CSV out. No interpretation is written into it.
"""

import argparse
import base64
import csv
import pathlib
import subprocess
import tempfile

PATTERNS = [0x01, 0x02, 0x08, 0x80, 0xFF]


def verdict(cli, path):
    proc = subprocess.run([cli, "verify", str(path)], capture_output=True, text=True)
    blob = proc.stdout + proc.stderr
    if proc.returncode == 0 and "Proof verified successfully" in blob:
        return "VERIFY_ACCEPTED", ""
    if "failed to deserialize proof" in blob or "decoding base64" in blob:
        return "DESERIALIZE_REJECTED", " ".join(blob.split())[-120:]
    return "VERIFY_REJECTED", " ".join(blob.split())[-120:]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cli", required=True)
    ap.add_argument("--artifact", required=True)
    ap.add_argument("--task", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    raw = base64.b64decode(pathlib.Path(args.artifact).read_bytes())
    n = len(raw)
    tmp = pathlib.Path(tempfile.mkdtemp())
    probe = tmp / "probe.postcard"
    rows = []

    # 1 · Round-trip control: decode and re-encode with NO mutation.
    probe.write_bytes(base64.b64encode(raw))
    outcome, detail = verdict(args.cli, probe)
    rows.append({"task": args.task, "artifact_bytes": n, "offset": "", "offset_from_end": "",
                 "byte_before": "", "pattern": "none (round-trip control)",
                 "outcome": outcome, "detail": detail})
    print(f"[bits] {args.task} round-trip control -> {outcome}")
    if outcome != "VERIFY_ACCEPTED":
        print("[bits] ROUND TRIP FAILED — every other result in this file is meaningless")

    # 2 · The three tail offsets the scan found accepted, plus their neighbours as contrast.
    targets = [n - 29, n - 15, n - 1, n - 2, n - 14, n - 16, n - 30]
    for off in targets:
        if not (0 <= off < n):
            continue
        for mask in PATTERNS:
            mutated = bytearray(raw)
            before = mutated[off]
            mutated[off] ^= mask
            probe.write_bytes(base64.b64encode(bytes(mutated)))
            outcome, detail = verdict(args.cli, probe)
            rows.append({"task": args.task, "artifact_bytes": n, "offset": off,
                         "offset_from_end": f"n-{n - off}",
                         "byte_before": f"0x{before:02x}", "pattern": f"^0x{mask:02x}",
                         "outcome": outcome, "detail": detail})
            print(f"[bits] {args.task} n-{n - off:<3} 0x{before:02x} ^0x{mask:02x} -> {outcome}")

    out = pathlib.Path(args.out)
    write_header = not out.exists()
    with out.open("a", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        if write_header:
            w.writeheader()
        w.writerows(rows)
    probe.unlink(missing_ok=True)
    print(f"[bits] -> {out}")


if __name__ == "__main__":
    main()
