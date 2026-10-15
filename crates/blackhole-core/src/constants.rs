//! Physical, mathematical and astronomical constants used by the physics core.
//!
//! Copyright 2026 Black Hole Lab contributors. Licensed under Apache-2.0.
//! See <LICENSE> in the project root for license details.
//!
//! # Provenance policy
//!
//! Every constant carries its source, and the values are split into two groups
//! that must not be confused:
//!
//! * **measured** constants (SI / CODATA) carry a relative standard
//!   uncertainty;
//! * **nominal** constants (IAU 2015 Resolution B3) are exact *by definition*
//!   and exist only as conversion factors — they are not, and must not be
//!   presented as, the true properties of the Sun, Earth or Jupiter.
//!
//! # Which uncertainty dominates
//!
//! The Newtonian constant of gravitation [`GRAVITATIONAL_CONSTANT`] has a
//! relative standard uncertainty of 2.2e-5 (CODATA 2018). That is about five
//! orders of magnitude larger than the uncertainty of any other constant used
//! here, so it dominates the physical uncertainty of every derived quantity in
//! this crate. Results are nevertheless computed and returned at full `f64`
//! precision: they are *computed* numbers, not measurements, and artificially
//! rounding them would destroy the exact algebraic identities that the test
//! suite verifies.
//!
//! # References
//!
//! * CODATA 2018: Tiesinga, Mohr, Newell & Taylor (2021), *Rev. Mod. Phys.*
//!   **93**, 025010. Values: `G = 6.67430(15)e-11 m^3 kg^-1 s^-2`,
//!   `c = 299792458 m s^-1` (exact by the SI definition of the metre).
//! * IAU 2015 Resolution B3: Prša et al. (2016), *AJ* **152**, 41
//!   (arXiv:1510.07674). Nominal solar and planetary conversion constants.
//! * IAU 2012 Resolution B2: definition of the astronomical unit.

use core::f64::consts::PI;

// ---------------------------------------------------------------------------
// Measured SI constants
// ---------------------------------------------------------------------------

/// Newtonian constant of gravitation `G` in `m^3 kg^-1 s^-2`.
///
/// CODATA 2018 recommended value `6.67430(15)e-11`, relative standard
/// uncertainty `2.2e-5`. This is the dominant uncertainty of the whole crate.
pub const GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11;

/// Speed of light in vacuum `c` in `m s^-1`.
///
/// Exact by the SI definition of the metre (17th CGPM, 1983): the metre is
/// defined such that `c = 299 792 458 m s^-1` exactly.
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;

/// Conversion factor from mass to Schwarzschild radius, `2G/c^2`, in `m kg^-1`.
///
/// `r_s = M * SCHWARZSCHILD_RADIUS_PER_KILOGRAM`. Numerically
/// `1.485 2e-27 m kg^-1`.
pub const SCHWARZSCHILD_RADIUS_PER_KILOGRAM: f64 =
    2.0 * GRAVITATIONAL_CONSTANT / (SPEED_OF_LIGHT * SPEED_OF_LIGHT);

// ---------------------------------------------------------------------------
// IAU 2015 Resolution B3 nominal conversion constants (exact by definition)
// ---------------------------------------------------------------------------

/// Nominal solar mass parameter `(GM)_sun^N` in `m^3 s^-2`. Exact.
///
/// The preferred way to express a solar mass is `(GM)/G` with `G` stated
/// explicitly (IAU 2015 B3, recommendation 5), which is exactly what
/// [`NOMINAL_SOLAR_MASS`] does.
pub const NOMINAL_SOLAR_MASS_PARAMETER: f64 = 1.327_124_4e20;

/// Nominal solar radius `R_sun^N` in `m`. Exact.
///
/// Adopted from the photospheric radius of Haberreiter et al. (2008).
pub const NOMINAL_SOLAR_RADIUS: f64 = 6.957e8;

/// Nominal terrestrial (Earth) equatorial radius `R_eE^N` in `m`. Exact.
pub const NOMINAL_EARTH_EQUATORIAL_RADIUS: f64 = 6.378_1e6;

/// Nominal terrestrial polar radius `R_pE^N` in `m`. Exact.
pub const NOMINAL_EARTH_POLAR_RADIUS: f64 = 6.356_8e6;

