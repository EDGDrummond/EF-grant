This range gadget was taken from https://github.com/privacy-scaling-explorations/halo2wrong/blob/master/maingate/src/range.rs

Run the command:

`cargo criterion --bench ecdsa

One iteration of ECDSA signature verification on (Secp256k1, BnScalar) takes between 2^17 and 2^18 constraints.

ecdsa() at 2^18 constraints:
Verifier Key Generation:   [3.8276 s 3.8502 s 3.8724 s]
Prover Key Generation:     [4.4071 s 4.4747 s 4.5498 s]
Proof Generation:          [19.086 s 19.146 s 19.222 s]  
Proof Verification:        [11.081 ms 11.122 ms 11.169 ms]

Note: these bench figures were recorded on a 2020 M1 MacBook Air with 16GB RAM