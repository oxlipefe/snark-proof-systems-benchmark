#!/usr/bin/env python3
"""Diagnostic probes that vary ONE variable at a time to isolate why T2/T3 do not prove.

These are NOT benchmark tasks and produce no figure in RESULTS.md; bench/TASKS.md is frozen.
They exist so that the report to the authors is "it fails because of X" rather than
"it crashed", which is what CHALLENGE.md's right of reply needs to be actionable.
"""
import json, pathlib, sys
import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper

SCALE = 128.0
OUT = pathlib.Path(sys.argv[1]); OUT.mkdir(parents=True, exist_ok=True)

def emit(name, widths, batch, seed):
    rng = np.random.default_rng(seed)
    nodes, inits = [], []
    cur = "input"
    last = len(widths) - 2
    for i, (fi, fo) in enumerate(zip(widths, widths[1:])):
        w = rng.integers(-128, 128, size=(fi, fo), dtype=np.int64).astype(np.float32) / SCALE
        inits.append(numpy_helper.from_array(w, f"W{i+1}"))
        out = "output" if i == last else f"mm{i+1}"
        nodes.append(helper.make_node("Gemm", [cur, f"W{i+1}"], [out], name=f"Gemm_{i}",
                                      alpha=1.0, beta=0.0, transA=0, transB=0))
        if i != last:
            nodes.append(helper.make_node("Relu", [out], [f"relu{i+1}"], name=f"Relu_{i}"))
            cur = f"relu{i+1}"
    g = helper.make_graph(nodes, name,
        [helper.make_tensor_value_info("input", TensorProto.FLOAT, [batch, widths[0]])],
        [helper.make_tensor_value_info("output", TensorProto.FLOAT, [batch, widths[-1]])], inits)
    m = helper.make_model(g, opset_imports=[helper.make_opsetid("", 11)])
    m.ir_version = 7
    onnx.checker.check_model(m)
    onnx.save(m, OUT / f"{name}.onnx")
    a = rng.integers(-128, 128, size=(batch, widths[0]), dtype=np.int64) * int(SCALE)
    json.dump({"input_shape": [batch, widths[0]], "input_data": [a.reshape(-1).tolist()]},
              open(OUT / f"{name}.inputs.json", "w"))
    print(f"{name}: widths {widths} batch {batch}")

# Vary ONLY the final output width. Everything else is held fixed.
for w in (1, 2, 4, 8):
    emit(f"probe-d{w}", [64, w], 1, 0xE0060900 + w)
# The same, with the full T2 stack in front of it, to rule out the depth.
for w in (1, 2, 4):
    emit(f"probe-w{w}", [200, 256, 128, 64, w], 1, 0xE0060A00 + w)
# Batch, with a width that works, to separate "batch" from "narrow output".
for b in (1, 8):
    emit(f"probe-b{b}", [200, 256, 128, 64, 4], b, 0xE0060B00 + b)
