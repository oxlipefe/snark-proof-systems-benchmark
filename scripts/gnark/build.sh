#!/usr/bin/env bash
# zk-prover-bench · gnark · build the harness and verify the pin.
#
# THE BUILD-INTEGRITY CHECK IS BLOCKING and it is the first thing that runs. In our own prior
# work a harness compiled without LTO measured our prover 9.0x slower and INVERTED the
# experiment's conclusion, and nothing in the timing output revealed it. Every system in this
# campaign gets its own documented check; gnark's has two parts.
#
# 1. THE PIN IS THE MODULE, AND THE MODULE IS THE CLONE. bench/tasks/gnark/go.mod depends on
#    github.com/consensys/gnark v0.16.2 through the module proxy, checksummed in go.sum. The
#    clone at ${GNARK_ROOT} is pinned to tag v0.16.2 / 9838556b92c7783cb82971cf37c0d081cc2b6aec.
#    This script diffs the two trees. If they differ, the harness is not measuring the pinned
#    commit and the build fails here rather than in a footnote.
# 2. gnark ships NO release profile of its own — it is a Go library, `go build` has no LTO
#    switch, and the only knobs are GOARCH/GOAMD64-class ones this campaign does not set. The
#    absence of tunables IS the check: there is no configuration in which we could have
#    measured gnark badly by compiling it wrong, and that is stated rather than assumed.
set -uo pipefail

# Repository root. Derived from this script's own location so a clone works anywhere.
readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly PKG="${ROOT}/tasks/gnark"
readonly GNARK_ROOT="${GNARK_ROOT:?set GNARK_ROOT to a gnark clone at the commit in systems/gnark/COMMIT}"
readonly PINNED_COMMIT="9838556b92c7783cb82971cf37c0d081cc2b6aec"
readonly PINNED_TAG="v0.16.2"

echo "=== 0. toolchain ==="
go version || exit 1

echo "=== 1. the clone is at the pinned commit (BLOCKING) ==="
have="$(git -C "${GNARK_ROOT}" rev-parse HEAD)"
if [[ "${have}" != "${PINNED_COMMIT}" ]]; then
  echo "BUILD INTEGRITY FAILED: clone is at ${have}, expected ${PINNED_COMMIT}"; exit 1
fi
echo "clone HEAD = ${have} (${PINNED_TAG})"

echo "=== 2. the module cache is byte-identical to the clone (BLOCKING) ==="
modpath="$(go env GOMODCACHE)/github.com/consensys/gnark@${PINNED_TAG}"
if [[ ! -d "${modpath}" ]]; then
  ( cd "${PKG}" && go mod download github.com/consensys/gnark ) || exit 1
fi
ndiff="$(diff -r -q --exclude='.git' --exclude='.github' "${modpath}" "${GNARK_ROOT}" 2>/dev/null | wc -l | tr -d ' ')"
if [[ "${ndiff}" != "0" ]]; then
  echo "BUILD INTEGRITY FAILED: module cache and clone differ in ${ndiff} path(s)"
  diff -r -q --exclude='.git' --exclude='.github' "${modpath}" "${GNARK_ROOT}" | head -20
  exit 1
fi
echo "module cache == clone, 0 differing paths"

echo "=== 3. build ==="
mkdir -p "${PKG}/bin"
( cd "${PKG}" && \
  go build -o bin/gnark-runner       ./runner       && \
  go build -o bin/gnark-compile-grid ./cmd/compile-grid && \
  go build -o bin/gnark-negative     ./cmd/negative && \
  go build -o bin/gnark-probe        ./cmd/probe ) || exit 1
ls -la "${PKG}/bin"

echo "=== 4. correctness of the gadgets (BLOCKING) ==="
# -tags=prover_checks makes gnark's own test.Assert run the FULL setup/prove/verify rather
# than stopping at the constraint solver. Without the tag the ReLU tests would establish that
# the constraints are satisfiable, not that a proof of them verifies.
( cd "${PKG}" && go test -tags=prover_checks -count=1 ./... ) || {
  echo "GADGET CORRECTNESS FAILED — no measurement may proceed"; exit 1; }

echo "=== 5. gnark's own example circuits through this harness (BLOCKING) ==="
# The check that stopped jolt-atlas from publishing three of our own expression errors as
# somebody else's limits. No limit is attributed to gnark until this passes.
"${PKG}/bin/gnark-probe" example || exit 1

echo "BUILD OK"
