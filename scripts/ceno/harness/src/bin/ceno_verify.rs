//! zk-prover-bench · Ceno · keygen, standalone verify, and the correctness control.
//!
//! # Why this does keygen instead of loading the vk from disk
//!
//! `e2e` writes a vk with `bincode::serialize`, and `cargo ceno verify --proof --vk` reads it
//! back. That path **cannot succeed at the pinned commit**, and the mechanism is exact:
//!
//!   * `ZKVMVerifyingKey::circuit_index_to_name` is `#[serde(skip)]`
//!     (`ceno_zkvm/src/structs.rs:1081`), with the comment "mainly used for debugging";
//!   * `ZKVMVerifier::new` only computes a digest — it does not rebuild that map
//!     (`ceno_zkvm/src/scheme/verifier.rs`);
//!   * but the main verification path looks every chip-proof index up in it and returns
//!     `VKNotFound` when it is missing (`verifier.rs:577-583`).
//!
//! So a round-tripped vk is always empty there, and every honest proof verified against one is
//! rejected in under a millisecond. We measured exactly that before finding the cause:
//! `VKNotFound("0th shard circuit index 0 missing from vk index map")`.
//!
//! This matters for the benchmark far more than it matters for timing. `bench/README.md`
//! requires that a corrupted trace make `verify()` fail — and a control run against a
//! round-tripped vk would report every corruption as rejected **for the wrong reason**,
//! passing while proving nothing. That is precisely the vacuous control this repository's
//! rules forbid. So the vk is regenerated in-process, and the honest-proof positive control is
//! what establishes the control is not vacuous.
//!
//! It is a completeness defect, not a soundness one: it makes valid proofs fail, never invalid
//! ones pass. Right of reply applies.
//!
//! # What is timed
//!
//! `keygen_s` is the one-off setup, reported separately and never amortised into prove time.
//! `verify_s` is `ceno_zkvm::e2e::verify` alone; proof deserialization and the digest
//! computation inside `ZKVMVerifier::new` are timed separately and never folded in.
//!
//! # The corruption control
//!
//! Corruption is applied to the **serialized proof bytes**. Ceno's witness is generated inside
//! the prover from the ELF and the hints and is never exposed as a mutable artifact the way
//! binius64's `ValueVec` is, so a witness-level flip is not available. That is a weaker control
//! than binius64's and RESULTS.md says so rather than presenting them as equivalent.
//!
//! # The composition control (`compose`, G-11d)
//!
//! A sharded Ceno proof is a `Vec<ZKVMProof<E, PCS>>` serialized with bincode: an 8-byte
//! little-endian length followed by the shards' own encodings, back to back and unpadded.
//! That layout is what makes this control possible without cloning a 222 MB deserialized
//! object: the proof is deserialized once, each shard is re-serialized on its own, and every
//! mutation is then assembled by concatenating shard blobs behind a rewritten length header.
//!
//! The layout claim is not assumed. `M0_CONTROL` reassembles all N shards in order and
//! asserts the result is **byte-identical** to `bincode::serialize` of the whole vector, and
//! then verifies it. If either half of that fails, the instrument is dead and no rejection
//! below means anything — repository rule 8: verify the instrument before accepting a
//! negative.
//!
//! `M3_SWAP` and `M4_GRAFT` are the load-bearing mutations. Both preserve the shard count, so
//! neither can be caught by a length check; only an actual binding of shard i+1 to shard i can
//! reject them.

use std::{fs, panic, path::PathBuf, sync::Mutex, time::Instant};

use ceno_emul::{Program, WORD_SIZE};
use ceno_host::memory_from_file;
use ceno_zkvm::{
    e2e::{
        Checkpoint, MultiProver, Preset, public_io_words_to_digest_words, run_e2e_with_checkpoint,
        setup_platform, verify,
    },
    scheme::{
        ZKVMProof, constants::MAX_NUM_VARIABLES, create_backend, create_prover,
        verifier::ZKVMVerifier,
    },
    structs::ZKVMVerifyingKey,
};
use ff_ext::BabyBearExt4;
use mpcs::{Basefold, BasefoldRSParams, Jagged, SecurityLevel};

