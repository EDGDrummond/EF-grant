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
use std::collections::{BTreeMap, BTreeSet};

/// Maximum number of cells in one line enabled with composition selector
pub const NUMBER_OF_LOOKUP_LIMBS: usize = 4;

fn criterion_benchmark(c: &mut Criterion) {
    /// Range gate configuration
    #[derive(Clone, Debug)]
    pub struct RangeConfig {
        main_gate_config: MainGateConfig,
        bit_len_tag: BTreeMap<usize, usize>,
        t_tag: TableColumn,
        t_value: TableColumn,
        s_composition: Selector,
        tag_composition: Option<Column<Fixed>>,
        s_overflow: Option<Selector>,
        tag_overflow: Option<Column<Fixed>>,
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

        /// Load table in sythnesis time
        fn load_table(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error>;
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

            self.main_gate()
                .decompose(ctx, &terms[..], F::zero(), |ctx, is_last| {
                    let composition_tag = self
                        .config
                        .bit_len_tag
                        .get(&limb_bit_len)
                        .unwrap_or_else(|| {
                            panic!("composition table is not set, bit lenght: {limb_bit_len}")
                        });
                    ctx.enable(self.config.s_composition)?;
                    if let Some(tag_composition) = self.config.tag_composition {
                        ctx.assign_fixed(
                            || "tag_composition",
                            tag_composition,
                            F::from(*composition_tag as u64),
                        )?;
                    }

                    if is_last && overflow_bit_len != 0 {
                        let overflow_tag = self
                            .config
                            .bit_len_tag
                            .get(&overflow_bit_len)
                            .unwrap_or_else(|| {
                                panic!("overflow table is not set, bit lenght: {overflow_bit_len}")
                            });
                        ctx.enable(self.config.s_overflow.unwrap())?;
                        if let Some(tag_overflow) = self.config.tag_overflow {
                            ctx.assign_fixed(
                                || "tag_overflow",
                                tag_overflow,
                                F::from(*overflow_tag as u64),
                            )?;
                        }
                    }

                    Ok(())
                })
        }

        fn load_table(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
            layouter.assign_table(
                || "",
                |mut table| {
                    let mut offset = 0;

                    table.assign_cell(
                        || "table tag",
                        self.config.t_tag,
                        offset,
                        || Value::known(F::zero()),
                    )?;
                    table.assign_cell(
                        || "table value",
                        self.config.t_value,
                        offset,
                        || Value::known(F::zero()),
                    )?;
                    offset += 1;

                    for (bit_len, tag) in self.config.bit_len_tag.iter() {
                        let tag = F::from(*tag as u64);
                        let table_values: Vec<F> = (0..1 << bit_len).map(|e| F::from(e)).collect();
                        for value in table_values.iter() {
                            table.assign_cell(
                                || "table tag",
                                self.config.t_tag,
                                offset,
                                || Value::known(tag),
                            )?;
                            table.assign_cell(
                                || "table value",
                                self.config.t_value,
                                offset,
                                || Value::known(*value),
                            )?;
                            offset += 1;
                        }
                    }

                    Ok(())
                },
            )?;

            Ok(())
        }
    }

    impl<F: FieldExt> RangeChip<F> {
        /// Given config creates new chip that implements ranging
        pub fn new(config: RangeConfig) -> Self {
            let main_gate = MainGate::new(config.main_gate_config.clone());
            let bases = config
                .bit_len_tag
                .keys()
                .filter_map(|&bit_len| {
                    if bit_len == 0 {
                        None
                    } else {
                        let bases = (0..F::NUM_BITS as usize / bit_len)
                            .map(|i| F::from(2).pow(&[(bit_len * i) as u64, 0, 0, 0]))
                            .collect();
                        Some((bit_len, bases))
                    }
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
            let [composition_bit_lens, overflow_bit_lens] =
                [composition_bit_lens, overflow_bit_lens].map(|mut bit_lens| {
                    bit_lens.sort_unstable();
                    bit_lens.dedup();
                    bit_lens
                });

            let bit_len_tag = BTreeMap::from_iter(
                BTreeSet::from_iter(composition_bit_lens.iter().chain(overflow_bit_lens.iter()))
                    .into_iter()
                    .enumerate()
                    .map(|(idx, bit_len)| (*bit_len, idx + 1)),
            );

            let t_tag = meta.lookup_table_column();
            let t_value = meta.lookup_table_column();

            // TODO: consider for a generic MainGateConfig
            let &MainGateConfig { a, b, c, d, .. } = main_gate_config;

            let s_composition = meta.complex_selector();
            let tag_composition = if composition_bit_lens.len() > 1 {
                let tag = meta.fixed_column();
                for (name, value) in [
                    ("composition_a", a),
                    ("composition_b", b),
                    ("composition_c", c),
                    ("composition_d", d),
                ] {
                    Self::configure_lookup_with_column_tag(
                        meta,
                        name,
                        s_composition,
                        tag,
                        value,
                        t_tag,
                        t_value,
                    )
                }
                Some(tag)
            } else {
                for (name, value) in [
                    ("composition_a", a),
                    ("composition_b", b),
                    ("composition_c", c),
                    ("composition_d", d),
                ] {
                    Self::configure_lookup_with_constant_tag(
                        meta,
                        name,
                        s_composition,
                        bit_len_tag[&composition_bit_lens[0]],
                        value,
                        t_tag,
                        t_value,
                    )
                }
                None
            };

            let (s_overflow, tag_overflow) = if !overflow_bit_lens.is_empty() {
                let s_overflow = meta.complex_selector();
                let tag_overflow = if overflow_bit_lens.len() > 1 {
                    let tag = meta.fixed_column();
                    Self::configure_lookup_with_column_tag(
                        meta,
                        "overflow_a",
                        s_overflow,
                        tag,
                        a,
                        t_tag,
                        t_value,
                    );
                    Some(tag)
                } else {
                    Self::configure_lookup_with_constant_tag(
                        meta,
                        "overflow_a",
                        s_overflow,
                        bit_len_tag[&overflow_bit_lens[0]],
                        a,
                        t_tag,
                        t_value,
                    );
                    None
                };

                (Some(s_overflow), tag_overflow)
            } else {
                (None, None)
            };

            RangeConfig {
                main_gate_config: main_gate_config.clone(),
                bit_len_tag,
                t_tag,
                t_value,
                s_composition,
                tag_composition,
                s_overflow,
                tag_overflow,
            }
        }

        fn configure_lookup_with_column_tag(
            meta: &mut ConstraintSystem<F>,
            name: &'static str,
            selector: Selector,
            tag: Column<Fixed>,
            value: Column<Advice>,
            t_tag: TableColumn,
            t_value: TableColumn,
        ) {
            meta.lookup(name, |meta| {
                let selector = meta.query_selector(selector);
                let tag = meta.query_fixed(tag, Rotation::cur());
                let value = meta.query_advice(value, Rotation::cur());
                vec![(tag, t_tag), (selector * value, t_value)]
            });
        }

        fn configure_lookup_with_constant_tag(
            meta: &mut ConstraintSystem<F>,
            name: &'static str,
            selector: Selector,
            tag: usize,
            value: Column<Advice>,
            t_tag: TableColumn,
            t_value: TableColumn,
        ) {
            meta.lookup(name, |meta| {
                let selector = meta.query_selector(selector);
                let tag = selector.clone() * Expression::Constant(F::from(tag as u64));
                let value = meta.query_advice(value, Rotation::cur());
                vec![(tag, t_tag), (selector * value, t_value)]
            });
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

            range_chip.load_table(&mut layouter)?;

            Ok(())
        }
    }

    const LIMB_BIT_LEN: usize = 8;
    const OVERFLOW_BIT_LEN: usize = 3;
    // Initialise the benching parameter, note that minimum k per iteration of range gadget is LIMB_BIT_LEN+1
    // Refer to readme for more detail
    let k = 14;
    let range_repeats = 2_u32.pow(7);

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
