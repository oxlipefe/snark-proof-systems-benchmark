#!/usr/bin/env python3
"""zk-prover-bench · gnark · derive results-gnark.csv and render the grid from the raw ledger.

Three rules govern everything here. The first two are the same two every other system's
reporter states; the third is specific to gnark.

  1. **Numerator and denominator come from the same run.** `MAC/s` divides the MAC count
     FROZEN in bench/TASKS.md by the prove-time median of that same cell. No ratio composes
     figures from different cells or different campaigns. The MAC counts below are copied
     from bench/TASKS.md and are NEVER recomputed — not from the shapes, not from the
     circuit, not from anything this script can see.

  2. **`MAC/s` and `bytes/MAC` are always emitted together.** A prover twice as fast that
     uses three times the memory is worse, not better, and publishing either alone is what
     this benchmark exists to stop.

  3. **REGIME A AND REGIME B ARE NEVER MIXED.** Regime A puts every weight in the witness and
     proves its 8-bit range; regime B bakes the weights in as circuit constants. Regime A is
     the cross-system comparable figure. Regime B is a declared lever — Groth16's per-circuit
     setup binds the weights into the verifying key, which is what a deployed fixed-model
     service wants and which no other system in this bank offers. Every row carries its
     regime, the grid is printed one regime at a time, and there is no code path in this file
     that can average them.

Derived columns are left EMPTY for any cell whose status is not OK. Such a process still had
a memory peak — the peak of failing — and dividing it by a MAC count the system never
completed would manufacture a number out of a crash.

`constraints` is emitted as its own column because it is gnark's natural unit the way cycles
were Ceno's: it is what the FFT domain, the proving key and therefore the memory are sized
from, and it is measurable several rungs above where a proof fits.

Nothing about std/rangecheck's per-value cost is hardcoded anywhere in this file. That cost
AMORTIZES a shared lookup table — measured at 4.19 R1CS/value at n=16 and 2.01 at n=65 792 —
so a per-value constant would be false at every n but one. Totals only.
"""

import csv
import math
import pathlib
import statistics
import sys

# Repository root. Derived from this file's own location so a clone works anywhere.
ROOT = pathlib.Path(__file__).resolve().parents[2]
LEDGER = ROOT / "data/cells-gnark.csv"
OUT = ROOT / "data/results-gnark.csv"

# bench/TASKS.md, frozen. NEVER recomputed here.
MACS = {
    "t1-0": 65_536, "t1-a": 589_824, "t1-b": 2_359_296,
    "t1-c": 9_437_184, "t1-d": 37_748_736, "t2": 92_224, "t3": 737_792,
}
# bench/TASKS.md states 448 for T2 and is silent for T3; 3 584 is 8 x 448, derived from the
# frozen T2 figure and marked as such. Activations are reported separately from MACs and are
# never folded into them.
RELUS = {"t2": 448, "t3": 3_584}

LADDER = ["t1-0", "t1-a", "t1-b", "t1-c", "t1-d"]

COLS = [
    "label", "task", "macs", "relus_published", "backend", "regime", "gadget",
    "threads", "nb_tasks", "gogc", "gomemlimit", "reps", "status",
    "constraints", "domain_cardinality", "padding_ratio",
    "compile_ms", "setup_ms", "srs_ms", "pk_bytes", "vk_bytes",
    "prove_ms_median", "prove_ms_min", "prove_ms_max", "verify_ms_median",
    "proof_bytes", "proof_bytes_raw", "peak_rss_bytes", "peak_footprint_bytes",
    "cpu_ratio", "max_abs_intermediate",
    "mac_per_s", "bytes_per_mac_footprint", "bytes_per_mac_rss",
    "constraints_per_mac", "bytes_per_constraint_footprint",
]


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


def derive(rows):
    out = []
    for r in rows:
        task = r["task"]
        macs = MACS.get(task)          # None for a probe circuit; then no rate is emitted
        rec = {c: r.get(c, "") for c in COLS if c in r}
        rec.update(
            label=r["label"], task=task, macs=macs if macs else "",
            relus_published=RELUS.get(task, 0),
            backend=r["backend"], regime=r["regime"], gadget=r.get("gadget", ""),
            threads=r["threads"], nb_tasks=r.get("nb_tasks", ""),
            gogc=r.get("gogc", ""), gomemlimit=r.get("gomemlimit", ""),
            reps=r["reps"], status=r["status"],
        )

        cons = num(r.get("constraints"))
        dom = num(r.get("domain_cardinality"))
        rec["padding_ratio"] = f"{dom / cons:.4f}" if cons and dom else ""
        if cons and macs:
            rec["constraints_per_mac"] = f"{cons / macs:.4f}"

        ok = r["status"] == "OK" and r.get("prove_ms_median")
        if not ok:
            # EMPTY, deliberately. The peak of a crash is not a bytes/MAC figure.
            for c in ("mac_per_s", "bytes_per_mac_footprint", "bytes_per_mac_rss",
                      "bytes_per_constraint_footprint"):
                rec[c] = ""
            out.append(rec)
            continue

        pm = num(r["prove_ms_median"])
        fp = num(r.get("peak_footprint_bytes"))
        rss = num(r.get("peak_rss_bytes"))
        if macs and pm:
            rec["mac_per_s"] = f"{macs / (pm / 1000.0):.0f}"
            if fp:
                rec["bytes_per_mac_footprint"] = f"{fp / macs:.2f}"
            if rss:
                rec["bytes_per_mac_rss"] = f"{rss / macs:.2f}"
        if cons and fp:
            rec["bytes_per_constraint_footprint"] = f"{fp / cons:.2f}"
        out.append(rec)
    return out


