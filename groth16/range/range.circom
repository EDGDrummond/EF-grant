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
}

// Repeat the less than gadget k times in order to get circuits of different size,
// where n representst the maximum bit index of values to be compared
template RepeatedLessThan(k, n) {
    signal input in[2];
    signal output out;

    component lt[k];

    for (var i = 0; i<k; i++) {
        lt[i] = LessThan(n);
        lt[i].in[0] <== in[0];
        lt[i].in[1] <== in[1];
    }

    out <== lt[0].out;
}

// 9 constraints per k value, so in order to get 2^m constraints set k = (2^m)/k
// (2^10)/9 = 113.666...
// (2^14)/9 = 1820.444...
// (2^18)/9 = 129127.111...
component main = RepeatedLessThan(1820,8);