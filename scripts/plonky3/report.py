#!/usr/bin/env python3
"""zk-prover-bench · Plonky3 · turn the raw ledger into the grid that goes in RESULTS.md.

Derives nothing that the ledger does not already carry per cell, and forms every ratio from
the same cell's own numerator and denominator. Rows the ledger marks SMOKE are printed in
their own block and are never mixed with campaign rows: a smoke row has one repetition and no
dispersion, and putting it in the same table as an N=5 median is how a benchmark starts lying.
"""
import csv
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
LEDGER = ROOT / "data" / "cells-plonky3.csv"

NS = 1e9
GB = 1e9


def num(row, key, cast=float):
    v = row.get(key, "")
    if v in ("", None):
        return None
    try:
        return cast(v)
    except ValueError:
        return None



def published_macs(r):
    """The MAC count TASKS.md publishes for the task — the denominator of MAC/s and B/MAC
    (TASKS.md line 4; amendment A6). The prover pads 768 -> 1024 in K and N, and that padding
    must show up as a WORSE MAC/s, never be divided away."""
    padded = num(r, "padded_macs", int)
    factor = num(r, "padding_factor", float)
    if padded is None or not factor:
        return None
    return int(round(padded / factor))

def fmt(v, spec="{:,.2f}"):
    return "—" if v is None else spec.format(v)


def regime(r):
    """Soundness regime, derived from `reps` because the ledger does not carry a `regime`
    column of its own (2026-09-03: the harness's WHIR soundness constant changed mid-campaign,
    see systems/plonky3/COMMIT). `…-n5` rows ran WhirConfig's default, CapacityBound, at PoW
    16; `…-n6` rows re-ran the same cells under UniqueDecoding at PoW 7 (G-13b'). This label
    is printed on every CAMPAIGN row — including `sumcheck` rows, which have no PCS and so no
    soundness regime of their own — so that no reader pairs an n5 row against an n6 row without
    noticing, the way the cross-field table below used to."""
    reps = r.get("reps")
    if reps == "5":
        return "capacity/pow16 (n5)"
    if reps == "6":
        return "unique/pow7 (n6)"
    return "n/a"


def main() -> int:
    if not LEDGER.exists():
        print(f"no ledger at {LEDGER}; run scripts/plonky3/run-cell.sh first", file=sys.stderr)
        return 1
    rows = list(csv.DictReader(LEDGER.open()))
    if not rows:
        print("ledger is empty", file=sys.stderr)
        return 1

    for block in ("CAMPAIGN", "SMOKE"):
        subset = [r for r in rows if r.get("smoke") == block]
        if not subset:
            continue
        print(f"\n## {block} rows ({len(subset)})")
        if block == "SMOKE":
            print("These carry ONE repetition and no dispersion. They establish that a cell")
            print("runs; they are not results and must not be compared with anything.")
        print()
        header = (
            "| task | field | route | thr | regime | status | prove ms | verify ms | proof B | "
            "peak fp GB | MAC/s | B/MAC (fp) | padded/published | rounds | int-faithful |"
        )
        print(header)
        print("|" + "---|" * 15)
        for r in subset:
            macs = published_macs(r)
            prove = num(r, "prove_median_nanos")
            verify = num(r, "verify_median_nanos")
            fp = num(r, "peak_footprint_bytes")
            proof = num(r, "proof_bytes_median")
            mac_s = macs / (prove / NS) if macs and prove else None
            b_mac = fp / macs if macs and fp else None
            print(
                f"| {r['task']} | {r['field']} | {r['route']} | {r['threads']} | "
                f"{regime(r)} | "
                f"{r['status']} | {fmt(prove / 1e6 if prove else None, '{:,.3f}')} | "
                f"{fmt(verify / 1e6 if verify else None, '{:,.4f}')} | "
                f"{fmt(proof, '{:,.0f}')} | {fmt(fp / GB if fp else None, '{:,.4f}')} | "
                f"{fmt(mac_s, '{:,.0f}')} | {fmt(b_mac, '{:,.1f}')} | "
                f"{r.get('padding_factor', '')} | {r.get('sumcheck_rounds', '')} | "
                f"{r.get('integer_faithful', '')} |"
            )

    # The one cross-field ratio this campaign exists to produce, formed only where both cells
    # of a pair are present, at the same task, route, thread count, block AND `reps` — `reps`
    # is included because two CAMPAIGN rows can share (task, route, threads, block) while
    # belonging to different soundness regimes (n5 vs n6; see `regime()`), and pairing a
    # koala-bear row from one regime against a binary128 row from the other is exactly the
    # silent mix this file's own rules forbid. Only binary128 has no `sumcheck-whir` cell and
    # no `n6` re-run at all, so cross-field pairs only ever form on the `sumcheck` route.
    print("\n## The cross-field ratio, per (task, route, threads, regime) pair")
    print()
    print("MAC/s(binary128) / MAC/s(koala-bear). Both cells are the SAME task on the SAME")
    print("machine in the SAME codebase; they are NOT the same theorem (see EXPRESSION.md §2).")
    print()
    by_key = {}
    for r in rows:
        if r["status"] != "OK":
            continue
        key = (r["task"], r["route"], r["threads"], r.get("smoke"), r.get("reps"))
        by_key.setdefault(key, {})[r["field"]] = r
    print("| task | route | thr | block | regime | MAC/s koala-bear | MAC/s binary128 | ratio |")
    print("|" + "---|" * 8)
    for (task, route, thr, block, reps), pair in sorted(by_key.items()):
        if "koala-bear" not in pair or "binary128" not in pair:
            continue
        rates = {}
        for field, r in pair.items():
            macs, prove = published_macs(r), num(r, "prove_median_nanos")
            rates[field] = macs / (prove / NS) if macs and prove else None
        kb, bn = rates["koala-bear"], rates["binary128"]
        ratio = bn / kb if kb and bn else None
        print(
            f"| {task} | {route} | {thr} | {block} | {regime(pair['koala-bear'])} | "
            f"{fmt(kb, '{:,.0f}')} | {fmt(bn, '{:,.0f}')} | {fmt(ratio, '{:.4f}')} |"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
