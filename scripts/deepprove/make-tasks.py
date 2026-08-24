#!/usr/bin/env python3
"""zk-prover-bench · DeepProve · generate the ONNX + io.json pair for every task.

bench/TASKS.md fixes each task by an exact MAC count, and that count is the denominator of
both `MAC/s` and `bytes/MAC`. It is never recomputed here: this script *asserts* that the
graph it emitted performs exactly the published number of multiply-accumulates, and refuses
to write a file that does not.

Nothing in this script is DeepProve code. It emits a plain ONNX graph, in the same shape
DeepProve's own MLP benchmark script emits (`zkml/assets/scripts/MLP/mlp.py`: `nn.Linear`
stack, `opset_version=12`, dynamic batch axis, flat per-sample input vectors in a JSON file
with `input_data` / `output_data` / `pytorch_output`). The two deviations from that script
are forced by bench/TASKS.md and are declared in EXPRESSION.md: no biases, and no activation
on the output layer.

Encoding. bench/TASKS.md specifies INT8 operands in [-128, 127]. DeepProve's frontend takes
a *float* model and quantizes it itself; `zkml/src/inputs.rs:validate` requires every input
value to lie in QUANTIZATION_RANGE = [-1.0, 1.0] (`zkml/src/quantization/mod.rs:40-42`). So
every INT8 value v is carried as the float v/128, which lands in [-1, 0.9921875]. With
ZKML_BIT_LEN=8 DeepProve's symmetric quantizer maps that range back onto the 8-bit integer
domain, so the arithmetic it proves is the INT8 arithmetic the task specifies.

Seeds are the ones already published for these tasks in
bench/systems/binius64/EXPRESSION.md §7. The RNG is not the same one (numpy PCG64 here,
Rust StdRng there), so the two systems prove the same *shape* and the same MAC count but not
the same *instance*. That is declared rather than hidden; the benchmark compares tasks, not
witnesses.
"""

import argparse
import json
import pathlib
import sys

import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

# bench/TASKS.md, frozen. Never recomputed.
PUBLISHED_MACS = {
    "t1-0": 65_536,
    "t1-a": 589_824,
    "t1-b": 2_359_296,
    "t1-c": 9_437_184,
    "t1-d": 37_748_736,
    "t2": 92_224,
    "t3": 737_792,
}

# bench/systems/binius64/EXPRESSION.md §7.
SEEDS = {
    "t1-0": 0xE0060100,
    "t1-a": 0xE00601A0,
    "t1-b": 0xE00601B0,
    "t1-c": 0xE00601C0,
    "t1-d": 0xE00601D0,
    "t2": 0xE0060200,
    "t3": 0xE0060300,
}

# T1 rungs: (M, K, N)
T1_SHAPES = {
    "t1-0": (1, 256, 256),
    "t1-a": (1, 768, 768),
    "t1-b": (4, 768, 768),
    "t1-c": (16, 768, 768),
    "t1-d": (64, 768, 768),
}

# T2/T3: 200 -> 256 -> 128 -> 64 -> 1, ReLU after layers 1-3, linear output.
MLP_WIDTHS = [200, 256, 128, 64, 1]
T3_BATCH = 8

INT8_SCALE = 128.0
# DeepProve's MLP exporter uses opset 12 (zkml/assets/scripts/MLP/mlp.py:176). IR version 8
# is the one paired with opset 12 in the ONNX release matrix; tract-onnx 0.21 is the parser
# on the other side.
OPSET = 12
IR_VERSION = 8


def int8_matrix(rng, rows, cols):
    """INT8 values over the full [-128, 127], carried as float v/128."""
    return (rng.integers(-128, 128, size=(rows, cols), dtype=np.int64)
            .astype(np.float32) / INT8_SCALE)


def build_matmul_graph(name, m, k, n, rng):
    """T1: a single MatMul. A is the witness (model input), B the committed weights."""
    weights = int8_matrix(rng, k, n)
    # DeepProve dispatches on the node NAME, not the op type: it lowercases the name and
    # looks for a parser key as a substring (zkml/src/parser/onnx.rs:257-264, keys at
    # :230-238). A node whose name does not contain "matmul" / "gemm.ab" / "relu" is
    # rejected as "Unknown node type" no matter what its op is. Node names here are chosen
    # to hit exactly one key each.
    node = helper.make_node("MatMul", ["input", "W"], ["output"], name="MatMul_0")
    graph = helper.make_graph(
        [node],
        name,
        [helper.make_tensor_value_info("input", TensorProto.FLOAT, [m, k])],
        [helper.make_tensor_value_info("output", TensorProto.FLOAT, [m, n])],
        [numpy_helper.from_array(weights, "W")],
    )
    macs = m * k * n
    return graph, [weights], macs


