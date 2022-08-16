Here we want to bench circom at various circuit sizes (just filled with the iszero function). There are shell
files prepared to do all of this, so benching just requires running those shell files that run it all and 
delete created files once done. Below is a description of the steps we need to walk through to bench


NOTE: Benching was achieved by installing bench via Haskell's stack tool.
In order to run the benchmarking choose the circuit size you want to bench and run
`bash bench_.sh`
where the _ represents the size of the circuit. For example 3 would be a circuit of size 10^3

NOTE: if you wish to learn the size of any of the circuits, simply run (inserting the desired path)
`time circom ../4/iszero4.circom --r1cs`
