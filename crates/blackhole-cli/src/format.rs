//! Deterministic number, length, mass and density formatting.
//!
//! # Policy
//!
//! Every primary value is printed in scientific notation with seven
//! significant digits (`{:.6e}`). That is uniform, unambiguous about magnitude,
//! and avoids the readability cliff that a fixed/scientific switch introduces at
//! arbitrary thresholds. Where a more intuitive unit exists, a secondary
//! *hint* is appended in parentheses.
//!
//! The hint unit is chosen by a fixed ladder of thresholds, so the same number
//! always renders the same way — the CLI's output is byte-reproducible, which
//! the test suite asserts.

use blackhole_core::constants::{
    ASTRONOMICAL_UNIT, NOMINAL_EARTH_EQUATORIAL_RADIUS, NOMINAL_EARTH_MASS, NOMINAL_SOLAR_MASS,
    NOMINAL_SOLAR_RADIUS, PARSEC,
};

/// Renders a dimensionless or SI value in scientific notation.
///
/// Uses Rust's `LowerExp` formatting, so the exponent carries a sign only when
/// it is negative and is not zero-padded: `5.972200e24`, `8.870100e-3`. That is
/// the convention of the Rust ecosystem and therefore what any downstream tool
/// parsing this output already expects.
#[must_use]
pub fn number(value: f64) -> String {
    format!("{value:.6e}")
}

/// A length paired with the most readable unit for its magnitude.
///
/// Returns `(value, symbol)`; the ladder is, from the top down: parsec,
/// astronomical unit, nominal solar radius, nominal Earth radius, kilometre,
/// metre, millimetre, micrometre, nanometre.
#[must_use]
fn length_hint(metres: f64) -> (&'static str, f64) {
    const LADDER: [(f64, &str, f64); 9] = [
        (PARSEC, "pc", PARSEC),
        (ASTRONOMICAL_UNIT, "au", ASTRONOMICAL_UNIT),
        (NOMINAL_SOLAR_RADIUS, "Rsun", NOMINAL_SOLAR_RADIUS),
        (
            NOMINAL_EARTH_EQUATORIAL_RADIUS,
            "Rearth",
            NOMINAL_EARTH_EQUATORIAL_RADIUS,
        ),
        (1.0e3, "km", 1.0e3),
        (1.0, "m", 1.0),
        (1.0e-3, "mm", 1.0e-3),
        (1.0e-6, "um", 1.0e-6),
        (1.0e-9, "nm", 1.0e-9),
    ];
    for (threshold, symbol, factor) in LADDER {
        if metres >= threshold {
            return (symbol, metres / factor);
        }
    }
    ("m", metres)
}

/// Renders a length as `"<sci> m (= <hint>)"`.
#[must_use]
pub fn length(metres: f64) -> String {
    let (symbol, value) = length_hint(metres);
    if symbol == "m" {
        format!("{} m", number(metres))
    } else {
        format!("{} m (= {} {})", number(metres), number(value), symbol)
    }
}

/// Renders a mass as `"<sci> kg (= <hint>)"` where a familiar unit applies.
///
/// The rung switches at *half* a familiar mass rather than at exactly one, so
/// that a value quoted to a few significant figures lands on the rung a reader
/// expects. This matters in practice: the tabulated solar mass of 1.9884e30 kg
/// sits marginally *below* the IAU nominal solar mass of 1.98841e30 kg, and an
/// exact threshold would render the Sun in Earth masses. Below the switch the
/// coefficient stays above 0.5, so the hint never reads worse than the unit
/// beneath it.
#[must_use]
pub fn mass(kilograms: f64) -> String {
    if kilograms >= 0.5 * NOMINAL_SOLAR_MASS {
        format!(
            "{} kg (= {} Msun)",
            number(kilograms),
            number(kilograms / NOMINAL_SOLAR_MASS)
        )
    } else if kilograms >= 0.5 * NOMINAL_EARTH_MASS {
        format!(
            "{} kg (= {} Mearth)",
            number(kilograms),
            number(kilograms / NOMINAL_EARTH_MASS)
        )
    } else if kilograms >= 1.0e3 {
        format!(
            "{} kg (= {} t)",
            number(kilograms),
            number(kilograms / 1.0e3)
        )
    } else {
        format!("{} kg", number(kilograms))
    }
}

/// Renders a density as `"<sci> kg/m3 (= <hint> g/cm3)"` for geological scales.
#[must_use]
pub fn density(kilograms_per_cubic_metre: f64) -> String {
    if kilograms_per_cubic_metre >= 100.0 {
        format!(
            "{} kg/m3 (= {} g/cm3)",
            number(kilograms_per_cubic_metre),
            number(kilograms_per_cubic_metre / 1.0e3)
        )
    } else {
        format!("{} kg/m3", number(kilograms_per_cubic_metre))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_use_seven_significant_digits() {
        assert_eq!(number(0.0), "0.000000e0");
        assert_eq!(number(1.0), "1.000000e0");
        assert_eq!(number(5.972_2e24), "5.972200e24");
        assert_eq!(number(8.870_1e-3), "8.870100e-3");
    }

    #[test]
    fn length_hint_ladder_picks_sensible_units() {
        assert!(length(8.870_1e-3).ends_with("mm)"));
        assert!(length(2953.4).ends_with("km)"));
        // Earth's volumetric mean radius (6371.0 km) sits just below the
        // nominal Earth equatorial radius, so the kilometre rung is correct.
        assert!(length(6.371e6).ends_with("km)"));
        assert!(length(6.4e6).ends_with("Rearth)"));
        assert!(length(6.957e8).ends_with("Rsun)"));
        assert!(length(4.009_2e11).ends_with("au)"));
        assert!(length(3.085_7e16).ends_with("pc)"));
        // Below the nanometre rung the unit stays a plain metre.
        assert!(!length(1.0e-12).contains("(= "));
    }

    #[test]
    fn mass_hint_switches_at_solar_and_terrestrial_scale() {
        // 1.9884e30 kg is the tabulated solar mass, marginally below the IAU
        // nominal value; the half-unit threshold keeps it on the Msun rung.
        assert!(mass(1.988_4e30).contains("Msun"));
        assert!(mass(5.972_2e24).contains("Mearth"));
        assert!(mass(4188.79).ends_with("t)"));
        assert!(!mass(1.0).contains("(= "));
    }

    #[test]
    fn density_hint_only_for_condensed_matter() {
        assert!(density(5513.0).contains("g/cm3"));
        assert!(!density(1.225).contains("(= "));
    }
}
