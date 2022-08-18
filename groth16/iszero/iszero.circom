pragma circom 2.0.0;

template IsZero() {
    signal input in;
    signal output out;

    signal inv;

    inv <-- in!=0 ? 1/in : 0;

    out <== -in*inv +1;
    in*out === 0;
}

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

// Alter value in here to decide circuit size to bench
component main = IsZero_Generic(2**10);