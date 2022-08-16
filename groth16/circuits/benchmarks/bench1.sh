#!/bin/sh
# Circuit compilation
echo "BENCHING CIRCUIT COMPILATION"
bench 'circom ../1/iszero1.circom --r1cs --wasm --json'

# Witness generation
cd iszero1_js
echo "BENCHING WITNESS GENERATION"
bench 'node generate_witness.js iszero1.wasm ../../input.json witness.wtns'

# Circuit specific setup
echo "BENCHING CIRCUIT SET-UP"
bench 'snarkjs groth16 setup ../iszero1.r1cs ../../pot/pot20_final.ptau iszero1.zkey
snarkjs zkey contribute iszero1.zkey iszero1-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348'
snarkjs zkey export verificationkey iszero1-1.zkey verification_key.json'

# Proof generation
echo "BENCHING PROOF GENERATION"
bench 'snarkjs groth16 prove iszero1-1.zkey witness.wtns proof.json public.json'

# Verify the Proof
echo "BENCHING PROOF VERIFICATION"
bench 'snarkjs groth16 verify verification_key.json public.json proof.json'


## Clean-up
cd ..
rm iszero1.r1cs
rm iszero1_constraints.json
rm -r iszero1_js