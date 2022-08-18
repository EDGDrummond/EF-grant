use std::marker::PhantomData;

use halo2_proofs::dev::MockProver;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Chip, Layouter, SimpleFloorPlanner},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Fixed, Instance},
    poly::Rotation,
};
use pairing::bn256::Fr as Fp;

#[allow(non_snake_case, dead_code)]
#[derive(Debug, Clone)]
struct TutorialConfig {
    l: Column<Advice>,
    r: Column<Advice>,
    o: Column<Advice>,

    sl: Column<Fixed>,
    sr: Column<Fixed>,
    so: Column<Fixed>,
    sm: Column<Fixed>,
    sc: Column<Fixed>,
    sp: Column<Fixed>,
    PI: Column<Instance>,
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
        FM: FnMut() -> Result<(F, F, F), Error>;
    fn raw_add<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Result<(F, F, F), Error>;
    fn copy(&self, layouter: &mut impl Layouter<F>, a: Cell, b: Cell) -> Result<(), Error>;
}

impl<F: FieldExt> TutorialComposer<F> for TutorialChip<F> {
    fn raw_multiply<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        mut f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Result<(F, F, F), Error>,
    {
        layouter.assign_region(
            || "mul",
            |mut region| {
                let mut values = None;
                let lhs = region.assign_advice(
                    || "lhs",
                    self.config.l,
                    0,
                    || {
                        values = Some(f()?);
                        Ok(values.ok_or(Error::Synthesis)?.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.r,
                    0,
                    || Ok(values.ok_or(Error::Synthesis)?.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.o,
                    0,
                    || Ok(values.ok_or(Error::Synthesis)?.2),
                )?;

                region.assign_fixed(|| "m", self.config.sm, 0, || Ok(F::one()))?;
                region.assign_fixed(|| "o", self.config.so, 0, || Ok(F::one()))?;

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
        FM: FnMut() -> Result<(F, F, F), Error>,
    {
        layouter.assign_region(
            || "mul",
            |mut region| {
                let mut values = None;
                let lhs = region.assign_advice(
                    || "lhs",
                    self.config.l,
                    0,
                    || {
                        values = Some(f()?);
                        Ok(values.ok_or(Error::Synthesis)?.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.r,
                    0,
                    || Ok(values.ok_or(Error::Synthesis)?.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.o,
                    0,
                    || Ok(values.ok_or(Error::Synthesis)?.2),
                )?;

                region.assign_fixed(|| "l", self.config.sl, 0, || Ok(F::one()))?;
                region.assign_fixed(|| "r", self.config.sr, 0, || Ok(F::one()))?;
                region.assign_fixed(|| "o", self.config.so, 0, || Ok(F::one()))?;

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
}

#[derive(Default)]
struct TutorialCircuit<F: FieldExt> {
    x: Option<F>,
    y: Option<F>,
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
        let sc = meta.fixed_column();
        let sp = meta.fixed_column();
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
            let sc = meta.query_fixed(sc, Rotation::cur());

            vec![l.clone() * sl + r.clone() * sr + l * r * sm + (o * so * (-F::one())) + sc]
        });

        meta.create_gate("Public input", |meta| {
            let l = meta.query_advice(l, Rotation::cur());
            #[allow(non_snake_case)]
            let PI = meta.query_instance(PI, Rotation::cur());
            let sp = meta.query_fixed(sp, Rotation::cur());

            vec![sp * (l - PI)]
        });

        TutorialConfig {
            l,
            r,
            o,
            sl,
            sr,
            so,
            sm,
            sc,
            sp,
            PI,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let cs = TutorialChip::new(config);

        // Initialise these values so that we can access them more easily outside the block we actually give them a value in
        let mut xsquared = None;
        let mut ysquared = None;
        let mut xysquared = None;

        // Create x squared
        // Note that the variables named ai for some i are just place holders, meaning that a0 isn't
        // necessarily the first entry in the column a; though in the code we try to make things clear
        let (a0, b0, c0) = cs.raw_multiply(&mut layouter, || {
            xsquared = self.x.map(|x| x.square());
            Ok((
                self.x.ok_or(Error::Synthesis)?,
                self.x.ok_or(Error::Synthesis)?,
                xsquared.ok_or(Error::Synthesis)?,
            ))
        })?;
        cs.copy(&mut layouter, a0, b0)?;

        // Create y squared
        let (a1, b1, c1) = cs.raw_multiply(&mut layouter, || {
            ysquared = self.y.map(|y| y.square());
            Ok((
                self.y.ok_or(Error::Synthesis)?,
                self.y.ok_or(Error::Synthesis)?,
                ysquared.ok_or(Error::Synthesis)?,
            ))
        })?;
        cs.copy(&mut layouter, a1, b1)?;

        // Create xy squared. Note that we need to use the value xsquared here, hence the initialisation
        let (a2, b2, c2) = cs.raw_multiply(&mut layouter, || {
            xysquared = xsquared.and_then(|xsquared| self.y.map(|y| y * y * xsquared));
            Ok((
                xsquared.ok_or(Error::Synthesis)?,
                ysquared.ok_or(Error::Synthesis)?,
                xysquared.ok_or(Error::Synthesis)?,
            ))
        })?;
        cs.copy(&mut layouter, c0, a2)?;
        cs.copy(&mut layouter, c1, b2)?;

        let (a3, b3, c3) = cs.raw_add(&mut layouter, || {
            let finished = xysquared.and_then(|xysquared| Some(xysquared + self.constant));
            Ok((
                xysquared.ok_or(Error::Synthesis)?,
                Some(self.constant).ok_or(Error::Synthesis)?,
                finished.ok_or(Error::Synthesis)?,
            ))
        })?;
        cs.copy(&mut layouter, c2, a3)?;

        // Ensure that the constant in the TutorialCircuit struct is correctly used and that the
        // result of the circuit computation is what is expected.
        layouter.constrain_instance(b3, cs.config.PI, 0)?;
        layouter.constrain_instance(c3, cs.config.PI, 1)?;

        Ok(())
    }
}

#[test]
fn main() {
    // The number of rows in our circuit cannot exceed 2^k. Since our example
    // circuit is very small, we can pick a very small value here.
    let k = 4;

    let constant = Fp::from(7);
    let x = Fp::from(5);
    let y = Fp::from(9);
    let z = Fp::from(25 * 81 + 7);

    let circuit: TutorialCircuit<Fp> = TutorialCircuit {
        x: Some(x),
        y: Some(y),
        constant: constant,
    };

    // let mut public_inputs = vec![constant, z];
    let mut public_inputs = vec![constant, z];

    // Given the correct public input, our circuit will verify.
    let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));

    // If we try some other public input, the proof will fail!
    public_inputs[0] += Fp::one();
    let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
    assert!(prover.verify().is_err());
}
