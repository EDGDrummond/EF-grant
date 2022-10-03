use halo2wrong::{
    halo2::{
        arithmetic::FieldExt,
        circuit::{Layouter, SimpleFloorPlanner, Value},
        halo2curves::bn256::{Bn256, Fr as Fp, G1Affine},
        plonk::*,
        poly::{
            commitment::ParamsProver,
            kzg::commitment::{KZGCommitmentScheme, ParamsKZG},
            kzg::multiopen::{ProverGWC, VerifierGWC},
            kzg::strategy::SingleStrategy,
        },
        transcript::{
            Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer,
        },
    },
    RegionCtx,
};
use maingate::{
    MainGate, MainGateConfig, MainGateInstructions, RangeChip, RangeConfig, RangeInstructions, Term,
};
use num_integer::Integer;
use rand_core::OsRng;

/// Maximum number of cells in one line enabled with composition selector
pub const NUMBER_OF_LOOKUP_LIMBS: usize = 4;

#[derive(Clone, Debug)]
struct TestCircuitConfig {
    range_config: RangeConfig,
    main_gate_config: MainGateConfig,
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
        Self {
            range_config,
            main_gate_config,
        }
    }

    fn main_gate<F: FieldExt>(&self) -> MainGate<F> {
        MainGate::<F>::new(self.main_gate_config.clone())
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
}

impl<F: FieldExt> TestCircuit<F> {
    fn composition_bit_lens(limb_bit_len: usize) -> Vec<usize> {
        [limb_bit_len].to_vec()
    }

    fn overflow_bit_lens(overflow_bit_len: [usize; 2]) -> Vec<usize> {
        overflow_bit_len.to_vec()
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
        TestCircuit { inputs }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        TestCircuitConfig::new(
            meta,
            Self::composition_bit_lens(LIMB_BIT_LEN),
            Self::overflow_bit_lens(OVERFLOW_BIT_LEN),
        )
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let range_chip = config.range_chip();
        let main_gate = config.main_gate();

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

                    let mut bases: Vec<F> = Vec::new();

                    let (num_limbs, overflow_len) = bit_len.div_rem(&limb_bit_len);

                    for i in 0..num_limbs {
                        bases.push(F::from(2).pow(&[(limb_bit_len * i) as u64, 0, 0, 0]));
                    }
                    if overflow_len != 0 {
                        bases.push(F::from(2).pow(&[(limb_bit_len * num_limbs) as u64, 0, 0, 0]));
                    }

                    let terms: Vec<Term<F>> = decomposed
                        .iter()
                        .zip(bases.as_slice())
                        .map(|(limb, base)| Term::Assigned(limb, *base))
                        .collect();
                    let a_1 = main_gate.compose(ctx, &terms[..], F::zero())?;
                    main_gate.assert_equal(ctx, &a_0, &a_1)?;
                }

                Ok(())
            },
        )?;

        range_chip.load_table(&mut layouter)?;

        Ok(())
    }
}

// Set lbl and obl values depending upon the breakdown of the values required
const LIMB_BIT_LEN: usize = 8;
const OVERFLOW_BIT_LEN: [usize; 2] = [4, 3];

#[test]
fn test_range_multi() {
    let k = 9;
    let first = 68;
    let second = 67;
    let input = vec![
        Input {
            value: Value::known(Fp::from_u128((1 << first) - 1)),
            limb_bit_len: 8,
            bit_len: first,
        },
        Input {
            value: Value::known(Fp::from_u128((1 << second) - 1)),
            limb_bit_len: 8,
            bit_len: second,
        },
        Input {
            value: Value::known(Fp::from_u128((1 << 30) - 1)),
            limb_bit_len: 8,
            bit_len: first,
        },
        // If you uncomment this input and try to produce a proof it will fail
        // Input {
        //     value: Value::known(Fp::from_u128((1 << 66) - 1)),
        //     limb_bit_len: 8,
        //     bit_len: 66,
        // },
    ];

    // Initialise circuit, and an empty version of it
    let circuit = TestCircuit::<Fp> {
        inputs: input.clone(),
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

    verify_proof::<_, VerifierGWC<Bn256>, _, _, _>(
        &params,
        pk.get_vk(),
        strategy.clone(),
        &[&[&[]]],
        &mut transcript.clone(),
    )
    .unwrap();
}
