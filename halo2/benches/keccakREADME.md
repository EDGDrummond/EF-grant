This keccak gadget was taken from https://github.com/privacy-scaling-explorations/zkevm-circuits

As per the explainer in this hackmd: https://hackmd.io/NaTuIvmaQCybaOYgd-DG1Q,
keccak-bit is the version that works on bits, whilst keccak-packed is the version where multiple bits are packed
into a single field element.
The bit version requires around 2000 columns, but most os the entries are 0 or 1 so the
MSM calculation doesn't blow up too much. There are also not too many lookups involved.
The packed version in comparison requires around 800 columns, but it also requires around 500 lookups
per row. This version is a prediction that in future lookups will get much cheaper and make this version a 
better choice. For now you can see the resulting differences in the benching times below.

In keccak-bit a new row in `inputs` will add an additional `KeccakRow`, and this row will manage to absorb 
135 of the bytes in that input, so going over would require another such row.

Run one of the commands:

`cargo criterion --bench keccak_bit`
or 
`cargo criterion --bench keccak_packed`

One iteration of range gadget takes at least 2^8 constraints to run.

keccak_bit() at 2^8 constraints:
Verifier Key Generation:   [85.005 ms 85.570 ms 86.122 ms]
Prover Key Generation:     [1.2242 s 1.2252 s 1.2262 s]
Proof Generation:          [5.0634 s 5.1126 s 5.1814 s]  
Proof Verification:        [69.283 ms 69.360 ms 69.468 ms] 

keccak_packed() at 2^9 constraints:
Verifier Key Generation:   [32.616 ms 32.714 ms 32.895 ms]
Prover Key Generation:     [56.087 ms 56.262 ms 56.683 ms]
Proof Generation:          [20.494 s 20.555 s 20.614 s] 
Proof Verification:        [134.07 ms 134.80 ms 135.68 ms]

For comparison take a look at the relevant values for Vocdoni's circom keccak implementation
https://github.com/vocdoni/keccak256-circom

Note: these bench figures were recorded on a 2020 M1 MacBook Air with 16GB RAM