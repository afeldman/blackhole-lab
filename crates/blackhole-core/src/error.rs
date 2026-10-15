//! Error type shared by every fallible operation in the physics core.
//!
//! Copyright 2026 Black Hole Lab contributors. Licensed under Apache-2.0.
//! See <LICENSE> in the project root for license details.
//!
//! The core never panics and never returns `NaN`. Every invalid input — a
//! negative mass, a zero radius, a `NaN`, an infinity, or a result that is not
//! representable as a finite `f64` — is reported as a [`PhysicsError`].

use core::fmt;

/// The physical quantity an error refers to.
///
/// Used to make the message actionable without the caller having to know which
/// constructor was called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Quantity {
    /// A mass, in kilograms.
    Mass,
    /// A length or radius, in metres.
    Length,
    /// A (mass) density, in kilograms per cubic metre.
    Density,
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Mass => "mass",
            Self::Length => "length",
            Self::Density => "density",
        };
        f.write_str(name)
    }
}

/// Everything that can go wrong in the physics core.
///
/// The variants are deliberately coarse: they distinguish *why* a value was
/// rejected (not finite vs. not positive vs. not representable), not which
/// function rejected it. Callers that need to report a specific operation
/// supply that context themselves.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PhysicsError {
    /// The supplied value was `NaN` or infinite.
    NotFinite {
        /// Which quantity was supplied.
        quantity: Quantity,
        /// The rejected value.
        value: f64,
    },
    /// The supplied value was zero or negative, but the quantity is defined
    /// here as strictly positive.
    ///
    /// See [`crate::units`] for why zero is rejected rather than clamped: a
    /// zero mass, radius or density is degenerate, and admitting it would
    /// create division-by-zero and `0/0` paths in the physics functions.
    NotPositive {
        /// Which quantity was supplied.
        quantity: Quantity,
        /// The rejected value.
        value: f64,
    },
    /// The result of an operation is not representable as a finite, positive
    /// `f64` — it overflowed to infinity, or underflowed to zero.
    ///
    /// The inputs were individually valid; the *combination* is outside the
    /// representable range. The caller can often recover by rescaling.
    NotRepresentable {
        /// Name of the public function that produced the unrepresentable
        /// result.
        operation: &'static str,
    },
}

impl fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite { quantity, value } => {
                write!(f, "{quantity} must be a finite number, got {value}")
            }
            Self::NotPositive { quantity, value } => {
                write!(f, "{quantity} must be greater than zero, got {value}")
            }
            Self::NotRepresentable { operation } => write!(
                f,
                "the result of `{operation}` is not representable as a finite 64-bit float"
            ),
        }
    }
}

impl std::error::Error for PhysicsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_quantity_and_the_value() {
        let err = PhysicsError::NotPositive {
            quantity: Quantity::Length,
            value: -2.0,
        };
        assert_eq!(err.to_string(), "length must be greater than zero, got -2");

        let err = PhysicsError::NotFinite {
            quantity: Quantity::Density,
            value: f64::NAN,
        };
        assert_eq!(err.to_string(), "density must be a finite number, got NaN");
    }
}