def grid(rows, backend, regime):
    sel = [r for r in rows if r["backend"] == backend and r["regime"] == regime]
    if not sel:
        return
    order = {t: i for i, t in enumerate(["t1-0", "t2", "t1-a", "t3", "t1-b", "t1-c", "t1-d"])}
    sel.sort(key=lambda r: (order.get(r["task"], 99), r["label"]))

    tag = "HEADLINE, cross-system comparable" if regime == "A" else \
          "DECLARED LEVER — weights baked into the circuit and bound into the verifying key. NEVER a cross-system number."
    print(f"\n### {backend} · regime {regime} — {tag}\n")
    hdr = ["Task", "MACs", "constraints", "domain", "thr", "GOGC", "N", "status",
           "compile s", "setup s", "prove ms (median)", "[min–max]", "verify ms",
           "proof B", "peak RSS MB", "peak fp MB", "(u+s)/real", "**MAC/s**", "**B/MAC fp**"]
    print("| " + " | ".join(hdr) + " |")
    print("|" + "|".join(["---"] * len(hdr)) + "|")
    for r in sel:
        pmin, pmax = num(r.get("prove_ms_min")), num(r.get("prove_ms_max"))
        rng = f"[{pmin:.1f}–{pmax:.1f}]" if pmin is not None and pmax is not None else "—"
        fp, rss = num(r.get("peak_footprint_bytes")), num(r.get("peak_rss_bytes"))
        cells = [
            r["task"].upper(),
            sp(num(r["macs"])) if r["macs"] else "—",
            sp(num(r.get("constraints"))),
            sp(num(r.get("domain_cardinality"))),
            str(r["threads"]), str(r.get("gogc", "")), str(r["reps"]),
            r["status"] if r["status"] == "OK" else f"**{r['status']}**",
            sp(num(r.get("compile_ms", 0)) / 1000 if num(r.get("compile_ms")) else None, 2),
            sp(num(r.get("setup_ms", 0)) / 1000 if num(r.get("setup_ms")) else None, 2),
            sp(num(r.get("prove_ms_median")), 1), rng,
            sp(num(r.get("verify_ms_median")), 2),
            sp(num(r.get("proof_bytes"))),
            sp(rss / 1e6 if rss else None, 1),
            sp(fp / 1e6 if fp else None, 1),
            sp(num(r.get("cpu_ratio")), 4),
            f"**{sp(num(r.get('mac_per_s')))}**" if r.get("mac_per_s") else "—",
            f"**{sp(num(r.get('bytes_per_mac_footprint')), 1)}**" if r.get("bytes_per_mac_footprint") else "—",
        ]
        print("| " + " | ".join(cells) + " |")


def memory_curve(rows, backend, regime, threads, gogc):
    """The curve is cut at ONE configuration. Fitting an exponent across two thread counts
    or two GOGC settings would fit it across two different provers."""
    sel = [r for r in rows
           if r["task"] in LADDER and r["status"] == "OK" and r["backend"] == backend
           and r["regime"] == regime and str(r["threads"]) == str(threads)
           and str(r.get("gogc", "")) == str(gogc) and num(r.get("peak_footprint_bytes"))]
    sel.sort(key=lambda r: MACS[r["task"]])
    print(f"\nMemory curve — {backend}, regime {regime}, GOMAXPROCS = {threads}, GOGC = {gogc}:\n")
    if len(sel) < 2:
        print("Fewer than two proved ladder rungs at this cut; no curve is reported. "
              "Nothing is extrapolated from a single point.")
        return
    print("| Task | MACs | constraints | domain | peak footprint | **B/MAC** | vs previous rung | local exponent |")
    print("|---|---:|---:|---:|---:|---:|---|---:|")
    prev = None
    for r in sel:
        fp = num(r["peak_footprint_bytes"])
        macs = MACS[r["task"]]
        row = [r["task"].upper(), sp(macs), sp(num(r.get("constraints"))), sp(num(r.get("domain_cardinality"))),
               f"{sp(fp / 1e6, 1)} MB", f"**{sp(fp / macs, 1)}**"]
        if prev is None:
            row += ["—", "—"]
        else:
            dm = macs / MACS[prev["task"]]
            df = fp / num(prev["peak_footprint_bytes"])
            exp = f"{math.log(df) / math.log(dm):.3f}" if dm != 1.0 else "—"
            row += [f"MACs ×{dm:.2f}, memory ×{df:.2f}", exp]
        print("| " + " | ".join(row) + " |")
        prev = r


def main():
    if not LEDGER.exists():
        sys.exit(f"no ledger at {LEDGER}")
    raw = list(csv.DictReader(LEDGER.open()))
    rows = derive(raw)

    with OUT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=COLS, extrasaction="ignore")
        w.writeheader()
        w.writerows(rows)

    for backend in ("groth16", "plonk"):
        for regime in ("A", "B"):
            grid(rows, backend, regime)

    # Primary cut: the machine default thread count and gnark's default GC.
    ths = sorted({str(r["threads"]) for r in rows}, key=lambda s: -int(s) if s.isdigit() else 0)
    primary = ths[0] if ths else "10"
    memory_curve(rows, "groth16", "A", primary, "default")

    print(f"\n{OUT}", file=sys.stderr)


if __name__ == "__main__":
    main()
