#!/usr/bin/env python3
"""zk-prover-bench · Ceno · render the results grid and the memory curve from the raw ledger.

Two rules govern everything here, and they are the same two the other three systems' reporters
state:

  1. **Numerator and denominator come from the same run.** `MAC/s` divides the MAC count frozen
     in bench/TASKS.md by the prove-time median of that same cell. No ratio composes figures
     from different cells or different campaigns.
  2. **`MAC/s` and `bytes/MAC` are always emitted together.** A prover twice as fast that uses
     three times the memory is worse, not better, and publishing either alone is what this
     benchmark exists to stop.

A third rule is specific to this system: **`bytes/MAC` is reported per cell together with the
shard configuration that produced it**, because for a segmented prover peak memory is a
function of `--max-cycle-per-shard` and not of the task alone.

Warmup repetitions are excluded from every statistic and kept in the raw ledger.
"""

import csv
import pathlib
import statistics
import sys

# Repository root. Derived from this file's own location so a clone works anywhere.
ROOT = pathlib.Path(__file__).resolve().parents[2]
LEDGER = ROOT / "data/cells-ceno.csv"
OUT = ROOT / "data/results-ceno.csv"

# bench/TASKS.md, frozen. Never recomputed here.
MACS = {
    "t1-0": 65_536, "t1-a": 589_824, "t1-b": 2_359_296,
    "t1-c": 9_437_184, "t1-d": 37_748_736, "t2": 92_224, "t3": 737_792,
}
# Emulated exactly by bench/scripts/ceno/harness (see bench/data/cycles-ceno/).
CYCLES = {
    "t1-0": 3_203_656, "t1-a": 25_265_476, "t1-b": 100_983_792,
    "t1-c": 403_791_196, "t1-d": 1_615_020_812, "t2": 4_499_472, "t3": 35_505_916,
}
LADDER = ["t1-0", "t1-a", "t1-b", "t1-c", "t1-d"]

# The primary cut. 1 thread — binius64's primary — is NOT available for Ceno: the prover
# aborts there on the authors' own examples (NOT_EXPRESSIBLE.md §1). 10 is the machine default.
PRIMARY_THREADS = "10"
PRIMARY_SHARD_CAP = "536870912"


