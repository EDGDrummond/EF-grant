This folder contains all the relevant circom code/circuits that we wish to compare the halo2 circuits to, though
ECDSA and Keccak benching times can be found in other repos:
https://github.com/0xPARC/circom-ecdsa
https://github.com/vocdoni/keccak256-circom

In order to get a powers of tau file you can either run it yourself or download and process one here
https://github.com/iden3/snarkjs#7-prepare-phase-2

NOTE: Benching was originally achieved by installing bench via Haskell's stack tool, but after a mac update this 
stopped working because a dependency wouldn't work on an M1 chip. So instead we temporarily use touch to record the time
of one iteration rather than average many. Some of the original benching code is left commented to be used when it works
again or by others.