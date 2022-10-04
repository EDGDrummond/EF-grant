use halo2wrong::{
    halo2::{
        arithmetic::FieldExt,
        circuit::{Layouter, SimpleFloorPlanner, Value},
        dev::MockProver,
        halo2curves::bn256::Fr as Fp,
        plonk::*,
    },
    RegionCtx,
};
use maingate::{
    MainGate, MainGateConfig, MainGateInstructions, RangeChip, RangeConfig, RangeInstructions, Term,
};
use num_integer::Integer;

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
                    let mut bases: Vec<F> = Vec::new();
                    let (num_limbs, overflow_len) = bit_len.div_rem(&limb_bit_len);

                    let a_0 = main_gate.assign_value(ctx, value)?;
                    let (a_1, decomposed) =
                        range_chip.decompose(ctx, value, limb_bit_len, bit_len)?;

                    main_gate.assert_equal(ctx, &a_0, &a_1)?;

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
    let mut inputs = vec![
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
    ];

    // Initialise circuit, and an empty version of it
    let circuit = TestCircuit::<Fp> {
        inputs: inputs.clone(),
    };

    let public_inputs = vec![vec![]];
    let prover = match MockProver::run(k, &circuit, public_inputs.clone()) {
        Ok(prover) => prover,
        Err(e) => panic!("{:#?}", e),
    };
    assert_eq!(prover.verify(), Ok(()));

    // Add an input that is bigger than claimed; proof should fail
    inputs.push(Input {
        value: Value::known(Fp::from_u128((1 << 69) - 1)),
        limb_bit_len: 8,
        bit_len: 68,
    });
    let circuit = TestCircuit::<Fp> {
        inputs: inputs.clone(),
    };
    let prover = match MockProver::run(k, &circuit, public_inputs) {
        Ok(prover) => prover,
        Err(e) => panic!("{:#?}", e),
    };
    assert_ne!(prover.verify(), Ok(()));
}
