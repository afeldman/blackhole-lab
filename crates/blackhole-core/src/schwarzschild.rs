//! Schwarzschild radius, its inverse, the compactness ratio, and the critical
//! radius of a homogeneous sphere at an assumed density.
//!
//! # The two different questions
//!
//! This module answers two questions that are easy to conflate, and the
//! documentation keeps them apart deliberately.
//!
//! **A. Given a mass, how large is its Schwarzschild radius?**
//!
//! ```text
//! r_s = 2 G M / c^2
//! ```
//!
//! Every mass has a Schwarzschild radius. There is no mass for which `r_s` is
//! undefined or zero, and there is therefore **no universal minimum mass for a
//! black hole**. [`schwarzschild_radius`] answers this question, and the
//! physically meaningful statement is a *comparison*: is the object's actual
//! radius `R` larger or smaller than `r_s`?
//!
//! **B. Given an assumed constant density, how large must a homogeneous sphere
//! be for its Schwarzschild radius to equal its own radius?**
//!
//! Setting `R = r_s` in `M = (4/3) pi R^3 rho` gives
//!
//! ```text
//! R_krit = sqrt( 3 c^2 / (8 pi G rho) )
//! ```
//!
//! implemented as [`minimum_radius_at_density`]. This is the size at which a
//! *homogeneous sphere of that fixed density* would have its horizon at its
//! surface. It is **not** a general minimum size for black holes: change the
//! assumed density and the number changes, because the density is an input
//! assumption of the model, not a property of the object.
//!
//! # What these functions are not
//!
//! Comparing `R` with `r_s` is a comparison of two lengths. It is **not** a
//! simulation of gravitational collapse, and no function in this module
//! predicts whether an object "becomes" a black hole. Whether a horizon forms
//! depends on the spacetime geometry and on the dynamical state of the matter
//! — its pressure, angular momentum, radiation content and the rate at which it
//! is compressed — none of which appears in these formulae. See
//! `docs/physics.md` for the full statement of model limits.

use core::f64::consts::PI;

use crate::constants::{GRAVITATIONAL_CONSTANT, SCHWARZSCHILD_RADIUS_PER_KILOGRAM, SPEED_OF_LIGHT};
use crate::error::PhysicsError;
use crate::sphere::sphere_volume;
use crate::units::{Density, Length, LengthUnit, Mass, MassUnit};

/// Wraps a computed length, reporting any representability failure with the
/// name of the public operation that produced it.
fn length_result(metres: f64, operation: &'static str) -> Result<Length, PhysicsError> {
    Length::from_in(metres, LengthUnit::Metre)
        .map_err(|_| PhysicsError::NotRepresentable { operation })
}

/// Wraps a computed mass, reporting any representability failure with the name
/// of the public operation that produced it.
fn mass_result(kilograms: f64, operation: &'static str) -> Result<Mass, PhysicsError> {
    Mass::from_in(kilograms, MassUnit::Kilogram)
        .map_err(|_| PhysicsError::NotRepresentable { operation })
}

/// Schwarzschild radius of a mass: `r_s = 2 G M / c^2`.
///
/// The radius of the event horizon of a non-rotating, uncharged (Schwarzschild)
/// black hole of the given mass. Valid for any strictly positive mass; there is
/// no minimum mass.
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if the product overflows to
/// infinity or underflows to zero. With `G` and `c` at their SI values this
/// cannot happen for a mass that is itself representable: `2G/c^2` is about
/// `1.49e-27 m kg^-1`, so even `f64::MAX` kilograms yields a finite radius.
/// The check is retained so that the guarantee does not depend on the numerical
/// value of the constants.
pub fn schwarzschild_radius(mass: Mass) -> Result<Length, PhysicsError> {
    length_result(
        mass.kilograms() * SCHWARZSCHILD_RADIUS_PER_KILOGRAM,
        "schwarzschild_radius",
    )
}

/// Mass whose Schwarzschild radius equals `radius`: `M = r_s c^2 / (2 G)`.
///
/// The exact inverse of [`schwarzschild_radius`].
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if the result overflows the
/// `f64` range — for a radius greater than about `2.7e281 m` — or underflows to
/// zero.
pub fn mass_from_schwarzschild_radius(radius: Length) -> Result<Mass, PhysicsError> {
    mass_result(
        radius.metres() / SCHWARZSCHILD_RADIUS_PER_KILOGRAM,
        "mass_from_schwarzschild_radius",
    )
}

