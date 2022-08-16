pragma circom 2.0.0;

include "../iszero.circom";

template IsZero_5() {
    signal input in;
    signal output out[50000];

    component isz[50000];
    var i;

    for (i=0; i<50000; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = IsZero_5();