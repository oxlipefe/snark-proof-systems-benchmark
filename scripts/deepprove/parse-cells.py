#!/usr/bin/env python3
"""zk-prover-bench · DeepProve · build the results table from the raw, uncurated cell data.

Two rules this script exists to enforce, the same two the binius64 reporter enforces:

1. **Numerator and denominator come from the same run.** `MAC/s` divides the MAC count
   frozen in bench/TASKS.md by a prove time read out of *that cell's own log*. No ratio here
   composes figures from different cells.

2. **`MAC/s` and `bytes/MAC` are always emitted together.** A prover twice as fast that uses
   three times the memory is worse, not better.

WHERE THE NUMBERS COME FROM. DeepProve's internals are not instrumented — its license is not
OSI and forbids derivative works. Everything below is read from outside the process:

  * memory and CPU: `/usr/bin/time -l`, recorded per cell in bench/data/cells-deepprove.csv
  * time boundaries: the timestamps DeepProve's own tracing layer prints to stdout, at the
    four INFO/DEBUG markers its worker emits (deep-prove/src/bin/worker/main.rs:94, :107,
    :131, :146-186 loop, :186 "Proving done.").

WHAT THE `prove` BRACKET CONTAINS, stated because it is wider than binius64's. Each
repetition is bracketed by consecutive "Running input" markers, and the last by
"Proving done.". Between those markers DeepProve does `model.reset()`, a tensor-store run
scope, `load_input_flat`, **quantized inference** (`model.run`) and `Prover::prove`
(deep-prove/src/bin/worker/main.rs:146-180). Inference is therefore INSIDE the prove figure.
DeepProve's own LLM benchmark separates `inference_time` from `prove_full`; the ONNX worker
path emits no marker between them, so the split is NOT DETERMINED here and the combined
figure is what is published, labelled as such.
"""

import csv
import json
import pathlib
import re
import statistics
import sys
from datetime import datetime

ROOT = pathlib.Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
CELLS = DATA / "cells-deepprove"
LEDGER = DATA / "cells-deepprove.csv"
MANIFEST = ROOT / "tasks" / "deepprove" / "manifest.json"

ANSI = re.compile(r"\x1b\[[0-9;]*m")
TS = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+)Z")

# bench/TASKS.md, frozen. t3-as-8 is eight independent T2 proofs, so its denominator is the
# published T3 count and its numerator is the total time of those eight proofs.
MACS = {
    "t1-0": 65_536,
    "t1-a": 589_824,
    "t1-b": 2_359_296,
    "t1-c": 9_437_184,
    "t1-d": 37_748_736,
    "t2": 92_224,
    "t3": 737_792,
    "t3-as-8": 737_792,
    "t3-batch8": 737_792,
}


def parse_ts(line):
    m = TS.match(line)
    return datetime.strptime(m.group(1)[:26], "%Y-%m-%dT%H:%M:%S.%f") if m else None


def parse_log(path):
    """Pull the timing markers out of one cell's stdout."""
    marks = {"prepared": None, "ctx_start": None, "ctx_stored": None,
             "inputs": [], "done": None, "error": None, "threads_note": None}
    if not path.exists():
        return marks
    for raw in path.read_text(errors="replace").splitlines():
        line = ANSI.sub("", raw)
        ts = parse_ts(line)
        if "Model prepared:" in line:
            marks["prepared"] = ts
        elif "Generating proving and verifier contexts" in line:
            marks["ctx_start"] = ts
        elif "Stored generated proving parameters" in line:
            marks["ctx_stored"] = ts
        elif "Running input" in line:
            marks["inputs"].append(ts)
        elif "Proving done." in line:
            marks["done"] = ts
        elif "is not power of 2, using" in line:
            marks["threads_note"] = line.split(":")[-1].strip()
    return marks


def secs(a, b):
    return None if (a is None or b is None) else (b - a).total_seconds()


