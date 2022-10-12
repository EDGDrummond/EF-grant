// Note that all the code here was copied from the original repository for the purposes
// of placing it with other tutorial/example code in this repo.

use halo2wrong::{
    curves::{
        bn256::Fr as BnScalar,
        pasta::{Fp as PastaFp, Fq as PastaFq},
        secp256k1::Secp256k1Affine as Secp256k1,
    },
    halo2::{
        arithmetic::{CurveAffine, FieldExt},
        circuit::{Layouter, SimpleFloorPlanner, Value},
        plonk::*,
    },
};
use rand_core::OsRng;

use ecc::{integer::Range, EccConfig, GeneralEccChip};
use ecdsa::ecdsa::{AssignedEcdsaSig, AssignedPublicKey, EcdsaChip};
use group::{
    ff::{Field, PrimeField},
    Curve, Group,
};
use integer::IntegerInstructions;
use maingate::{
    big_to_fe, fe_to_big, mock_prover_verify, MainGate, MainGateConfig, RangeChip, RangeConfig,
    RangeInstructions, RegionCtx,
};
use std::marker::PhantomData;

const BIT_LEN_LIMB: usize = 68;
const NUMBER_OF_LIMBS: usize = 4;

#[derive(Clone, Debug)]
struct TestCircuitEcdsaVerifyConfig {
    main_gate_config: MainGateConfig,
    range_config: RangeConfig,
}

impl TestCircuitEcdsaVerifyConfig {
    pub fn new<C: CurveAffine, N: FieldExt>(meta: &mut ConstraintSystem<N>) -> Self {
        let (rns_base, rns_scalar) = GeneralEccChip::<C, N, NUMBER_OF_LIMBS, BIT_LEN_LIMB>::rns();
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

    pub fn config_range<N: FieldExt>(&self, layouter: &mut impl Layouter<N>) -> Result<(), Error> {
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
        let mut ecc_chip =
            GeneralEccChip::<E, N, NUMBER_OF_LIMBS, BIT_LEN_LIMB>::new(config.ecc_chip_config());

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

                let r_assigned = scalar_chip.assign_integer(ctx, integer_r, Range::Remainder)?;
                let s_assigned = scalar_chip.assign_integer(ctx, integer_s, Range::Remainder)?;
                let sig = AssignedEcdsaSig {
                    r: r_assigned,
                    s: s_assigned,
                };

                let pk_in_circuit = ecc_chip.assign_point(ctx, self.public_key)?;
                let pk_assigned = AssignedPublicKey {
                    point: pk_in_circuit.clone(),
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

fn run_fixed<C: CurveAffine, N: FieldExt>(
    sk: <Secp256k1 as CurveAffine>::ScalarExt,
    msg_hash: <Secp256k1 as CurveAffine>::ScalarExt,
    r: <Secp256k1 as CurveAffine>::ScalarExt,
    s: <Secp256k1 as CurveAffine>::ScalarExt,
) -> (TestCircuitEcdsaVerify<Secp256k1, N>, Vec<Vec<N>>) {
    // This function always returns the same generator, it is not random
    let g = Secp256k1::generator();

    // Generate a key pair
    let public_key = (g * sk).to_affine();

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
        let r_candidate = mod_n::<Secp256k1>(*x_candidate);
        assert_eq!(r, r_candidate);
    }

    let aux_generator = <Secp256k1 as CurveAffine>::CurveExt::random(OsRng).to_affine();
    let circuit = TestCircuitEcdsaVerify::<Secp256k1, N> {
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

#[test]
fn test_ecdsa_example() {
    run::<Secp256k1, BnScalar>();
    run::<Secp256k1, PastaFp>();
    run::<Secp256k1, PastaFq>();
}

#[test]
fn test_ecdsa_fixed_example() {
    let sk = <Secp256k1 as CurveAffine>::ScalarExt::from_repr([
        85, 8, 121, 127, 160, 231, 119, 122, 216, 131, 130, 203, 38, 231, 124, 27, 11, 122, 244,
        109, 99, 249, 147, 130, 189, 222, 247, 234, 155, 205, 119, 230,
    ])
    .unwrap();
    let msg_hash = <Secp256k1 as CurveAffine>::ScalarExt::from_repr([
        251, 142, 21, 204, 206, 83, 7, 53, 187, 208, 223, 135, 194, 120, 41, 19, 50, 10, 0, 111,
        146, 22, 131, 22, 127, 227, 198, 128, 216, 154, 189, 201,
    ])
    .unwrap();
    let r = <Secp256k1 as CurveAffine>::ScalarExt::from_repr([
        187, 151, 126, 77, 29, 227, 214, 208, 36, 160, 93, 31, 184, 219, 54, 16, 215, 255, 156,
        170, 254, 250, 154, 148, 221, 149, 99, 130, 248, 137, 199, 233,
    ])
    .unwrap();
    let s = <Secp256k1 as CurveAffine>::ScalarExt::from_repr([
        38, 232, 67, 121, 242, 30, 166, 41, 58, 239, 95, 83, 165, 58, 48, 91, 22, 240, 26, 107,
        212, 145, 50, 76, 170, 156, 236, 121, 98, 85, 169, 185,
    ])
    .unwrap();

    run_fixed::<Secp256k1, BnScalar>(sk, msg_hash, r, s);
}