type E = BabyBearExt4;
type PCS = Jagged<Basefold<BabyBearExt4, BasefoldRSParams>>;

const DEFAULT_MAX_CYCLE_PER_SHARD: u64 = 536_870_912;
const DEFAULT_MAX_CELL_PER_SHARD: u64 = 2_147_483_648;

fn verdict_of(proof_bytes: &[u8], verifier: &ZKVMVerifier<E, PCS>) -> String {
    let parsed = panic::catch_unwind(|| bincode::deserialize::<Vec<ZKVMProof<E, PCS>>>(proof_bytes));
    let proofs = match parsed {
        Err(_) => return "DESERIALIZE_PANIC".into(),
        Ok(Err(_)) => return "DESERIALIZE_REJECTED".into(),
        Ok(Ok(p)) => p,
    };
    match panic::catch_unwind(panic::AssertUnwindSafe(|| verify(proofs, verifier))) {
        Err(_) => "VERIFY_PANIC".into(),
        Ok(Err(_)) => "VERIFY_REJECTED".into(),
        Ok(Ok(_)) => "VERIFY_ACCEPTED".into(),
    }
}

// ---- composition control (G-11d) ----------------------------------------------------------
//
// The last panic message seen by the process-wide hook. `catch_unwind` gives back an opaque
// payload; the hook is the only place the message and its location are available. A panic at
// deserialization time proves the *format* broke, not that the verifier binds anything, so the
// two have to be told apart and the literal text kept.
static PANIC_MSG: Mutex<String> = Mutex::new(String::new());

fn install_recording_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        };
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<no location>".into());
        if let Ok(mut slot) = PANIC_MSG.lock() {
            *slot = format!("{payload} @ {loc}");
        }
    }));
}

fn take_panic_msg() -> String {
    PANIC_MSG
        .lock()
        .map(|mut s| std::mem::take(&mut *s))
        .unwrap_or_else(|_| "<panic message lock poisoned>".into())
}

fn csv_quote(s: &str) -> String {
    let flat = s.replace(['\n', '\r'], " ");
    format!("\"{}\"", flat.replace('"', "\"\""))
}

/// Assemble a bincode `Vec<ZKVMProof>` from individually serialized shard blobs.
///
/// bincode's free `serialize` uses fixed-width little-endian u64 for sequence lengths and
/// writes elements consecutively with no padding, so this is exactly the wire format. That is
/// asserted at run time by M0_CONTROL rather than trusted.
fn assemble(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + parts.iter().map(|p| p.len()).sum::<usize>());
    out.extend_from_slice(&(parts.len() as u64).to_le_bytes());
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

/// Classify one attempt. Returns `(verdict, detail, verify_elapsed_s)`.
///
/// The five verdicts are kept distinct on purpose: a deserialization error or a panic says the
/// byte stream stopped parsing, which is *not* evidence that the verifier binds composition.
/// `VKNotFound` is broken out as OTHER for the same reason — BUILD.md §5 documents it as a
/// completeness defect of the vk index map, so it would be a rejection for the wrong cause.
fn classify(proof_bytes: &[u8], verifier: &ZKVMVerifier<E, PCS>) -> (String, String, f64) {
    let _ = take_panic_msg();
    let parsed = panic::catch_unwind(|| bincode::deserialize::<Vec<ZKVMProof<E, PCS>>>(proof_bytes));
    let proofs = match parsed {
        Err(_) => return ("PANIC".into(), format!("stage=deserialize {}", take_panic_msg()), 0.0),
        Ok(Err(e)) => return ("ERROR_DESERIALIZE".into(), format!("{e}"), 0.0),
        Ok(Ok(p)) => p,
    };
    let t = Instant::now();
    let res = panic::catch_unwind(panic::AssertUnwindSafe(|| verify(proofs, verifier)));
    let elapsed = t.elapsed().as_secs_f64();
    match res {
        Err(_) => ("PANIC".into(), format!("stage=verify {}", take_panic_msg()), elapsed),
        Ok(Err(e)) => {
            let detail = format!("{e:?}");
            if detail.contains("VKNotFound") {
                ("OTHER".into(), format!("VKNotFound (BUILD.md §5 completeness defect) {detail}"), elapsed)
            } else {
                ("REJECTED_VERIFY".into(), detail, elapsed)
            }
        }
        Ok(Ok(())) => ("ACCEPTED".into(), String::new(), elapsed),
    }
}