def build_mlp_graph(name, batch, rng):
    """T2/T3: the MLP. No biases and no activation on the output layer (bench/TASKS.md)."""
    nodes, inits, weights, macs = [], [], [], 0
    cur = "input"
    last = len(MLP_WIDTHS) - 2
    for layer, (fan_in, fan_out) in enumerate(zip(MLP_WIDTHS, MLP_WIDTHS[1:])):
        w = int8_matrix(rng, fan_in, fan_out)
        weights.append(w)
        w_name = f"W{layer + 1}"
        inits.append(numpy_helper.from_array(w, w_name))
        is_last = layer == last
        # The last MatMul writes the graph output directly. There is no Identity node:
        # DeepProve's ONNX parser has no "Identity" key and rejects unknown nodes outright
        # (zkml/src/parser/onnx.rs:262-264).
        out = "output" if is_last else f"h{layer + 1}"
        nodes.append(helper.make_node("MatMul", [cur, w_name], [out], name=f"MatMul_{layer + 1}"))
        macs += batch * fan_in * fan_out
        if is_last:
            cur = out
        else:
            act = f"a{layer + 1}"
            nodes.append(helper.make_node("Relu", [out], [act], name=f"Relu_{layer + 1}"))
            cur = act

    graph = helper.make_graph(
        nodes,
        name,
        [helper.make_tensor_value_info("input", TensorProto.FLOAT, [batch, MLP_WIDTHS[0]])],
        [helper.make_tensor_value_info("output", TensorProto.FLOAT, [batch, MLP_WIDTHS[-1]])],
        inits,
    )
    return graph, weights, macs


def reference_forward(task, weights, x):
    """The float forward pass, out of circuit, for `pytorch_output`.

    Also the place the accumulator bound is checked: bench/TASKS.md Amendment A1 requires an
    implementation to assert the no-requantization bound rather than trust the seed.
    """
    act = x
    max_abs_int = 0.0
    for i, w in enumerate(weights):
        act = act @ w
        # In INT8 units: the float carries a factor 1/128 per operand, so an accumulator
        # after `i+1` layers is `act * 128**(i+2)` in integer units.
        max_abs_int = max(max_abs_int, float(np.abs(act).max()) * (INT8_SCALE ** (i + 2)))
        if task in ("t2", "t3") and i < len(weights) - 1:
            act = np.maximum(act, 0.0)
    return act, max_abs_int


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--samples", type=int, default=6,
                    help="io.json samples: 1 warmup + N timed repetitions")
    ap.add_argument("--tasks", default=",".join(PUBLISHED_MACS))
    args = ap.parse_args()

    out_dir = pathlib.Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest = {}

    for task in args.tasks.split(","):
        task = task.strip()
        if task not in PUBLISHED_MACS:
            sys.exit(f"unknown task {task}")
        rng = np.random.default_rng(SEEDS[task])

        if task.startswith("t1"):
            m, k, n = T1_SHAPES[task]
            graph, weights, macs = build_matmul_graph(task, m, k, n, rng)
            in_shape, out_shape = (m, k), (m, n)
        else:
            batch = T3_BATCH if task == "t3" else 1
            graph, weights, macs = build_mlp_graph(task, batch, rng)
            in_shape = (batch, MLP_WIDTHS[0])
            out_shape = (batch, MLP_WIDTHS[-1])

        # The MAC-count gate. bench/TASKS.md is frozen; a graph that drifted from it is a
        # bug in this script, not a new task.
        published = PUBLISHED_MACS[task]
        if macs != published:
            sys.exit(f"{task}: graph performs {macs} MACs but bench/TASKS.md fixes {published}")

        model = helper.make_model(
            graph,
            opset_imports=[helper.make_opsetid("", OPSET)],
            producer_name="zk-prover-bench",
        )
        model.ir_version = IR_VERSION
        onnx.checker.check_model(model)
        onnx_path = out_dir / f"{task}.onnx"
        onnx.save(model, onnx_path)

        input_data, output_data, pytorch_output = [], [], []
        max_abs_int = 0.0
        for _ in range(args.samples):
            x = int8_matrix(rng, *in_shape)
            y, sample_max = reference_forward(task, weights, x)
            max_abs_int = max(max_abs_int, sample_max)
            assert y.shape == out_shape, (task, y.shape, out_shape)
            flat = y.reshape(-1).astype(float).tolist()
            input_data.append(x.reshape(-1).astype(float).tolist())
            # `output_data` is the ground truth DeepProve's bench binary compares by argmax
            # to score accuracy. These tasks are not classifiers, so it is set to the float
            # reference output; the accuracy column it feeds is not a benchmark metric here
            # and is reported as such.
            output_data.append(flat)
            pytorch_output.append(flat)

        # Amendment A1's bound, asserted rather than assumed. Accumulators are never
        # requantized, so the worst case grows down the network; the published instance must
        # stay inside i64 with at least a factor-2 margin.
        i64_max = 9_223_372_036_854_775_807
        if max_abs_int * 2 >= i64_max:
            sys.exit(f"{task}: max |accumulator| {max_abs_int:.3e} lacks a factor-2 margin "
                     f"under i64::MAX")

        io_path = out_dir / f"{task}.io.json"
        io_path.write_text(json.dumps({
            "input_data": input_data,
            "output_data": output_data,
            "pytorch_output": pytorch_output,
        }))

        manifest[task] = {
            "onnx": onnx_path.name,
            "io": io_path.name,
            "published_macs": published,
            "emitted_macs": macs,
            "seed": hex(SEEDS[task]),
            "input_shape": list(in_shape),
            "output_shape": list(out_shape),
            "samples": args.samples,
            "max_abs_accumulator_int8_units": max_abs_int,
            "onnx_bytes": onnx_path.stat().st_size,
            "io_bytes": io_path.stat().st_size,
        }
        print(f"[tasks] {task}: {macs} MACs (= published), in={in_shape} out={out_shape}, "
              f"max|acc|={max_abs_int:.3e}, onnx={onnx_path.stat().st_size}B")

    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
