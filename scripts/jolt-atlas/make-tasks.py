#!/usr/bin/env python3
"""zk-prover-bench · jolt-atlas · generate the ONNX + inputs.json pair for every task.

bench/TASKS.md fixes each task by an exact MAC count, and that count is the denominator of
both `MAC/s` and `bytes/MAC`. It is never recomputed here: this script *asserts* that the
graph it emitted performs exactly the published number of multiply-accumulates and refuses to
write a file that does not.

Nothing in this script is jolt-atlas code. It emits plain ONNX.

ENCODING — why the operands look the way they do.
jolt-atlas is not an INT8 system. `atlas-onnx-tracer` quantizes a float ONNX graph into
i32 fixed point at a global log2 scale (`common::consts::MODEL_SCALE = 14`), and every
`Einsum` node fuses "accumulate in i64, floor-rebase by `1 << scale`, saturating-clamp to
i32" unconditionally (`atlas-onnx-tracer/src/ops/einsum.rs:13-20`).

So the task's INT8 domain is carried like this, and the choice is exact rather than
approximate:

  INT8 value v  ->  ONNX float  v / 128  ->  tracer integer  round(v/128 * 2^14) = v * 128

For a single matmul the einsum then computes

  acc      = sum_k (a_k * 128) * (w_k * 128) = 2^14 * sum_k a_k w_k
  rebased  = floor(acc / 2^14)               =        sum_k a_k w_k        (EXACT)

because 128 * 128 = 2^14 exactly. So on T1 the integer jolt-atlas proves as the output is
bit-for-bit the INT32 accumulator bench/TASKS.md specifies, and no precision is lost.

On T2/T3 that exactness does NOT survive past the first layer: layer 1's output is already
`sum a w` at the tracer's fixed scale, and layer 2's rebase divides by 2^14 again. That is
requantization between layers, which bench/TASKS.md Amendment A1 forbids, and it cannot be
switched off. See systems/jolt-atlas/EXPRESSION.md.
"""

import argparse, json, pathlib, sys
import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

# bench/TASKS.md, frozen. Never recomputed.
PUBLISHED_MACS = {
    "t1-0": 65_536, "t1-a": 589_824, "t1-b": 2_359_296,
    "t1-c": 9_437_184, "t1-d": 37_748_736, "t2": 92_224, "t3": 737_792,
}
# bench/systems/binius64/EXPRESSION.md §7 — the same published seeds, reused.
SEEDS = {
    "t1-0": 0xE0060100, "t1-a": 0xE00601A0, "t1-b": 0xE00601B0,
    "t1-c": 0xE00601C0, "t1-d": 0xE00601D0, "t2": 0xE0060200, "t3": 0xE0060300,
}
T1_SHAPES = {
    "t1-0": (1, 256, 256), "t1-a": (1, 768, 768), "t1-b": (4, 768, 768),
    "t1-c": (16, 768, 768), "t1-d": (64, 768, 768),
}
MLP_WIDTHS = [200, 256, 128, 64, 1]
T3_BATCH = 8
INT8_SCALE = 128.0     # 2^7; paired with MODEL_SCALE=14 this makes T1's rebase exact
OPSET = 11
IR_VERSION = 6


def int8_matrix(rng, rows, cols):
    return rng.integers(-128, 128, size=(rows, cols), dtype=np.int64)


def build_matmul(name, m, k, n, rng):
    w_int = int8_matrix(rng, k, n)
    a_int = int8_matrix(rng, m, k)
    node = helper.make_node("MatMul", ["input", "W"], ["output"], name="MatMul_0")
    graph = helper.make_graph(
        [node], name,
        [helper.make_tensor_value_info("input", TensorProto.FLOAT, [m, k])],
        [helper.make_tensor_value_info("output", TensorProto.FLOAT, [m, n])],
        [numpy_helper.from_array((w_int.astype(np.float32) / INT8_SCALE), "W")],
    )
    acc = a_int @ w_int                       # the exact INT32 accumulator
    return graph, a_int, m * k * n, int(np.abs(acc).max())


