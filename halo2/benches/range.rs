#[macro_use]
extern crate criterion;
use criterion::{BenchmarkId, Criterion};

use halo2::{AssignedValue, MainGate, MainGateConfig, MainGateInstructions, Term};
use halo2wrong::{
    halo2::{
        arithmetic::FieldExt,
        circuit::{Chip, Layouter, SimpleFloorPlanner, Value},
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
    },
    utils::decompose,
    RegionCtx,
};
use num_integer::Integer;
use rand_core::OsRng;
use std::collections::BTreeMap;

/// Maximum number of cells in one line enabled with composition selector
pub const NUMBER_OF_LOOKUP_LIMBS: usize = 4;

fn criterion_benchmark(c: &mut Criterion) {
    #[derive(Clone, Debug, Eq, PartialEq, Hash)]
    struct TableConfig {
        selector: Selector,
        column: TableColumn,
    }

    impl TableConfig {}

    /// Range gate configuration
    #[derive(Clone, Debug)]
    pub struct RangeConfig {
        main_gate_config: MainGateConfig,
        composition_tables: BTreeMap<usize, TableConfig>,
        overflow_tables: BTreeMap<usize, TableConfig>,
    }

    /// ['RangeChip'] applies binary range constraints
    #[derive(Clone, Debug)]
    pub struct RangeChip<F: FieldExt> {
        config: RangeConfig,
        main_gate: MainGate<F>,
        bases: BTreeMap<usize, Vec<F>>,
    }

    impl<F: FieldExt> RangeChip<F> {
        fn main_gate(&self) -> &MainGate<F> {
            &self.main_gate
        }
    }

    impl<F: FieldExt> Chip<F> for RangeChip<F> {
        type Config = RangeConfig;
        type Loaded = ();
        fn config(&self) -> &Self::Config {
            &self.config
        }
        fn loaded(&self) -> &Self::Loaded {
            &()
        }
    }

    /// Generic chip interface for bitwise ranging values
    pub trait RangeInstructions<F: FieldExt>: Chip<F> {
        /// Assigns new witness
        fn assign(
            &self,
            ctx: &mut RegionCtx<'_, F>,
            unassigned: Value<F>,
            limb_bit_len: usize,
            bit_len: usize,
        ) -> Result<AssignedValue<F>, Error>;

        /// Decomposes and assign new witness
        fn decompose(
            &self,
            ctx: &mut RegionCtx<'_, F>,
            unassigned: Value<F>,
            limb_bit_len: usize,
            bit_len: usize,
        ) -> Result<(AssignedValue<F>, Vec<AssignedValue<F>>), Error>;

        /// Appends base limb length table in sythnesis time
        fn load_composition_tables(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error>;
        /// Appends shorter range tables in sythesis time
        fn load_overflow_tables(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error>;
    }

    impl<F: FieldExt> RangeInstructions<F> for RangeChip<F> {
        fn assign(
            &self,
            ctx: &mut RegionCtx<'_, F>,
            unassigned: Value<F>,
            limb_bit_len: usize,
            bit_len: usize,
        ) -> Result<AssignedValue<F>, Error> {
            let (assigned, _) = self.decompose(ctx, unassigned, limb_bit_len, bit_len)?;
            Ok(assigned)
        }

        fn decompose(
            &self,
            ctx: &mut RegionCtx<'_, F>,
            unassigned: Value<F>,
            limb_bit_len: usize,
            bit_len: usize,
        ) -> Result<(AssignedValue<F>, Vec<AssignedValue<F>>), Error> {
            let (number_of_limbs, overflow_bit_len) = bit_len.div_rem(&limb_bit_len);

            let number_of_limbs = number_of_limbs + if overflow_bit_len > 0 { 1 } else { 0 };
            let decomposed = unassigned
                .map(|unassigned| decompose(unassigned, number_of_limbs, limb_bit_len))
                .transpose_vec(number_of_limbs);

            let terms: Vec<Term<F>> = decomposed
                .into_iter()
                .zip(self.bases(limb_bit_len))
                .map(|(limb, base)| Term::Unassigned(limb, *base))
                .collect();

            let composition_table =
                self.config
                    .composition_tables
                    .get(&limb_bit_len)
                    .expect(&format!(
                        "composition table is not set, bit lenght: {}",
                        limb_bit_len,
                    ));
            self.main_gate()
                .decompose(ctx, &terms[..], F::zero(), |is_last| {
                    if is_last && overflow_bit_len != 0 {
                        let overflow_table = self
                            .config
                            .overflow_tables
                            .get(&overflow_bit_len)
                            .expect(&format!(
                                "overflow table is not set, bit lenght: {}",
                                overflow_bit_len
                            ));
                        vec![composition_table.selector, overflow_table.selector]
                    } else {
                        vec![composition_table.selector]
                    }
                })
        }

        fn load_composition_tables(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
            for (bit_len, config) in self.config.composition_tables.iter() {
                let table_values: Vec<F> = (0..1 << bit_len).map(|e| F::from(e)).collect();
                layouter.assign_table(
                    || "",
                    |mut table| {
                        for (index, &value) in table_values.iter().enumerate() {
                            table.assign_cell(
                                || "composition table",
                                config.column,
                                index,
                                || Value::known(value),
                            )?;
                        }
                        Ok(())
                    },
                )?;
            }

            Ok(())
        }

        fn load_overflow_tables(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
            for (bit_len, config) in self.config.overflow_tables.iter() {
                let table_values: Vec<F> = (0..1 << bit_len).map(|e| F::from(e)).collect();
                layouter.assign_table(
                    || "",
                    |mut table| {
                        for (index, &value) in table_values.iter().enumerate() {
                            table.assign_cell(
                                || "composition table",
                                config.column,
                                index,
                                || Value::known(value),
                            )?;
                        }
                        Ok(())
                    },
                )?;
            }

            Ok(())
        }
    }

    impl<F: FieldExt> RangeChip<F> {
        /// Given config creates new chip that implements ranging
        pub fn new(config: RangeConfig) -> Self {
            let main_gate = MainGate::new(config.main_gate_config.clone());
            let bases = config
                .composition_tables
                .keys()
                .map(|&bit_len| {
                    let bases = (0..F::NUM_BITS as usize / bit_len)
                        .map(|i| F::from(2).pow(&[(bit_len * i) as u64, 0, 0, 0]))
                        .collect();
                    (bit_len, bases)
                })
                .collect();
            Self {
                config,
                main_gate,
                bases,
            }
        }

        /// Configures subset argument and returns the
        /// resuiting config
        pub fn configure(
            meta: &mut ConstraintSystem<F>,
            main_gate_config: &MainGateConfig,
            composition_bit_lens: Vec<usize>,
            overflow_bit_lens: Vec<usize>,
        ) -> RangeConfig {
            let mut overflow_bit_lens = overflow_bit_lens;
            overflow_bit_lens.sort_unstable();
            overflow_bit_lens.dedup();
            let overflow_bit_lens: Vec<usize> =
                overflow_bit_lens.into_iter().filter(|e| *e != 0).collect();

            let mut composition_bit_lens = composition_bit_lens;
            composition_bit_lens.sort_unstable();
            composition_bit_lens.dedup();
            let composition_bit_lens: Vec<usize> = composition_bit_lens
                .into_iter()
                .filter(|e| *e != 0)
                .collect();

            // TODO: consider for a generic MainGateConfig
            let (a, b, c, d) = (
                main_gate_config.a,
                main_gate_config.b,
                main_gate_config.c,
                main_gate_config.d,
            );

            macro_rules! meta_lookup {
                ($prefix:literal, $column:expr, $table_config:expr) => {
                    meta.lookup(concat!($prefix, "_", stringify!($column)), |meta| {
                        let exp = meta.query_advice($column, Rotation::cur());
                        let s = meta.query_selector($table_config.selector);
                        vec![(exp * s, $table_config.column)]
                    });
                };
            }

            let mut composition_tables = BTreeMap::<usize, TableConfig>::new();
            let mut overflow_tables = BTreeMap::<usize, TableConfig>::new();

            for bit_len in composition_bit_lens.iter() {
                let config = TableConfig {
                    selector: meta.complex_selector(),
                    column: meta.lookup_table_column(),
                };
                meta_lookup!("composition", a, config);
                meta_lookup!("composition", b, config);
                meta_lookup!("composition", c, config);
                meta_lookup!("composition", d, config);
                composition_tables.insert(*bit_len, config);
            }
            for bit_len in overflow_bit_lens.iter() {
                let config = TableConfig {
                    selector: meta.complex_selector(),
                    column: meta.lookup_table_column(),
                };

                meta_lookup!("overflow", a, config);
                overflow_tables.insert(*bit_len, config);
            }

            RangeConfig {
                main_gate_config: main_gate_config.clone(),
                composition_tables,
                overflow_tables,
            }
        }

        fn bases(&self, limb_bit_len: usize) -> &[F] {
            self.bases
                .get(&limb_bit_len)
                .unwrap_or_else(|| {
                    panic!("composition table is not set, bit lenght: {}", limb_bit_len)
                })
                .as_slice()
        }
    }

    #[derive(Clone, Debug)]
    struct TestCircuitConfig {
        range_config: RangeConfig,
    }

    impl TestCircuitConfig {
        fn new<F: FieldExt>(
            meta: &mut ConstraintSystem<F>,
            composition_bit_lens: Vec<usize>,
            overflow_bit_lens: Vec<usize>,
        ) -> Self {
            let main_gate_config = MainGate::<F>::configure(meta);

            let range_config = RangeChip::<F>::configure(
                meta,
                &main_gate_config,
                composition_bit_lens,
                overflow_bit_lens,
            );
            Self { range_config }
        }

        fn main_gate<F: FieldExt>(&self) -> MainGate<F> {
            MainGate::<F>::new(self.range_config.main_gate_config.clone())
        }

        fn range_chip<F: FieldExt>(&self) -> RangeChip<F> {
            RangeChip::<F>::new(self.range_config.clone())
        }
    }

    #[derive(Clone, Debug)]
    struct Input<F: FieldExt> {
        bit_len: usize,
        limb_bit_len: usize,
        value: Value<F>,
    }

    #[derive(Default, Clone, Debug)]
    struct TestCircuit<F: FieldExt> {
        inputs: Vec<Input<F>>,
        range_repeats: u32,
    }

    impl<F: FieldExt> TestCircuit<F> {
        fn composition_bit_lens() -> Vec<usize> {
            vec![8]
        }

        fn overflow_bit_lens() -> Vec<usize> {
            vec![3]
        }
    }

    impl<F: FieldExt> Circuit<F> for TestCircuit<F> {
        type Config = TestCircuitConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            let mut inputs = vec![];
            for i in 0..self.inputs.len() {
                inputs.push(Input {
                    bit_len: self.inputs[i].bit_len,
                    limb_bit_len: self.inputs[i].limb_bit_len,
                    value: Value::unknown(),
                })
            }
            TestCircuit {
                inputs,
                range_repeats: self.range_repeats,
            }
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            TestCircuitConfig::new(
                meta,
                Self::composition_bit_lens(),
                Self::overflow_bit_lens(),
            )
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            let range_chip = config.range_chip();
            let main_gate = config.main_gate();

            for _ in 0..self.range_repeats {
                layouter.assign_region(
                    || "region 0",
                    |region| {
                        let offset = 0;
                        let ctx = &mut RegionCtx::new(region, offset);

                        for input in self.inputs.iter() {
                            let value = input.value;
                            let limb_bit_len = input.limb_bit_len;
                            let bit_len = input.bit_len;

                            let a_0 = main_gate.assign_value(ctx, value)?;
                            let (a_1, decomposed) =
                                range_chip.decompose(ctx, value, limb_bit_len, bit_len)?;

                            main_gate.assert_equal(ctx, &a_0, &a_1)?;

                            let terms: Vec<Term<F>> = decomposed
                                .iter()
                                .zip(range_chip.bases(limb_bit_len))
                                .map(|(limb, base)| Term::Assigned(limb, *base))
                                .collect();
                            let a_1 = main_gate.compose(ctx, &terms[..], F::zero())?;
                            main_gate.assert_equal(ctx, &a_0, &a_1)?;
                        }

                        Ok(())
                    },
                )?;
            }

            range_chip.load_composition_tables(&mut layouter)?;
            range_chip.load_overflow_tables(&mut layouter)?;

            Ok(())
        }
    }

    const LIMB_BIT_LEN: usize = 8;
    const OVERFLOW_BIT_LEN: usize = 3;
    // Initialise the benching parameter, note that minimum k per iteration of range gadget is LIMB_BIT_LEN+1
    // Refer to readme for more detail
    let k = 12;
    let range_repeats = 2_u32.pow(3);

    let inputs: Vec<Input<Fp>> = (2..15)
        .map(|number_of_limbs| {
            let bit_len = LIMB_BIT_LEN * number_of_limbs + OVERFLOW_BIT_LEN;
            Input {
                value: Value::known(Fp::from_u128((1 << bit_len) - 1)),
                limb_bit_len: LIMB_BIT_LEN,
                bit_len,
            }
        })
        .collect();

    // Initialise circuit, and an empty version of it
    let circuit = TestCircuit::<Fp> {
        inputs: inputs.clone(),
        range_repeats: range_repeats,
    };
    let empty_circuit = circuit.clone().without_witnesses();

    // Prepare benching for verifier key generation
    let mut verifier_key_generation = c.benchmark_group("Range Verifier Key Generation");
    verifier_key_generation.sample_size(10);
    {
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
    let mut prover_key_generation = c.benchmark_group("Range Prover Key Generation");
    prover_key_generation.sample_size(10);
    {
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
    let mut proof_generation = c.benchmark_group("Range Proof Generation");
    proof_generation.sample_size(10);
    {
        let circuit = TestCircuit::<Fp> {
            inputs: inputs.clone(),
            range_repeats: range_repeats,
        };
        let params: ParamsKZG<Bn256> = ParamsKZG::<Bn256>::new(k);
        let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");
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
    let mut proof_verification = c.benchmark_group("Range Proof Verification");
    proof_verification.sample_size(10);
    {
        let circuit = TestCircuit::<Fp> {
            inputs: inputs.clone(),
            range_repeats: range_repeats,
        };
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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
