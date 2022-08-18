pragma circom 2.0.0;

include "../iszero.circom";

template IsZero_Generic(k) {
    signal input in;
    signal output out[k];

    component isz[k];
    var i;

    for (i=0; i<k; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = IsZero_Generic(2**10);