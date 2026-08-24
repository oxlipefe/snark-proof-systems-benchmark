#!/usr/bin/env python3
"""Derive bench/data/results-jolt-atlas.csv from the raw cell ledger.

The MAC counts come from bench/TASKS.md and are NEVER recomputed. Derived columns are left
EMPTY for any cell that produced no proof: such a process still had a memory peak, and
dividing the peak of a crash by a MAC count the system never performed would manufacture a
number out of a failure.
"""
import csv, pathlib

MACS = {"t1-0": 65_536, "t1-a": 589_824, "t1-b": 2_359_296,
        "t1-c": 9_437_184, "t1-d": 37_748_736, "t2": 92_224, "t3": 737_792}
# Repository root. Derived from this file's own location so a clone works anywhere.
ROOT = pathlib.Path(__file__).resolve().parents[2]
src = ROOT / "data/cells-jolt-atlas.csv"
dst = ROOT / "data/results-jolt-atlas.csv"

cols = ["label","task","macs","threads","padding","status","prove_ms_median","verify_ms_median",
        "proof_bytes","setup_ms","peak_rss_bytes","peak_footprint_bytes","cpu_ratio",
        "mac_per_s","bytes_per_mac_footprint","bytes_per_mac_rss"]
with open(src) as f, open(dst, "w", newline="") as g:
    w = csv.DictWriter(g, fieldnames=cols); w.writeheader()
    for r in csv.DictReader(f):
        macs = MACS[r["task"]]
        row = {k: r.get(k, "") for k in cols if k in r}
        row.update(label=r["label"], task=r["task"], macs=macs, threads=r["threads"],
                   padding=r["padding"], status=r["status"])
        ok = r["status"] == "OK" and r["prove_ms_median"]
        if ok:
            pm = float(r["prove_ms_median"])
            fp = int(r["peak_footprint_bytes"]); rss = int(r["peak_rss_bytes"])
            row["mac_per_s"] = f"{macs / (pm / 1000.0):.0f}"
            row["bytes_per_mac_footprint"] = f"{fp / macs:.2f}"
            row["bytes_per_mac_rss"] = f"{rss / macs:.2f}"
        else:
            row["mac_per_s"] = row["bytes_per_mac_footprint"] = row["bytes_per_mac_rss"] = ""
        w.writerow(row)
print(dst)
