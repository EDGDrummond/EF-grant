Commands to run and verify halo circuits in this folder. The circuit to be benched (alongside its name)
should be seen in the cargo file. Insert the desired circuit sizes to be benched via the k_range parameter
in the benching file. Then we can run the command below with the relevant benching target:

cargo criterion --bench iszero

Alternatively run the below command to bench all

cargo criterion --benches

NOTE: prover key geneartion conducts verifier key generation as part of its time

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