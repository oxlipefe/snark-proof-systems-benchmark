#!/usr/bin/env bash
# Asserts that no third-party prover source is present in this repository.
#
# Two of the six measured systems (DeepProve, jolt-atlas) are licensed for
# evaluation only and explicitly forbid redistribution. Redistributing their
# code here would violate those licences. This check runs before every publish.
#
# It asserts rather than excludes: name-based exclusion silently deletes our own
# systems/<name>/ documentation, which is exactly the mistake this file replaces.
set -euo pipefail
cd "$(dirname "$0")/.."
fail=0

while IFS= read -r f; do
  if head -20 "$f" | grep -qiE 'copyright.*(lagrange|icme|scroll|consensys|irreducible|plonky3)'; then
    echo "VENDORED THIRD-PARTY SOURCE: $f" >&2; fail=1
  fi
done < <(find . -name '*.rs' -o -name '*.go' -o -name '*.c' -o -name '*.h' | grep -v '/target/')

while IFS= read -r f; do
  echo "THIRD-PARTY LICENCE FILE: $f" >&2; fail=1
done < <(find . -iname 'LICENSE*' -o -iname 'COPYING*' | grep -v '^./LICENSE')

for d in systems/binius64 systems/deepprove systems/jolt-atlas systems/ceno systems/gnark systems/plonky3; do
  [ -d "$d" ] || { echo "MISSING OUR OWN DOCS: $d" >&2; fail=1; }
done

[ "$fail" -eq 0 ] && echo "OK: no vendored third-party source; all six system directories present"
exit "$fail"
