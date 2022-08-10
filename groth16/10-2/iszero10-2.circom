pragma circom 2.0.0;

include "../../circomlib/circuits/comparators.circom";

template IsZero2() {
    signal input in;
    signal output out[100];

    component isz[100];
    var i;

    for (i=0; i<100; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = IsZero2();