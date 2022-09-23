#[macro_use]
extern crate criterion;
use criterion::{BenchmarkId, Criterion};

use halo2wrong::{
    curves::{bn256::Fr as BnScalar, secp256k1::Secp256k1Affine as Secp256k1},
    halo2::{
        arithmetic::{CurveAffine, FieldExt},
        circuit::{Layouter, SimpleFloorPlanner, Value},
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
    },
};
use rand_core::OsRng;

use ecc::{integer::Range, EccConfig, GeneralEccChip};
use ecdsa::ecdsa::{AssignedEcdsaSig, AssignedPublicKey, EcdsaChip};
use group::{ff::Field, Curve, Group};
use integer::IntegerInstructions;
use maingate::{
    big_to_fe, fe_to_big, mock_prover_verify, MainGate, MainGateConfig, RangeChip, RangeConfig,
    RangeInstructions, RegionCtx,
};
use std::marker::PhantomData;

const BIT_LEN_LIMB: usize = 68;
const NUMBER_OF_LIMBS: usize = 4;

fn criterion_benchmark(c: &mut Criterion) {
    #[derive(Clone, Debug)]
    struct TestCircuitEcdsaVerifyConfig {
        main_gate_config: MainGateConfig,
        range_config: RangeConfig,
    }

    impl TestCircuitEcdsaVerifyConfig {
        pub fn new<C: CurveAffine, N: FieldExt>(meta: &mut ConstraintSystem<N>) -> Self {
            let (rns_base, rns_scalar) =
                GeneralEccChip::<C, N, NUMBER_OF_LIMBS, BIT_LEN_LIMB>::rns();
            let main_gate_config = MainGate::<N>::configure(meta);
            let mut overflow_bit_lens: Vec<usize> = vec![];
            overflow_bit_lens.extend(rns_base.overflow_lengths());
            overflow_bit_lens.extend(rns_scalar.overflow_lengths());
            let composition_bit_lens = vec![BIT_LEN_LIMB / NUMBER_OF_LIMBS];

            let range_config = RangeChip::<N>::configure(
                meta,
                &main_gate_config,
                composition_bit_lens,
                overflow_bit_lens,
            );
            TestCircuitEcdsaVerifyConfig {
                main_gate_config,
                range_config,
            }
        }

        pub fn ecc_chip_config(&self) -> EccConfig {
            EccConfig::new(self.range_config.clone(), self.main_gate_config.clone())
        }

        pub fn config_range<N: FieldExt>(
            &self,
            layouter: &mut impl Layouter<N>,
        ) -> Result<(), Error> {
            let range_chip = RangeChip::<N>::new(self.range_config.clone());
            range_chip.load_table(layouter)?;

            Ok(())
        }
    }

    #[derive(Default, Clone)]
    struct TestCircuitEcdsaVerify<E: CurveAffine, N: FieldExt> {
        public_key: Value<E>,
        signature: Value<(E::Scalar, E::Scalar)>,
        msg_hash: Value<E::Scalar>,

        aux_generator: E,
        window_size: usize,
        _marker: PhantomData<N>,
    }

    impl<E: CurveAffine, N: FieldExt> Circuit<N> for TestCircuitEcdsaVerify<E, N> {
        type Config = TestCircuitEcdsaVerifyConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<N>) -> Self::Config {
            TestCircuitEcdsaVerifyConfig::new::<E, N>(meta)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<N>,
        ) -> Result<(), Error> {
            let mut ecc_chip = GeneralEccChip::<E, N, NUMBER_OF_LIMBS, BIT_LEN_LIMB>::new(
                config.ecc_chip_config(),
            );

            layouter.assign_region(
                || "assign aux values",
                |region| {
                    let offset = 0;
                    let ctx = &mut RegionCtx::new(region, offset);

                    ecc_chip.assign_aux_generator(ctx, Value::known(self.aux_generator))?;
                    ecc_chip.assign_aux(ctx, self.window_size, 1)?;
                    Ok(())
                },
            )?;

            let ecdsa_chip = EcdsaChip::new(ecc_chip.clone());
            let scalar_chip = ecc_chip.scalar_field_chip();

            layouter.assign_region(
                || "region 0",
                |region| {
                    let offset = 0;
                    let ctx = &mut RegionCtx::new(region, offset);

                    let r = self.signature.map(|signature| signature.0);
                    let s = self.signature.map(|signature| signature.1);
                    let integer_r = ecc_chip.new_unassigned_scalar(r);
                    let integer_s = ecc_chip.new_unassigned_scalar(s);
                    let msg_hash = ecc_chip.new_unassigned_scalar(self.msg_hash);

                    let r_assigned =
                        scalar_chip.assign_integer(ctx, integer_r, Range::Remainder)?;
                    let s_assigned =
                        scalar_chip.assign_integer(ctx, integer_s, Range::Remainder)?;
                    let sig = AssignedEcdsaSig {
                        r: r_assigned,
                        s: s_assigned,
                    };

                    let pk_in_circuit = ecc_chip.assign_point(ctx, self.public_key)?;
                    let pk_assigned = AssignedPublicKey {
                        point: pk_in_circuit,
                    };
                    let msg_hash = scalar_chip.assign_integer(ctx, msg_hash, Range::Remainder)?;
                    ecdsa_chip.verify(ctx, &sig, &pk_assigned, &msg_hash)
                },
            )?;

            config.config_range(&mut layouter)?;

            Ok(())
        }
    }

    fn mod_n<C: CurveAffine>(x: C::Base) -> C::Scalar {
        let x_big = fe_to_big(x);
        big_to_fe(x_big)
    }

    fn run<C: CurveAffine, N: FieldExt>() -> (TestCircuitEcdsaVerify<C, N>, Vec<Vec<N>>) {
        let g = C::generator();

        // Generate a key pair
        let sk = <C as CurveAffine>::ScalarExt::random(OsRng);
        let public_key = (g * sk).to_affine();

        // Generate a valid signature
        // Suppose `m_hash` is the message hash
        let msg_hash = <C as CurveAffine>::ScalarExt::random(OsRng);

        // Draw randomness
        let k = <C as CurveAffine>::ScalarExt::random(OsRng);
        let k_inv = k.invert().unwrap();

        // Calculate `r`
        let r_point = (g * k).to_affine().coordinates().unwrap();
        let x = r_point.x();
        let r = mod_n::<C>(*x);

        // Calculate `s`
        let s = k_inv * (msg_hash + (r * sk));

        // Sanity check. Ensure we construct a valid signature. So lets verify it
        {
            let s_inv = s.invert().unwrap();
            let u_1 = msg_hash * s_inv;
            let u_2 = r * s_inv;
            let r_point = ((g * u_1) + (public_key * u_2))
                .to_affine()
                .coordinates()
                .unwrap();
            let x_candidate = r_point.x();
            let r_candidate = mod_n::<C>(*x_candidate);
            assert_eq!(r, r_candidate);
        }

        let aux_generator = C::CurveExt::random(OsRng).to_affine();
        let circuit = TestCircuitEcdsaVerify::<C, N> {
            public_key: Value::known(public_key),
            signature: Value::known((r, s)),
            msg_hash: Value::known(msg_hash),
            aux_generator,
            window_size: 2,
            ..Default::default()
        };
        let instance = vec![vec![]];
        assert_eq!(mock_prover_verify(&circuit, instance.clone()), Ok(()));

        (circuit, instance)
    }

    let k = 18;

    let (circuit, _) = run::<Secp256k1, BnScalar>();

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
                    create_proof::<KZGCommitmentScheme<Bn256>, ProverGWC<Bn256>, _, _, _, _>(
                        &params,
                        &pk,
                        &[circuit.clone()],
                        &[&[&[]]],
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
            &[circuit.clone()],
            &[&[&[]]],
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
                    &[&[&[]]],
                    &mut transcript.clone(),
                )
                .unwrap();
            });
        });
    }
    proof_verification.finish();

    // use crate::curves::bn256::Fr as BnScalar;
    // use crate::curves::pasta::{Fp as PastaFp, Fq as PastaFq};
    // use crate::curves::secp256k1::Secp256k1Affine as Secp256k1;
    // run::<Secp256k1, BnScalar>();
    // run::<Secp256k1, PastaFp>();
    // run::<Secp256k1, PastaFq>();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
