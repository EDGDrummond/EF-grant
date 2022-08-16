pragma circom 2.0.0;

include "../iszero.circom";

template IsZero_3() {
    signal input in;
    signal output out[500];

    component isz[500];
    var i;

    for (i=0; i<500; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = IsZero_3();