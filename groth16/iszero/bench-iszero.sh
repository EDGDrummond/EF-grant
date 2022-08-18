#!/bin/sh
# Circuit compilation
echo "______BENCHING CIRCUIT COMPILATION______"
time circom iszero.circom --r1cs --wasm --json

# Witness generation
cd iszero_js
echo "______BENCHING WITNESS GENERATION______"
time node generate_witness.js iszero.wasm ../input.json witness.wtns

# Circuit specific setup
echo "______BENCHING CIRCUIT SET-UP______"
time (snarkjs groth16 setup ../iszero.r1cs ../../pot/pot20_final.ptau iszero.zkey &&  \
snarkjs zkey contribute iszero.zkey iszero-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348' &&  \
snarkjs zkey export verificationkey iszero-1.zkey verification_key.json)

# Proof generation
echo "______BENCHING PROOF GENERATION______"
time snarkjs groth16 prove iszero-1.zkey witness.wtns proof.json public.json

# Verify the Proof
echo "______BENCHING PROOF VERIFICATION______"
time snarkjs groth16 verify verification_key.json public.json proof.json


## Clean-up
cd ..
rm iszero.r1cs
rm iszero_constraints.json
rm -r iszero_js