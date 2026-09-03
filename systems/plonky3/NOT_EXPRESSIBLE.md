# Plonky3 — what could not be expressed, and with what evidence

Two things. The first is the campaign's headline and is established with a compiler; the second
is a limit of the route, not of the toolkit, and it is stated as such.

---

## 1. A multilinear PCS over the binary field — **absent, and measured**

`bench/scripts/plonky3/harness/src/matmul.rs` runs over both fields. `src/pcs.rs` runs over one.
The reason is that **`p3-whir` is the only implementor of `p3_commit::MultilinearPcs` in the
whole tree** at the pinned commit, and it does not accept a binary field.

### The bounds, with file and line

| where | bound |
|---|---|
| `whir/src/pcs/adapter.rs:61-72` — `impl MultilinearPcs for WhirProver` | `F: TwoAdicField + Ord`, `EF: ExtensionField<F> + TwoAdicField`, `Dft: TwoAdicSubgroupDft<F>` |
| `whir/src/pcs/adapter.rs:200-206` — `impl PrescribedPointPcs for WhirProver` | the same three |
| `whir/src/parameters/whir.rs:203-207` — `impl WhirConfig::new` | `F: TwoAdicField`, `EF: ExtensionField<F> + TwoAdicField` |
| `whir/src/pcs/zk/adapter.rs:92` — the hiding twin | the same |
| `p3-binary-field` | implements `TwoAdicField` **nowhere** (`grep -rn TwoAdicField binary-field/src/` returns no line) |

And it is not an omission upstream could fix by writing an impl: the multiplicative group of
`GF(2^128)` has order `2^128 − 1`, which is **odd**. Its two-adicity is zero, so there is no
subgroup of order `2^k` for any `k ≥ 1` and no Reed–Solomon evaluation domain of the kind
`RoundConfig::folded_domain_gen` names. WHIR over a binary tower needs the *additive* NTT, and
that is a different code, not a different generic parameter.

### The evidence is a compiler, not a grep

This project's method holds that a claim of **absence** is the cheapest kind to assert and the
most expensive to withdraw, and that reading the source tells you where to look rather than
what a system does. So the claim is not made from the table above. It is made from
`bench/scripts/plonky3/run-probe-binary-pcs.sh`, which instantiates `WhirConfig` and
`WhirProver` over `BinaryField128` and records the refusal in
[`bench/data/probe-plonky3-whir-binary.txt`](../../data/probe-plonky3-whir-binary.txt):

```
error[E0599]: the associated function or constant `new` exists for struct
`WhirConfig<BinaryField128, BinaryField128, BinaryChallenger<..., ...>>`,
but its trait bounds were not satisfied
   --> src/probe_binary_pcs.rs:42:45
   = note: the following trait bounds were not satisfied:
           `BinaryField128: TwoAdicField`
```

**If that build ever succeeds, this section is wrong and must be withdrawn.** The script says
so in its own output.

### How far the binary field DOES get, which is further than "absent" suggests

It reaches **exactly the commitment and stops**, and the boundary is sharp:

* `p3_sumcheck::commit::commit_base` is generic over `p3_commit::Encoder<F>` for any
  `F: Field` (`sumcheck/src/commit.rs:26-39`), and so is `Layout::commit`
  (`sumcheck/src/layout/prover/mod.rs:50-61`);
* `p3-binary-dft` implements that `Encoder` for `BinaryField128` through an additive NTT
  (`binary-dft/src/encoder.rs:34-36`);
* upstream's own integration test for it is titled *"Phase 2 exit criterion: the multilinear
  commit path runs over a binary tower field"* (`binary-dft/tests/commit.rs:1`), and it commits
  a `Table<BinaryField128>` end to end through `PrefixProver::commit`.

So: **encode yes, Merkle-commit yes, open no, prove proximity no, verify no.** The `Encoder`
abstraction that makes the first half work was added in the pinned commit itself
(`feat(binary-dft): additive NTT and the Encoder abstraction (#2003)`, 2026-08-31) and has, at
that commit, **no consumer other than `commit_base`**. This is a codebase in the middle of
building the thing, and the measurement is a snapshot of a four-day-old crate. See
[`COMMIT`](COMMIT).

