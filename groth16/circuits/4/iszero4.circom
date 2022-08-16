pragma circom 2.0.0;

include "../iszero.circom";

template IsZero_4() {
    signal input in;
    signal output out[5000];

    component isz[5000];
    var i;

    for (i=0; i<5000; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = IsZero_4();