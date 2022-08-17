#!/bin/sh
# Circuit compilation
echo "______BENCHING CIRCUIT COMPILATION______"
time circom iszero_generic.circom --r1cs --wasm --json

# Witness generation
cd iszero_generic_js
echo "______BENCHING WITNESS GENERATION______"
time node generate_witness.js iszero_generic.wasm ../../input.json witness.wtns

# Circuit specific setup
echo "______BENCHING CIRCUIT SET-UP______"
time (snarkjs groth16 setup ../iszero_generic.r1cs ../../pot/pot20_final.ptau iszero_generic.zkey &&  \
snarkjs zkey contribute iszero_generic.zkey iszero_generic-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348' &&  \
snarkjs zkey export verificationkey iszero_generic-1.zkey verification_key.json)

# Proof generation
echo "______BENCHING PROOF GENERATION______"
time snarkjs groth16 prove iszero_generic-1.zkey witness.wtns proof.json public.json

# Verify the Proof
echo "______BENCHING PROOF VERIFICATION______"
time snarkjs groth16 verify verification_key.json public.json proof.json


## Clean-up
cd ..
rm iszero_generic.r1cs
rm iszero_generic_constraints.json
rm -r iszero_generic_js