# Plonky3 — results

**Scope: this file reports one system — SMOKE in §2, CAMPAIGN in §5–§8.** Read
[`BUILD.md`](BUILD.md) before any number here, [`EXPRESSION.md`](EXPRESSION.md) for how each
task was written — **especially §2, which says the two fields do not prove the same theorem** —
and [`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) for what did not run and why.

> ## §2's SMOKE rows are still not campaign figures. §5–§8 are.
>
> §2 below is what this file shipped with first: one repetition at one thread on a shared
> machine, produced only to establish that the cells **run**. Those rows are still labelled
> `SMOKE` in [`bench/data/cells-plonky3.csv`](../../data/cells-plonky3.csv), the report script
> still prints them in their own block, and **no SMOKE figure may be placed in
> `bench/RESULTS.md` §3, §4 or §5, quoted as a rate, or compared with any other system.**
>
> **A campaign was run on 2026-09-03**: warmup 1, N = 5 or N = 6, median with min–max, at 1 and
> 10 threads, over `sumcheck` (both fields) and `sumcheck-whir` (`koala-bear` only) — the twelve
> cells §4 below asked for. Those rows are labelled `CAMPAIGN` in the same CSV. **§5** reports
> them under the soundness regime the harness ran first (`CapacityBound`, PoW 16), which turned
> out to rest on a conjecture that is refuted. **§6** reports the same four `sumcheck-whir`
> cells re-run the same day under the corrected regime (`UniqueDecoding`, PoW 7 — `G-13b′`).
> **§7** is the one cell in this whole file admissible beside another system: T1-a, same day,
> same machine, against binius64 re-measured in the same window. **§8** answers the
> pre-registered G-13b decision criterion and says why the answer it gives is not usable as
> stated.
>
> What IS established, and did not need a campaign, is in §1: the correctness controls, the
> build-integrity gate, and the absence probe. Those are verdicts, not timings.

---

## Conditions line

Applies to §1 and §2 (the SMOKE rows) — this block predates the 2026-09-03 campaign and its
`threads`, `N` and `security` lines describe only that first, single-repetition run. **§5, §6
and §7 each carry their own conditions line** (threads 1 and 10, N = 5 or 6, and — critically —
two different soundness regimes, neither of which is the "12 final STIR queries" figure below).
Do not read `security` or `N` here into any campaign section.

```
system      Plonky3
commit      3152b14a89067c83775a8076cc262ffc48a1fd7c (pristine; no local patch)
git describe  p3-whir-v0.6.0-177-g3152b14a
maturity    per crate, in COMMIT. p3-binary-field is FOUR DAYS OLD at this commit
            (first commit 2026-08-30, two commits total); p3-binary-dft is one day and
            one commit. The tree's only audit is dated 2024-08-01 and predates every
            crate measured here.
expression  Thaler's MATMULT on p3-sumcheck. NOT a constraint system: no intermediate
            is committed. EXPRESSION.md §1. `constraints` does not exist for this
            system; its natural units are (sumcheck rounds, reduction field multiplies)
            and both are in every row.
fields      koala-bear = KoalaBear (p = 2^31 - 2^24 + 1) + BinomialExtensionField<_,4>
            binary128  = BinaryField128, p3-binary-field's Wiedemann tower, F = EF
            THE TWO DO NOT PROVE THE SAME THEOREM. EXPRESSION.md §2.
routes      sumcheck       — no commitment; the closing evaluations are unbound
            sumcheck-whir  — WHIR commitment to A and B, prescribed-point opening
                             KOALA-BEAR ONLY. NOT_EXPRESSIBLE.md §1.
security    sumcheck route: soundness is (rounds x degree)/|EF|, |EF| = 2^124 for
            koala-bear and 2^128 for binary128. No PoW.
            sumcheck-whir: 96 bits declared, rate 1, folding 4, PoW budget 16 bits
            (zero is REJECTED by WhirConfig::new), 12 final STIR queries.
trusted setup   no
ZK              no
quantization    signed INT8 in [-128,127]; NO RANGE CONSTRAINT on either field or
                either route (EXPRESSION.md §7) — the same omission binius64 declares
                and the opposite of gnark regime A, which pays 3.006x for it
weights         sumcheck:      NONE OF AMENDMENT A2's FOUR REGIMES. Nothing binds them.
                sumcheck-whir: WITNESS; weight cost in PROVE.
                EXPRESSION.md §5.
padding         T1-0 1.0000x (aligned) · T1-a 1.7778x (768 -> 1024 on K and N)
threads     1, via RAYON_NUM_THREADS. NO 10-THREAD CELL WAS RUN.
machine     Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, NOT dedicated
OS          macOS 26.5.2, Darwin 25.5.0
N           1 per cell, warmup 1. THIS IS BELOW THE PROTOCOL'S N >= 5.
date        2026-09-03
```

---

## 1 · What IS established: the controls

These are verdicts and they do not need a campaign.

### 1.1 Correctness — 11 corruptions, 11 rejected

`bench/scripts/plonky3/run-negative.sh t1-0`, raw output in
[`bench/data/negative-plonky3/report.txt`](../../data/negative-plonky3/report.txt). Every
operand corruption recomputes the reference forward pass first, per `bench/TASKS.md`
Amendment A3; none was inert.

| task | field | route | corruption | verdict |
|---|---|---|---|---|
| T1-0 | koala-bear | sumcheck | `weight_bit` — `B[0][0]` low bit, 32 → 33 | **REJECTED** |
| T1-0 | koala-bear | sumcheck | `input_bit` — `A[0][0]` low bit, −125 → −126 | **REJECTED** |
| T1-0 | koala-bear | sumcheck | `public_output_bit` — `C[0][0] + 1` on the verifier's side | **REJECTED** |
| T1-0 | koala-bear | sumcheck | `round_message` — round 0, `h(0) + 1` | **REJECTED** |
| T1-0 | koala-bear | sumcheck | `closing_opening` — `A~(r1,r3) + 1` | **REJECTED** |
| T1-0 | binary128 | sumcheck | the same five | **REJECTED** ×5 |
| T1-0 | koala-bear | **sumcheck-whir** | `weight_bit`, against the commitment | **REJECTED** (`sumcheck_ok=false opening_ok=false`) |

**What each one establishes is not the same, and the strongest is not the weight flip.**

* `weight_bit` and `input_bit` prove the CORRUPTED operands against the PUBLISHED output. On
  the `sumcheck` route this shows the claim is bound to `C`; it shows **nothing** about binding
  the operands, because nothing binds them. On `sumcheck-whir` the same flip additionally has
  to survive the commitment, and the run reports both halves failing independently.
* `public_output_bit` corrupts the statement itself and is the strongest control available on
  either route.
* `round_message` and `closing_opening` corrupt the proof after the fact and are
  route-independent.

**`sumcheck-whir` is the only route here that binds the weights.** `bench/RESULTS.md` §2's
column, filled in: `sumcheck` binds **nothing about the operands**; `sumcheck-whir` binds
**the operands, per proof**.

### 1.2 Build integrity — PASS, and one number that is a result rather than a check

[`BUILD.md`](BUILD.md) §2, raw output in
[`bench/data/probe-p3-fieldmul.txt`](../../data/probe-p3-fieldmul.txt), 3 launches:

| | measured | criterion |
|---|---:|---|
| `p3-binary-field` B128 × B128 | **57.8–58.1 Mmul/s** | — |
| hand-written 6-PMULL (GHASH basis, byte-identical to E-001's) | 1 012.2–1 015.5 Mmul/s | — |
| `p3-B128` / hand-written | **0.057** | floor 0.02 — **PASS** |
| `p3-koala-bear` KoalaBear × KoalaBear | **1 761.6–1 770.5 Mmul/s** | — |
| raw u32 multiplies per KoalaBear multiply | 5.5 | ceiling 24 — **PASS** |

**The 0.057 is not a broken build.** It is a 17.5× algorithm difference: binius64 and the
reference kernel store `GF(2^128)` in the GHASH polynomial basis and pay 6 `PMULL`;
`p3-binary-field` stores it in the **Wiedemann tower basis** and pays 4 `clmul` plus a
byte-table change of basis in both directions. So *"the binary field"* is not one substrate,
and **a comparison between Plonky3's binary field and binius64's compares two representations
of `GF(2^128)` as much as it compares two provers.**

Measured on the same machine within four days of E-006, the shared rows reproduce to within
1 % (raw PMULL 3 220 vs 3 130–3 227 Mops/s; hand-written 1 012–1 016 vs 1 002–1 015 Mmul/s),
which is what makes any comparison with binius64's directory admissible at all.

**Within this system, at the multiply:** `KoalaBear` runs **30.4×** the rate of
`BinaryField128` (1 766 / 58.0, same process, same launch). That ratio is a property of the two
implementations, not of the two fields.

### 1.3 The absence of a binary-field PCS — measured with a compiler

[`NOT_EXPRESSIBLE.md`](NOT_EXPRESSIBLE.md) §1, raw output in
[`bench/data/probe-plonky3-whir-binary.txt`](../../data/probe-plonky3-whir-binary.txt).
`BinaryField128: TwoAdicField` is not satisfied, and cannot be: `|GF(2^128)*| = 2^128 − 1` is
odd. The binary field reaches the Merkle commitment (`p3-binary-dft`'s additive-NTT `Encoder`,
consumed by `p3_sumcheck::commit::commit_base`) and stops there.

---

## 2 · The smoke rows — they establish that the cells run, and nothing else

> **NOT RESULTS.** N = 1, warmup 1, one thread, shared machine, `loadavg` 3.20 and 5.4 GB of
> swap committed at launch. `(user+sys)/real` is unusable at these durations — `/usr/bin/time`
> reports `real` to 0.01 s and two of the three cells finished inside one tick.

| task | field | route | N | prove ms | verify ms | proof B | peak footprint B | rounds | reduction field muls | padding | int-faithful |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| T1-0 | koala-bear | sumcheck | 1 | 0.406 | 0.0371 | 290 | 2 703 672 | 8 | 66 561 | 1.0000 | **True** |
| T1-0 | binary128 | sumcheck | 1 | 3.494 | 0.0535 | 339 | 2 965 816 | 8 | 66 561 | 1.0000 | **False** |
| T1-0 | koala-bear | sumcheck-whir | 1 | 38.981 | 1.0765 | 42 505 | 9 339 192 | 8 | 66 561 | 1.0000 | **True** |

`setup` for the WHIR cell — deriving the round configuration, the Poseidon2 constants, the
Merkle scheme and the DFT twiddles — is **0.169 ms**, reported apart and never folded into
prove. The other two routes have no setup at all.

### 2.1 What these three rows do and do not say

**They say the three cells run and produce proofs their own verifiers accept.** That is what a
smoke row is for.

**They do not say what any of the ratios between them is.** One repetition has no dispersion;
E-006's own campaign measured 5–6 % drift on this machine between a published run and a rerun
of the identical cell, and 22.9 % dispersion on `peak RSS`. A ratio of two single runs carries
both errors and no bar.

Three things are nevertheless worth writing down as **directions to check with a campaign**,
marked as hypotheses and not as figures:

1. **`binary128 / koala-bear` on `prove` reads ≈ 0.12 here.** The gate's own multiply ratio is
   0.033 (58.0 / 1 766) — so the protocol ratio is **less lopsided than the multiply ratio**,
   which is what one would expect if the round work is not dominated by base-field multiplies.
   A campaign would say whether that survives. **The G-13b decision criterion (`≥ 1/2` = the
   binary field does not lose, `≤ 1/10` = it loses) must not be evaluated on this row.**
2. **The commitment costs about two orders of magnitude of what the sumcheck costs**, on this
   rung, on this route: 38.98 ms against 0.41 ms, and 42 505 B against 290 B. But the two
   routes also do not carry the same Fiat-Shamir cost — `sumcheck-whir` grinds up to 16 bits
   per round and `sumcheck` grinds nothing (`EXPRESSION.md` §4) — so the difference is the
   commitment **plus** the grinding and nobody here has separated them.
3. **Peak footprint is 2.7–9.3 MB.** Three orders of magnitude below binius64's 0.535 GB at the
   same rung — which is a statement about the two EXPRESSIONS, not about the two provers: this
   route commits no intermediate at all. It is the reason a `bytes/MAC` from this system does
   not belong in the same column as one from a constraint-system prover, and it is the reason
   §3 exists.

---

## 3 · What this system may and may not be placed beside

Written before there are figures, so that it constrains them rather than excusing them.

| comparison | admissible? | why |
|---|---|---|
| `koala-bear/sumcheck` vs `binary128/sumcheck`, same task, same campaign | **YES, with the theorem caveat in the same line.** Same protocol, same instance, same machine, same codebase | this is the cell the campaign exists to produce |
| `sumcheck` vs `sumcheck-whir` on `koala-bear` | **YES, with the PoW caveat in the same line** | `EXPRESSION.md` §4 |
| Plonky3 `binary128` vs binius64 | **NOT as a field comparison.** Two representations of `GF(2^128)` (§1.2) AND two expressions of the task AND two theorems | §1.2, `EXPRESSION.md` §1 and §2 |
| Plonky3 `bytes/MAC` vs any constraint-system prover's | **NO** | this route commits no intermediate; the two denominators cover different constructions |
| Plonky3 `sumcheck` in `bench/RESULTS.md` §1's `witness` bucket | **NO** | nothing binds the operands. Only `sumcheck-whir` belongs there |
| Plonky3 in `bench/RESULTS.md` §6 (batching) | **NO CELL** | T2 and T3 are not expressible on this route |
| Plonky3 `MAC/s` vs gnark regime A's | **NOT without §7's decomposition** | gnark proves INT8-ness and this system does not |

---

## 4 · What a campaign would have to run

**Satisfied 2026-09-03 for items (1) and (2) below** — see §5–§8. Items (3) and (4) are still
open: the ladder above `t1-a` was not run this campaign, and no `p3-fieldmul-sanity` rerun
followed the last cell. Left as originally written, as the request that §5–§8 answer:

For this directory to carry a result rather than a smoke:

1. `warmup 1, reps 5`, at **1 and 10 threads**, for `t1-0` and `t1-a`, over both fields on the
   `sumcheck` route and over `koala-bear` on `sumcheck-whir` — the twelve cells of
   `scripts/plonky3/run-all.sh` §3 and §4;
2. with **no subagent and no other load on the machine**, which is the condition E-006's ninth
   recorded failure imposed;
3. the T1 ladder above `t1-a`, which is where the memory question lives
   (`NOT_EXPRESSIBLE.md` §3);
4. and a rerun of `p3-fieldmul-sanity` **after** the last cell, as binius64's §2 does, to show
   no build or thermal drift across the campaign.

Nothing in this file should be quoted until (1) and (2) have been done.

---

## 5 · Campaign, 2026-09-03 — `CapacityBound`, PoW 16

The first run of the day. All twelve cells §4 asked for: `t1-0` and `t1-a`, both fields on
`sumcheck`, `koala-bear` only on `sumcheck-whir`, 1 and 10 threads. `MAC/s` is the published MAC
count (§6 of this file's own text; A6 of `bench/RESULTS.md`) divided by the median prove time —
**never** the padded count, so T1-a's 1.7778× padding shows up as a worse rate, not a better one.

**Conditions.** Machine: Apple M1 Max, 10 physical / 10 logical cores, 32 GiB, macOS 26.5.2
(Darwin 25.5.0), **not dedicated**. Date: 2026-09-03, morning (`…-n5` labels,
`started_utc` ≈ 10:25 UTC in `bench/data/cells-plonky3.csv`). Warmup 1, N = 5, median with
[min–max], threads ∈ {1, 10} via `RAYON_NUM_THREADS`. Soundness regime: `WhirConfig`'s
`CapacityBound` (the crate's default at this commit), which rests on the mutual
correlated-agreement-up-to-capacity conjecture. Declared security 96 bits, PoW budget **16
bits** (Plonky3's own `DEFAULT_MAX_POW`), rate 1 (`starting_log_inv_rate`), folding factor 4
(constant). **No range constraint** on the INT8 operands, on either field or either route
(`EXPRESSION.md` §7) — matches binius64's declared omission (`../binius64/EXPRESSION.md` §5).
`K` and `N` padded to the next power of two: T1-0 is aligned (1.0000×), T1-a's `K = N = 768`
pads to 1024 (**1.7778×**). `whir_stacked_vars` = 17 (T1-0) / 21 (T1-a): `A` and `B` are
committed as **one stacked multilinear**, and the committed size is not the sum of the two
operands rounded up once — it is rounded up to `2^17` / `2^21`, which is **≈ 2× the operands'
own padded size** (T1-a: 2 097 152 committed elements for 1 049 600 operands, i.e. 1 048 576
from `K×N` plus 1 024 from `M×K`). That 2× is a stacking cost this campaign does not remove;
every WHIR figure below carries it.

| task | field | route | threads | prove median [min–max] ms | verify ms | proof B | peak footprint GB | MAC/s (÷ published MACs) | padded/published MACs | WHIR final queries | integer_faithful |
|---|---|---|---:|---:|---:|---:|---:|---:|---|---:|---|
| T1-0 | koala-bear | sumcheck | 1 | 1.716 [1.072–2.160] | 0.1859 | 290 | 0.0022 | 38 180 017 | 65 536/65 536 | — | True |
| T1-0 | binary128 | sumcheck | 1 | 3.495 [3.410–3.587] | 0.0424 | 339 | 0.0036 | 18 750 688 | 65 536/65 536 | — | False |
| T1-a | koala-bear | sumcheck | 1 | 5.114 [4.982–6.083] | 0.1144 | 354 | 0.0157 | 115 329 520 | 1 048 576/589 824 | — | True |
| T1-a | binary128 | sumcheck | 1 | 56.876 [56.541–57.479] | 0.1365 | 413 | 0.0284 | 10 370 403 | 1 048 576/589 824 | — | False |
| T1-0 | koala-bear | sumcheck | 10 | 0.331 [0.326–0.353] | 0.0340 | 290 | 0.0023 | 198 218 526 | 65 536/65 536 | — | True |
| T1-0 | binary128 | sumcheck | 10 | 3.549 [3.428–3.692] | 0.0393 | 339 | 0.0036 | 18 465 396 | 65 536/65 536 | — | False |
| T1-a | koala-bear | sumcheck | 10 | 5.095 [5.035–5.416] | 0.1133 | 354 | 0.0158 | 115 757 694 | 1 048 576/589 824 | — | True |
| T1-a | binary128 | sumcheck | 10 | 56.980 [56.749–57.193] | 0.1333 | 413 | 0.0284 | 10 351 376 | 1 048 576/589 824 | — | False |
| T1-0 | koala-bear | sumcheck-whir | 1 | 40.842 [40.724–41.311] | 1.1164 | 42 505 | 0.0131 | 1 604 618 | 65 536/65 536 | 12 | True |
| T1-a | koala-bear | sumcheck-whir | 1 | 618.559 [615.262–630.288] | 1.7758 | 63 935 | 0.1040 | 953 545 | 1 048 576/589 824 | 9 | True |
| T1-0 | koala-bear | sumcheck-whir | 10 | 14.192 [12.441–16.617] | 1.1419 | 42 857 | 0.0155 | 4 617 745 | 65 536/65 536 | 12 | True |
| T1-a | koala-bear | sumcheck-whir | 10 | 106.145 [103.963–111.873] | 1.7467 | 63 839 | 0.1131 | 5 556 800 | 1 048 576/589 824 | 9 | True |

**One cell in this table is contaminated, and it is kept, not deleted.**
`t1-0-koala-bear-sumcheck-t1-n5` — the first row above — shows `real_s` 0.30, `user_s` 0.01,
`cpu_ratio` 0.033 in the raw ledger (`bench/data/cells-plonky3.csv`), and its own
`prove_median_nanos` is 1 716 500 ns against **331 042 ns** measured for the same cell re-run as
`t1-0-koala-bear-sumcheck-t1-n6` (§6) — 5.19× slower than its own re-run and than its 10-thread
twin in the same campaign (330 625 ns). Something external stole time from this one repetition
set; the raw file is not edited and this row is not removed, following the same rule A6 of
`bench/RESULTS.md` applied to a padding artifact.

**`proof_bytes_median` omits the Merkle root of the WHIR commitment.** The harness reports the
serialized proof body; the root is a fixed 32-byte value carried separately in the transcript
setup, not inside the proof the prover emits per call. At T1-0 that is 32 / 42 505 = 0.075 % of
the reported figure; at T1-a, 32 / 63 935 = 0.050 %. Declared, not corrected — the omission is
under 0.1 % everywhere in this table and does not change any ratio in §7 at the precision it is
quoted.

---

## 6 · G-13b′ — the same cells under `UniqueDecoding`, PoW 7 (2026-09-03, same day)

**Why the first run's soundness regime does not hold.** `CapacityBound` is `WhirConfig`'s
default query-derivation mode at this commit, and it rests on the **mutual
correlated-agreement-up-to-capacity conjecture**. Crites and Stewart, ePrint 2025/2046
(<https://eprint.iacr.org/2025/2046>, "On Reed–Solomon Proximity Gaps Conjectures", revision
2025-12-19), list that exact conjecture among the ones they disprove. §5's `sumcheck-whir`
figures are proven under an assumption now known to be false.

**Why the PoW budget was not free soundness on top of 96 bits.** WHIR does not add its
grinding budget to the algebraic security level — it **subtracts** the PoW budget from the
declared security level before deriving the query count:

```rust
// PoW contributes an independent additive term to security,
// so the algebraic protocol only needs to cover the remainder.
let protocol_security_level = whir_parameters
    .security_level
    .saturating_sub(whir_parameters.pow_bits);
```

(`whir/src/parameters/whir.rs:251-254` at the pinned commit `3152b14a`, read directly in the
clone measured for `BUILD.md`). So §5's declared "96 bits, PoW 16" is **80 algebraic bits plus
16 bits of grinding**, and the 9 (T1-a) / 12 (T1-0) final queries in §5 are the query count for
80 bits, not 96. binius64 runs 232 FRI queries at rate 1 with **no PoW** (`../binius64/RESULTS.md`
Conditions line, `SECURITY_BITS = 96`) — the unique-decoding query count for 96 bits outright.
The two were never equal-soundness, and §5's proof-size and query figures may not be placed
beside binius64's.

**The rerun.** Same four `sumcheck-whir` cells, same day, `SOUNDNESS = UniqueDecoding` (no
conjecture — proven regime), `POW_BITS = 7` — the minimum `WhirConfig::new` accepts at folding
factor 4 (`PowBitsExceedBudget { required: 7, budget: 0 }` at budget 0, confirmed by sweeping
the parameter space, `EXPRESSION.md` §4). N = 6 reps this time (one more than §5, no fewer),
same machine, same warmup, same threads. The **2× stacking overhead** (`whir_stacked_vars`, §5)
is still unpaid-for and still not corrected: these figures remain a **floor**.

| task | field | route | threads | prove median [min–max] ms | verify ms | proof B | peak footprint GB | MAC/s (÷ published MACs) | padded/published MACs | WHIR final queries | integer_faithful |
|---|---|---|---:|---:|---:|---:|---:|---:|---|---:|---|
| T1-a | koala-bear | sumcheck-whir | 1 | 538.606 [536.819–540.166] | 5.5880 | 228 814 | 0.1070 | 1 095 093 | 1 048 576/589 824 | 90 | True |
| T1-a | koala-bear | sumcheck-whir | 10 | 98.692 [91.145–124.124] | 5.5911 | 229 150 | 0.1109 | 5 976 387 | 1 048 576/589 824 | 90 | True |
| T1-0 | koala-bear | sumcheck-whir | 1 | 30.792 [30.354–31.312] | 3.2677 | 132 519 | 0.0132 | 2 128 316 | 65 536/65 536 | 91 | True |
| T1-0 | koala-bear | sumcheck | 1 | 0.331 [0.328–0.364] | 0.0339 | 290 | 0.0022 | 197 968 838 | 65 536/65 536 | — | True |

The fourth row is the re-run of the contaminated §5 cell, on the `sumcheck` route (no PCS, so
no soundness regime applies to it) — included here because it shares the `n6` label and campaign
window, not because it belongs to the soundness-regime story. Its own prove time, 331 042 ns
median, is what §5 compares its contaminated twin against.

**Predicted against measured.** Before this rerun, the query jump from `CapacityBound` to
`UniqueDecoding` was predicted at T1-a: queries 9 → 97, proof size ≈ 600 KB. Measured: **90
queries, 228 814 B**. The direction was right — both increase substantially — and the
magnitude was not: proof size does not scale linearly with query count (each additional query
adds a Merkle authentication path, not a fixed per-query byte cost, and the stacked-commitment
depth is shared across queries). The prediction and its error are both recorded here, not
silently replaced by the measurement.

---

## 7 · The one same-day, same-regime cross-system cell

T1-a, `koala-bear`, `sumcheck-whir`, `UniqueDecoding`/PoW 7 (§6) against binius64's T1-a
re-measured the same day (`bench/data/cells.csv`, labels `t1-a-r1-t1-n6` and
`t1-a-r1-t10-n6`; per-cell detail in `bench/data/cells/t1-a-r1-t1-n6/cell.json` and
`.../t1-a-r1-t10-n6/cell.json`).

**Header, read before the numbers.** One cell. `M = 1`. Output public on both sides. **No
range check on either side** — neither system constrains the INT8 operands to be bytes
(`EXPRESSION.md` §7; `../binius64/EXPRESSION.md` §5). Plonky3's figures carry the **2× stacking
overhead** from §5/§6, uncorrected — they are a **floor**, not a ceiling. Both runs sit inside
the same campaign window with `loadavg_1m` 8.3–9.1, from this campaign's own preceding runs, not
an external process. Both processes have 5 376 MB of swap committed. **`MATMULT` does not
express T2/T3 or a chain of layers** — this cell says nothing about ReLU or multi-layer cost.

| | Plonky3 KoalaBear, MATMULT + WHIR, `UniqueDecoding`, PoW 7 | binius64, rate 1, no PoW | ratio |
|---|---:|---:|---:|
| prove, 1 thread | **538.6 ms** [536.8–540.2] | 2 741.1 ms [2 669.8–3 822.6] | **5.09×** |
| prove, 10 threads | **98.7 ms** [91.1–124.1] | 881.1 ms [814.2–1 775.2] | **8.93×** |
| verify | **5.59 ms** | 73.5 ms (1 thr) / 33.1 ms (10 thr) | 13× / 5.9× |
| proof | **228 814 B** | 460 304 B | **2.01×** |
| peak footprint | **0.107 GB** / 0.111 GB | 7.27 GB / 7.29 GB | **68×** |

Under §5's conjectured regime the same comparison read 4.26× / 6.70× on prove time and **7.2×**
on proof size — the regime correction moved the proof-size ratio the most. binius64's own T1-a
drifted +4.0 % between the 2026-08-24 campaign (2 634.9 ms) and this same-day re-measurement
(2 741.1 ms); within the machine's own run-to-run dispersion, not a system change.

---

## 8 · The pre-registered G-13b question was not answered

`DECISIONS.md` D-013 (`DEC-13.4`) registers `G-13b` as "tile INT8 de E-001 sobre Plonky3
directo (no zkVM), CPU vs CPU en esta máquina, conteo de constraints de ambos lados en la misma
frase" — a same-task, same-machine comparison of the two fields. This directory has no
constraint count (§1 of this file: *"`constraints` does not exist for this system"*), so §2.1's
own decision rule, stated before there were figures, is the one in force here: **`≥ 1/2`** on
`MAC/s(binary128) / MAC/s(koala-bear)` means the binary field does not lose; **`≤ 1/10`** means
it does. The cell both criteria need is T1-a, 1 thread, `sumcheck` route (§5).

**The ratio.** `prove(binary128) / prove(koala-bear)` = 56.876 ms / 5.114 ms ≈ **11.12× slower**,
or as a rate ratio, `MAC/s(binary128) / MAC/s(koala-bear)` = 10 370 403 / 115 329 520 =
**0.090**. By the pre-registered criterion (`≥ 1/2` = the binary field does not lose; `≤ 1/10` =
it loses), **0.090 ≤ 1/10 reads as a loss for the binary field.**

**Why that reading does not settle anything.** Two reasons, both already declared in this
directory:

1. **The two cells do not prove the same theorem** (`EXPRESSION.md` §2). `koala-bear`'s
   `integer_faithful = True`; `binary128`'s is `False`. Characteristic 2 makes `−1 = 1`, so the
   binary cell's field product is not the task's INT8 product — it measures **the same
   protocol on the same-shaped bilinear form over a different substrate**, not T1.
2. **The 11.12× / 0.090 gap is a representation gap, not a field gap.** `p3-binary-field`'s
   Wiedemann tower multiplies at 57.8 Mmul/s; the GHASH kernel binius64 uses on the same CPU
   multiplies at 1 012 Mmul/s (`§1.2` above) — a **17.5×** difference in the multiply itself,
   measured with the same build-integrity gate on the same machine. The protocol-level ratio
   (0.090, i.e. ~11×) is **less lopsided** than the multiply-level ratio (0.033, i.e. ~30×),
   which is the opposite of what a field-cost explanation predicts and consistent with the
   round work not being dominated by base-field multiplies at `M = 1`.

**There is no PCS route that would settle it either.** `sumcheck-whir` exists for `koala-bear`
only; `BinaryField128: TwoAdicField` is not satisfied and cannot be (`|GF(2^128)*| = 2^128 − 1`
is odd), so no commitment-bearing binary cell exists in this codebase to compare against (§1.3,
`NOT_EXPRESSIBLE.md` §1). **G-13b as pre-registered — a constraint-count comparison — cannot be
answered here at all: this route has no constraints.** And the cell that stands in for it
measures a representation, not a field. This is reported as the finding, not patched with an
estimate.

---

## 9 · G-13b″ — the split commitment, same window

`bench/RESULTS.md` A8, `EXPRESSION.md` §11. §5–§7 above carry a **2× stacking overhead**: `A`
and `B` were committed as one multilinear whose arity is `log2_ceil` of the SUM of the two
tables' cell counts (`sumcheck/src/layout/plan.rs:52-57`), which for T1-a rounds `1 049 600`
operands up to `2 097 152` committed elements — very nearly double. §6 and §7 declared every
`sumcheck-whir` figure a **floor** because of it and did not correct it. This section corrects
it with a new route, `sumcheck-whir-split`, that commits `A` and `B` under two separate WHIR
schemes instead of one stacked one. **`sumcheck-whir` is untouched** — same code path, same
`WhirSetup::Stacked` — and its rows (`…-n5`, `…-n6`) stay exactly as published above.

### 9.a Why this route, and why it was the only one available

Three ways to remove the 2× were considered (`EXPRESSION.md` §11.2):

| | idea | verdict |
|---|---|---|
| (a) | two commitments, `A` under a scheme of `a_vars` variables and `B` under one of `b_vars` | **available; implemented** |
| (b) | one commitment sized to `2^b_vars`, with `A` packed into the slack | **impossible** |
| (c) | one commitment, several polynomials batched *without* stacking | **does not exist — (c) IS the stacking** |

**(b) is refused by the type that carries the commitment.** A WHIR commitment is a multilinear
over a hypercube sized by the config, not by the witness: `commit()` asserts
`witness.num_variables() == self.config.num_variables` (`whir/src/pcs/adapter.rs:86-91`, commit
`3152b14a`). One `WhirConfig` is one power of two, and `2^20 + 2^10` is not one — there is no
"sized" commitment to ask for. **(c) collapses into the thing being removed.** The harness
already passes a batch of two `TableSpec`s to `PrescribedPointPcs::open_at`
(`whir/src/pcs/adapter.rs:218-252`), but the batch is realized by `Witness::new`
(`sumcheck/src/layout/witness.rs:246-286`), which calls the same `plan_layout` that does the
stacking — there is no second batching path in the tree. **(a) was therefore the only option
that did not require writing a PCS**, and it is what `sumcheck-whir-split` does. The statement
and transcript order are unchanged: `commit(A)` then `commit(B)`, both roots absorbed before the
transcript fixes `(r1, r2)`, `C` stays public and re-evaluated by the verifier, and each
commitment is opened separately at the point the sumcheck produced for it — the same condition
`PrescribedPointPcs` states for prescribed openings and the one §4/§5 record for the stacked
route.

### 9.b The cell — `koala-bear`, `sumcheck-whir-split`, `UniqueDecoding`, PoW 7, N = 6

Source: `bench/data/cells-plonky3.csv` rows `*-sumcheck-whir-split-*-n6`;
`bench/data/cells-plonky3/<label>/cell.json` for `whir_vars`, `whir_final_queries`,
`whir_committed_elements`, `whir_padding_factor`. **`WHIR padding` here is the commitment's own
padding factor** (§11.4 of `EXPRESSION.md`) — it is not T1-a's `1.7778×` `K×N` padding from §5,
which is unrelated and still applies to the matmul dimensions on both the stacked and the split
route. `MAC/s` divides published MACs (65 536 for T1-0, 589 824 for T1-a) by the median prove
time, per §5's own rule.

| task | threads | prove median [min–max] ms | verify ms | proof B (both roots included) | peak footprint GB | MAC/s (÷ published MACs) | `whir_vars` | `whir_final_queries` | `whir_committed_elements` | WHIR padding |
|---|---:|---:|---:|---:|---:|---:|---|---|---:|---:|
| T1-0 | 1 | 16.237 [15.961–16.372] | 2.9976 | 121 594 | 0.0072 | 4 036 260 | 8+16 | 215+91 | 65 792 | 1.0000 |
| T1-0 | 10 | 7.535 [7.128–14.774] | 3.0498 | 122 378 | 0.0092 | 8 697 521 | 8+16 | 215+91 | 65 792 | 1.0000 |
| T1-a | 1 | 273.202 [271.872–285.524] | 5.5886 | 221 794 | 0.0714 | 2 158 927 | 10+20 | 215+90 | 1 049 600 | 1.0000 |
| T1-a | 10 | 57.230 [53.978–61.740] | 5.6266 | 222 210 | 0.0740 | 10 306 188 | 10+20 | 215+90 | 1 049 600 | 1.0000 |

`whir_committed_elements` now equals the operand count exactly (65 792 = 65 536 `K×N` + 256
`M×K`; 1 049 600 = 1 048 576 + 1 024) — the stacking's ≈2× is gone. The 10-thread T1-0 row's
[7.128–14.774] spread is wide relative to its own median (max is 1.96× the min); this matches
the pattern §5 already flagged for short single-thread cells on a shared machine and is not
specific to this route.

### 9.c The cross-system row — T1-a, same day as §7's binius64 re-run

**Header, read before the numbers.** One cell. `M = 1`. Output public on both sides. **No range
check on either side.** `MATMULT` does not express T2/T3 or a chain of layers. **This is a
system-vs-system cell, not a measurement of the field** — everything §7's header says about
scope applies unchanged. binius64: `bench/data/cells.csv` labels `t1-a-r1-t1-n6` /
`t1-a-r1-t10-n6`, same 2026-09-03 re-run §7 already uses (`started_utc` 10:40 UTC); Plonky3's
split cells ran the same day at 11:16–11:21 UTC, ~40 minutes later, at lower `loadavg_1m` (3.7–3.8
against binius64's 8.3–9.1) — same day, not the identical minute; noted rather than assumed.

| | Plonky3 KoalaBear, MATMULT + WHIR **split** | binius64 (2026-09-03) | ratio |
|---|---:|---:|---:|
| prove, 1 thread | **273.2 ms** [271.9–285.5] | 2 741.1 ms [2 669.8–3 822.6] | **10.03×** |
| prove, 10 threads | **57.2 ms** [54.0–61.7] | 881.1 ms [814.2–1 775.2] | **15.4×** |
| verify | **5.59 ms** | 73.5 ms (1 thr) / 33.1 ms (10 thr) | 13× / 5.9× |
| proof (roots included) | **221 794 B** | 460 304 B | **2.08×** |
| peak footprint | **0.071 GB** / 0.074 GB | 7.27 GB / 7.29 GB | **102×** |
| committed elements | 1 049 600 (= operands) | 589 824 IMUL padded to 1 048 576 | — |

Every ratio reproduces from `cell.json`/`cells.csv` to the precision `bench/RESULTS.md` A8
publishes it at (verify: 5.5886 ms / 5.6266 ms, essentially flat across thread counts, is quoted
as the single figure 5.59 ms against binius64's two).

### 9.d The cost of the split, declared

The short `A` commitment (8 or 10 variables) needs **215** final STIR queries against the
stack's 90–91: WHIR's query count *rises* as the code shortens, and 215 holds for both rungs
even though `a_vars` differs (8 vs 10). Its Merkle paths are correspondingly short, so the proof
does not grow by the same factor the query count suggests. Comparing like-for-like (adding back
the root each accounting omits — see 9.e): T1-a's split proof is **3.08 %** smaller than its
stacked twin (228 814 + 33 = 228 847 B stacked vs 221 794 B split) — the figure `bench/RESULTS.md`
A8 reports as "shrinks only 3 %". T1-0's equivalent shrink is larger, **8.27 %** (132 519 + 33 =
132 552 B vs 121 594 B) — not stated in A8, computed here from the same two cells; T1-0's stacked
baseline is much smaller, so the fixed ~11 KB the short commitment's extra queries cost is a
bigger fraction of it.

### 9.e Accounting note — the two routes do not count the same bytes

`sumcheck-whir`'s `proof_bytes_median` **omits** the WHIR Merkle root (`whir_root_bytes = 33` B,
postcard) — kept that way because correcting it would move every published `…-n5`/`…-n6` figure
in §5–§7. `sumcheck-whir-split`'s `proof_bytes_median` **includes both** roots
(`whir_root_bytes = 66` B) — a route whose entire content is that it carries two commitments may
not hide the second one. Both omissions are recoverable from the row (`whir_root_bytes` is
published beside every split cell). **9.c's `221 794 B` therefore already includes what §7's
`228 814 B` for the stacked route does not** — the two are comparable as published, and 9.d's
3.08 % correction is the like-for-like version, not the headline one.

### 9.f Controls

`p3_negative.rs` (`COMMITTED_ROUTES = [SumcheckWhir, SumcheckWhirSplit]`) runs four corruption
kinds — `weight_bit`, `input_bit`, `public_output_bit`, and `committed_binding` — against each
committed route, plus the five `sumcheck` corruptions (§1.1) on both fields. For one task that is
`2×5` (`sumcheck`, both fields) `+ 2×4` (both committed routes) `= 18` corruptions, all designed
to REJECT. `committed_binding` is new here: it commits a corrupted `B` and runs the honest
sumcheck on the true statement, so the sumcheck is valid and the WHIR opening is a valid opening
— of the wrong polynomial. Every other control here corrupts something the sumcheck itself
already desynchronizes on; `committed_binding` is the first control in this directory that tests
what the commitment *binds* rather than whether the proof is rejected, and both committed routes
reject it (`sumcheck_ok=true opening_ok=true bound_matches=false`).

**Discrepancy to flag, not silently resolved.** `bench/RESULTS.md` A8 reports "18 corruptions, 18
rejected." The `18` is exactly what the current `p3_negative.rs` source runs for one task (above)
— but `bench/data/negative-plonky3/report.txt` and `negative.csv` on disk are the **pre-split**
run: 22 rows (T1-0 **and** T1-a, 11 each), one committed-route kind only (`sumcheck-whir`,
`weight_bit`), no `sumcheck-whir-split` row and no `committed_binding` row at all — this is what
§1.1 above still cites. `git status` confirms `p3_negative.rs` is modified relative to the commit
that produced those files, and `route.rs`/`pcs.rs`/`matmul.rs` (which `committed_binding_control`
and the split route depend on) are modified too. So A8's 18/18 is traceable to what the current
harness *would* produce, not to a regenerated artifact in this repo — no file here backs the
`committed_binding` verdict or the split-route corruption rows as an executed, logged run. This
section does not run that control (out of scope here); the gap is reported so `data/negative-
plonky3/` gets regenerated before `committed_binding` is cited as a verified result elsewhere.