/// Dimensionless compactness ratio `r_s / R = 2 G M / (c^2 R)`, returned as a
/// plain `f64` because it is a pure number.
///
/// # Naming warning
///
/// In the general-relativity literature "compactness" is frequently defined as
/// `G M / (c^2 R)`, which is **exactly half** of what this function returns.
/// This crate uses the ratio of the object's Schwarzschild radius to its actual
/// radius, because that is the quantity the CLI compares and the one the model
/// limits are stated in terms of. Whenever a compactness number is published or
/// compared with the literature, state which convention is meant.
///
/// # Interpretation
///
/// * `compactness < 1` — the surface lies outside the Schwarzschild radius
///   *under the stated assumptions*.
/// * `compactness >= 1` — the surface lies inside `r_s`, i.e. the model places
///   the object within its own horizon, which means the model's own premises
///   (a static sphere of constant density in flat space) have broken down.
///   It is not a dynamical prediction that a black hole forms.
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if the ratio overflows to
/// infinity, which requires a mass and radius more than about 300 orders of
/// magnitude apart.
pub fn compactness(mass: Mass, radius: Length) -> Result<f64, PhysicsError> {
    // Evaluated as (M / R) * kappa rather than (M * kappa) / R. The two are
    // algebraically identical, but the first avoids a premature underflow: for
    // M = 1e-300 kg and R = 1e-6 m the product M * kappa = 1.4852e-327
    // underflows to zero before the division, while the true ratio 1.4852e-321
    // is representable as a subnormal and is delivered by this ordering.
    let ratio = mass.kilograms() / radius.metres() * SCHWARZSCHILD_RADIUS_PER_KILOGRAM;
    if !ratio.is_finite() {
        return Err(PhysicsError::NotRepresentable {
            operation: "compactness",
        });
    }
    Ok(ratio)
}

/// Mass of a homogeneous sphere: `M = (4/3) pi R^3 rho`.
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if the volume or the product
/// overflows the `f64` range. For a density of `1e3 kg m^-3` this happens for a
/// radius above about `3.5e101 m`, since the mass `(4/3) pi R^3 rho` then
/// exceeds `f64::MAX`; for a density of `1 kg m^-3` the bound is `3.5e102 m`.
pub fn mass_from_radius_density(radius: Length, density: Density) -> Result<Mass, PhysicsError> {
    let kilograms = sphere_volume(radius)? * density.kilograms_per_cubic_metre();
    mass_result(kilograms, "mass_from_radius_density")
}

/// Schwarzschild radius of a homogeneous sphere of the given radius and
/// density: `r_s = (2G/c^2) (4/3) pi R^3 rho`.
///
/// # Errors
///
/// Propagates [`PhysicsError::NotRepresentable`] from
/// [`mass_from_radius_density`] or [`schwarzschild_radius`]; the error names
/// the inner operation that failed.
pub fn schwarzschild_radius_from_radius_density(
    radius: Length,
    density: Density,
) -> Result<Length, PhysicsError> {
    schwarzschild_radius(mass_from_radius_density(radius, density)?)
}

