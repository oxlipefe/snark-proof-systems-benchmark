//! zk-prover-bench · jolt-atlas · correctness control (negative test).
//!
//! bench/README.md: "A corrupted trace must make verify() fail, in every system, on every
//! task." Without this control the benchmark is not measuring proofs, it is measuring
//! computations that happen to produce bytes.
//!
//! OUR code. It calls jolt-atlas's public API only; nothing of jolt-atlas is copied or
//! patched. Three corruption families, chosen to match the ones binius64 was tested with so
//! that the two controls are comparable:
//!
//!   output_word  one element of the PUBLIC claimed output (io.outputs) is flipped.
//!                The prover claims a different result for the same model and input.
//!   input_word   one element of the PUBLIC input (io.inputs) is flipped.
//!                The verifier is told the proof is about a different input than it is.
//!   proof_byte   one bit of the serialized proof is flipped, then the proof is
//!                deserialized and verified. The artifact itself is tampered with.
//!
//! POSITIVE CONTROLS RUN FIRST, both of them, because a negative test that passes because
//! nothing ever verifies proves nothing:
//!   honest        the untouched proof and IO verify.
//!   roundtrip     serialize -> deserialize -> verify, unmodified, still verifies, so the
//!                 method itself does not corrupt.

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use atlas_onnx_tracer::{
    model::{trace::ModelExecutionIO, Model, RunArgs},
    tensor::Tensor,
};

/// `ModelExecutionIO` is not `Clone` at this commit, and we do not patch jolt-atlas. All four
/// of its fields are public, so a copy is built from them here.
fn clone_io(io: &ModelExecutionIO) -> ModelExecutionIO {
    ModelExecutionIO {
        inputs: io.inputs.clone(),
        outputs: io.outputs.clone(),
        input_indices: io.input_indices.clone(),
        output_indices: io.output_indices.clone(),
    }
}
use jolt_atlas_core::onnx_proof::{
    AtlasProverPreprocessing, AtlasSharedPreprocessing, AtlasVerifierPreprocessing,
    Blake2bTranscript, Bn254, Fr, HyperKZG, ONNXProof,
};

