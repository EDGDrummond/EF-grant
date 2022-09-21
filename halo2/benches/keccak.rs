#[macro_use]
extern crate criterion;
use criterion::{BenchmarkId, Criterion};

use halo2wrong::halo2::{
    halo2curves::bn256::{Bn256, G1Affine},
    plonk::*,
    poly::{
        commitment::ParamsProver,
        kzg::{
            commitment::{KZGCommitmentScheme, ParamsKZG},
            multiopen::{ProverGWC, VerifierGWC},
            strategy::SingleStrategy,
        },
    },
    transcript::{
        Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer,
    },
};
use rand_core::OsRng;

use zkevm_circuits::keccak_circuit::keccak_bit::KeccakBitCircuit;

fn criterion_benchmark(c: &mut Criterion) {
    let k = 8;
    let inputs = vec![
        vec![],
        (0u8..1).collect::<Vec<_>>(),
        (0u8..135).collect::<Vec<_>>(),
        (0u8..136).collect::<Vec<_>>(),
        (0u8..200).collect::<Vec<_>>(),
    ];

    let mut circuit = KeccakBitCircuit::new(2usize.pow(k));
    circuit.generate_witness(&inputs);

    // Prepare benching for verifier key generation
    let mut verifier_key_generation = c.benchmark_group("ECDSA Verifier Key Generation");
    verifier_key_generation.sample_size(10);
    {
        let params: ParamsKZG<Bn256> = ParamsKZG::<Bn256>::new(k);

        verifier_key_generation.bench_with_input(
            BenchmarkId::from_parameter(k),
            &(&params, &circuit),
            |b, &(params, circuit)| {
                b.iter(|| {
                    keygen_vk(params, circuit).expect("keygen_vk should not fail");
                });
            },
        );
    }
    verifier_key_generation.finish();

    // Prepare benching for prover key generation
    let mut prover_key_generation = c.benchmark_group("ECDSA Prover Key Generation");
    prover_key_generation.sample_size(10);
    {
        let params: ParamsKZG<Bn256> = ParamsKZG::<Bn256>::new(k);
        let vk = keygen_vk(&params, &circuit).expect("keygen_vk should not fail");

        prover_key_generation.bench_with_input(
            BenchmarkId::from_parameter(k),
            &(&params, &circuit, &vk),
            |b, &(params, circuit, vk)| {
                b.iter(|| {
                    keygen_pk(params, vk.clone(), circuit).expect("keygen_pk should not fail");
                });
            },
        );
    }
    prover_key_generation.finish();

    // Prepare benching for proof generation
    let mut proof_generation = c.benchmark_group("ECDSA Proof Generation");
    proof_generation.sample_size(10);
    {
        let mut circuit = KeccakBitCircuit::new(2usize.pow(k));
        circuit.generate_witness(&inputs);
        let params: ParamsKZG<Bn256> = ParamsKZG::<Bn256>::new(k);
        let vk = keygen_vk(&params, &circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &circuit).expect("keygen_pk should not fail");
        let mut transcript: Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>> =
            Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);

        proof_generation.bench_with_input(
            BenchmarkId::from_parameter(k),
            &(&params, &pk),
            |b, &(params, pk)| {
                b.iter(|| {
                    let mut circuit = KeccakBitCircuit::new(2usize.pow(k));
                    circuit.generate_witness(&inputs);
                    create_proof::<KZGCommitmentScheme<Bn256>, ProverGWC<Bn256>, _, _, _, _>(
                        &params,
                        &pk,
                        &[circuit],
                        &[&[]],
                        OsRng,
                        &mut transcript,
                    )
                    .expect("proof generation should not fail")
                });
            },
        );
    }
    proof_generation.finish();

    // Prepare benching for proof verification
    let mut proof_verification = c.benchmark_group("ECDSA Proof Verification");
    proof_verification.sample_size(10);
    {
        let params: ParamsKZG<Bn256> = ParamsKZG::new(k);
        let strategy = SingleStrategy::new(&params);
        let vk = keygen_vk(&params, &circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &circuit).expect("keygen_pk should not fail");
        let mut transcript: Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>> =
            Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        create_proof::<KZGCommitmentScheme<Bn256>, ProverGWC<Bn256>, _, _, _, _>(
            &params,
            &pk,
            &[circuit],
            &[&[]],
            OsRng,
            &mut transcript,
        )
        .expect("proof generation should not fail");
        let proof = transcript.finalize();
        let transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);

        proof_verification.bench_with_input(BenchmarkId::from_parameter(k), &(), |b, ()| {
            b.iter(|| {
                verify_proof::<_, VerifierGWC<Bn256>, _, _, _>(
                    &params,
                    pk.get_vk(),
                    strategy.clone(),
                    &[&[]],
                    &mut transcript.clone(),
                )
                .unwrap();
            });
        });
    }
    proof_verification.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
