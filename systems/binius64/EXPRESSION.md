# binius64 — how each task was expressed

`bench/TASKS.md` fixes each task by an **exact operation count**, not by a shape. That count
is the denominator of both `MAC/s` and `bytes/MAC`, so it is never recomputed here. Instead,
every builder **asserts** that the circuit it produced emits exactly the published number of
IMUL constraints, and refuses to return a circuit that does not:

```rust
anyhow::ensure!(
    n_imul == task.published_macs(),
    "{}: emitted {n_imul} IMUL constraints but bench/TASKS.md fixes {} MACs; the \
     expression drifted from the published task",
    task.name(), task.published_macs()
);
```

So the MAC count in every table is a **measured constraint count that was checked against the
published spec**, not a number carried over from the spec on trust.

Source: `scripts/binius64/harness/src/e006/`.

---

## 1. The INT8 encoding, declared

`bench/TASKS.md` specifies INT8 operands in `[-128, 127]` — **signed**. Binius64's native
value is a 64-bit two's-complement word, so a signed 8-bit operand is carried
**sign-extended into a 64-bit word**. No range constraint is attached to it; see §5 for what
that does and does not mean.

The multiplication is the interesting part. Binius64's frontend offers `imul` (unsigned
64×64 → 128) at **1 IMUL constraint**, and `smul` (signed) as a gadget costing **1 IMUL + 4
AND**. This expression uses `imul` and takes only its **low word**:

```rust
let product = |kk: usize| builder.imul(a_row[kk], b_wires[kk][col]).1;   // .1 is the low word
```

That is correct signed arithmetic, not an approximation. For two's-complement operands the
low 64 bits of the unsigned product are exactly the low 64 bits of the signed product; the
corrections `smul` applies land entirely in the **high word**, which a dot product discards.
Since `|a·b| ≤ 128·128 = 16 384`, the signed product fits in the low word with 47 bits to
spare.

**Consequence: a signed INT8 multiply-accumulate costs 1 IMUL constraint** — the same as the
unsigned INT8 multiply our earlier experiment E-001 measured, so the T1 numbers here are
directly comparable to that historical series.

Accumulation is `iadd` (1 AND + 1 linear constraint), whose two's-complement sum is exact as
long as the running total stays inside `i64`. Every builder checks this against an
out-of-circuit reference in `i128` before emitting anything.

## 2. T1 — the INT8 matmul ladder

`C = A[M×K] · B[K×N]`, signed INT8 in, INT32 out, **not requantised**. One `inout` wire per
output element, so no dot product can be eliminated as dead code and the verifier sees the
full output matrix.

```rust
for (i, out_row) in out_wires.iter().enumerate() {
    for (j, &out) in out_row.iter().enumerate() {
        let dot = dot_product(&builder, &a_wires[i], &b_wires, j, k);
        builder.assert_eq(format!("t1_out[{i}][{j}]"), dot, out);
    }
}

fn dot_product(builder: &CircuitBuilder, a_row: &[Wire], b_wires: &[Vec<Wire>],
               col: usize, k: usize) -> Wire {
    let product = |kk: usize| builder.imul(a_row[kk], b_wires[kk][col]).1;
    (1..k).fold(product(0), |acc, kk| builder.iadd(acc, product(kk)).0)
}
```

The first product seeds the accumulator, so a depth-`K` dot product costs `K` IMUL and
`K − 1` accumulating additions. Total IMUL = `M·K·N`, which is the MAC count.

Both `A` and `B` are private witness wires. The weights are committed, not public: that is
the shape a real inference proof has, and it matches what E-001 and E-005 measured.

**INT32 output is enforced, not assumed.** The reference product is computed in `i128` and
each output element is checked against the INT32 range before the circuit is built:
`|acc| ≤ K · 128 · 128 = 12 582 912` at `K = 768`, comfortably inside INT32.

## 3. T2 and T3 — the complete MLP

200-256-128-64-1, ReLU after layers 1–3, linear output. T3 is the same network over 8
independent inputs, with the weights **committed once and shared across the batch**, in a
single circuit and therefore **a single proof** — see §6.

```rust
for (b, input_row) in input_wires.iter().enumerate() {
    let mut act: Vec<Wire> = input_row.clone();
    for (layer, matrix) in weight_wires.iter().enumerate() {
        let is_last = layer == weight_wires.len() - 1;
        act = matrix.iter().map(|row| {
            let acc = dot_product(&builder, &act, row);
            if is_last { acc } else { relu(&builder, acc) }
        }).collect();
    }
    builder.assert_eq(format!("t2_out[{b}]"), act[0], out_wires[b]);
}
```

ReLU on a two's-complement accumulator is a shift and a mask — no comparison, no branch:

```rust
pub fn relu(builder: &CircuitBuilder, x: Wire) -> Wire {
    let sign_mask = builder.sar(x, 63);          // all-ones iff x < 0
    builder.band(x, builder.bnot(sign_mask))     // x & !(x >> 63)
}
```

Only the model's scalar prediction is public — one `inout` word per batch element. The
activations, the weights and the input are all private. Nothing about the forward pass is
revealed beyond the answer, which is again the shape a real inference proof has.

