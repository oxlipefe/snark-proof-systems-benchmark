#!/usr/bin/env bash
# Materialise Cargo.toml from the template, pinning the clone path.
set -euo pipefail
readonly HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly CENO_ROOT="${CENO_ROOT:?set CENO_ROOT to the pinned Ceno clone outside this repository}"
sed "s|@CENO_ROOT@|${CENO_ROOT}|g" "${HERE}/Cargo.toml.in" > "${HERE}/Cargo.toml"
echo "[ceno] wrote ${HERE}/Cargo.toml pinned to ${CENO_ROOT}"
cd "${HERE}"
caffeinate -dimsu cargo build --release --bins
echo "[ceno] built $(pwd)/target/release/ceno_cycles"
