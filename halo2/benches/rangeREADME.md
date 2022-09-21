This range gadget was taken from https://github.com/privacy-scaling-explorations/halo2wrong/blob/master/maingate/src/range.rs

Run the command:

`cargo criterion --bench range`

One iteration of range gadget takes at least 2^(LIMB_BIT_LEN+1) constraints to run, so if we want a circuit
with 2^k constraints we should repeat this gadget ~2^(k-(LIMB_BIT_LEN+1)) times. Though this isn't perfect;
manually checking revealed that for 2^14 constraints we can run the gadget 2^7 times when LIMB_BIT_LEN=8.
Similarly under the same conditions, for 2^18 constraints 2^11 repeats of the gadget is allowed

Range() at 2^14 constraints:
Verifier Key Generation:   [340.69 ms 345.90 ms 350.69 ms]
Prover Key Generation:     [211.37 ms 214.15 ms 217.37 ms]
Proof Generation:          [1.2038 s 1.2300 s 1.2556 s]   
Proof Verification:        [4.1800 ms 4.3821 ms 4.5567 ms]  

Range() at 2^18 constraints:
Verifier Key Generation:   [3.5611 s 3.5867 s 3.6135 s]
Prover Key Generation:     [4.1551 s 4.2227 s 4.3289 s]
Proof Generation:          [16.924 s 17.290 s 17.699 s]   
Proof Verification:        [11.959 ms 12.798 ms 13.756 ms] 

Note: these bench figures were recorded on a 2020 M1 MacBook Air with 16GB RAM