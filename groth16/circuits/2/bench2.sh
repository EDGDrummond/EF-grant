#!/bin/sh
# Circuit compilation
echo "BENCHING CIRCUIT COMPILATION"
bench 'circom iszero2.circom --r1cs --wasm --json'

# Witness generation
cd iszero2_js
echo "BENCHING WITNESS GENERATION"
bench 'node generate_witness.js iszero2.wasm ../../input.json witness.wtns'

## Circuit specific setup & proof generation
echo "BENCHING CIRCUIT SET-UP & PROOF GENERATION"
bench 'bash ../bench2-prover.sh'

# Verify the Proof
echo "BENCHING PROOF VERIFICATION"
bench 'snarkjs groth16 verify verification_key.json public.json proof.json'


## Clean-up
cd ..
rm iszero2.r1cs
rm iszero2_constraints.json
rm -r iszero2_js