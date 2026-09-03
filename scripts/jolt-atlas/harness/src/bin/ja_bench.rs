//! zk-prover-bench · jolt-atlas · one measured cell.
//!
//! OUR code. It calls jolt-atlas's public API and nothing else; no jolt-atlas source is
//! copied, patched or instrumented. Its licence §2(i) permits internal use "solely for the
//! purpose of testing and evaluating it", which is what this is.
//!
//! WHAT IS TIMED, AND WHERE THE BRACKETS ARE. Stated here because RESULTS.md has to publish
//! them:
//!   setup    Model::load + AtlasSharedPreprocessing::preprocess + AtlasProverPreprocessing
//!            + AtlasVerifierPreprocessing.  One-off per model.  Reported apart, never
//!            amortized into prove.
//!   prove    ONNXProof::prove only.  It includes jolt-atlas's own quantized graph
//!            execution (Model::trace), because prove() calls it and there is no public
//!            entry point that separates them.  Declared, not hidden.
//!   verify   ONNXProof::verify only, warm, in the same process.
//!   size     the proof serialized with ark_serialize::CanonicalSerialize::serialize_compressed,
//!            which is what jolt-atlas's own gpt2_zk_bench example measures.  Proof only:
//!            it does NOT carry the verifier preprocessing.
//!
//! N repetitions run inside ONE process so that /usr/bin/time -l attributes one memory peak
//! to one cell, matching the convention binius64 and DeepProve were measured under.

use ark_serialize::CanonicalSerialize;
use atlas_onnx_tracer::{
    model::{Model, RunArgs},
    tensor::Tensor,
};
use jolt_atlas_core::onnx_proof::{
    AtlasProverPreprocessing, AtlasSharedPreprocessing, AtlasVerifierPreprocessing,
    Blake2bTranscript, Bn254, Fr, HyperKZG, ONNXProof,
};
use std::time::Instant;

fn env_usize(k: &str, d: usize) -> usize {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: ja_bench <label> <model.onnx> <inputs.json>");
        std::process::exit(2);
    }
    let label = args[1].clone();
    let onnx = args[2].clone();
    let inputs_path = args[3].clone();

    let reps = env_usize("JA_REPS", 5);
    let warmup = env_usize("JA_WARMUP", 1);
    let padding = std::env::var("JA_PADDING").map(|v| v != "0").unwrap_or(true);
    let scale: i32 = std::env::var("JA_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(14);

    // The witness comes from a JSON file our generator wrote: {"input_data": [[...]]}
    // Values are the ALREADY-QUANTIZED integers jolt-atlas will prove, so the harness does
    // no quantization of its own and EXPRESSION.md can state exactly what was proved.
    let raw: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&inputs_path).expect("open inputs"))
            .expect("parse inputs");
    let rows = raw["input_data"].as_array().expect("input_data array");
    let shape: Vec<usize> = raw["input_shape"]
        .as_array()
        .expect("input_shape")
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    let data: Vec<i32> = rows[0]
        .as_array()
        .expect("row")
        .iter()
        .map(|v| v.as_i64().unwrap() as i32)
        .collect();
    let input = Tensor::new(Some(&data), &shape).unwrap();

    let t_setup = Instant::now();
    // `RunArgs::default()` binds the symbolic `batch_size` axis to 1
    // (atlas-onnx-tracer/src/model/mod.rs:401). T3 is a batch of 8, so the binding is taken
    // from the task's own input shape instead of being left at the default; otherwise the
    // tracer rejects the witness with "Input tensor 0 has dims [8, 200], expected [1, 200]"
    // and we would have reported our own defaulting as a limit of their system.
    let run_args = RunArgs::default()
        .set_scale(scale)
        .with_padding(padding)
        .with("batch_size", shape[0]);
    let model = Model::load(&onnx, &run_args);
    let max_num_vars = model.max_num_vars();
    let pp = AtlasSharedPreprocessing::preprocess(model);
    let prover_pp = AtlasProverPreprocessing::<Fr, HyperKZG<Bn254>>::new(pp);
    let verifier_pp = AtlasVerifierPreprocessing::<Fr, HyperKZG<Bn254>>::from(&prover_pp);
    let setup_ms = t_setup.elapsed().as_secs_f64() * 1000.0;

    println!("META label={label} onnx={onnx} padding={padding} scale={scale} max_num_vars={max_num_vars} reps={reps} warmup={warmup}");
    println!("SETUP ms={setup_ms:.3}");

    let mut proof_bytes = 0usize;
    for i in 0..(warmup + reps) {
        let t = Instant::now();
        let (proof, io, _dbg) =
            ONNXProof::<Fr, Blake2bTranscript, HyperKZG<Bn254>>::prove(&prover_pp, &[input.clone()]);
        let prove_ms = t.elapsed().as_secs_f64() * 1000.0;

        let mut buf = Vec::new();
        proof.serialize_compressed(&mut buf).expect("serialize proof");
        proof_bytes = buf.len();

        let t = Instant::now();
        let ok = proof.verify(&verifier_pp, &io, None).is_ok();
        let verify_ms = t.elapsed().as_secs_f64() * 1000.0;

        let kind = if i < warmup { "WARMUP" } else { "REP" };
        println!(
            "{kind} i={i} prove_ms={prove_ms:.3} verify_ms={verify_ms:.3} proof_bytes={proof_bytes} verify_ok={ok}"
        );
        if !ok {
            eprintln!("HONEST PROOF FAILED TO VERIFY — cell is invalid");
            std::process::exit(3);
        }
    }
    println!("DONE proof_bytes={proof_bytes}");
}
