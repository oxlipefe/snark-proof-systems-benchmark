#!/usr/bin/env python3
"""Build the binius64 results table from the raw, uncurated cell data.

Two rules this script exists to enforce:

1. **Numerator and denominator come from the same run.** `MAC/s` divides the MAC count
   recorded in a cell's own `cell.json` by the prove-time median recorded in that same
   `cell.json`. No ratio in the output composes figures from different cells or different
   campaigns.

2. **`MAC/s` and `bytes/MAC` are always emitted together.** A prover twice as fast that uses
   three times the memory is worse, not better, and publishing either number alone is what
   this benchmark exists to stop.

`bytes/MAC` is reported twice, against peak footprint and against peak RSS, because the two
diverge by a wide margin at the large rungs and the divergence is itself a result. Both peaks
are per-process and cover circuit construction, witness generation, setup and proving — the
convention is stated in the results file, not implied here.

Every row of `cells.csv` is rendered, including cells that failed or were killed. Nothing is
dropped.
"""

import csv
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DATA = ROOT / "data"


def load_cells():
    ledger = DATA / "cells.csv"
    if not ledger.exists():
        print(f"no ledger at {ledger}", file=sys.stderr)
        return []
    with ledger.open() as handle:
        return list(csv.DictReader(handle))


def load_cell_json(label):
    path = DATA / "cells" / label / "cell.json"
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError:
        return None


def fnum(value, digits=0):
    if value is None:
        return "—"
    return f"{value:,.{digits}f}".replace(",", " ")


def main():
    rows = load_cells()
    if not rows:
        return 1

    out = []
    out.append(
        "| Task | rate | thr | N | status | prove ms (median) | [min–max] | verify ms | "
        "proof B | setup ms | build ms | peak RSS GB | peak footprint GB | (u+s)/real | "
        "**MAC/s** | **B/MAC footprint** | **B/MAC RSS** |"
    )
    out.append("|" + "---|" * 17)

    for row in rows:
        label = row["label"]
        cell = load_cell_json(label)
        status = row["status"]

        # A cell directory is keyed by label, so rerunning the same label overwrites its
        # per-repetition JSON while the ledger keeps both rows. Pairing the surviving JSON
        # with an earlier ledger row would take the numerator from one run and the
        # denominator from another -- the exact composition this benchmark forbids. Detect
        # it and refuse to derive anything for the superseded row.
        if cell is not None and (
            int(cell["measured_reps"]) != int(row["reps"])
            or int(cell["log_inv_rate"]) != int(row["log_inv_rate"])
        ):
            cell = None
            # Only relabel a row that otherwise succeeded. A row that already failed keeps
            # its own status, so a killed or crashed cell is never disguised as a
            # bookkeeping artefact.
            if status == "OK":
                status = "SUPERSEDED"

        if cell is None:
            out.append(
                f"| {row['task'].upper()} | {row['log_inv_rate']} | {row['threads']} | "
                f"{row['reps']} | **{status}** | — | — | — | — | — | — | "
                f"{fnum(int(row['peak_rss_bytes'])/1e9, 2) if row['peak_rss_bytes'] else '—'} | "
                f"{fnum(int(row['peak_footprint_bytes'])/1e9, 2) if row['peak_footprint_bytes'] else '—'} | "
                f"{row['cpu_ratio'] or '—'} | — | — | — |"
            )
            continue

        macs = cell["n_macs_measured_imul"]
        prove_med = cell["prove_nanos_median"]
        prove_min = cell["prove_nanos_min"]
        prove_max = cell["prove_nanos_max"]
        verify_med = cell["verify_nanos_median"]
        proof_b = cell["proof_bytes_median"]
        setup_ns = int(cell["setup_nanos"])
        build_ns = int(cell["circuit_build_nanos"])

        rss = int(row["peak_rss_bytes"]) if row["peak_rss_bytes"] else None
        foot = int(row["peak_footprint_bytes"]) if row["peak_footprint_bytes"] else None

        # Same cell, same run: MACs and prove time both come from `cell`.
        mac_s = macs / (prove_med / 1e9) if prove_med else None
        b_mac_foot = foot / macs if foot else None
        b_mac_rss = rss / macs if rss else None

        out.append(
            f"| {cell['task']} | {cell['log_inv_rate']} | {row['threads']} | "
            f"{cell['measured_reps']} | {status} | "
            f"{fnum(prove_med/1e6, 2)} | [{fnum(prove_min/1e6, 2)}–{fnum(prove_max/1e6, 2)}] | "
            f"{fnum(verify_med/1e6, 2)} | {fnum(proof_b)} | "
            f"{fnum(setup_ns/1e6, 1)} | {fnum(build_ns/1e6, 1)} | "
            f"{fnum(rss/1e9, 2) if rss else '—'} | {fnum(foot/1e9, 2) if foot else '—'} | "
            f"{row['cpu_ratio'] or '—'} | "
            f"**{fnum(mac_s)}** | **{fnum(b_mac_foot)}** | **{fnum(b_mac_rss)}** |"
        )

    print("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