/// Critical radius of a homogeneous sphere at assumed constant density:
/// `R_krit = sqrt( 3 c^2 / (8 pi G rho) )`.
///
/// This is the radius at which a sphere of the *assumed* constant density
/// `rho` would have its Schwarzschild radius equal to its own radius, i.e. the
/// solution of `r_s(M(R)) = R` for `M(R) = (4/3) pi R^3 rho`.
///
/// # Reading this number correctly
///
/// * `rho` is a **model assumption**. A real object that is compressed changes
///   its density, so the number moves with the assumption.
/// * The result scales as `rho^(-1/2)`: a hundredfold denser assumption gives a
///   tenfold smaller radius.
/// * The result is a property of the *model* (a static, homogeneous, constant
///   density sphere), not a universal threshold. Nothing here implies that an
///   object merely larger than some radius must collapse.
/// * It does **not** mean that any body reaching this size forms a black hole;
///   see the module documentation and `docs/physics.md`.
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if the result overflows the `f64`
/// range, which happens for a density below about `5.0e-283 kg m^-3` — an
/// absurd value, but one a caller could reach by arithmetic accident, so the
/// bound is stated exactly.
pub fn minimum_radius_at_density(density: Density) -> Result<Length, PhysicsError> {
    let rho = density.kilograms_per_cubic_metre();
    let squared = 3.0 * SPEED_OF_LIGHT * SPEED_OF_LIGHT / (8.0 * PI * GRAVITATIONAL_CONSTANT * rho);
    length_result(squared.sqrt(), "minimum_radius_at_density")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::DensityUnit;

    fn len(m: f64) -> Length {
        Length::from_in(m, LengthUnit::Metre).expect("valid length")
    }

    fn mass(kg: f64) -> Mass {
        Mass::from_in(kg, MassUnit::Kilogram).expect("valid mass")
    }

    fn rho(kg_m3: f64) -> Density {
        Density::from_in(kg_m3, DensityUnit::KilogramPerCubicMetre).expect("valid density")
    }

    #[test]
    fn earth_radius_matches_the_published_figure() {
        // The commonly quoted value is 8.87 mm — three significant figures,
        // i.e. about 1e-3 relative. The tolerance is taken from that precision,
        // not from this crate's own digits: a tighter one could only pass by
        // reproducing the same closed form with the same constants, which would
        // test the author's ability to write the expression twice.
        let r = schwarzschild_radius(mass(5.972_2e24)).expect("valid");
        assert!((r.metres() - 8.87e-3).abs() / 8.87e-3 < 1e-3);
    }

    #[test]
    fn sun_radius_matches_the_published_value() {
        // 2GM/c^2 for 1.9884e30 kg. The commonly quoted figure is 2.95 km;
        // 1e-3 relative accommodates the reference being quoted to three
        // significant figures.
        let r = schwarzschild_radius(mass(1.988_4e30)).expect("valid");
        assert!((r.metres() - 2953.0).abs() / 2953.0 < 1e-3);
    }

    #[test]
    fn compactness_does_not_underflow_prematurely() {
        // (M * kappa) / R would compute 1.4852e-327 here, which underflows to
        // zero before the division; the true ratio 1.4852e-321 is representable
        // as a subnormal and must be delivered.
        let ratio = compactness(mass(1.0e-300), len(1.0e-6)).expect("valid");
        assert!(ratio > 0.0, "the ratio underflowed to {ratio}");
        assert!((ratio - 1.485_2e-321).abs() / 1.485_2e-321 < 1e-3);
    }

    #[test]
    fn compactness_equals_one_exactly_at_the_schwarzschild_radius() {
        let m = mass(4.0e30);
        let rs = schwarzschild_radius(m).expect("valid");
        let ratio = compactness(m, rs).expect("valid");
        assert!((ratio - 1.0).abs() < 1e-15);
    }

    #[test]
    fn critical_radius_alternative_algebraic_form_agrees() {
        // R_krit^2 = 3 c^2 / (8 pi G rho) = 3 / (4 pi rho kappa)  with kappa = 2G/c^2.
        let density = rho(1000.0);
        let via_definition = minimum_radius_at_density(density).expect("valid").metres();
        let via_kappa = (3.0 / (4.0 * PI * 1000.0 * SCHWARZSCHILD_RADIUS_PER_KILOGRAM)).sqrt();
        assert!((via_definition - via_kappa).abs() / via_kappa < 1e-14);
    }

    #[test]
    fn critical_radius_is_self_consistent_by_construction() {
        // At R_krit, the sphere's own Schwarzschild radius must equal R_krit.
        for density in [1.0, 1000.0, 2900.0, 5.514e3, 3.85e17] {
            let r = minimum_radius_at_density(rho(density)).expect("valid");
            let m = mass_from_radius_density(r, rho(density)).expect("valid");
            let rs = schwarzschild_radius(m).expect("valid");
            assert!(
                (rs.metres() - r.metres()).abs() / r.metres() < 1e-12,
                "density {density}"
            );
        }
    }

    #[test]
    fn critical_radius_scales_as_inverse_sqrt_density() {
        let r1 = minimum_radius_at_density(rho(1000.0))
            .expect("valid")
            .metres();
        let r100 = minimum_radius_at_density(rho(100_000.0))
            .expect("valid")
            .metres();
        assert!((r1 / r100 - 10.0).abs() < 1e-12);
    }

    #[test]
    fn mass_and_radius_round_trip() {
        let m = mass(8.54e36);
        let r = schwarzschild_radius(m).expect("valid");
        let back = mass_from_schwarzschild_radius(r).expect("valid");
        assert!((back.kilograms() - m.kilograms()).abs() / m.kilograms() < 1e-15);
    }

    #[test]
    fn overflow_is_reported_not_panicked() {
        let r = len(1.0e308);
        assert!(matches!(
            mass_from_schwarzschild_radius(r),
            Err(PhysicsError::NotRepresentable { .. })
        ));
        assert!(matches!(
            mass_from_radius_density(r, rho(1000.0)),
            Err(PhysicsError::NotRepresentable { .. })
        ));
    }
}
