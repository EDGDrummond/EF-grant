Here we want to bench circom at various circuit sizes (just filled with the iszero function). There are shell
files prepared to do all of this, so benching just requires running those shell files that run it all and 
delete created files once done. Below is a description of the steps we need to walk through to bench







## Adjust as you go
In all the CLI commands below I set the filename to iszero2, alter this depending on the starting file


# Compile circuit
# Compile the circuit (and create the needed files)
circom iszero2.circom --r1cs --wasm --json

# Generate Witness (file from input json file)
----> Enter the iszero2.js folder
node generate_witness.js iszero2.wasm ../../input.json witness.wtns

_______UNNECESSARY_________
# Powers of Tau phase 1 (circuit independent)
-----> This is unneccesary as already one (though obviously not secure)
snarkjs powersoftau new bn128 12 pot12_0000.ptau -v
snarkjs powersoftau contribute pot12_0000.ptau pot12_0001.ptau --name="First contribution" -v
snarkjs powersoftau prepare phase2 pot12_0001.ptau pot12_final.ptau -v
_______UNNECESSARY_________

# Powers of Tau phase 2 (circuit dependent)
snarkjs groth16 setup ../iszero2.r1cs ../../pot/pot12_final.ptau iszero2.zkey
snarkjs zkey contribute iszero2.zkey iszero2-1.zkey --name="1st Contributor Name" -v
snarkjs zkey export verificationkey iszero2-1.zkey verification_key.json

# Generate a Proof
snarkjs groth16 prove iszero2-1.zkey witness.wtns proof.json public.json

# Verify the Proof
snarkjs groth16 verify verification_key.json public.json proof.json


## Clean-up
This process of course creates a bunch of files that clog things up, so feel free to delete all the generated files after this process