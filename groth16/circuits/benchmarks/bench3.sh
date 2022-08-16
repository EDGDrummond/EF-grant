#!/bin/sh
# Circuit compilation
echo "BENCHING CIRCUIT COMPILATION"
bench 'circom ../3/iszero3.circom --r1cs --wasm --json'

# Witness generation
cd iszero3_js
echo "BENCHING WITNESS GENERATION"
bench 'node generate_witness.js iszero3.wasm ../../input.json witness.wtns'

# Circuit specific setup
echo "BENCHING CIRCUIT SET-UP"
bench 'snarkjs groth16 setup ../iszero3.r1cs ../../pot/pot20_final.ptau iszero3.zkey
snarkjs zkey contribute iszero3.zkey iszero3-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348'
snarkjs zkey export verificationkey iszero3-1.zkey verification_key.json'

# Proof generation
echo "BENCHING PROOF GENERATION"
bench 'snarkjs groth16 prove iszero3-1.zkey witness.wtns proof.json public.json'

# Verify the Proof
echo "BENCHING PROOF VERIFICATION"
bench 'snarkjs groth16 verify verification_key.json public.json proof.json'


## Clean-up
cd ..
rm iszero3.r1cs
rm iszero3_constraints.json
rm -r iszero3_js