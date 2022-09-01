//! `maingate` defines basic instructions for a starndart like PLONK gate and
//! implments a 5 width gate with two multiplication and one rotation
//! customisation
//!
//! This code was taken from the halo2wrong implementation in September 2022,
//! none of the code in this folder is my own. (Though it may be adapted to work in this repo))

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

use halo2wrong::halo2::circuit::AssignedCell;

#[macro_use]
mod main_gate;
mod instructions;
mod range;
mod range2;

pub use instructions::{CombinationOptionCommon, MainGateInstructions, Term};
pub use main_gate::*;
pub use range::*;
pub use range2::*;

#[cfg(test)]
use halo2wrong::curves;
#[cfg(test)]
pub use halo2wrong::utils::mock_prover_verify;

/// AssignedValue
pub type AssignedValue<F> = AssignedCell<F, F>;
/// AssignedCondition
pub type AssignedCondition<F> = AssignedCell<F, F>;
