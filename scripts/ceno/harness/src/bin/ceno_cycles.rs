//! zk-prover-bench · Ceno · emulate a guest and report its instruction and cycle counts.
//!
//! A zkVM's natural unit is not the MAC. `bench/TASKS.md` denominates every task in MACs and
//! that denominator is frozen, but Ceno pays per **cycle**, and the ratio between the two is
//! the whole story of what a zkVM costs relative to a circuit. This tool reports that ratio
//! without proving anything, so it can be run on rungs that are too large to prove and so
//! that the grid can state a cycle count for every cell, including the ones that produced no
//! proof.
//!
//! It emulates only — no witness generation, no commitment, no proof. It is therefore fast
//! (seconds) and its numbers are exact, not sampled: `VMState::iter_until_halt` yields one
//! item per executed instruction, and `Tracer` reports the cycle counter the proof system
//! itself shards on.
//!
//! This program links `ceno_emul` and `ceno_zkvm` directly. That is a licence affordance, not
//! a cleverness: Ceno is MIT OR Apache-2.0. The two competing systems in this benchmark whose
//! licences forbid derivation got no equivalent instrument, and RESULTS.md says so.

use std::{fs, path::PathBuf, sync::Arc};

use ceno_emul::{IterAddresses, Program, VMState, WORD_SIZE};
use ceno_host::memory_from_file;
use ceno_zkvm::e2e::{Preset, setup_platform};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let elf_path = PathBuf::from(
        args.next()
            .ok_or_else(|| anyhow::anyhow!("usage: ceno_cycles <elf> [hints-file]"))?,
    );
    let hints_path = args.next().map(PathBuf::from);

    let elf_bytes = fs::read(&elf_path)?;
    let program = Program::load_elf(&elf_bytes, u32::MAX)?;

    // The same platform the measured `e2e` runs build, with the same defaults, so the cycle
    // count reported here is the cycle count that run would have proved.
    let stack_size: u32 = 2 * 1024 * 1024;
    let heap_size: u32 = 2 * 1024 * 1024;
    let platform = setup_platform(
        Preset::Ceno,
        &program,
        stack_size.next_multiple_of(WORD_SIZE as u32),
        heap_size.next_multiple_of(WORD_SIZE as u32),
    );

    let hints: Vec<u32> = match &hints_path {
        Some(p) => memory_from_file(p)?,
        None => vec![],
    };
    anyhow::ensure!(
        hints.len() <= platform.hints.iter_addresses().len(),
        "hints do not fit in the platform's hint window"
    );

    let hints_range = platform.hints.clone();
    let mut state = VMState::new(platform, Arc::new(program));
    for (addr, value) in hints_range.iter_addresses().zip(hints.iter().copied()) {
        state.init_memory(addr.into(), value);
    }

    let steps = state
        .iter_until_halt()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("emulation failed: {e:?}"))?;

    // The instruction mix is NOT computed here. `iter_until_halt` yields a step index rather
    // than a decoded record at this revision, and reconstructing the opcode from the program
    // image would be our arithmetic rather than Ceno's. The mix is read instead from the
    // prover's own log lines — `tracer generated <OPCODE> <n> records`, emitted per opcode by
    // ceno_zkvm::instructions::riscv::rv32im — which is the system reporting on itself.
    let cycles = state.tracer().cycle();
    println!("elf,{}", elf_path.display());
    println!("hints,{}", hints_path.map(|p| p.display().to_string()).unwrap_or_default());
    println!("instructions,{}", steps.len());
    println!("cycles,{cycles}");
    Ok(())
}
