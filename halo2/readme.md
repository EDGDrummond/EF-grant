Commands to run and verify halo circuits in this folder. The circuit to benched (alongside is name)
should be seen in the cargo file. Then we can run the command below with the relevant benching target

cargo criterion --bench iszero2

Alternatively run the below command to bench all

cargo criterion --benches