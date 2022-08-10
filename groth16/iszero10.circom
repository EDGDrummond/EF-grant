pragma circom 2.0.0;

include "../circomlib/circuits/comparators.circom";

template Main() {
    signal input in;
    signal output out[10];

    component isz[10];
    var i;

    for (i=0; i<10; i++) {
        isz[i] = IsZero();
        isz[i].in <== in;
        out[i] <== isz[i].out;
    }

}

component main = Main();
