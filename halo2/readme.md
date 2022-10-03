Commands to run and verify halo circuits in this folder. The circuit to be benched (alongside its name)
should be seen in the cargo file. Insert the desired circuit sizes to be benched via the k_range parameter
in the benching file. Then we can run the command below with the relevant benching target, for example:

`cargo criterion --bench iszero`

Alternatively run one of the two below commands to bench all

`cargo bench`
`cargo criterion --benches`

NOTE: prover key generation conducts verifier key generation as part of its time