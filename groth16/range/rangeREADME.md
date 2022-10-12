Here we want to bench circom at various circuit sizes. There are shell files prepared to do all of this,
so benching just requires running those shell files that run it all and delete created files once done.

In order to run the benchmarking choose the circuit size you want to bench by inserting the correct power of 2
in the `range.circom` file and then (assuming you're in the 'benchmarks' folder) run:
`bash bench-range.sh`

TO-DO: When using the time command for circuit set-up, the 'user' time is more than the 'real' time.
The former should always be less than th1e latter, something glitched. Probably because we are timing 3 
commands and not 1. Need to fix - also need to bench several rather than time 1


Range() at 2^14 constraints:
circuit compilation:    real    0m0.636s   user    0m0.459s
witness generation:     real    0m0.115s   user    0m0.039s
circuit set-up:         real    0m43.092s  user    1m8.619s
proof generation:       real    0m0.955s   user    0m2.581s
proof verification:     real    0m0.438s   user    0m0.818s

Range() at 2^18 constraints:
circuit compilation:    real    0m9.694s    user    0m7.302s
witness generation:     real    0m0.106s    user    0m0.101s
circuit set-up:         real    3m59.784s   user    22m35.661s
proof generation:       real    0m11.843s   user    0m44.485s
proof verification:     real    0m0.459s    user    0m0.820s