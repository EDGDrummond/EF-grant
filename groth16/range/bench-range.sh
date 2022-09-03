#!/bin/sh

# circom range.circom --r1cs --wasm --json
# cd range_js
# node generate_witness.js range.wasm ../input.json witness.wtns
# snarkjs groth16 setup ../range.r1cs ../../pot/pot20_final.ptau range.zkey &&  \
# snarkjs zkey contribute range.zkey range-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348' &&  \
# snarkjs zkey export verificationkey range-1.zkey verification_key.json
# snarkjs groth16 prove range-1.zkey witness.wtns proof.json public.json
# snarkjs groth16 verify verification_key.json public.json proof.json
# cd ..
# rm range.r1cs
# rm range_constraints.json
# rm -r range_js

# Circuit compilation
echo "______BENCHING CIRCUIT COMPILATION______"
time circom range.circom --r1cs --wasm --json

# Witness generation
cd range_js
echo "______BENCHING WITNESS GENERATION______"
time node generate_witness.js range.wasm ../input.json witness.wtns

# Circuit specific setup
echo "______BENCHING CIRCUIT SET-UP______"
time (snarkjs groth16 setup ../range.r1cs ../../pot/pot20_final.ptau range.zkey &&  \
snarkjs zkey contribute range.zkey range-1.zkey --name="1st Contributor Name" -v <<< 'jhcag7f23gr9fg4y38gfib43gfn348' &&  \
snarkjs zkey export verificationkey range-1.zkey verification_key.json)

# Proof generation
echo "______BENCHING PROOF GENERATION______"
time snarkjs groth16 prove range-1.zkey witness.wtns proof.json public.json

# Verify the Proof
echo "______BENCHING PROOF VERIFICATION______"
time snarkjs groth16 verify verification_key.json public.json proof.json


## Clean-up
cd ..
rm range.r1cs
rm range_constraints.json
rm -r range_js