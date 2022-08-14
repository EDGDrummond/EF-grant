pragma circom 2.0.0;

include "../iszero.circom";

template IsZero_1() {
    signal input in;
    signal output out[5];

    component isz[5];
    var i;

    for (i=0; i<5; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = IsZero_1();
