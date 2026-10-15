//! Homogeneous-sphere geometry: volume and mean density.
//!
//! This module holds the *only* place where the sphere volume `(4/3) pi R^3` is
//! written down. Every function that needs it (spherical mass, mean density,
//! critical radius) goes through [`sphere_volume`], so the geometric model
//! assumption exists once and can be audited once.
//!
//! # Model assumption
//!
//! A homogeneous sphere of constant density is an idealisation. It is a
//! reasonable description of a small, self-gravitating body and a poor one for
//! a star, a planet or an accretion flow, all of which are radially
//! stratified. Every result derived from [`sphere_volume`] inherits that
//! assumption.

use core::f64::consts::PI;

use crate::error::PhysicsError;
use crate::units::{Density, DensityUnit, Length, LengthUnit, Mass};

/// Volume of a sphere of radius `radius` in cubic metres: `V = (4/3) pi R^3`.
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if the volume is not a finite,
/// strictly positive `f64`:
///
/// * overflow — for a radius above about `3.5e102 m`, where `(4/3) pi R^3`
///   exceeds `f64::MAX`;
/// * underflow — for a radius below about `8.4e-109 m`, where the cube rounds
///   to zero even as a subnormal.
///
/// Both bounds are stated so that the caller knows the domain rather than
/// discovering it. Neither is reachable from the CLI's input ranges.
pub fn sphere_volume(radius: Length) -> Result<f64, PhysicsError> {
    let r = radius.metres();
    let volume = 4.0 / 3.0 * PI * r * r * r;
    if !volume.is_finite() || volume <= 0.0 {
        return Err(PhysicsError::NotRepresentable {
            operation: "sphere_volume",
        });
    }
    Ok(volume)
}

/// Mean density of a sphere: `rho = M / ((4/3) pi R^3)`.
///
/// This is the density a homogeneous sphere of mass `mass` and radius `radius`
/// would need in order to have that mass. For a body that is not homogeneous it
/// is the *mean* density, not the local density anywhere in particular.
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if the volume overflows, or if
/// the resulting density underflows to zero.
pub fn mean_density(mass: Mass, radius: Length) -> Result<Density, PhysicsError> {
    let density = mass.kilograms() / sphere_volume(radius)?;
    Density::from_in(density, DensityUnit::KilogramPerCubicMetre).map_err(|_| {
        PhysicsError::NotRepresentable {
            operation: "mean_density",
        }
    })
}

/// Radius of a homogeneous sphere of given mass and density:
/// `R = (3 M / (4 pi rho))^(1/3)`.
///
/// The exact inverse of the spherical mass relation. It is required whenever a
/// caller specifies mass and density and wants the radius — the third of the
/// three two-out-of-three combinations the CLI accepts.
///
/// This function is an addition to the API list in the project brief, which
/// names only the six Schwarzschild/sphere relations; it exists because the
/// "custom object" input mode cannot be complete without it.
///
/// # Errors
///
/// Returns [`PhysicsError::NotRepresentable`] if `M / rho` overflows the `f64`
/// range, or if the cube root is not a finite, strictly positive length.
pub fn radius_from_mass_density(mass: Mass, density: Density) -> Result<Length, PhysicsError> {
    let cubed = 3.0 * mass.kilograms() / (4.0 * PI * density.kilograms_per_cubic_metre());
    Length::from_in(cubed.cbrt(), LengthUnit::Metre).map_err(|_| PhysicsError::NotRepresentable {
        operation: "radius_from_mass_density",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{LengthUnit, MassUnit};

    fn len(m: f64) -> Length {
        Length::from_in(m, LengthUnit::Metre).expect("valid length")
    }

    fn mass(kg: f64) -> Mass {
        Mass::from_in(kg, MassUnit::Kilogram).expect("valid mass")
    }

    #[test]
    fn unit_sphere_volume_is_four_thirds_pi() {
        // Independently computed decimal expansion of 4 pi / 3.
        assert!((sphere_volume(len(1.0)).expect("valid") - 4.188_790_204_786_39).abs() < 1e-14);
    }

    #[test]
    fn volume_scales_with_the_cube_of_radius() {
        let v1 = sphere_volume(len(1.0)).expect("valid");
        let v2 = sphere_volume(len(2.0)).expect("valid");
        assert!((v2 / v1 - 8.0).abs() < 1e-12);
    }

    #[test]
    fn volume_overflow_is_reported() {
        // (1e120)^3 = 1e360 -> infinity. The exact overflow bound is about
        // 3.5e102 m for the volume with the (4/3) pi factor included.
        let err = sphere_volume(len(1.0e120)).unwrap_err();
        assert!(matches!(err, PhysicsError::NotRepresentable { .. }));
    }

    #[test]
    fn volume_bounds_are_where_the_documentation_says() {
        // Just inside each bound, the volume is representable and positive.
        assert!(
            sphere_volume(len(3.4e102))
                .expect("representable")
                .is_finite()
        );
        assert!(sphere_volume(len(8.5e-109)).expect("representable") > 0.0);
        // Just outside each bound, the function reports rather than returning
        // zero or infinity.
        assert!(sphere_volume(len(3.6e102)).is_err());
        assert!(sphere_volume(len(8.0e-109)).is_err());
    }

    #[test]
    fn earth_mean_density_is_reproduced() {
        // NASA NSSDCA Earth Fact Sheet: M = 5.9722e24 kg, volumetric mean
        // radius = 6371.0 km, mean density = 5513 kg/m^3.
        let earth = mean_density(mass(5.972_2e24), len(6.371e6)).expect("valid");
        assert!((earth.kilograms_per_cubic_metre() - 5513.0).abs() < 1.0);
    }

    #[test]
    fn radius_from_mass_density_inverts_the_sphere_mass() {
        use crate::units::DensityUnit;
        let density = Density::from_in(2700.0, DensityUnit::KilogramPerCubicMetre).unwrap();
        let radius = radius_from_mass_density(mass(1.0e12), density).expect("valid");
        let back = crate::schwarzschild::mass_from_radius_density(radius, density).expect("valid");
        assert!((back.kilograms() - 1.0e12).abs() / 1.0e12 < 1e-12);
    }

    #[test]
    fn mean_density_is_never_nan_and_never_panics() {
        // Radii spanning the representable range, including values for which
        // R^3 underflows (R below ~8.4e-109 m) or overflows (R above ~3.5e102 m).
        // The exact bounds are asserted in `volume_bounds_are_where_the_documentation_says`.
        for exponent in [-300, -150, -100, 0, 6, 100, 200, 300] {
            let radius = len(10.0_f64.powi(exponent));
            match mean_density(mass(1.0), radius) {
                Ok(density) => {
                    let value = density.kilograms_per_cubic_metre();
                    assert!(
                        value.is_finite() && value > 0.0,
                        "R = 1e{exponent} m produced {value}"
                    );
                }
                Err(PhysicsError::NotRepresentable { .. }) => {}
                Err(other) => panic!("R = 1e{exponent} m gave an unexpected error: {other}"),
            }
        }
    }
}