def main():
    if not LEDGER.exists():
        sys.exit(f"no ledger at {LEDGER}")
    manifest = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else {}
    rows = list(csv.DictReader(LEDGER.open()))

    out = []
    for cell in rows:
        label = cell["label"]
        task = cell["task"]
        marks = parse_log(CELLS / label / "stdout.txt")
        warmup = int(cell["warmup"] or 0)

        # Per-repetition prove times: consecutive "Running input" markers, the last closed
        # by "Proving done.".
        stamps = marks["inputs"]
        reps = []
        if stamps and marks["done"]:
            edges = stamps + [marks["done"]]
            reps = [secs(edges[i], edges[i + 1]) for i in range(len(stamps))]
        timed = reps[warmup:] if len(reps) > warmup else []

        macs = MACS.get(task)
        footprint = int(cell["peak_footprint_bytes"]) if cell["peak_footprint_bytes"] else None
        rss = int(cell["peak_rss_bytes"]) if cell["peak_rss_bytes"] else None

        if task.startswith("t3-as"):
            # T3 is the whole batch of 8, in one proof each, so its time is the total of the
            # eight and its MAC count is the published T3 total. Numerator and denominator
            # come from the same run.
            prove_med = sum(timed) if timed else None
            prove_min = prove_max = prove_med
            n_used = len(timed)
        else:
            prove_med = statistics.median(timed) if timed else None
            prove_min = min(timed) if timed else None
            prove_max = max(timed) if timed else None
            n_used = len(timed)

        # A cell that never produced a proof has no rate and no bytes-per-MAC. Its process
        # still had a memory peak — the peak of loading a model and then failing — and
        # dividing that by a MAC count the system never performed would manufacture a
        # number out of a crash. The peaks stay in the raw ledger; the derived columns stay
        # empty, and the status column says why.
        proved = bool(timed)
        rate = (macs / prove_med) if (proved and prove_med and macs) else None
        bpm_fp = (footprint / macs) if (proved and footprint and macs) else None
        bpm_rss = (rss / macs) if (proved and rss and macs) else None

        out.append({
            "label": label,
            "task": task,
            "macs": macs,
            "bitlen": cell["bitlen"],
            "rayon_threads": cell["threads"],
            "status": cell["status"],
            "N": n_used,
            "warmup": warmup,
            "prove_s_median": round(prove_med, 4) if prove_med else "",
            "prove_s_min": round(prove_min, 4) if prove_min else "",
            "prove_s_max": round(prove_max, 4) if prove_max else "",
            "parse_quantize_s": "",
            "setup_ctx_s": round(secs(marks["ctx_start"], marks["ctx_stored"]), 4)
                           if secs(marks["ctx_start"], marks["ctx_stored"]) is not None else "",
            "ctx_store_roundtrip_s": round(secs(marks["ctx_stored"], stamps[0]), 4)
                                     if (stamps and secs(marks["ctx_stored"], stamps[0]) is not None) else "",
            "real_s": cell["real_s"],
            "cpu_ratio": cell["cpu_ratio"],
            "peak_rss_bytes": rss or "",
            "peak_footprint_bytes": footprint or "",
            "MAC_per_s": round(rate) if rate else "",
            "bytes_per_MAC_footprint": round(bpm_fp, 1) if bpm_fp else "",
            "bytes_per_MAC_rss": round(bpm_rss, 1) if bpm_rss else "",
            "sumcheck_thread_note": marks["threads_note"] or "",
            "swap_used_mb": cell["swap_used_mb"],
            "loadavg_1m": cell["loadavg_1m"],
            "sleep_verdict": cell["sleep_verdict"],
            "onnx_padded_note": manifest.get(task, {}).get("input_shape", ""),
        })

    dest = DATA / "results-deepprove.csv"
    with dest.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(out[0].keys()))
        w.writeheader()
        w.writerows(out)
    print(f"wrote {dest} ({len(out)} cells)")

    for r in out:
        print(f"{r['label']:<22} {r['status']:<15} N={r['N']} "
              f"prove={r['prove_s_median']}s MAC/s={r['MAC_per_s']} "
              f"B/MAC(fp)={r['bytes_per_MAC_footprint']} cpu={r['cpu_ratio']}")


if __name__ == "__main__":
    main()