type Proof = ONNXProof<Fr, Blake2bTranscript, HyperKZG<Bn254>>;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: ja_negative <label> <model.onnx> <inputs.json>");
        std::process::exit(2);
    }
    let (label, onnx, inputs_path) = (args[1].clone(), args[2].clone(), args[3].clone());
    let padding = std::env::var("JA_PADDING").map(|v| v != "0").unwrap_or(true);
    let scale: i32 = std::env::var("JA_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(14);

    let raw: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&inputs_path).unwrap()).unwrap();
    let shape: Vec<usize> = raw["input_shape"].as_array().unwrap().iter()
        .map(|v| v.as_u64().unwrap() as usize).collect();
    let data: Vec<i32> = raw["input_data"].as_array().unwrap()[0].as_array().unwrap().iter()
        .map(|v| v.as_i64().unwrap() as i32).collect();
    let input = Tensor::new(Some(&data), &shape).unwrap();

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
    let pp = AtlasSharedPreprocessing::preprocess(model);
    let prover_pp = AtlasProverPreprocessing::<Fr, HyperKZG<Bn254>>::new(pp);
    let verifier_pp = AtlasVerifierPreprocessing::<Fr, HyperKZG<Bn254>>::from(&prover_pp);

    let (proof, io, _dbg) = Proof::prove(&prover_pp, &[input]);

    println!("task,family,position,detail,verdict");

    // ── positive control 1: the honest proof verifies ────────────────────
    let honest = proof.verify(&verifier_pp, &io, None).is_ok();
    println!("{label},none,-,honest,{}", if honest { "VERIFY_ACCEPTED" } else { "VERIFY_REJECTED" });
    if !honest {
        eprintln!("honest proof does not verify — the control would be vacuous. Aborting.");
        std::process::exit(3);
    }

    // ── positive control 2: serialize -> deserialize -> verify ───────────
    let mut bytes = Vec::new();
    proof.serialize_compressed(&mut bytes).unwrap();
    let n = bytes.len();
    match Proof::deserialize_compressed(&bytes[..]) {
        Ok(rt) => {
            let ok = rt.verify(&verifier_pp, &io, None).is_ok();
            println!("{label},none,-,roundtrip,{}", if ok { "VERIFY_ACCEPTED" } else { "VERIFY_REJECTED" });
            if !ok {
                eprintln!("round-trip control failed — proof_byte results would be meaningless");
                std::process::exit(4);
            }
        }
        Err(e) => {
            println!("{label},none,-,roundtrip,DESERIALIZE_REJECTED({e:?})");
            eprintln!("round-trip control failed to deserialize");
            std::process::exit(4);
        }
    }

    // ── family: output_word ──────────────────────────────────────────────
    for &pos in &[0usize, 1, 2] {
        let mut bad = clone_io(&io);
        if bad.outputs.is_empty() || bad.outputs[0].len() <= pos { continue; }
        let mut d = bad.outputs[0].data().to_vec();
        d[pos] ^= 1;
        let dims = bad.outputs[0].dims().to_vec();
        bad.outputs[0] = Tensor::construct(d, dims);
        let v = proof.verify(&verifier_pp, &bad, None).is_ok();
        println!("{label},output_word,{pos},xor1,{}", if v { "VERIFY_ACCEPTED" } else { "VERIFY_REJECTED" });
    }
    // last output element too
    if let Some(t) = io.outputs.first() {
        let last = t.len() - 1;
        let mut bad = clone_io(&io);
        let mut d = bad.outputs[0].data().to_vec();
        d[last] ^= 1;
        let dims = bad.outputs[0].dims().to_vec();
        bad.outputs[0] = Tensor::construct(d, dims);
        let v = proof.verify(&verifier_pp, &bad, None).is_ok();
        println!("{label},output_word,{last},xor1_last,{}", if v { "VERIFY_ACCEPTED" } else { "VERIFY_REJECTED" });
    }

    // ── family: input_word ───────────────────────────────────────────────
    for &pos in &[0usize, 1] {
        let mut bad = clone_io(&io);
        if bad.inputs.is_empty() || bad.inputs[0].len() <= pos { continue; }
        let mut d = bad.inputs[0].data().to_vec();
        d[pos] ^= 1;
        let dims = bad.inputs[0].dims().to_vec();
        bad.inputs[0] = Tensor::construct(d, dims);
        let v = proof.verify(&verifier_pp, &bad, None).is_ok();
        println!("{label},input_word,{pos},xor1,{}", if v { "VERIFY_ACCEPTED" } else { "VERIFY_REJECTED" });
    }
    if let Some(t) = io.inputs.first() {
        let last = t.len() - 1;
        let mut bad = clone_io(&io);
        let mut d = bad.inputs[0].data().to_vec();
        d[last] ^= 1;
        let dims = bad.inputs[0].dims().to_vec();
        bad.inputs[0] = Tensor::construct(d, dims);
        let v = proof.verify(&verifier_pp, &bad, None).is_ok();
        println!("{label},input_word,{last},xor1_last,{}", if v { "VERIFY_ACCEPTED" } else { "VERIFY_REJECTED" });
    }

    // ── family: proof_byte ───────────────────────────────────────────────
    // A systematic walk rather than a handful of offsets: DeepProve's coarse pass missed a
    // whole accepted region and only a fine sweep found it.
    let mut offsets: Vec<usize> = Vec::new();
    if let Ok(path) = std::env::var("JA_OFFSETS_FILE") {
        // A whole-artifact sweep is tens of thousands of offsets, which does not fit in an
        // environment variable. The list comes from a file so that the sweep can be
        // exhaustive rather than sampled: bench/README.md forbids inferring an absence.
        let txt = std::fs::read_to_string(path).expect("read JA_OFFSETS_FILE");
        for tok in txt.split_whitespace() {
            if let Ok(o) = tok.parse::<usize>() { if o < n { offsets.push(o); } }
        }
    } else if let Ok(spec) = std::env::var("JA_OFFSETS") {
        // Focused re-probe of a region the systematic walk flagged.
        for tok in spec.split(',') {
            if let Ok(o) = tok.trim().parse::<usize>() { if o < n { offsets.push(o); } }
        }
    } else {
        for i in 0..64 { if i < n { offsets.push(i); } }                 // head, every byte
        for p in 1..=19 { let o = n * p / 20; if o < n { offsets.push(o); } }  // 5%..95%
        for i in 0..32 { if n > i { offsets.push(n - 1 - i); } }         // tail, every byte
    }
    offsets.sort_unstable(); offsets.dedup();
    let patterns: Vec<u8> = match std::env::var("JA_PATTERNS") {
        Ok(s) => s.split(',').filter_map(|t| u8::from_str_radix(t.trim().trim_start_matches("0x"), 16).ok()).collect(),
        Err(_) => vec![1u8],
    };

    // A corrupted length prefix makes arkworks' Vec deserializer panic with "capacity
    // overflow" instead of returning Err. That is a refusal, but an ungraceful one, and it
    // aborts the sweep. We do NOT patch jolt-atlas or arkworks; the panic is caught here, in
    // our own code, and recorded as its own verdict so the sweep can continue and so the
    // behaviour is reported rather than hidden.
    std::panic::set_hook(Box::new(|_| {}));
    for off in offsets {
      for &pat in &patterns {
        let mut b = bytes.clone();
        let before = b[off];
        b[off] ^= pat;
        let verdict = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match Proof::deserialize_compressed(&b[..]) {
                Ok(p2) => {
                    if p2.verify(&verifier_pp, &io, None).is_ok() {
                        "VERIFY_ACCEPTED"
                    } else {
                        "VERIFY_REJECTED"
                    }
                }
                Err(_) => "DESERIALIZE_REJECTED",
            }
        }))
        .unwrap_or("DESERIALIZE_PANIC");
        println!("{label},proof_byte,{off},xor{pat:02x}_from{before:02x},{verdict}");
      }
    }
    let _ = std::panic::take_hook();
    eprintln!("proof bytes = {n}");
}
