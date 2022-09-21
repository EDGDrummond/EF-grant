#[macro_use]
extern crate criterion;
use criterion::{BenchmarkId, Criterion};

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Layouter, SimpleFloorPlanner, Value},
    halo2curves::bn256::{Bn256, Fr as Fp, G1Affine},
    plonk::*,
    poly::{
        commitment::ParamsProver,
        kzg::commitment::{KZGCommitmentScheme, ParamsKZG},
        kzg::multiopen::{ProverGWC, VerifierGWC},
        kzg::strategy::SingleStrategy,
        Rotation,
    },
    transcript::{
        Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer,
    },
};
use rand_core::OsRng;

use std::marker::PhantomData;
use std::ops::Neg;

fn criterion_benchmark(c: &mut Criterion) {
    #[derive(Clone)]
    struct PlonkConfig {
        a: Column<Advice>,
        b: Column<Advice>,
        c: Column<Advice>,

        sa: Column<Fixed>,
        sb: Column<Fixed>,
        sc: Column<Fixed>,
        sm: Column<Fixed>,
    }

    trait StandardCs<FF: FieldExt> {
        fn raw_multiply<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Value<(Assigned<FF>, Assigned<FF>, Assigned<FF>)>;
        fn raw_add<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Value<(Assigned<FF>, Assigned<FF>, Assigned<FF>)>;
        fn copy(&self, layouter: &mut impl Layouter<FF>, a: Cell, b: Cell) -> Result<(), Error>;
    }

    #[derive(Clone)]
    struct MyCircuit<F: FieldExt> {
        a: Value<F>,
        k: u32,
    }

    struct StandardPlonk<F: FieldExt> {
        config: PlonkConfig,
        _marker: PhantomData<F>,
    }

    impl<FF: FieldExt> StandardPlonk<FF> {
        fn new(config: PlonkConfig) -> Self {
            StandardPlonk {
                config,
                _marker: PhantomData,
            }
        }
    }

    impl<FF: FieldExt> StandardCs<FF> for StandardPlonk<FF> {
        fn raw_multiply<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            mut f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Value<(Assigned<FF>, Assigned<FF>, Assigned<FF>)>,
        {
            layouter.assign_region(
                || "mul",
                |mut region| {
                    let mut values = None;
                    let lhs = region.assign_advice(
                        || "lhs",
                        self.config.a,
                        0,
                        || {
                            values = Some(f());
                            values.unwrap().map(|v| v.0)
                        },
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || values.unwrap().map(|v| v.1),
                    )?;

                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || values.unwrap().map(|v| v.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Value::known(FF::zero()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Value::known(FF::zero()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Value::known(FF::one()))?;
                    region.assign_fixed(
                        || "a * b",
                        self.config.sm,
                        0,
                        || Value::known(FF::one()),
                    )?;

                    Ok((lhs.cell(), rhs.cell(), out.cell()))
                },
            )
        }

        fn raw_add<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            mut f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Value<(Assigned<FF>, Assigned<FF>, Assigned<FF>)>,
        {
            layouter.assign_region(
                || "mul",
                |mut region| {
                    let mut values = None;
                    let lhs = region.assign_advice(
                        || "lhs",
                        self.config.a,
                        0,
                        || {
                            values = Some(f());
                            values.unwrap().map(|v| v.0)
                        },
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || values.unwrap().map(|v| v.1),
                    )?;

                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || values.unwrap().map(|v| v.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Value::known(FF::one()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Value::known(FF::one()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Value::known(FF::one()))?;
                    region.assign_fixed(
                        || "a * b",
                        self.config.sm,
                        0,
                        || Value::known(FF::zero()),
                    )?;

                    Ok((lhs.cell(), rhs.cell(), out.cell()))
                },
            )
        }

        fn copy(
            &self,
            layouter: &mut impl Layouter<FF>,
            left: Cell,
            right: Cell,
        ) -> Result<(), Error> {
            layouter.assign_region(
                || "copy",
                |mut region| {
                    region.constrain_equal(left, right)?;
                    region.constrain_equal(left, right)
                },
            )
        }
    }

    impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
        type Config = PlonkConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self {
                a: Value::unknown(),
                k: self.k,
            }
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> PlonkConfig {
            let a = meta.advice_column();
            let b = meta.advice_column();
            let c = meta.advice_column();

            meta.enable_equality(a);
            meta.enable_equality(b);
            meta.enable_equality(c);

            let sm = meta.fixed_column();
            let sa = meta.fixed_column();
            let sb = meta.fixed_column();
            let sc = meta.fixed_column();

            meta.create_gate("mini plonk", |meta| {
                let a = meta.query_advice(a, Rotation::cur());
                let b = meta.query_advice(b, Rotation::cur());
                let c = meta.query_advice(c, Rotation::cur());

                let sa = meta.query_fixed(sa, Rotation::cur());
                let sb = meta.query_fixed(sb, Rotation::cur());
                let sc = meta.query_fixed(sc, Rotation::cur());
                let sm = meta.query_fixed(sm, Rotation::cur());

                vec![a.clone() * sa + b.clone() * sb + a * b * sm + (c * sc * (-F::one()))]
            });

            PlonkConfig {
                a,
                b,
                c,
                sa,
                sb,
                sc,
                sm,
                // perm,
            }
        }

        fn synthesize(
            &self,
            config: PlonkConfig,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            let cs = StandardPlonk::new(config);

            for _ in 0..(((1 << self.k) / 3) - 2) {
                let a: Value<Assigned<_>> = self.a.into();
                // such that a * inv_neg = -1
                let inv_neg: Value<Assigned<_>> = a.clone().invert().neg();
                let one = Assigned::from(F::one());
                let zero = Assigned::Zero;

                // first gate, the mul gate
                let (_a1, b1, c1) = cs.raw_multiply(&mut layouter, || {
                    a.zip(inv_neg).map(|(a, inv_neg)| (inv_neg, a, inv_neg * a))
                })?;

                // addition gate, where we are going to create out
                let (a2, _b2, c2) = cs.raw_add(&mut layouter, || {
                    a.zip(inv_neg)
                        .map(|(a, inv_neg)| (inv_neg * a, one, one + (inv_neg * a)))
                })?;

                // final gate, the second multiplication gate
                let (a3, b3, _c3) = cs.raw_multiply(&mut layouter, || {
                    a.zip(inv_neg)
                        .map(|(a, inv_neg)| (one + (inv_neg * a), a, zero))
                })?;

                // copy constraints
                cs.copy(&mut layouter, c1, a2).unwrap();
                cs.copy(&mut layouter, c2, a3).unwrap();
                cs.copy(&mut layouter, b1, b3).unwrap();
            }

            Ok(())
        }
    }

    // Initialise parameters for the circuit
    let a_value = Value::known(Fp::from(2));

    // Initialise the benching parameter
    let k = 10;

    // Prepare benching for verifier key generation
    let mut verifier_key_generation = c.benchmark_group("IsZero Verifier Key Generation");
    verifier_key_generation.sample_size(10);
    {
        let empty_circuit: MyCircuit<Fp> = MyCircuit {
            a: Value::unknown(),
            k,
        };
        let params: ParamsKZG<Bn256> = ParamsKZG::<Bn256>::new(k);

        verifier_key_generation.bench_with_input(
            BenchmarkId::from_parameter(k),
            &(&params, &empty_circuit),
            |b, &(params, empty_circuit)| {
                b.iter(|| {
                    keygen_vk(params, empty_circuit).expect("keygen_vk should not fail");
                });
            },
        );
    }
    verifier_key_generation.finish();

    // Prepare benching for prover key generation
    let mut prover_key_generation = c.benchmark_group("IsZero Prover Key Generation");
    prover_key_generation.sample_size(10);
    {
        let empty_circuit: MyCircuit<Fp> = MyCircuit {
            a: Value::unknown(),
            k,
        };
        let params: ParamsKZG<Bn256> = ParamsKZG::<Bn256>::new(k);
        let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");

        prover_key_generation.bench_with_input(
            BenchmarkId::from_parameter(k),
            &(&params, &empty_circuit, &vk),
            |b, &(params, empty_circuit, vk)| {
                b.iter(|| {
                    keygen_pk(params, vk.clone(), empty_circuit)
                        .expect("keygen_pk should not fail");
                });
            },
        );
    }
    prover_key_generation.finish();

    // Prepare benching for proof generation
    let mut proof_generation = c.benchmark_group("IsZero Proof Generation");
    proof_generation.sample_size(10);
    {
        let circuit: MyCircuit<Fp> = MyCircuit { a: a_value, k };
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
    let mut proof_verification = c.benchmark_group("IsZero Proof Verification");
    proof_verification.sample_size(10);
    {
        let empty_circuit: MyCircuit<Fp> = MyCircuit {
            a: Value::unknown(),
            k,
        };
        let params: ParamsKZG<Bn256> = ParamsKZG::new(k);
        let strategy = SingleStrategy::new(&params);
        let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");
        let circuit: MyCircuit<Fp> = MyCircuit { a: a_value, k };
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
