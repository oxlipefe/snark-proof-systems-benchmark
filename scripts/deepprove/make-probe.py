#!/usr/bin/env python3
"""zk-prover-bench · DeepProve · diagnostic probes. NOT benchmark tasks.

T2 (200-256-128-64-1) is rejected by DeepProve at proving time with
`ceil_log2: x must be positive` from dp-crypto's sumcheck. "It crashed" is a weaker result
than "it crashed *because of X*", and the authors deserve the second one for their right of
reply. These probes vary exactly one thing at a time to isolate it.

**Nothing produced here is a T2 number.** bench/TASKS.md is frozen; these graphs are not the
task and no figure from them enters any results table. They exist only to name the cause.

  probe-w2 / probe-w4   T2 with the final layer widened 64->2 and 64->4
  probe-d1 / probe-d2   a single dense layer, 64->1 and 64->2

If probe-d1 fails and probe-d2 passes, the cause is a width-1 output, isolated to one layer.
"""

import json
import pathlib
import sys

import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

INT8_SCALE = 128.0
OPSET, IR_VERSION = 12, 8
SEED = 0xE0060299

PROBES = {
    "probe-w2": [200, 256, 128, 64, 2],
    "probe-w4": [200, 256, 128, 64, 4],
    "probe-d1": [64, 1],
    "probe-d2": [64, 2],
}


def int8_matrix(rng, rows, cols):
    return (rng.integers(-128, 128, size=(rows, cols), dtype=np.int64)
            .astype(np.float32) / INT8_SCALE)


def build(widths, rng):
    nodes, inits, weights = [], [], []
    cur, last = "input", len(widths) - 2
    for layer, (fan_in, fan_out) in enumerate(zip(widths, widths[1:])):
        w = int8_matrix(rng, fan_in, fan_out)
        weights.append(w)
        name = f"W{layer + 1}"
        inits.append(numpy_helper.from_array(w, name))
        is_last = layer == last
        out = "output" if is_last else f"h{layer + 1}"
        nodes.append(helper.make_node("MatMul", [cur, name], [out], name=f"MatMul_{layer + 1}"))
        if is_last:
            cur = out
        else:
            act = f"a{layer + 1}"
            nodes.append(helper.make_node("Relu", [out], [act], name=f"Relu_{layer + 1}"))
            cur = act
    graph = helper.make_graph(
        nodes, "probe",
        [helper.make_tensor_value_info("input", TensorProto.FLOAT, [1, widths[0]])],
        [helper.make_tensor_value_info("output", TensorProto.FLOAT, [1, widths[-1]])],
        inits)
    return graph, weights


def main():
    out_dir = pathlib.Path(sys.argv[1])
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, widths in PROBES.items():
        rng = np.random.default_rng(SEED)
        graph, weights = build(widths, rng)
        model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", OPSET)],
                                  producer_name="zk-prover-bench-probe")
        model.ir_version = IR_VERSION
        onnx.checker.check_model(model)
        onnx.save(model, out_dir / f"{name}.onnx")

        rows = []
        for _ in range(2):
            x = int8_matrix(rng, 1, widths[0])
            rows.append(x.reshape(-1).astype(float).tolist())
        (out_dir / f"{name}.io.json").write_text(json.dumps({"input_data": rows}))
        print(f"[probe] {name}: widths={widths}")


if __name__ == "__main__":
    main()
