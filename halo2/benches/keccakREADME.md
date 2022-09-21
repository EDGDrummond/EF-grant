This keccak gadget was taken from https://github.com/privacy-scaling-explorations/zkevm-circuits

Run the command:

`cargo criterion --bench keccak`

One iteration of range gadget takes at least 2^8 constraints to run.

Keccak() at 2^8 constraints:
Verifier Key Generation:   [85.005 ms 85.570 ms 86.122 ms]
Prover Key Generation:     [1.2242 s 1.2252 s 1.2262 s]
Proof Generation:          [5.0634 s 5.1126 s 5.1814 s]  
Proof Verification:        [69.283 ms 69.360 ms 69.468 ms] 

Note: these bench figures were recorded on a 2020 M1 MacBook Air with 16GB RAM