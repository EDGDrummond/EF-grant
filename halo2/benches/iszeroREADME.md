In order to conduct an `iszero(in)` (wheere the output `out` is 1 if true and 0 otherwise), we need to satisfy 2 constraints:
- First define `inv` as the inverse of `in`
1. `1 - (inv * in) = out`
2. `in * out = 0`

To account for the negative sign we instead use `inv_neg` such that `in * inv_neg = -1`, converting the first constraint to
1. `1 + (ing_neg * in)`

These 2 constriants are defined by 3 gates. The second constraint is one gate and the first is split into:
a. `in * inv_neg = int`
b. `1 + int = out`


Run the command:

`cargo criterion --bench iszero`

IsZero() at 2^14 constraints:
Verifier Key Generation:   [168.32 ms 168.94 ms 169.75 ms]
Prover Key Generation:     [302.82 ms 306.60 ms 312.28 ms]
Proof Generation:          [694.85 ms 706.39 ms 719.67 ms] 
Proof Verification:        [2.6609 ms 2.6653 ms 2.6712 ms]  

IsZero() at 2^18 constraints:
Verifier Key Generation:   [1.8569 s 1.8852 s 1.9184 s]
Prover Key Generation:     [3.8963 s 3.9675 s 4.0449 s]
Proof Generation:          [8.4708 s 8.5382 s 8.6053 s] 
Proof Verification:        [2.6158 ms 2.6222 ms 2.6264 ms]  