def num(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def sp(v, digits=0):
    """Thousands separated by a narrow space, as everywhere else in this repository."""
    if v is None:
        return "—"
    return f"{v:,.{digits}f}".replace(",", " ")


def main():
    if not LEDGER.exists():
        sys.exit(f"no ledger at {LEDGER}")
    all_rows = list(csv.DictReader(LEDGER.open()))
    rows = [r for r in all_rows if r["is_warmup"] == "0"]

    # `num_shards` is only present in the warmup repetition: `--profiling 1` installs a filter
    # that keeps only spans carrying a `profiling_N` field, which is what makes the
    # ZKVM_create_proof span readable and which also suppresses Ceno's own INFO lines. The
    # warmup deliberately runs without it, so shard counts are looked up from there.
    shards_by_label = {r["label"]: r["num_shards"] for r in all_rows if r.get("num_shards")}

    cells = {}
    for r in rows:
        cells.setdefault(r["label"], []).append(r)

    out = []
    for label, reps in sorted(cells.items()):
        first = reps[0]
        task = first["task"]
        ok = [r for r in reps if r["status"] == "OK"]
        proves = [num(r["create_proof_s"]) for r in ok]
        proves = [p for p in proves if p is not None]
        # Peak memory is the MAXIMUM over repetitions, not the median: the question the metric
        # answers is whether the task fits, and the worst repetition is what decides that.
        rss = max((num(r["peak_rss_bytes"]) or 0 for r in ok), default=None) or None
        fp = max((num(r["peak_footprint_bytes"]) or 0 for r in ok), default=None) or None
        macs = MACS.get(task)

        rec = {
            "label": label,
            "task": task,
            "macs": macs,
            "cycles": CYCLES.get(task),
            "rayon_threads": first["threads"],
            "max_cycle_per_shard": first["max_cycle_per_shard"],
            "num_shards": first.get("num_shards") or shards_by_label.get(label, ""),
            "status": first["status"] if not ok else "OK",
            "N": len(ok),
            "prove_s_median": statistics.median(proves) if proves else "",
            "prove_s_min": min(proves) if proves else "",
            "prove_s_max": max(proves) if proves else "",
            "real_s_median": statistics.median(
                [num(r["real_s"]) for r in ok if num(r["real_s"]) is not None]
            ) if ok else "",
            "cpu_ratio_median": statistics.median(
                [num(r["cpu_ratio"]) for r in ok if num(r["cpu_ratio"]) is not None]
            ) if ok else "",
            "peak_rss_bytes": int(rss) if rss else "",
            "peak_footprint_bytes": int(fp) if fp else "",
            "proof_bytes": first.get("proof_bytes") or "",
        }
        # Derived columns stay EMPTY for any cell that produced no proof. Those processes still
        # had a memory peak — the peak of failing — and dividing it by a MAC count the system
        # never completed would manufacture a number out of a crash.
        if ok and macs:
            if proves:
                rec["MAC_per_s"] = macs / statistics.median(proves)
                rec["cycles_per_s"] = CYCLES[task] / statistics.median(proves)
            if fp:
                rec["bytes_per_MAC_footprint"] = fp / macs
                rec["bytes_per_cycle_footprint"] = fp / CYCLES[task]
            if rss:
                rec["bytes_per_MAC_rss"] = rss / macs
        out.append(rec)

    fields = [
        "label", "task", "macs", "cycles", "rayon_threads", "max_cycle_per_shard",
        "num_shards", "status", "N", "prove_s_median", "prove_s_min", "prove_s_max",
        "real_s_median", "cpu_ratio_median", "peak_rss_bytes", "peak_footprint_bytes",
        "proof_bytes", "MAC_per_s", "cycles_per_s", "bytes_per_MAC_footprint",
        "bytes_per_cycle_footprint", "bytes_per_MAC_rss",
    ]
    with OUT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fields, extrasaction="ignore")
        w.writeheader()
        w.writerows(out)

    # ---- the grid ----
    hdr = ["Task", "MACs", "cycles", "RAYON thr", "shard cap", "shards", "N", "status",
           "prove s (median)", "[min-max]", "proof B", "peak RSS GB", "peak fp GB",
           "(u+s)/real", "**MAC/s**", "**cycles/s**", "**B/MAC fp**", "**B/cycle fp**"]
    print("| " + " | ".join(hdr) + " |")
    print("|" + "|".join(["---"] * len(hdr)) + "|")

    def f(v, d=2):
        return "—" if not isinstance(v, float) else sp(v, d)

    for r in out:
        rng = ("[%.2f-%.2f]" % (r["prove_s_min"], r["prove_s_max"])
               if isinstance(r.get("prove_s_min"), float) else "—")
        cells_out = [
            r["task"].upper(),
            sp(r["macs"]),
            sp(r["cycles"]),
            str(r["rayon_threads"]),
            sp(int(r["max_cycle_per_shard"])),
            r["num_shards"] or "—",
            str(r["N"]),
            r["status"] if r["status"] == "OK" else "**%s**" % r["status"],
            f(r.get("prove_s_median")),
            rng,
            sp(num(r["proof_bytes"])),
            f(r["peak_rss_bytes"] / 2**30 if r["peak_rss_bytes"] else None),
            f(r["peak_footprint_bytes"] / 2**30 if r["peak_footprint_bytes"] else None),
            f(r.get("cpu_ratio_median"), 4),
            "**%s**" % sp(r.get("MAC_per_s")) if r.get("MAC_per_s") else "—",
            "**%s**" % sp(r.get("cycles_per_s")) if r.get("cycles_per_s") else "—",
            "**%s**" % sp(r.get("bytes_per_MAC_footprint"), 1) if r.get("bytes_per_MAC_footprint") else "—",
            "**%s**" % sp(r.get("bytes_per_cycle_footprint"), 1) if r.get("bytes_per_cycle_footprint") else "—",
        ]
        print("| " + " | ".join(cells_out) + " |")

    # ---- the memory curve, with the local exponent rung by rung ----
    print()
    # The curve is cut at ONE configuration. Mixing thread counts or shard caps into a single
    # curve would fit an exponent across two different provers; PRIMARY_THREADS and the default
    # shard cap are the primary cut, and any other configuration gets its own curve.
    ladder = [r for r in out
              if r["task"] in LADDER and r["status"] == "OK" and r["peak_footprint_bytes"]
              and r["rayon_threads"] == PRIMARY_THREADS
              and r["max_cycle_per_shard"] == PRIMARY_SHARD_CAP]
    ladder.sort(key=lambda r: r["macs"])
    print(f"Memory curve, RAYON thr = {PRIMARY_THREADS}, shard cap = {sp(int(PRIMARY_SHARD_CAP))}:")
    if len(ladder) >= 2:
        print("| Task | MACs | cycles | shards | peak footprint | **B/MAC** | vs previous rung | local exponent |")
        print("|---|---:|---:|---:|---:|---:|---|---:|")
        prev = None
        for r in ladder:
            fp = r["peak_footprint_bytes"]
            import math
            row = [r["task"].upper(), sp(r["macs"]), sp(r["cycles"]), r["num_shards"] or "—",
                   "%s MB" % sp(fp / 1e6, 1), "**%s**" % sp(fp / r["macs"], 1)]
            if prev is None:
                row += ["—", "—"]
            else:
                dm = r["macs"] / prev["macs"]
                df = fp / prev["peak_footprint_bytes"]
                exp = "%.3f" % (math.log(df) / math.log(dm)) if dm != 1.0 else "—"
                row += ["MACs x%.2f, memory x%.2f" % (dm, df), exp]
            print("| " + " | ".join(row) + " |")
            prev = r
    else:
        print("Fewer than two proved ladder rungs; no curve is reported. "
              "Nothing is extrapolated from a single point.")


if __name__ == "__main__":
    main()
