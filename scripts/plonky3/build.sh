#!/usr/bin/env bash
# zk-prover-bench · Plonky3 · build the harness and run the blocking build-integrity gate.
#
# Thin wrapper over harness/setup.sh so the entry point matches the other systems'. Plonky3
# itself is NEVER vendored into this repository: point PLONKY3_ROOT at a clone of the revision
# named in systems/plonky3/COMMIT.
set -euo pipefail
readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export PLONKY3_ROOT="${PLONKY3_ROOT:?set PLONKY3_ROOT to a Plonky3 clone at the commit in systems/plonky3/COMMIT}"
caffeinate -dimsu env RUSTFLAGS="${RUSTFLAGS:--C target-cpu=native}" "${HERE}/harness/setup.sh"
