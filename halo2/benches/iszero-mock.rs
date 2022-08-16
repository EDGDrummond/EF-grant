#[macro_use]
extern crate criterion;

use halo2_proofs::dev::MockProver;
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Layouter, SimpleFloorPlanner},
    plonk::*,
    poly::Rotation,
};
use pairing::bn256::Fr as Fp;

use std::marker::PhantomData;

use criterion::Criterion;

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
            F: FnMut() -> Result<(FF, FF, FF), Error>;
        fn raw_add<F>(
            &self,
            layouter: &mut impl Layouter<FF>,
            f: F,
        ) -> Result<(Cell, Cell, Cell), Error>
        where
            F: FnMut() -> Result<(FF, FF, FF), Error>;
        fn copy(&self, layouter: &mut impl Layouter<FF>, a: Cell, b: Cell) -> Result<(), Error>;
    }

    #[derive(Clone)]
    struct MyCircuit<F: FieldExt> {
        a: Option<F>,
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
            F: FnMut() -> Result<(FF, FF, FF), Error>,
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
                            values = Some(f()?);
                            Ok(values.ok_or(Error::Synthesis)?.0)
                        },
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || Ok(values.ok_or(Error::Synthesis)?.1),
                    )?;

                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || Ok(values.ok_or(Error::Synthesis)?.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Ok(FF::zero()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Ok(FF::zero()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "a * b", self.config.sm, 0, || Ok(FF::one()))?;

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
            F: FnMut() -> Result<(FF, FF, FF), Error>,
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
                            values = Some(f()?);
                            Ok(values.ok_or(Error::Synthesis)?.0)
                        },
                    )?;
                    let rhs = region.assign_advice(
                        || "rhs",
                        self.config.b,
                        0,
                        || Ok(values.ok_or(Error::Synthesis)?.1),
                    )?;

                    let out = region.assign_advice(
                        || "out",
                        self.config.c,
                        0,
                        || Ok(values.ok_or(Error::Synthesis)?.2),
                    )?;

                    region.assign_fixed(|| "a", self.config.sa, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "b", self.config.sb, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "c", self.config.sc, 0, || Ok(FF::one()))?;
                    region.assign_fixed(|| "a * b", self.config.sm, 0, || Ok(FF::zero()))?;

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
            Self { a: None, k: self.k }
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

            for _ in 0..self.k {
                let inv = if self.a.unwrap() == F::zero() {
                    Some(F::zero())
                } else {
                    Some(-self.a.clone().unwrap().invert().unwrap())
                };
                // first gate, the mul gate
                let (_a1, b1, c1) = cs.raw_multiply(&mut layouter, || {
                    Ok((
                        inv.clone().unwrap(),
                        self.a.ok_or(Error::Synthesis)?,
                        inv.clone().unwrap() * self.a.unwrap(),
                    ))
                })?;
                // addition gate, where we are going to create out
                let (a2, _b2, c2) = cs.raw_add(&mut layouter, || {
                    Ok((
                        inv.clone().unwrap() * self.a.unwrap(),
                        F::one(),
                        inv.clone().unwrap() * self.a.unwrap() + F::one(),
                    ))
                })?;
                // final gate, the second multiplication gate
                let (a3, b3, _c3) = cs.raw_multiply(&mut layouter, || {
                    Ok((
                        inv.clone().unwrap() * self.a.unwrap() + F::one(),
                        self.a.ok_or(Error::Synthesis)?,
                        F::zero(),
                    ))
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
    let k = 5;
    let a_value = Some(Fp::from(100000));

    let empty_circuit: MyCircuit<Fp> = MyCircuit { a: a_value, k };

    c.bench_function("keygen_and_prover", |b| {
        b.iter(|| {
            for _ in 5..8 {
                let prover = MockProver::run(k, &empty_circuit, vec![]).unwrap();
                assert_eq!(prover.verify(), Ok(()));
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
