#!/bin/sh
# Don't use this script, it is called by the other script

## Circuit specific setup & proof generation
# POT phase 2
snarkjs groth16 setup ../iszero2.r1cs ../../pot/pot12_final.ptau iszero2.zkey
snarkjs zkey contribute iszero2.zkey iszero2-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348'
snarkjs zkey export verificationkey iszero2-1.zkey verification_key.json

# Generate a Proof
snarkjs groth16 prove iszero2-1.zkey witness.wtns proof.json public.json