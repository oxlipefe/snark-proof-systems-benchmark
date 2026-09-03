# Plonky3 measurement harness

**Our code.** No Plonky3 source is copied into this repository. This is a separate Cargo
workspace that depends on a pinned Plonky3 clone through path dependencies and calls only its
public API. `Cargo.toml` is **generated** from `Cargo.toml.in` by `setup.sh`, which substitutes
`@PLONKY3_ROOT@`; the generated file carries a machine-local absolute path and is gitignored.

```
PLONKY3_ROOT=/path/to/Plonky3 ./setup.sh
```

Clone the revision named in [`../../../systems/plonky3/COMMIT`](../../../systems/plonky3/COMMIT):

```
git clone https://github.com/Plonky3/Plonky3 /path/to/Plonky3
git -C /path/to/Plonky3 checkout 3152b14a89067c83775a8076cc262ffc48a1fd7c
```

`setup.sh` builds with `RUSTFLAGS=-C target-cpu=native` and then runs the **blocking**
build-integrity gate. Do not edit the generated `Cargo.toml`: `setup.sh` overwrites it.

## What is here

| file | what it is |
|---|---|
| `src/tasks.rs` | the published instances, drawn to be **binius64's** instances and checked against its published `max \|accumulator\|` |
| `src/fields.rs` | the two field pairs, and `INTEGER_FAITHFUL`, the one property that separates them |
| `src/mle.rs` | multilinear extensions with one declared index convention, tested against Plonky3's own `Poly::eval_base` |
| `src/matmul.rs` | Thaler's MATMULT on `p3-sumcheck`, generic over the field pair |
| `src/pcs.rs` | the WHIR commitment route. **Prime field only, and that is the campaign's result** |
| `src/route.rs` | one measured repetition per route, with the timing brackets declared |
| `src/sanity/` | the build-integrity gate; `handmul.rs` is byte-identical to E-001's reference kernel |
| `src/stats.rs` | byte-identical to E-001's and E-006's order statistics |
| `src/probe_binary_pcs.rs` | a **deliberate compile failure**, behind the `probe-binary-pcs` feature, that measures the absence of a binary-field PCS instead of asserting it |
| `src/bin/p3_bench.rs` | one measured cell |
| `src/bin/p3_negative.rs` | the corrupted-proof control; blocking |
| `src/bin/fieldmul_sanity.rs` | the gate's entry point |

## Tests

```
RUSTFLAGS="-C target-cpu=native" cargo test --release
```

Sixteen unit tests. The ones that matter are `tasks::tests::t1_0_is_the_instance_binius64_measured`
(the two harnesses draw the same numbers), `mle::tests::our_convention_is_plonky3s` (our
hypercube indexing is theirs, so an opened point is not off by a bit reversal),
`matmul::tests::corrupted_weight_is_rejected_over_the_{prime,binary}_field`, and
`pcs::tests::t1_0_closes_on_a_commitment` (the whole committed route, with the opened values
checked against our own multilinear evaluation).

## What was deliberately NOT built

* **An AIR route** on `p3-uni-stark` / `p3-circle`. It would express T2 and T3, which the
  sumcheck route cannot. Not written; no cost estimated. `systems/plonky3/NOT_EXPRESSIBLE.md` §2.
* **A range check** on the INT8 operands. `p3-lookup` would express it. Not used, and no cost
  for it is guessed at. `systems/plonky3/EXPRESSION.md` §7.
* **Any instrumentation inside Plonky3.** The measured tree is pristine, so field
  multiplications inside `p3-sumcheck`'s rounds are not counted and no figure claims them.