**No biases**: `bench/TASKS.md` specifies none.

## 4. Requantisation between layers — a decision `bench/TASKS.md` did not make

`bench/TASKS.md` says for T1 that the output is "INT32, not requantised — requantisation is a
separate concern", and says **nothing** about requantisation between the layers of T2.

This expression **does not requantise**. Each layer's accumulator is fed straight into the
next, which is T1's stated rule extended down the network. The consequence is stated rather
than hidden: the accumulator grows by roughly `log₂(fan_in) + 7` bits per layer, so the
**worst case over all INT8 inputs** reaches ≈1.44·10¹⁹ at layer 4, against
`i64::MAX = 9.22·10¹⁸`. The worst case does not fit.

The published instance is nowhere near it, and this is **checked, not assumed**. The builder
computes the whole forward pass in `i128`, records the largest magnitude any accumulator
reaches, and refuses to emit a circuit unless it fits in `i64` with at least a factor of two
in hand:

| Task | max &#124;accumulator&#124; observed | headroom under `i64::MAX` |
|---|---|---|
| T2 | 8 955 951 054 519 (8.96·10¹²) | 1.03·10⁶× |
| T3 | 19 638 755 553 042 (1.96·10¹³) | 4.69·10⁵× |

The alternative — requantising with a fixed right shift between layers — is a modelling
decision the frozen spec did not make, and making it here would change the task rather than
express it. **This is flagged as a gap in `bench/TASKS.md`, not resolved unilaterally.**

## 5. What the circuit does *not* constrain, stated plainly

The INT8 operands are witnessed as full 64-bit words with **no range constraint**. The proof
therefore establishes: *"the prover knows 64-bit words which, multiplied and accumulated as
specified, yield the published output"* — it does **not** establish that those words were in
`[-128, 127]`.

For a benchmark of prover cost this is the right choice, and it is the choice `bench/TASKS.md`
anticipates when it asks systems to "declare their encoding". Adding an 8-bit range check per
operand would add constraints that are a property of the encoding rather than of the
multiply, and would make the cost incomparable with systems whose native field already bounds
the operand. **A production deployment would need those range constraints, and they are not
in these numbers.**

## 6. Measured circuit shape

Read off the built circuits, not derived. `private values` is the quantity Binius64's
`MAX_VALUES_PER_SEGMENT = 2²⁶` bounds; see `NOT_EXPRESSIBLE.md`.

| Task | MACs (= IMUL) | ReLU | AND | ZERO | BMUL | private values | inout | values/MAC | max &#124;acc&#124; |
|---|---|---|---|---|---|---|---|---|---|
| T1-0 | 65 536 | 0 | 65 280 | 8 192 | 0 | 270 080 | 256 | 4.121 | 270 167 |
| T1-a | 589 824 | 0 | 589 056 | 73 728 | 0 | 2 432 256 | 768 | 4.124 | 421 915 |
| T1-b | 2 359 296 | 0 | 2 356 224 | 294 912 | 0 | 7 959 552 | 3 072 | 3.374 | 636 963 |
| T1-c | 9 437 184 | 0 | 9 424 896 | 1 179 648 | 0 | 30 068 736 | 12 288 | 3.186 | 611 801 |
| T1-d | — | — | — | — | — | — | — | — | see `NOT_EXPRESSIBLE.md` |
| T2 | 92 224 | 448 | 92 223 | 11 528 | 0 | 380 622 | 1 | 4.127 | 8.96·10¹² |
| T3 | 737 792 | 3 584 | 737 784 | 92 224 | 0 | 2 399 408 | 8 | 3.252 | 1.96·10¹³ |

Every row is read from the `task=` line the harness printed for that cell, preserved in
`bench/data/cells/<label>.time.txt`. `values/MAC` falls from 4.12 to ~3.2 as the circuit grows
because a fixed per-proof overhead is being amortised; it is **not** flat, which matters for
`bytes/MAC` and is why the ladder spans three orders of magnitude.

Every MAC count equals the value `bench/TASKS.md` publishes, and equals the IMUL constraint
count the circuit actually emitted.

**T3 is one proof, not eight.** The batch is expressed as a single circuit with a single
committed weight set and 8 public outputs; the harness produces one serialized transcript for
it. The 8 outputs are visible in the `inout` column above and in the negative control, which
corrupts `inout[0]` and `inout[7]` independently.

## 7. Witness seeds

Fixed per task so a rerun proves the same instance, as `bench/TASKS.md` requires.

| Task | seed |
|---|---|
| T1-0 | `0xE0060100` |
| T1-a | `0xE00601A0` |
| T1-b | `0xE00601B0` |
| T1-c | `0xE00601C0` |
| T1-d | `0xE00601D0` |
| T2 | `0xE0060200` |
| T3 | `0xE0060300` |

RNG: `rand::rngs::StdRng::seed_from_u64`, operands drawn as `i8` over the full
`[-128, 127]`. A unit test asserts the instance actually contains both signs, so a silent
degradation back to the unsigned encoding would fail the build rather than produce a
plausible number.
