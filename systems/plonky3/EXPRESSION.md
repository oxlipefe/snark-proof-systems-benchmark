# Plonky3 — how each task was expressed

`bench/TASKS.md` fixes each task by an **exact operation count**, not by a shape. That count is
the denominator of both `MAC/s` and `bytes/MAC`, so it is never recomputed here.

**Read §2 before any number in [`RESULTS.md`](RESULTS.md).** This system is measured over two
fields of one codebase, and the two cells **do not prove the same theorem**. Every other
caveat in this file is smaller than that one.

Source: `bench/scripts/plonky3/harness/src/`.

---

## 1. The expression: Thaler's MATMULT, not a constraint system

The other five systems in this benchmark express T1 as arithmetic in a circuit or a trace, and
their cost is the cost of committing that circuit. Plonky3 is measured on a different
expression of the same task, and the difference is not incidental — it is the reason the
figures land where they do.

For `C = A[M×K] · B[K×N]`, the multilinear extension of `C` satisfies

```
    C~(r1, r2) = Σ_{k ∈ {0,1}^log K}  A~(r1, k) · B~(k, r2)
```

which is a sum of a **product of two multilinears** over `log K` variables — exactly the shape
`p3_sumcheck::product_polynomial::ProductPolynomial` drives. So the protocol is:

1. the verifier absorbs the public output `C` and samples `r1 ∈ EF^{log M}`, `r2 ∈ EF^{log N}`;
2. it computes `C~(r1, r2)` from `C` alone;
3. `log K` sumcheck rounds reduce that claim to one claim at a random `r3`;
4. the claim closes on `A~(r1, r3) · B~(r3, r2)`.

```rust
// weights[k] = B~(k, r2) — kp·np multiplies, the dominant term and one per MAC.
let weights: Vec<P::EF> = (0..kp).map(|kk| {
    let row = &st.b[kk * np..(kk + 1) * np];
    eq2.iter().zip(row).map(|(&e, &v)| e * v).sum::<P::EF>()
}).collect();

let poly = ProductPolynomial::new_unpacked(VariableOrder::Prefix, Poly::new(evals), Poly::new(weights));
let mut prover = SumcheckProver::new(poly, claimed);
let r3 = prover.compute_sumcheck_polynomials(&mut sumcheck, challenger, st.log_k, POW_BITS, None);
```

**No intermediate is ever committed and no constraint system is built.** The `K·N` products of
the matmul do not appear as `K·N` committed values; they appear as `K·N` field multiplications
inside one contraction whose *output* is `K` field elements. That is the whole point of the
protocol, and it is why a `bytes/MAC` from this system and a `bytes/MAC` from binius64 are not
measuring the same construction even before the weight regime is considered.

**Consequence for `bench/RESULTS.md` §11:** `constraints` does not exist for this system. Its
per-system natural unit is the pair (`sumcheck rounds`, `reduction field multiplies`), and both
are in every cell, in the same line as the time.

---

## 2. THE ASYMMETRY — the two fields do not prove the same theorem

`bench/TASKS.md` fixes an **integer** matmul: signed INT8 operands in `[-128, 127]`, an INT32
accumulator, no requantisation.

**KoalaBear carries that arithmetic. `GF(2^128)` does not, and cannot.**

| | `koala-bear` | `binary128` |
|---|---|---|
| field | KoalaBear, `p = 2^31 − 2^24 + 1`, with `BinomialExtensionField<_,4>` for challenges | `BinaryField128`, top of `p3-binary-field`'s Wiedemann tower; `F = EF` |
| INT8 embedding | the integer itself: `v ≥ 0 → from_u8(v)`, `v < 0 → −from_u8(|v|)` | the byte's **bit pattern**, `BinaryField8::from_u8(v as u8)` lifted into `GF(2^128)` |
| is the field product the task's product? | **YES** — checked, not asserted: `\|acc\| ≤ K·128·128 < 2^24 < p`, and the builder compares the field output against the INT32 reference elementwise and refuses to emit a cell if they differ | **NO.** Characteristic 2: `−1 = 1`, there is no order, and the sum of two embedded bytes is their XOR. `embed(−1)` and `embed(255)` are the same element |
| `integer_faithful` column | `True` | `False` |
| what the cell measures | the task | **the same protocol on the same-shaped bilinear form over a different substrate** |