/// Nominal jovian equatorial radius `R_eJ^N` in `m`. Exact.
pub const NOMINAL_JOVIAN_EQUATORIAL_RADIUS: f64 = 7.149_2e7;

/// Nominal jovian polar radius `R_pJ^N` in `m`. Exact.
pub const NOMINAL_JOVIAN_POLAR_RADIUS: f64 = 6.685_4e7;

/// Nominal terrestrial mass parameter `(GM)_E^N` in `m^3 s^-2`. Exact.
pub const NOMINAL_TERRESTRIAL_MASS_PARAMETER: f64 = 3.986_004e14;

/// Nominal jovian mass parameter `(GM)_J^N` in `m^3 s^-2`. Exact.
pub const NOMINAL_JOVIAN_MASS_PARAMETER: f64 = 1.266_865_3e17;

/// Nominal solar mass `M_sun^N = (GM)_sun^N / G` in `kg`.
///
/// Derived, not a direct IAU constant: the value follows from the *nominal*
/// mass parameter and the CODATA 2018 `G`. Recomputing it with a future `G`
/// changes it, which is the intended behaviour.
pub const NOMINAL_SOLAR_MASS: f64 = NOMINAL_SOLAR_MASS_PARAMETER / GRAVITATIONAL_CONSTANT;

/// Nominal terrestrial (Earth) mass `M_E^N = (GM)_E^N / G` in `kg`.
pub const NOMINAL_EARTH_MASS: f64 = NOMINAL_TERRESTRIAL_MASS_PARAMETER / GRAVITATIONAL_CONSTANT;

/// Nominal jovian (Jupiter) mass `M_J^N = (GM)_J^N / G` in `kg`.
pub const NOMINAL_JOVIAN_MASS: f64 = NOMINAL_JOVIAN_MASS_PARAMETER / GRAVITATIONAL_CONSTANT;

// ---------------------------------------------------------------------------
// Astronomical length units
// ---------------------------------------------------------------------------

/// Astronomical unit in `m`. Exact (IAU 2012 Resolution B2).
pub const ASTRONOMICAL_UNIT: f64 = 149_597_870_700.0;

/// Julian year in `s` (exactly 365.25 days of 86400 s).
pub const JULIAN_YEAR: f64 = 31_557_600.0;

/// Light-year in `m`: `c` times one Julian year.
///
/// Equals `9 460 730 472 580 800 m` exactly.
pub const LIGHT_YEAR: f64 = SPEED_OF_LIGHT * JULIAN_YEAR;

/// Parsec in `m`: `(648000/pi)` astronomical units.
///
/// Follows from the definition of the parsec as the distance at which one
/// astronomical unit subtends one arcsecond.
pub const PARSEC: f64 = 648_000.0 / PI * ASTRONOMICAL_UNIT;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schwarzschild_factor_matches_its_definition() {
        let expected = 2.0 * 6.674_30e-11 / (299_792_458.0_f64 * 299_792_458.0);
        assert!((SCHWARZSCHILD_RADIUS_PER_KILOGRAM - expected).abs() / expected < 1e-15);
        // Published value: 2G/c^2 = 1.4852e-27 m/kg
        assert!((SCHWARZSCHILD_RADIUS_PER_KILOGRAM - 1.485_2e-27).abs() / 1.485_2e-27 < 1e-4);
    }

    #[test]
    fn nominal_solar_mass_follows_from_mass_parameter() {
        // M_sun = 1.3271244e20 / 6.67430e-11 = 1.98841e30 kg
        assert!((NOMINAL_SOLAR_MASS - 1.988_41e30).abs() / 1.988_41e30 < 1e-5);
    }

    #[test]
    fn light_year_and_parsec_match_published_values() {
        assert!((LIGHT_YEAR - 9_460_730_472_580_800.0).abs() < 1.0);
        assert!((PARSEC - 3.085_677_6e16).abs() / 3.085_677_6e16 < 1e-6);
    }

    #[test]
    fn astronomical_unit_is_exact() {
        assert_eq!(ASTRONOMICAL_UNIT, 149_597_870_700.0);
    }
}
