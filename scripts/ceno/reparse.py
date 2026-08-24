#!/usr/bin/env python3
"""zk-prover-bench · Ceno · rebuild the cell ledger from the saved per-repetition logs.

The ledger is derived data. The authority is the pair of files every repetition writes:

    <cell>/<tag>.stdout.txt   Ceno's own tracing output — the ZKVM_create_proof span, the
                              `program executed <n> instructions in <m> cycles` line, and
                              `num_shards: <k>`
    <cell>/<tag>.log.txt      /usr/bin/time -l — real/user/sys and the two memory peaks

Those two streams are separate on purpose and were mixed up once: Ceno logs to stdout while
`/usr/bin/time -l` writes to stderr, so an early version of the harness looked for the timing
span in the wrong file and recorded it as empty. Nothing was re-run to fix it — the raw logs
already held every field, and this script re-derives the ledger from them. That is the point
of committing raw output uncurated.

Reads nothing but the logs; writes bench/data/cells-ceno.csv.
"""

import csv
import pathlib
import re
import sys

# Repository root. Derived from this file's own location so a clone works anywhere.
ROOT = pathlib.Path(__file__).resolve().parents[2]
CELLS = ROOT / "data/cells-ceno"
LEDGER = ROOT / "data/cells-ceno.csv"
OLD = ROOT / "data/cells-ceno.raw.csv"

FIELDS = [
    "label", "task", "elf", "threads", "max_cycle_per_shard", "max_cell_per_shard",
    "warmup", "reps", "rep", "is_warmup", "status", "real_s", "user_s", "sys_s",
    "cpu_ratio", "peak_rss_bytes", "peak_footprint_bytes", "create_proof_s",
    "instructions", "cycles", "num_shards", "proof_bytes", "vk_bytes",
    "create_proof_spans", "mono_s", "wall_s", "slept_s", "sleep_verdict", "loadavg_1m",
    "swap_used_mb", "started_utc",
]

# Same conversion the authors' own CI applies to this span
# (.github/workflows/gpu-integration.yml).
SPAN = re.compile(r"ZKVM_create_proof \[ *([0-9.]+)(ns|µs|us|ms|m|s)")
UNIT = {"ns": 1e-9, "µs": 1e-6, "us": 1e-6, "ms": 1e-3, "s": 1.0, "m": 60.0}


def strip_ansi(text):
    return re.sub(r"\x1b\[[0-9;]*m", "", text)


def parse_stdout(path):
    if not path.exists():
        return {}
    text = strip_ansi(path.read_text(errors="replace"))
    out = {}
    # ONE ZKVM_create_proof SPAN IS EMITTED PER SHARD, not per proof. An earlier version of
    # this parser took the first match, which recorded one shard's time as the whole task's and
    # made segmented cells look FASTER than the unsegmented one — 11.2 s against 20.3 s for the
    # same task — while their wall-clock time was in fact rising. The spans are summed, and the
    # count is kept: it is also the shard count, and it is available even for cells that ran
    # without the non-profiling warmup that emits `num_shards`.
    spans = SPAN.findall(text)
    if spans:
        out["create_proof_s"] = f"{sum(float(v) * UNIT[u] for v, u in spans):.6f}"
        out["create_proof_spans"] = str(len(spans))
        out["num_shards"] = str(len(spans))
    m = re.search(r"program executed (\d+) instructions in (\d+) cycles", text)
    if m:
        out["instructions"], out["cycles"] = m.group(1), m.group(2)
    # Ceno's own line wins over the span count when both are present.
    m = re.search(r"num_shards: (\d+)", text)
    if m:
        out["num_shards"] = m.group(1)
    return out


def parse_time(path):
    if not path.exists():
        return {}
    text = path.read_text(errors="replace")
    out = {}
    for key, pat in (
        ("real_s", r"^\s*([0-9.]+)\s+real"),
        ("user_s", r"([0-9.]+)\s+user"),
        ("sys_s", r"([0-9.]+)\s+sys"),
        ("peak_rss_bytes", r"(\d+)\s+maximum resident set size"),
        ("peak_footprint_bytes", r"(\d+)\s+peak memory footprint"),
    ):
        m = re.search(pat, text, re.M)
        if m:
            out[key] = m.group(1)
    try:
        r, u, s = float(out["real_s"]), float(out["user_s"]), float(out["sys_s"])
        if r > 0:
            out["cpu_ratio"] = f"{(u + s) / r:.4f}"
    except (KeyError, ValueError):
        pass
    return out


def main():
    if not LEDGER.exists():
        sys.exit(f"no ledger at {LEDGER}; nothing to reparse")
    rows = list(csv.DictReader(LEDGER.open()))
    LEDGER.replace(OLD)

    for row in rows:
        tag = "warmup" if row["is_warmup"] == "1" else f"rep{row['rep']}"
        d = CELLS / row["label"]
        row.update(parse_stdout(d / f"{tag}.stdout.txt"))
        row.update(parse_time(d / f"{tag}.log.txt"))

    with LEDGER.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=FIELDS, extrasaction="ignore")
        w.writeheader()
        w.writerows(rows)
    print(f"reparsed {len(rows)} rows -> {LEDGER} (previous ledger kept at {OLD.name})")


if __name__ == "__main__":
    main()
