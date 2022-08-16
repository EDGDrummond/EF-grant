#!/bin/sh
# Circuit compilation
echo "BENCHING CIRCUIT COMPILATION"
bench 'circom ../5/iszero5.circom --r1cs --wasm --json'

# Witness generation
cd iszero5_js
echo "BENCHING WITNESS GENERATION"
bench 'node generate_witness.js iszero5.wasm ../../input.json witness.wtns'

# Circuit specific setup
echo "BENCHING CIRCUIT SET-UP"
bench 'snarkjs groth16 setup ../iszero5.r1cs ../../pot/pot20_final.ptau iszero5.zkey
snarkjs zkey contribute iszero5.zkey iszero5-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348'
snarkjs zkey export verificationkey iszero5-1.zkey verification_key.json'

# Proof generation
echo "BENCHING PROOF GENERATION"
bench 'snarkjs groth16 prove iszero5-1.zkey witness.wtns proof.json public.json'

# Verify the Proof
echo "BENCHING PROOF VERIFICATION"
bench 'snarkjs groth16 verify verification_key.json public.json proof.json'


## Clean-up
cd ..
rm iszero5.r1cs
rm iszero5_constraints.json
rm -r iszero5_js