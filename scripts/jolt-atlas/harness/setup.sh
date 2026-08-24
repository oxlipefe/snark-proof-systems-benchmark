#!/usr/bin/env bash
# Materialise the harness manifest against a local jolt-atlas clone.
#
# The harness is OUR code and contains no jolt-atlas source. It depends on the pinned clone
# by path, so the path has to be supplied; jolt-atlas's licence forbids vendoring its tree
# into this repository, which is why this indirection exists at all.
#
#   JA_ROOT=/path/to/jolt-atlas ./setup.sh && cargo build --release
set -euo pipefail
: "${JA_ROOT:?set JA_ROOT to a jolt-atlas clone at the commit in ../../systems/jolt-atlas/COMMIT}"
here="$(cd "$(dirname "$0")" && pwd)"
sed "s|JA_ROOT|${JA_ROOT}|g" "${here}/Cargo.toml.in" > "${here}/Cargo.toml"
echo "wrote ${here}/Cargo.toml against JA_ROOT=${JA_ROOT}"
echo
echo "NOTE: a fresh workspace resolves newer versions of three transitive dependencies than"
echo "the measured tree does, and two of them raise the MSRV above the 1.88 toolchain"
echo "jolt-atlas pins. Pin them to the versions in jolt-atlas's own Cargo.lock before"
echo "building, or the harness is not linking the code under test:"
echo "  cargo update tract-onnx --precise c484b3ee9a22e7d2bfca8394619771397b61c0d6"
echo "  cargo update enum-ordinalize --precise 4.3.2"
echo "  cargo update enum-ordinalize-derive --precise 4.3.2"
echo "  cargo update kstring --precise 2.0.2"