The binary cell's public output `C` is therefore computed **in the field** and is not the
INT32 matrix `bench/TASKS.md` publishes. It is the same number of field multiplies over the
same multilinear structure with the same seeds, which is what makes it a legitimate measurement
of substrate cost — and it is not T1.

**This is the finding, not a caveat around one.** The cross-field comparison that R-086 and the
2026 SoK say does not exist cannot be made faithful even inside a single codebase, because
`GF(2^128)` does not carry integer arithmetic. A binary-field system that *does* prove T1 —
binius64 — pays for it in 64-bit word gadgets (`imul`, `iadd`), and that cost is exactly the
difference between the two expressions.

---

## 3. Padding — declared, and it is not free

A multilinear extension is indexed by a hypercube, so every dimension is rounded up to a power
of two and the operands are zero-padded.

| task | shape | `mp × kp × np` | padded MACs | **padding factor** |
|---|---|---|---:|---:|
| T1-0 | `[1×256]·[256×256]` | 1 × 256 × 256 | 65 536 | **1.0000** |
| T1-a | `[1×768]·[768×768]` | 1 × 1024 × 1024 | 1 048 576 | **1.7778** |

T1-0 is aligned and pays nothing. T1-a's `K = N = 768` is not a power of two, so the prover
does **1.7778× the task's arithmetic** — the same 768 → 1024 padding jolt-atlas and DeepProve
pay in this benchmark, and it is reported in the cell rather than absorbed into `MAC/s`.

---

## 4. What binds the operands — two routes, and only one of them exists over both fields

Route 1's steps 1–4 bind the claim to the public `C`. They bind **nothing about `A` and `B`**:
the two closing evaluations are numbers the prover sends. A proof of knowledge needs a
multilinear PCS. So the system is measured on two routes:

| route | what it is | available over |
|---|---|---|
| `sumcheck` | steps 1–4 only. The closing evaluations are unbound | **both fields** |
| `sumcheck-whir` | plus a WHIR commitment to `A` and `B`, opened at the prescribed points the sumcheck produced | **`koala-bear` only** |

