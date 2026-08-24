# binius64 — tasks this system could not run, and why

`bench/README.md` commits to reporting a task a system cannot express as a **result**, not as
a gap. This file is that report for binius64.

**Summary: all three tasks are expressible. One rung of T1 could not be *run*.** The
distinction matters and is kept throughout: T1-d is not a task binius64 cannot describe, it
is a task whose circuit this build refuses to set up.

| Task | Expressible? | Ran? | Note |
|---|---|---|---|
| T1-0, T1-a, T1-b, T1-c | yes | yes | |
| **T1-d** | yes, as a circuit | **no** | rejected at setup — see §1 |
| T2 | yes | yes | one requantisation question the spec left open; see `EXPRESSION.md` §4 |
| T3 | yes, **in a single proof** | yes | see §2 |

---

## 1. T1-d — `[64×768] · [768×768]`, 37,748,736 MACs

**Not run. Rejected by a declared policy limit in binius64's own constraint system, not by
the machine and not by the protocol.**

Binius64 caps the number of values in one segment:

```
ConstraintSystem::MAX_VALUES_PER_SEGMENT = 1 << 26      // 67,108,864
crates/core/src/constraint_system/system.rs:122
```

Its own source comment classifies it:

> *"This is a policy limit rather than a structural one: 2^26 values is 512 MiB per
> segment."* … *"Raising it is fine."*

It is an anti-DoS bound in front of an allocation, sized so an unauthenticated payload cannot
declare an arbitrarily large segment. It is not a soundness bound and not a limit of the
proof system.

**Where our circuits land against it**, measured on the four rungs that built:

| Task | MACs | private values | values/MAC | vs 2²⁶ |
|---|---|---|---|---|
| T1-0 | 65 536 | 270 080 | 4.121 | 0.4% |
| T1-a | 589 824 | 2 432 256 | 4.124 | 3.6% |
| T1-b | 2 359 296 | 7 959 552 | 3.374 | 11.9% |
| T1-c | 9 437 184 | 30 068 736 | 3.186 | **44.8%** |
| **T1-d** | **37 748 736** | **118 505 472** | 3.139 | **176.6% — over by 1.766×** |

### 1.1 What actually happened, measured

The circuit **builds**. It is the *setup* that refuses it, and the message is the system's
own:

```
task=T1-d macs=37748736 relus=0 IMUL=37748736 AND=37699584 ZERO=4718592 BMUL=0
         private_values=118505472 inout=49152 max_abs_acc=664120 build=126.862s

Error: setting up the prover for T1-d
Caused by:
    0: constraint system error: the Private segment declares 118505472 values,
       over the maximum of 67108864
```

| | |
|---|---|
| circuit build time | **126.862 s** — it built, with the exact MAC count `bench/TASKS.md` fixes |
| IMUL constraints emitted | **37 748 736**, equal to the published MAC count |
| private values declared | **118 505 472** = 1.766 × the 67 108 864 limit |
| peak RSS, build only | 21.60 GB |
| peak footprint, build only | **52.03 GB** |
| wall time to rejection | 151.37 s (`user` 71.58 s, `sys` 55.82 s) |

So the task **is expressible in binius64's frontend** — the constraint system exists, is
correct, and carries exactly the operations the spec asks for. What does not exist is a
prover configuration at this commit that will accept it. That distinction is the result.

Raw output: [`bench/data/cells/t1-d-r1-t1-n1.time.txt`](../../data/cells/t1-d-r1-t1-n1.time.txt).


### 1.2 This is the same ceiling our own prior experiment hit

Our E-005 campaign, on the same prover and the same machine but with a different matmul
shape (reduction depth 64 rather than 768), found the identical wall: it proved
**20,849,152 MACs** and `setup` rejected the next rung up. The ceiling is on **values**, not
on MACs, so the exact MAC number at which a system hits it depends on how many values per
MAC the circuit costs — 3.19 for T1-c here, 3.22 for E-005's shape.

**What this means for the benchmark, stated carefully:** binius64's practical ceiling on this
machine and at this commit is between **9,437,184 MACs (proved)** and **37,748,736 MACs
(rejected)**. We do not interpolate a threshold inside that interval, because we did not
measure one. A reader who wants the exact crossing point can compute it from the
values/MAC column, but that would be our arithmetic, not our measurement.

### 1.3 What would make T1-d run, and why we did not do it

Raising `MAX_VALUES_PER_SEGMENT` is a one-line change its authors explicitly bless. We did
**not** patch it, for two reasons:

1. The fairness protocol measures each system **in the configuration its own authors
   document**. Editing a constant to make our number look better is precisely the failure
   mode that protocol exists to prevent.
2. Memory says it would not have finished anyway on this machine, and that is a separate,
   harder wall than the policy limit. T1-c already reached a **93.0 GB peak footprint** on a
   32 GiB machine, sustained by ~76 GB of swap; T1-d is four times the circuit.

**Right of reply applies here more than anywhere else in this file.** If the binius64
authors consider a raised segment limit the correct configuration for a circuit of this size,
we will re-run T1-d with it and publish both outcomes, per `CHALLENGE.md`.

## 2. T3 — the batch of 8 is one proof, not eight

`bench/TASKS.md` asks whether a system can prove 8 independent inputs in **one** proof, and
says that producing 8 separate proofs is a result about the system rather than a failure.

**binius64 produces one proof.** The batch is expressed as a single circuit holding one
committed copy of the weights and eight input vectors, with eight public outputs. The harness
emits a single serialized transcript for it, and the negative control corrupts `inout[0]` and
`inout[7]` independently within that one proof.

So T3 needs no entry in this file. What batching *bought* is a question for the results, not
for this one — and the answer there is measured, not assumed.

## 3. What binius64 was NOT asked to do, and so is not reported as unable to do

For completeness, so that absence of a number is not read as a failure:

- **ZK.** Every measurement in this repository uses binius64's **non-ZK** prover
  (`setup` / `create_proof`, not `setup_zk` / `create_proof_zk`). The system *has* a ZK
  prover; it was out of scope for round one, which compares integrity-proving cost. The
  conditions line records `zk: false` for every cell.
- **GPU.** Round one is CPU-only by design (`bench/README.md`).
- **Recursion / proof aggregation.** Not part of any task.
