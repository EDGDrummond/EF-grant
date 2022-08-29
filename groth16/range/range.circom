pragma circom 2.0.0;

// Converts `in` into bit format, where `n` is the largest bit index of `in`
template Num2Bits(n) {
    signal input in;
    signal output out[n];
    var lc1=0;

    var e2=1;
    for (var i = 0; i<n; i++) {
        out[i] <-- (in >> i) & 1;
        out[i] * (out[i] -1 ) === 0;
        lc1 += out[i] * e2;
        e2 = e2+e2;
    }

    lc1 === in;
}

// Checks whether `in[0]` is less than `in[1], returning 1 if true, else 0
// in[0] = 1<<n & in[1] = 0, then returns 1?!
template LessThan(n) {
    assert(n <= 252);
    signal input in[2];
    signal output out;

    component n2b = Num2Bits(n+1);

    n2b.in <== in[0]+ (1<<n) - in[1];

    out <== 1-n2b.out[n];
    log(out);
}

component main = LessThan(6);