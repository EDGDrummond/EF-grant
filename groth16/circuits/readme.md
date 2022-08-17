Here we want to bench circom at various circuit sizes (just filled with the iszero function). There are shell
files prepared to do all of this, so benching just requires running those shell files that run it all and 
delete created files once done.


NOTE: Benching was achieved by installing bench via Haskell's stack tool.
In order to run the benchmarking choose the circuit size you want to bench by inserting the correft power of 2
in the `iszero_generic.circom` file and then (assuming you're in the 'benchmarks' folder) run:
`bash bench-generic-time.sh`
where the _ represents the size of the circuit. For example 3 would be a circuit of size 10^3

NOTE: Due to the large file sizes of the powers of tau, these had to be saved via git Large 
File Storage (LFS)


IsZero() at 2^14 constraints:
circuit compilation:    real    0m1.273s   user    0m0.962s
witness generation:     real    0m0.044s   user    0m0.042s
circuit set-up:         real    0m15.128s  user    0m56.053s
proof genertion:        real    0m1.677s   user    0m6.275s
proof verification:     real    0m0.587s   user    0m0.986s

IsZero() at 2^18 constraints:
circuit compilation:    real    0m21.078s   user    0m16.709s
witness generation:     real    0m0.248s    user    0m0.167s
circuit set-up:         real    2m28.676s   user    15m14.167s
proof genertion:        real    0m18.322s   user    1m23.522s
proof verification:     real    0m2.401s    user    0m3.222s