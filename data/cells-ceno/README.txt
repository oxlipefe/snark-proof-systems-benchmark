Raw per-repetition output for the Ceno campaign, uncurated.

<cell>/<tag>.stdout.txt   Ceno's tracing output: the ZKVM_create_proof span (one PER SHARD),
                          the per-opcode witness counts, and — for the warmup, which runs
                          without --profiling — `program executed <n> instructions in <m>
                          cycles` and `num_shards`.
<cell>/<tag>.log.txt      /usr/bin/time -l: real/user/sys and the two memory peaks.
<cell>/<tag>.proof.bin    the serialized proof, kept for the last repetition of each cell.

The verifying keys these runs wrote have been DELETED, and the deletion is itself worth
recording: each was 89 880 549 bytes (~90 MB), and every one of them was unusable. A vk that
has been through bincode cannot verify any proof at this commit, because
ZKVMVerifyingKey::circuit_index_to_name is #[serde(skip)] while the verifier requires it
(systems/ceno/BUILD.md §5). Their sizes are recorded in cells-ceno.csv; the artifacts
themselves would have been ~540 MB of bytes that cannot do the one thing a vk is for.
The correctness control regenerates the key in process instead.
