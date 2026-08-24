package gnarkbench

// Both backends are exercised by every correctness test in this package: the R1CS and
// SparseR1CS builders lower these gadgets differently, and a gadget that is correct under
// one and not the other is a gadget the campaign would have published a wrong number for.
// See test.WithBackends(backend.GROTH16, backend.PLONK) at each call site.