/// One mutation to apply: a name, the index it is parameterised by, and the shard order.
struct Mutation {
    id: &'static str,
    k: String,
    detail: String,
    /// Indices into `shard_blobs`; `None` means "take the donor blob here".
    order: Vec<Option<usize>>,
}

fn build_mutations(n: usize, donor_n: usize) -> Vec<Mutation> {
    let identity: Vec<Option<usize>> = (0..n).map(Some).collect();
    let mut out = vec![Mutation {
        id: "M0_CONTROL",
        k: "-".into(),
        detail: format!("reassemble all {n} shards in order"),
        order: identity.clone(),
    }];

    let dedup = |mut v: Vec<usize>| {
        v.sort_unstable();
        v.dedup();
        v
    };
    // k = 0, N/2, N-1 for the single-shard mutations.
    let solo_ks = dedup(vec![0, n / 2, n - 1]);
    // Pair mutations need k+1 in range, so the last usable k is N-2.
    let pair_ks = dedup(vec![0, n / 2, n.saturating_sub(2)]);

    for k in &solo_ks {
        let mut order = identity.clone();
        order.remove(*k);
        out.push(Mutation {
            id: "M1_DROP",
            k: k.to_string(),
            detail: format!("shard {k} removed"),
            order,
        });
    }
    for k in &pair_ks {
        let mut order = identity.clone();
        order[k + 1] = Some(*k); // shard k appears twice; shard k+1 is gone
        out.push(Mutation {
            id: "M2_DUP",
            k: k.to_string(),
            detail: format!("shard {k} duplicated over position {}", k + 1),
            order,
        });
    }
    for k in &pair_ks {
        let mut order = identity.clone();
        order.swap(*k, k + 1);
        out.push(Mutation {
            id: "M3_SWAP",
            k: k.to_string(),
            detail: format!("shards {k} and {} transposed", k + 1),
            order,
        });
    }
    if donor_n > 0 {
        for k in &solo_ks {
            let mut order = identity.clone();
            order[*k] = None;
            out.push(Mutation {
                id: "M4_GRAFT",
                k: k.to_string(),
                detail: format!("position {k} replaced by donor shard {}", *k % donor_n),
                order,
            });
        }
    }
    let half = n / 2;
    out.push(Mutation {
        id: "M5_TRUNC",
        k: "-".into(),
        detail: format!("kept first {half} of {n} shards"),
        order: identity[..half].to_vec(),
    });
    out
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap_or_default();
    let elf_path = PathBuf::from(args.next().ok_or_else(|| {
        anyhow::anyhow!(
            "usage: ceno_verify <time|negative|compose> <elf> <hints> <public-io-file> <proof> <task> [stride|donor-proof]"
        )
    })?);
    let hints_path = PathBuf::from(args.next().ok_or_else(|| anyhow::anyhow!("missing hints"))?);
    let pio_path = PathBuf::from(args.next().ok_or_else(|| anyhow::anyhow!("missing public-io"))?);
    let proof_path = PathBuf::from(args.next().ok_or_else(|| anyhow::anyhow!("missing proof"))?);
    let task = args.next().unwrap_or_else(|| "unknown".into());
    // Seventh positional: STRIDE for `negative`, the donor proof path for `compose`.
    let extra: Option<String> = args.next();
    let stride: usize = extra.as_deref().and_then(|s| s.parse().ok()).unwrap_or(1);

    let elf_bytes = fs::read(&elf_path)?;
    let program = Program::load_elf(&elf_bytes, u32::MAX)?;
    let platform = setup_platform(
        Preset::Ceno,
        &program,
        (2u32 * 1024 * 1024).next_multiple_of(WORD_SIZE as u32),
        (2u32 * 1024 * 1024).next_multiple_of(WORD_SIZE as u32),
    );
    let hints: Vec<u32> = memory_from_file(&hints_path)?;
    let pio_text = fs::read_to_string(&pio_path)?;
    let public_io: Vec<u32> = pio_text
        .trim()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<u32>())
        .collect::<Result<_, _>>()?;
    let public_io_digest = public_io_words_to_digest_words(&public_io);

    // Keygen. This is the one-off setup; it is reported separately and never amortised into
    // prove time. `Checkpoint::PrepE2EProving` is exactly what `cargo ceno keygen` runs
    // (ceno_cli/src/commands/common_args/ceno.rs :: keygen_inner).
    let t = Instant::now();
    let backend = create_backend::<E, PCS>(MAX_NUM_VARIABLES, SecurityLevel::default());
    let prover = create_prover(backend);
    let multi_prover = MultiProver::new(
        0,
        1,
        DEFAULT_MAX_CELL_PER_SHARD,
        DEFAULT_MAX_CYCLE_PER_SHARD,
    );
    let result = run_e2e_with_checkpoint::<E, PCS, _, _>(
        prover,
        program,
        platform,
        multi_prover,
        &hints,
        public_io_digest,
        usize::MAX,
        Checkpoint::PrepE2EProving,
        None,
    );
    let keygen_s = t.elapsed().as_secs_f64();
    let vk: ZKVMVerifyingKey<E, PCS> = result.vk.expect("keygen must yield a vk");

    let t = Instant::now();
    let verifier = ZKVMVerifier::new(vk);
    let verifier_new_s = t.elapsed().as_secs_f64();

    let proof_bytes = fs::read(&proof_path)?;

    if mode == "time" {
        let t = Instant::now();
        let proofs: Vec<ZKVMProof<E, PCS>> = bincode::deserialize(&proof_bytes)?;
        let proof_deser_s = t.elapsed().as_secs_f64();
        let shards = proofs.len();

        let t = Instant::now();
        let res = verify(proofs, &verifier);
        let verify_s = t.elapsed().as_secs_f64();
        if let Err(e) = &res {
            println!("verify_error,{e:?}");
        }

        println!("task,{task}");
        println!("proof_bytes,{}", proof_bytes.len());
        println!("shards,{shards}");
        println!("keygen_s,{keygen_s:.6}");
        println!("verifier_new_s,{verifier_new_s:.6}");
        println!("proof_deserialize_s,{proof_deser_s:.6}");
        println!("verify_s,{verify_s:.6}");
        println!(
            "verdict,{}",
            if res.is_ok() { "VERIFY_ACCEPTED" } else { "VERIFY_REJECTED" }
        );
        return Ok(());
    }

    if mode == "compose" {
        install_recording_panic_hook();
        println!("task,mutation,k,shards_in,shards_out,verdict,detail,elapsed_s");

        // Split the subject proof into per-shard blobs, then release the deserialized form.
        let proofs: Vec<ZKVMProof<E, PCS>> = bincode::deserialize(&proof_bytes)?;
        let n = proofs.len();
        let shard_blobs: Vec<Vec<u8>> = proofs
            .iter()
            .map(bincode::serialize)
            .collect::<Result<_, _>>()?;
        let whole = bincode::serialize(&proofs)?;
        drop(proofs);

        // Instrument check A: the wire-format assumption. If the concatenation is not
        // byte-identical to the library's own output, every mutation below is built on sand.
        let reassembled = assemble(&shard_blobs.iter().map(|b| b.as_slice()).collect::<Vec<_>>());
        let layout_ok = reassembled == whole;
        println!(
            "{task},M0_LAYOUT,-,{n},{n},{},{},0.0",
            if layout_ok { "ACCEPTED" } else { "OTHER" },
            csv_quote(&format!(
                "reassembled={} whole={} on_disk={} identical={layout_ok}",
                reassembled.len(),
                whole.len(),
                proof_bytes.len()
            ))
        );
        if !layout_ok {
            eprintln!("[compose] wire-format assumption failed — aborting, instrument is dead");
            return Ok(());
        }

        // Instrument check B: the untouched on-disk bytes verify.
        eprintln!("=== M0_ORIGINAL ===");
        let (v, d, e) = classify(&proof_bytes, &verifier);
        println!("{task},M0_ORIGINAL,-,{n},{n},{v},{},{e:.6}", csv_quote(&d));
        if v != "ACCEPTED" {
            println!("{task},ABORT,-,{n},{n},OTHER,{},0.0",
                csv_quote("on-disk proof does not verify — the sweep would be vacuous"));
            eprintln!("[compose] honest proof rejected — aborting, instrument is dead");
            return Ok(());
        }
        drop(proof_bytes);
        drop(whole);

        // The donor, for M4_GRAFT: a different proof of the same shape (same ELF, same cap).
        let donor_blobs: Vec<Vec<u8>> = match extra.as_deref() {
            Some(path) if !path.is_empty() => {
                let bytes = fs::read(path)?;
                let dp: Vec<ZKVMProof<E, PCS>> = bincode::deserialize(&bytes)?;
                eprintln!("[compose] donor {path}: {} shards", dp.len());
                dp.iter().map(bincode::serialize).collect::<Result<_, _>>()?
            }
            _ => {
                eprintln!("[compose] no donor proof given — M4_GRAFT will be skipped");
                Vec::new()
            }
        };

        for m in build_mutations(n, donor_blobs.len()) {
            // Wrap rather than clamp. Clamping sends every high position to the donor's LAST
            // shard, which is the only one carrying the halt flag, so the graft would be caught
            // by the halt-position check instead of by continuation — a shallower reason than
            // the one under test. Wrapping keeps every grafted shard non-terminal.
            let donor_n = donor_blobs.len().max(1);
            let parts: Vec<&[u8]> = m
                .order
                .iter()
                .enumerate()
                .map(|(pos, src)| match src {
                    Some(i) => shard_blobs[*i].as_slice(),
                    None => donor_blobs[pos % donor_n].as_slice(),
                })
                .collect();
            let shards_out = parts.len();
            let bytes = assemble(&parts);
            drop(parts);
            eprintln!("=== {} k={} ({}) ===", m.id, m.k, m.detail);
            let (verdict, detail, elapsed) = classify(&bytes, &verifier);
            let joined = if detail.is_empty() {
                m.detail.clone()
            } else {
                format!("{} | {detail}", m.detail)
            };
            println!(
                "{task},{},{},{n},{shards_out},{verdict},{},{elapsed:.6}",
                m.id,
                m.k,
                csv_quote(&joined)
            );
        }
        return Ok(());
    }

    // ---- negative control ----
    panic::set_hook(Box::new(|_| {}));
    println!("task,family,position,detail,verdict");

    // Positive control 1: the honest proof verifies. Without this the whole sweep is vacuous.
    let honest = verdict_of(&proof_bytes, &verifier);
    println!("{task},none,-,honest,{honest}");
    if honest != "VERIFY_ACCEPTED" {
        println!("{task},none,-,control is vacuous — aborting,HONEST_REJECTED");
        return Ok(());
    }

    // Positive control 2: deserialize -> re-serialize -> verify, unmodified. If this failed,
    // every rejection below would be an artefact of the method, not of the corruption.
    let proofs: Vec<ZKVMProof<E, PCS>> = bincode::deserialize(&proof_bytes)?;
    let roundtrip = bincode::serialize(&proofs)?;
    println!("{task},none,-,roundtrip,{}", verdict_of(&roundtrip, &verifier));

    let n = proof_bytes.len();
    let step = stride.max(1);
    println!(
        "{task},coverage,-,artifact_bytes={n} stride={step} probed={},INFO",
        n.div_ceil(step)
    );
    let mut off = 0usize;
    while off < n {
        let mut corrupt = proof_bytes.clone();
        corrupt[off] ^= 0x01;
        println!(
            "{task},proof_byte,{off},xor01_from{},{}",
            proof_bytes[off],
            verdict_of(&corrupt, &verifier)
        );
        off += step;
    }
    Ok(())
}