def build_mlp(name, batch, rng):
    nodes, inits, macs = [], [], 0
    cur = "input"
    a_int = int8_matrix(rng, batch, MLP_WIDTHS[0])
    act = a_int.astype(np.int64)
    last = len(MLP_WIDTHS) - 2
    max_abs = 0
    for layer, (fi, fo) in enumerate(zip(MLP_WIDTHS, MLP_WIDTHS[1:])):
        w_int = int8_matrix(rng, fi, fo)
        wn = f"W{layer + 1}"
        # transB=1 with the weight stored [out, in]. This is not a stylistic choice: it is the
        # form torch.onnx.export emits and the form jolt-atlas's OWN bundled MLPs use
        # (models/perceptron, models/mlp_square_4layer). With transB=0 tract collapses the rank
        # of the activation and jolt-atlas rejects the resulting einsum equation, which would
        # have been reported as a limit of their system when it was a limit of our emission.
        inits.append(numpy_helper.from_array((w_int.T.astype(np.float32) / INT8_SCALE), wn))
        is_last = layer == last
        out = "output" if is_last else f"mm{layer + 1}"
        # Gemm, not MatMul. jolt-atlas parses through tract, and with a `MatMul` node tract's
        # shape inference collapses the rank of a batch-1 activation, producing an einsum
        # equation (`k,kn->mn`) that jolt-atlas's registry does not carry. Its OWN bundled
        # MLPs — `models/perceptron` and `models/mlp_square_4layer`, both four dense layers at
        # batch 1 — are emitted as `Gemm`, and both prove through this harness unchanged. So
        # `Gemm` is the expression its authors use, and using `MatMul` here would have
        # reported our own emission choice as a limit of their system. See
        # systems/jolt-atlas/NOT_EXPRESSIBLE.md.
        nodes.append(helper.make_node("Gemm", [cur, wn], [out], name=f"Gemm_{layer}",
                                      alpha=1.0, beta=0.0, transA=0, transB=1))
        macs += batch * fi * fo
        act = act @ w_int
        max_abs = max(max_abs, int(np.abs(act).max()))
        if not is_last:
            rn = f"relu{layer + 1}"
            nodes.append(helper.make_node("Relu", [out], [rn], name=f"Relu_{layer}"))
            act = np.maximum(act, 0)
            cur = rn
    graph = helper.make_graph(
        nodes, name,
        [helper.make_tensor_value_info("input", TensorProto.FLOAT, [batch, MLP_WIDTHS[0]])],
        [helper.make_tensor_value_info("output", TensorProto.FLOAT, [batch, MLP_WIDTHS[-1]])],
        inits,
    )
    return graph, a_int, macs, max_abs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--tasks", nargs="*", default=list(PUBLISHED_MACS))
    args = ap.parse_args()
    out = pathlib.Path(args.out); out.mkdir(parents=True, exist_ok=True)
    manifest = {}

    for task in args.tasks:
        rng = np.random.default_rng(SEEDS[task])
        if task.startswith("t1"):
            m, k, n = T1_SHAPES[task]
            graph, a_int, macs, max_abs = build_matmul(task, m, k, n, rng)
        elif task == "t2":
            graph, a_int, macs, max_abs = build_mlp(task, 1, rng)
        elif task == "t3":
            graph, a_int, macs, max_abs = build_mlp(task, T3_BATCH, rng)
        else:
            sys.exit(f"unknown task {task}")

        published = PUBLISHED_MACS[task]
        if macs != published:
            sys.exit(f"{task}: graph performs {macs} MACs but bench/TASKS.md fixes {published}")

        # bench/TASKS.md Amendment A1: assert the no-requantization accumulator bound with a
        # factor-2 margin rather than trusting the seed to be benign.
        if max_abs * 2 >= 2**63 - 1:
            sys.exit(f"{task}: max |accumulator| {max_abs} lacks a factor-2 margin under i64")

        model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", OPSET)])
        model.ir_version = IR_VERSION
        onnx.checker.check_model(model)
        onnx.save(model, out / f"{task}.onnx")

        # The harness feeds these integers to Tensor::new directly: they are the ALREADY
        # quantized witness, so nothing between this file and the prover reinterprets them.
        quantized = (a_int * int(INT8_SCALE)).astype(np.int64)
        json.dump(
            {"input_shape": list(a_int.shape), "input_data": [quantized.reshape(-1).tolist()]},
            open(out / f"{task}.inputs.json", "w"),
        )
        manifest[task] = {
            "macs_published": published, "macs_asserted": macs,
            "seed": hex(SEEDS[task]), "input_shape": list(a_int.shape),
            "max_abs_accumulator_int8_units": max_abs,
            "int8_carrier_scale": INT8_SCALE, "opset": OPSET, "ir_version": IR_VERSION,
        }
        print(f"{task}: {macs} MACs, input {a_int.shape}, max|acc| {max_abs}")

    json.dump(manifest, open(out / "manifest.json", "w"), indent=2)


if __name__ == "__main__":
    main()