**The cross-field cell is therefore the `sumcheck` one**, and it is the honest one: the same
protocol, the same instance, the same machine, the same codebase, two fields.
`sumcheck-whir` has no binary counterpart, and that absence is
[`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1 — measured with a compiler, not read off a grep.

### The committed route, in detail

`A` (`log M + log K` variables) and `B` (`log K + log N`) are committed as **one stacked
multilinear**, which is what `p3-sumcheck`'s layout exists for; committing them separately
would pay the FFT and Merkle overhead twice. For T1-0 the stack is `65 536 + 256` values padded
to `2^17`.

The openings are **prescribed**, not sampled: `PrescribedPointPcs::open_at` opens at exactly
the points the sumcheck produced. That trait's own documentation warns that in prescribed mode
soundness rests on the caller fixing the point through the shared transcript *after* the
commitment is absorbed, and this harness does: the order is **commit → absorb `C` → sample
`(r1, r2)` → sumcheck rounds → open**.

**Two soundness regimes were run, on the same commit, the same day (2026-09-03), and they are
not the same claim.** `WhirConfig` derives its query count under a named soundness type, and
the first campaign run used the crate's default without checking what it assumes:

| parameter | run 1 — `CapacityBound` (`bench/RESULTS.md` §5) | run 2 — `UniqueDecoding` / `G-13b′` (§6) | why |
|---|---|---|---|
| soundness type | `WhirConfig`'s default. Rests on the **mutual correlated-agreement-up-to-capacity conjecture** — Crites and Stewart, ePrint 2025/2046 (<https://eprint.iacr.org/2025/2046>, "On Reed–Solomon Proximity Gaps Conjectures", rev. 2025-12-19), list it among the conjectures they disprove | proven regime, no conjecture | `WhirSoundnessType` in the pinned crate |
| security level (declared) | 96 bits | 96 bits | binius64's `SECURITY_BITS` in this benchmark — equal-soundness **by declaration only** |
| PoW grinding budget | **16 bits** (Plonky3's own `DEFAULT_MAX_POW`) | **7 bits** — the minimum `WhirConfig::new` accepts at folding 4 (`PowBitsExceedBudget { required: 7, budget: 0 }` at budget 0; 11 bits at folding 5). Zero is never available at either folding factor | swept by hand, both regimes |
| algebraic security after PoW | **80 bits** (96 − 16) | **89 bits** (96 − 7) | WHIR does not add the grinding budget on top of the declared level — it subtracts it first: `protocol_security_level = whir_parameters.security_level.saturating_sub(whir_parameters.pow_bits)`, `whir/src/parameters/whir.rs:251-254` at the pinned commit |
| `starting_log_inv_rate` | 1 | 1 | binius64's primary cut |
| folding factor | 4 (constant) | 4 (constant) | |
| final STIR queries, T1-0 | 12 | **91** | read off the derived config, per cell |
| final STIR queries, T1-a | 9 | **90** | read off the derived config, per cell |

**The "final STIR queries: 12" this section reported before 2026-09-03 was T1-0's
`CapacityBound` figure only** — it was never checked against T1-a's own count under the same
regime (9, not 12) or against either task's count under the regime that does not rest on a
refuted conjecture (91 and 90). binius64 runs 232 FRI queries at rate 1 with **no PoW** — the
unique-decoding query count for 96 bits outright — so under run 1 the two systems were never
equal-soundness despite both declaring "96 bits." Full campaign detail, both regimes, both
tasks: `bench/systems/plonky3/RESULTS.md` §5 (`CapacityBound`) and §6 (`UniqueDecoding`).

**The two routes therefore do not carry the same Fiat-Shamir cost.** `sumcheck` grinds nothing;
`sumcheck-whir` grinds up to 16 bits per round in run 1, 7 in run 2. The difference between the
routes is the commitment **plus** that grinding, and no figure here attributes it to the
commitment alone.

**A second, separately declared cost: the 2× stacking overhead.** `A` and `B` are committed as
one stacked multilinear (above), and the committed size is not the operands' own padded size —
it is rounded up to the next power of two *of the stack*, which is close to **double** the
operands' combined size: `whir_stacked_vars` is 17 for T1-0 (`2^17 = 131 072` committed elements
for 65 792 operand values) and 21 for T1-a (`2^21 = 2 097 152` committed elements for 1 049 600
operand values) — a factor of 1.992× and 1.998× respectively. This is paid on every
`sumcheck-whir` cell in both regimes above, it is not corrected in any prove time or proof-size
figure in `RESULTS.md` §5–§7, and it is why those figures are declared there as a **floor**, not
a ceiling.

---

## 5. Weight regime, in `bench/TASKS.md` Amendment A2's vocabulary

| route | **weight regime** | **where the weight cost lands** | inside `bytes/MAC` and `MAC/s`? |
|---|---|---|---|
| `sumcheck` | **none of A2's four.** The weights are neither witnessed-and-committed, nor circuit constants, nor preprocessed, nor program data: they are *inputs to a claim the prover asserts and nothing binds*. | prove | YES — but see the next row of this file |
| `sumcheck-whir` | **`witness`** | **prove** — commitment, opening, prove time and peak memory | **YES** |

**A2's vocabulary has no word for the `sumcheck` route, and inventing one here would be worse
than saying so.** The row belongs in `RESULTS.md` §2's *"what the proof binds"* column with the
value **NOTHING about the operands**, and it must never be placed in the `witness` bucket
beside binius64 and gnark regime A. Only `sumcheck-whir` belongs there.

---

## 6. What is counted, and what is explicitly not

`reduction_field_muls = mp·kp + kp·np + 2·mp·np + mp + np` — the multiplications **our own
loops** perform to build `A~(r1,·)`, `B~(·,r2)`, the two `eq` tables and the claimed sum. For
T1-0 that is 66 561, i.e. **1.0157 field multiplies per MAC**.

**That ratio is 1 only because `M = 1`, and it is not a constant of the protocol.** The
dominant term `kp·np` does not depend on `M` at all, while the MAC count `mp·kp·np` does. So
the prover's reduction work is

```
    mp·kp + kp·np   field multiplies   against   mp·kp·np   MACs
```

— **sublinear in the number of rows**, which is the property Thaler's MATMULT exists for. Both
rungs run here have `M = 1`, so **no cell in this directory exercises it**, and no figure here
may be extrapolated to a larger `M`. T1-b (`M = 4`), T1-c (`M = 16`) and T1-d (`M = 64`) are
where it would show, and they were not run (`NOT_EXPRESSIBLE.md` §3). Stating this now, before
there are figures, is cheaper than withdrawing an extrapolation later.

**Field multiplications inside `p3-sumcheck`'s rounds are NOT counted.** The measured tree
carries no instrumentation patch (`COMMIT`), so this harness cannot see them, and no figure in
this directory claims them. The round work is `O(2^{log K})` and falls geometrically, so it is
a small fraction of the reduction — but *"small"* here is an argument, not a measurement, and it
is labelled as such.

**No `GF(2^128)` multiplication count is published for the binary cell**, for the same reason:
the toolkit exposes no counter and this harness did not add one.

---

## 7. What the circuit does *not* constrain, stated plainly

**The INT8 operands carry no range constraint, on either field or either route.** The
statement is *"the prover knows field elements which, contracted as specified, yield the
published output"* — it does **not** establish that those elements were images of bytes.

This is the same choice binius64 made and states (`../binius64/EXPRESSION.md` §5), and it is
the choice `bench/TASKS.md` anticipates when it asks systems to declare their encoding. It is
**not** the choice gnark made: gnark regime A range-checks every input and every weight and
pays **3.006×** in constraints for it (`bench/RESULTS.md` §7). **A row placing this system's
`MAC/s` beside gnark's compares a proof of *"the prover knows field elements whose contraction
is the output"* against a proof of *"the prover knows INT8 values whose product is the
output"*. Different theorems.** A production deployment would need those range constraints and
they are not in these numbers.

`p3-lookup` exists in the tree and would express them. It was **not** used, and no cost for it
is estimated here — an unmeasured estimate is what rule F.7 exists to stop.

---

## 8. Fiat–Shamir — the two fields do not share a hash

| field | challenger |
|---|---|
| `koala-bear` | `DuplexChallenger<KoalaBear, Poseidon2KoalaBear<16>, 16, 8>` |
| `binary128` | `BinaryChallenger<BinaryField128, HashChallenger<u8, Keccak256Hash, 32>>` |

There is no choice here: `p3-binary-field` ships exactly one challenger and it is byte-oriented
over a cryptographic hash, because a duplex sponge over `GF(2^128)` does not exist in the tree.
**So the cross-field comparison carries a hash difference it cannot remove.** On the `sumcheck`
route the transcript absorbs `mp·np` output elements plus `2·log K` round messages and samples
`log M + log N + log K` challenges — for T1-0, **256 absorptions and 16 samples** against the
cell's **66 561 field multiplies** — so the hash is a small term. **Small, and not zero, and
not measured separately.**

---

## 9. Witness seeds, and the cross-system instance check

The seeds are `bench/TASKS.md`'s, via `systems/binius64/EXPRESSION.md` §7: T1-0 `0xE0060100`,
T1-a `0xE00601A0`, drawn with `rand::rngs::StdRng::seed_from_u64`, operands as `i8` over the
full `[-128, 127]`, `A` row-major then `B` row-major.

That the two harnesses drew the **same numbers** is checked rather than asserted — see
[`BUILD.md`](BUILD.md) §1, *"The instance is checked against binius64's"*.

---

## 10. Which tasks were expressed

| task | `sumcheck`, both fields | `sumcheck-whir` |
|---|---|---|
| T1-0 | yes | yes (`koala-bear`) |
| T1-a | yes (padding 1.7778×) | yes (`koala-bear`) |
| T1-b, T1-c, T1-d | expressible; not run in this round | expressible; not run |
| **T2, T3** | **NOT EXPRESSIBLE on this route** | — |

T2 and T3 contain ReLU. A sumcheck over a product of two multilinears proves a bilinear form;
an activation is not one. Expressing them needs either a GKR circuit or an AIR, and neither was
written. See [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §2.

---

## 11. `G-13b''` — removing the stacking padding, and what it cost to remove it

**Status of this section:** it describes a route that exists in the harness and passes its
controls. **No campaign has been run on it.** Every figure below is either a *configuration*
fact — read off `WhirConfig` without proving anything — or a **SMOKE** measurement with one
repetition and no dispersion, and a smoke measurement is not a result.

### 11.1 What was wrong

§4 commits `A` and `B` as one stacked multilinear and justifies it with *"committing them
separately would pay the FFT and Merkle overhead twice."* That is true and it is the smaller
term. The larger one is that **the stack's arity is `log2_ceil` of the SUM of the tables' cell
counts** — `plan_layout`, `sumcheck/src/layout/plan.rs:52-57`:

```rust
let k = log2_ceil_usize(
    shapes.iter().map(|s| s.width * (1usize << s.arity)).sum::<usize>(),
);
```

With `a_vars < b_vars` the sum sits just above `2^b_vars`, so `k = b_vars + 1` and the
commitment covers **very nearly twice the operands**. `bench/RESULTS.md` A7 item 3 declares this
for T1-a and does not correct it. This section corrects it.

### 11.2 The three candidates, and which one Plonky3 permits

| | idea | verdict |
|---|---|---|
| **(a)** | two commitments, `A` under a scheme of `a_vars` variables and `B` under one of `b_vars` | **available; implemented** |
| **(b)** | one commitment sized to `2^b_vars` with `A` packed into the slack | **impossible** |
| **(c)** | one commitment, several polynomials batched *without* stacking | **does not exist: (c) IS the stacking** |

**(b) is refused by the type that carries the commitment.** A WHIR commitment is a multilinear
over a hypercube and its size is fixed by the config, not by the witness:

```rust
    fn commit(
        &self,
        witness: Self::Witness,
        challenger: &mut Challenger,
    ) -> (Self::Commitment, Self::ProverData) {
        assert_eq!(witness.num_variables(), self.config.num_variables);
```

`whir/src/pcs/adapter.rs:86-91`, at commit `3152b14a`. One `WhirConfig` is one power of two, and
`2^20 + 2^10` is not one. There is no "sized" commitment to ask for.

**(c) turns out to be the same thing as the stacking, not an alternative to it.** The harness
already passes a batch: `OpeningProtocol::new(vec![TableSpec…, TableSpec…])` with two
`TableShape`s, and `p3-whir` does support several opening claims against one commitment
(`PrescribedPointPcs::open_at`, `whir/src/pcs/adapter.rs:218-252`). But the batch is realised by
`Witness::new` (`sumcheck/src/layout/witness.rs:246-286`), which **is** the stacking: it calls
`plan_layout`, lays each column into a contiguous slot, and rounds the total up. There is no
second batching path in the tree. So the multi-polynomial API and the padding are the same
mechanism, and (c) collapses into the thing being removed.

**(a) is therefore the only option that does not require writing a PCS**, and it is what
`sumcheck-whir-split` does.

### 11.3 What the new route is, and what it deliberately keeps

`sumcheck-whir-split` (`route.rs`, `WhirSetup::Split`). **The statement and the transcript order
are unchanged**, which is the part that had to survive:

1. `commit(A)` — Merkle root absorbed;
2. `commit(B)` — second Merkle root absorbed;
3. absorb the public `C`; sample `(r1, r2)`;
4. `log K` sumcheck rounds; close on `A~(r1,r3)·B~(r3,r2)`;
5. open `A` at `(r1, r3)` and `B` at `(r3, r2)`, each under its own scheme.

Both commitments are fixed **before** the transcript produces `(r1, r2)`, which is the condition
`PrescribedPointPcs` states for prescribed openings and the one §4 records for the stacked
route. `C` stays public and the verifier still recomputes `C̃(r1, r2)` from it. The verifier
still checks that the values the commitments bind are the values the sumcheck closed on — for
the split route that is two separate `verify_open` calls, one per commitment, and the same
final equality.

**`sumcheck-whir` is untouched.** It runs the same code path through `WhirSetup::Stacked`, and
the smoke below reproduces its recorded proof size to the byte, so rows `…-n5` and `…-n6` stay
reproducible.

### 11.4 The commitments, per rung — configuration only, nothing proved

| task | route | commitments | `whir_vars` | committed elements | operands | padding | final STIR queries |
|---|---|---:|---|---:|---:|---:|---|
| T1-0 | `sumcheck-whir` | 1 | 17 | 131 072 | 65 792 | **1.9922×** | 91 |
| T1-0 | `sumcheck-whir-split` | 2 | 8 + 16 | **65 792** | 65 792 | **1.0000×** | 215 + 91 |
| T1-a | `sumcheck-whir` | 1 | 21 | 2 097 152 | 1 049 600 | **1.9980×** | 90 |
| T1-a | `sumcheck-whir-split` | 2 | 10 + 20 | **1 049 600** | 1 049 600 | **1.0000×** | 215 + 90 |

Read off `WhirConfig` at `UniqueDecoding`, 96 bits, rate 1, PoW 7, folding 4 — the §4 run-2
regime — via `p3-bench --stat-only`, which derives the configuration and proves nothing.

**The split is not free, and the cost is in the column on the right.** The `A` commitment is a
short polynomial (8 or 10 variables), and WHIR's query count *rises* as the code gets short:
**215** final queries against the stack's 90. Its Merkle paths are correspondingly short, so
this does not scale the proof the way the count alone suggests — but it is why the proof does
not shrink by the same factor the commitment does, and it is a term that would grow if `A` were
split further.

### 11.5 Proof-size accounting — the two routes do NOT use the same one

`systems/plonky3/RESULTS.md` declares that `proof_bytes_median` for `sumcheck-whir` **omits the
Merkle root**. That omission is kept for `sumcheck-whir`, because closing it would silently move
every published `…-n5` and `…-n6` figure. It is **not** kept for `sumcheck-whir-split`: a route
whose entire content is that it carries two commitments may not hide the second root, so its
`proof_bytes` includes both. The per-route root cost is published beside the cell as
`whir_root_bytes` (33 B for one root, 66 B for two, postcard), so either accounting can be
recovered from the row. **A reader comparing the two proof sizes below is comparing 132 519 B
without its root against 121 594 B with both of its roots.**

### 11.6 SMOKE — T1-0, one thread, one warm-up, ONE repetition, 2026-09-03

**These are not results.** One repetition carries no dispersion; on this machine `peak
footprint` reproduces to ±0.3 % between campaigns but nothing here establishes that these two
cells are separated by more than noise. They exist to show the route runs and that the stacked
route did not move.

| | `sumcheck-whir` | `sumcheck-whir-split` |
|---|---:|---:|
| prove | 31.66 ms | 16.50 ms |
| verify | 3.269 ms | 2.956 ms |
| `proof_bytes` | 132 519 B *(root excluded)* | 121 594 B *(both roots included)* |
| `whir_root_bytes` | 33 | 66 |
| peak footprint | 13 123 920 B | 7 209 272 B |
| committed elements | 131 072 | 65 792 |
| `whir_final_queries` | 91 | 215 + 91 |

**The control that licenses reading anything at all from the left column: 132 519 B is exactly
the `proof_bytes_median` of the published `t1-0-koala-bear-sumcheck-whir-t1-n6` cell.** The
refactor that introduced the split route left the stacked route bit-identical on the one
quantity that is deterministic.

### 11.7 Controls

`p3-negative` covers both committed routes, four corruptions each (T1-0, all REJECTED):

| kind | what it does | `sumcheck-whir` | `sumcheck-whir-split` |
|---|---|---|---|
| `weight_bit` | commit and prove a corrupted `B` against the published `C` | `sumcheck_ok=false opening_ok=false` | `sumcheck_ok=false opening_ok=false` |
| `input_bit` | the same for `A` | `sumcheck_ok=false opening_ok=false` | `sumcheck_ok=false opening_ok=false` |
| `public_output_bit` | the verifier holds a different `C` | `sumcheck_ok=false opening_ok=false` | `sumcheck_ok=false opening_ok=false` |
| `committed_binding` | **commit a corrupted `B`, prove the honest statement** | `sumcheck_ok=true opening_ok=true bound_matches=false` | `sumcheck_ok=true opening_ok=true bound_matches=false` |

The first three are the controls the route inherited, and on a committed route they all corrupt
something the sumcheck already catches: the transcript desynchronises and the opening fails with
it, which proves the proof is rejected and says **nothing** about whether the commitment binds
anything. `committed_binding` was added for exactly that gap. It commits a corrupted `B` and
runs the honest sumcheck, so the sumcheck is valid *and the WHIR opening is a valid opening* —
of the wrong polynomial. The only thing left standing between the verifier and a proof about
operands nobody committed is the equality between what the commitments bind and what the
sumcheck closed on, and that is where both routes fail it. **This is the first control in this
directory that tests the commitment rather than the sumcheck.**

Unit tests (`route.rs`): both committed routes prove the same statement on T1-0 and T1-a, both
verify, each route's opening sum equals `C̃(r1, r2)` recomputed from the public output by a
second implementation, each closing evaluation equals the operand's multilinear at the opened
point, and the split route's committed element count is **exactly** `2^a_vars + 2^b_vars`.

**What could not be asserted, and why.** The two routes' `claimed` field elements are not equal,
and no correct implementation could make them equal: the transcripts differ by construction —
one Merkle root against two, one WHIR domain separator against two — so `(r1, r2)` differ and
the claim is evaluated at a different point on each route. What the test asserts instead is the
property that equality was standing in for: on each route the claim **is** `C̃` of the same
public output at that route's own challenges, and both routes close on the multilinears of the
same `A` and `B`.
