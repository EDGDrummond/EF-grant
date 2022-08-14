pragma circom 2.0.0;

include "../iszero.circom";

template IsZero_2() {
    signal input in;
    signal output out[50];

    component isz[50];
    var i;

    for (i=0; i<50; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = IsZero_2();