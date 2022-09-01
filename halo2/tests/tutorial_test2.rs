use std::marker::PhantomData;

// halo2wrong = { git = "https://github.com/privacy-scaling-explorations/halo2wrong.git" }

use halo2_proofs::circuit::Value;
use halo2_proofs::plonk::Assigned;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Chip, Layouter, SimpleFloorPlanner},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance},
    poly::Rotation,
};

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
struct TutorialConfig {
    advice: [Column<Advice>; 3],
    fixed: [Column<Fixed>; 4],
    instance: [Column<Instance>; 1],
}

struct TutorialChip<F: FieldExt> {
    config: TutorialConfig,
    marker: PhantomData<F>,
}

impl<F: FieldExt> TutorialChip<F> {
    fn new(config: TutorialConfig) -> Self {
        TutorialChip {
            config,
            marker: PhantomData,
        }
    }
}

impl<F: FieldExt> Chip<F> for TutorialChip<F> {
    type Config = TutorialConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

trait TutorialComposer<F: FieldExt> {
    fn raw_multiply<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>;

    fn raw_add<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>;

    fn copy(&self, layouter: &mut impl Layouter<F>, a: Cell, b: Cell) -> Result<(), Error>;

    /// Exposes a number as a public input to the circuit.
    fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: Cell,
        row: usize,
    ) -> Result<(), Error>;
}

impl<F: FieldExt> TutorialComposer<F> for TutorialChip<F> {
    fn raw_multiply<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        mut f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>,
    {
        layouter.assign_region(
            || "mul",
            |mut region| {
                let mut values = None;
                let lhs = region.assign_advice(
                    || "lhs",
                    self.config.advice[0],
                    0,
                    || {
                        values = Some(f());
                        values.unwrap().map(|v| v.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.advice[1],
                    0,
                    || values.unwrap().map(|v| v.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.advice[2],
                    0,
                    || values.unwrap().map(|v| v.2),
                )?;

                region.assign_fixed(|| "m", self.config.fixed[3], 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "o", self.config.fixed[2], 0, || Value::known(F::one()))?;

                Ok((lhs.cell(), rhs.cell(), out.cell()))
            },
        )
    }

    fn raw_add<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        mut f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>,
    {
        layouter.assign_region(
            || "mul",
            |mut region| {
                let mut values = None;
                let lhs = region.assign_advice(
                    || "lhs",
                    self.config.advice[0],
                    0,
                    || {
                        values = Some(f());
                        values.unwrap().map(|v| v.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.advice[1],
                    0,
                    || values.unwrap().map(|v| v.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.advice[2],
                    0,
                    || values.unwrap().map(|v| v.2),
                )?;

                region.assign_fixed(|| "l", self.config.fixed[0], 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "r", self.config.fixed[1], 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "o", self.config.fixed[2], 0, || Value::known(F::one()))?;

                Ok((lhs.cell(), rhs.cell(), out.cell()))
            },
        )
    }

    fn copy(&self, layouter: &mut impl Layouter<F>, left: Cell, right: Cell) -> Result<(), Error> {
        layouter.assign_region(
            || "copy",
            |mut region| {
                region.constrain_equal(left, right)?;
                region.constrain_equal(left, right)
            },
        )
    }

    fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: Cell,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell, self.config.instance[0], row)
    }
}

#[derive(Default)]
struct TutorialCircuit<F: FieldExt> {
    x: Value<F>,
    y: Value<F>,
    constant: F,
}

impl<F: FieldExt> Circuit<F> for TutorialCircuit<F> {
    type Config = TutorialConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let l = meta.advice_column();
        let r = meta.advice_column();
        let o = meta.advice_column();

        meta.enable_equality(l);
        meta.enable_equality(r);
        meta.enable_equality(o);

        let sm = meta.fixed_column();
        let sl = meta.fixed_column();
        let sr = meta.fixed_column();
        let so = meta.fixed_column();
        #[allow(non_snake_case)]
        let PI = meta.instance_column();
        meta.enable_equality(PI);

        meta.create_gate("mini plonk", |meta| {
            let l = meta.query_advice(l, Rotation::cur());
            let r = meta.query_advice(r, Rotation::cur());
            let o = meta.query_advice(o, Rotation::cur());

            let sl = meta.query_fixed(sl, Rotation::cur());
            let sr = meta.query_fixed(sr, Rotation::cur());
            let so = meta.query_fixed(so, Rotation::cur());
            let sm = meta.query_fixed(sm, Rotation::cur());

            vec![l.clone() * sl + r.clone() * sr + l * r * sm + (o * so * (-F::one()))]
        });

        TutorialConfig {
            advice: [l, r, o],
            fixed: [sl, sr, so, sm],
            instance: [PI],
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let cs = TutorialChip::new(config);

        // Initialise these values so that we can access them more easily outside the block we actually give them a value in
        let x: Value<Assigned<_>> = self.x.into();
        let y: Value<Assigned<_>> = self.y.into();
        let consty = Assigned::from(self.constant);

        // Create x squared
        // Note that the variables named ai for some i are just place holders, meaning that a0 isn't
        // necessarily the first entry in the column a; though in the code we try to make things clear
        let (a0, b0, c0) = cs.raw_multiply(&mut layouter, || x.map(|x| (x, x, x * x)))?;
        cs.copy(&mut layouter, a0, b0)?;

        // Create y squared
        let (a1, b1, c1) = cs.raw_multiply(&mut layouter, || y.map(|y| (y, y, y * y)))?;
        cs.copy(&mut layouter, a1, b1)?;

        // Create xy squared. Note that we need to use the value xsquared here, hence the initialisation
        let (a2, b2, c2) = cs.raw_multiply(&mut layouter, || {
            x.zip(y).map(|(x, y)| (x * x, y * y, x * x * y * y))
        })?;
        cs.copy(&mut layouter, c0, a2)?;
        cs.copy(&mut layouter, c1, b2)?;

        let (a3, b3, c3) = cs.raw_add(&mut layouter, || {
            x.zip(y)
                .map(|(x, y)| (x * x * y * y, consty, x * x * y * y + consty))
        })?;
        cs.copy(&mut layouter, c2, a3)?;

        // Ensure that the constant in the TutorialCircuit struct is correctly used and that the
        // result of the circuit computation is what is expected.
        cs.expose_public(&mut layouter, b3, 0)?;
        // layouter.constrain_instance(b3, cs.config.PI, 0)?;
        layouter.constrain_instance(c3, cs.config.instance[0], 1)?;

        Ok(())
    }
}

#[test]
fn main() {
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::halo2curves::bn256::Fr as Fp;

    // The number of rows in our circuit cannot exceed 2^k. Since our example
    // circuit is very small, we can pick a very small value here.
    let k = 4;

    let constant = Fp::from(7);
    let x = Fp::from(5);
    let y = Fp::from(9);
    let z = Fp::from(25 * 81 + 7);

    let circuit: TutorialCircuit<Fp> = TutorialCircuit {
        x: Value::known(x),
        y: Value::known(y),
        constant: constant,
    };

    // let mut public_inputs = vec![constant, z];
    let mut public_inputs = vec![constant, z];

    // Given the correct public input, our circuit will verify.
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));

    // If we try some other public input, the proof will fail!
    public_inputs[1] += Fp::one();
    let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
    assert!(prover.verify().is_err());
}
