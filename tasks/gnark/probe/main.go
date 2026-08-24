package main

import (
	"bytes"
	"fmt"
	"math/big"
	"runtime"
	"time"

	"github.com/consensys/gnark-crypto/ecc"
	"github.com/consensys/gnark/backend/groth16"
	"github.com/consensys/gnark/frontend"
	"github.com/consensys/gnark/frontend/cs/r1cs"
	"github.com/consensys/gnark/logger"
	"github.com/consensys/gnark/std/rangecheck"
	"github.com/rs/zerolog"
)

// T1-0: [1x256] . [256x256], INT8 in, INT32 out, weights as WITNESS (regime A)
const K = 768
const Nn = 768

type T10 struct {
	X   [K]frontend.Variable     `gnark:",secret"`
	W   [K][Nn]frontend.Variable `gnark:",secret"`
	Out [Nn]frontend.Variable    `gnark:",public"`
}

func (c *T10) Define(api frontend.API) error {
	rc := rangecheck.New(api)
	for i := 0; i < K; i++ {
		rc.Check(api.Add(c.X[i], 128), 8)
		for j := 0; j < Nn; j++ {
			rc.Check(api.Add(c.W[i][j], 128), 8)
		}
	}
	for j := 0; j < Nn; j++ {
		acc := frontend.Variable(0)
		for i := 0; i < K; i++ {
			acc = api.Add(acc, api.Mul(c.X[i], c.W[i][j]))
		}
		api.AssertIsEqual(acc, c.Out[j])
	}
	return nil
}

func mem(tag string) {
	var m runtime.MemStats
	runtime.ReadMemStats(&m)
	fmt.Printf("  [mem %-12s] HeapAlloc=%6.2f GB  Sys=%6.2f GB  TotalAlloc=%6.2f GB\n",
		tag, float64(m.HeapAlloc)/1e9, float64(m.Sys)/1e9, float64(m.TotalAlloc)/1e9)
}

func main() {
	logger.Set(zerolog.New(zerolog.NewConsoleWriter()).Level(zerolog.Disabled))

	t := time.Now()
	cs, err := frontend.Compile(ecc.BN254.ScalarField(), r1cs.NewBuilder, &T10{})
	if err != nil {
		panic(err)
	}
	fmt.Printf("compile      %8.3f s   constraints=%d\n", time.Since(t).Seconds(), cs.GetNbConstraints())
	mem("after compile")

	t = time.Now()
	pk, vk, err := groth16.Setup(cs)
	if err != nil {
		panic(err)
	}
	fmt.Printf("setup        %8.3f s\n", time.Since(t).Seconds())
	mem("after setup")

	// build a real assignment
	var a T10
	xs := make([]int, K)
	for i := 0; i < K; i++ {
		xs[i] = (i*37)%255 - 127
		a.X[i] = xs[i]
	}
	ws := make([][]int, K)
	for i := 0; i < K; i++ {
		ws[i] = make([]int, Nn)
		for j := 0; j < Nn; j++ {
			ws[i][j] = ((i*31+j*17)%255) - 127
			a.W[i][j] = ws[i][j]
		}
	}
	maxabs := big.NewInt(0)
	for j := 0; j < Nn; j++ {
		s := 0
		for i := 0; i < K; i++ {
			s += xs[i] * ws[i][j]
		}
		a.Out[j] = s
		if v := new(big.Int).Abs(big.NewInt(int64(s))); v.Cmp(maxabs) > 0 {
			maxabs = v
		}
	}
	fmt.Printf("max |intermediate| = %s\n", maxabs.String())

	w, err := frontend.NewWitness(&a, ecc.BN254.ScalarField())
	if err != nil {
		panic(err)
	}
	pw, _ := w.Public()

	t = time.Now()
	proof, err := groth16.Prove(cs, pk, w)
	if err != nil {
		panic(err)
	}
	fmt.Printf("prove        %8.3f s\n", time.Since(t).Seconds())
	mem("after prove")

	var buf bytes.Buffer
	n, _ := proof.WriteTo(&buf)
	fmt.Printf("proof bytes  %8d  (WriteTo)\n", n)
	var buf2 bytes.Buffer
	n2, _ := proof.WriteRawTo(&buf2)
	fmt.Printf("proof bytes  %8d  (WriteRawTo, uncompressed)\n", n2)

	t = time.Now()
	if err := groth16.Verify(proof, vk, pw); err != nil {
		panic(err)
	}
	fmt.Printf("verify       %8.3f s   ACCEPTED\n", time.Since(t).Seconds())
}