### What this costs the campaign

The cross-field cell is therefore the **`sumcheck`** route, where neither field commits
anything, rather than the committed route, where only one field can. That is the honest
comparison and it is the one [`RESULTS.md`](RESULTS.md) publishes. It also means **this system
contributes no binary-field row to `bench/RESULTS.md` §1's `witness` bucket**, and the reason is
in this file rather than in a dash in a table.

---

## 2. T2 and T3 — not expressible on the sumcheck route

`bench/TASKS.md`'s T2 and T3 are a 200-256-128-64-1 MLP with ReLU after layers 1–3.

A sumcheck over `ProductPolynomial` proves a claim about a **bilinear** form. ReLU is not one.
Expressing T2 needs either

* a GKR circuit with a layer per activation, whose wiring predicate would have to be written
  from scratch on top of `p3-sumcheck`; or
* an AIR over `p3-uni-stark` with a row per dot product and the activation as a constraint,
  plus a range check for the sign decomposition.

**Neither was written, and no cost for either is estimated here.** An unmeasured estimate of a
prover cost is precisely what this benchmark's rule F.7 exists to stop. The second route is
the more likely one and `p3-uni-stark`, `p3-air`, `p3-circle`, `p3-fri` and `p3-lookup` are all
in the tree; it is a piece of work, not a limitation.

**Consequence for `bench/RESULTS.md` §6:** this system contributes no batching row. T2 against
T3 is the only measurement in the benchmark that isolates whether folding independent requests
into one proof is sublinear, and Plonky3 has no cell in it.

---

## 3. The T1 rungs that were expressible and were not run

T1-b, T1-c and T1-d are expressible on both routes and both fields — nothing in the protocol or
the field stops them. They were **not run in this round**, which is a scheduling fact and not a
system property. Two things the ladder would establish that this round does not:

**1 · The `M`-sublinearity, which no cell here exercises.** The reduction work is
`mp·kp + kp·np` field multiplies against `mp·kp·np` MACs, and the dominant term does not depend
on `M` at all (`EXPRESSION.md` §6). Both rungs run have `M = 1`, so every ratio in
[`RESULTS.md`](RESULTS.md) sits at the one point of the ladder where that term is invisible.
T1-b (`M = 4`), T1-c (`M = 16`) and T1-d (`M = 64`) are where it would show, and **no figure
here may be extrapolated across them.**

**2 · Where, if anywhere, the memory of the two fields diverges.** On the `sumcheck` route the
prover holds `mp·kp + kp·np + mp·np` field elements and nothing else:

| task | padded `mp × kp × np` | operand elements held | published MACs |
|---|---|---:|---:|
| T1-0 | 1 × 256 × 256 | 66 048 | 65 536 |
| T1-a | 1 × 1024 × 1024 | 1 050 624 | 589 824 |
| T1-c | 16 × 1024 × 1024 | 1 081 344 | 9 437 184 |
| T1-d | 64 × 1024 × 1024 | 1 179 648 | 37 748 736 |

The count grows by **12.3 %** from T1-a to T1-d while the MAC count grows by **64×**, because
`kp·np` dominates and does not depend on `M`. Against that, binius64 holds **30 068 736 private
values at T1-c** (`../binius64/EXPRESSION.md` §6) and measured **92.99 GB peak footprint**
there.

**That table is arithmetic on element counts, not a memory measurement**, and the two fields'
elements are not even the same width (4 B for a `KoalaBear` operand, 16 B for a
`BinaryField128` one, 16 B for either field's reduced polynomials). It omits every allocation
the toolkit makes: T1-0's measured `peak footprint` is already 2.70 MB (`koala-bear`) against roughly
0.27 MB of held operands. It is a reason to run the ladder, not a substitute for running it.
