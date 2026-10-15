//! Validated SI quantity types.
//!
//! # Why newtypes instead of `f64` or a units crate
//!
//! The brief requires that "physically invalid states" cannot be constructed
//! from unvalidated floating point values. A general-purpose dimension crate
//! (`uom` and friends) provides *dimensional* checking but happily stores a
//! negative mass: it cannot express "this mass must be positive and finite",
//! which is the actual invariant this crate needs. The types below therefore
//! own that invariant directly, at the cost of about a screen of code and zero
//! dependencies.
//!
//! # The invariant
//!
//! [`Length`], [`Mass`] and [`Density`] each store one `f64` in SI base units
//! (metre, kilogram, kilogram per cubic metre) and guarantee that the stored
//! value is **finite and strictly positive**. Zero is rejected, not clamped:
//!
//! * a zero mass or radius is not a physical object, it is the degenerate
//!   limit of the model;
//! * a zero density has no finite critical radius at all
//!   (`R_krit -> infinity`), so admitting it would force every consumer to
//!   handle a sentinel;
//! * rejecting zero removes every division-by-zero and `0/0` path from the
//!   physics functions.
//!
//! The consequence is that the physics functions never need to re-check their
//! inputs, and never return `NaN`.
//!
//! # Units
//!
//! Conversions are expressed through the `*Unit` enums rather than one
//! constructor per unit. That keeps every conversion factor in exactly one
//! place, and lets the CLI map user input to a unit via an exhaustive `match`
//! so that adding a unit cannot silently leave a parser branch unimplemented.

use crate::constants;
use crate::error::{PhysicsError, Quantity};

/// Rejects values that are not finite and strictly positive.
pub(crate) fn validate_positive(value: f64, quantity: Quantity) -> Result<(), PhysicsError> {
    if !value.is_finite() {
        return Err(PhysicsError::NotFinite { quantity, value });
    }
    if value <= 0.0 {
        return Err(PhysicsError::NotPositive { quantity, value });
    }
    Ok(())
}

/// Normalises a user-supplied unit token: lower case, no spaces or separators.
fn normalise(token: &str) -> String {
    token
        .chars()
        .filter(|c| !matches!(c, ' ' | '_' | '^' | '\t'))
        .flat_map(char::to_lowercase)
        .collect()
}

/// A length in metres.
///
/// Invariant: the stored value is finite and strictly positive. Construct one
/// with [`Length::from_in`]; the accessors return the value in a requested
/// unit.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Length(f64);

impl Length {
    /// Builds a length from `value` expressed in `unit`.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsError::NotFinite`] if the value, or the product of the
    /// value and the unit factor, is `NaN` or infinite, and
    /// [`PhysicsError::NotPositive`] if it is zero or negative.
    pub fn from_in(value: f64, unit: LengthUnit) -> Result<Self, PhysicsError> {
        let metres = value * unit.factor_to_si();
        validate_positive(metres, Quantity::Length)?;
        Ok(Self(metres))
    }

    /// Returns this length expressed in `unit`.
    ///
    /// # Range
    ///
    /// This is a plain `f64` conversion and is not range-checked: expressing a
    /// length within a few orders of magnitude of `f64::MAX` in a smaller unit
    /// overflows to infinity, e.g. `1e307 m` in centimetres. Use
    /// [`Length::metres`] — exact and unable to overflow — when the SI value is
    /// what is needed. The CLI uses the SI accessors for display for this
    /// reason.
    #[must_use]
    pub fn to_in(self, unit: LengthUnit) -> f64 {
        self.0 / unit.factor_to_si()
    }

    /// Returns this length in metres, the SI base unit.
    #[must_use]
    pub fn metres(self) -> f64 {
        self.0
    }
}

/// A mass in kilograms.
///
/// Invariant: the stored value is finite and strictly positive.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mass(f64);

impl Mass {
    /// Builds a mass from `value` expressed in `unit`.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsError::NotFinite`] if the value, or the product of the
    /// value and the unit factor, is `NaN` or infinite, and
    /// [`PhysicsError::NotPositive`] if it is zero or negative.
    pub fn from_in(value: f64, unit: MassUnit) -> Result<Self, PhysicsError> {
        let kilograms = value * unit.factor_to_si();
        validate_positive(kilograms, Quantity::Mass)?;
        Ok(Self(kilograms))
    }

    /// Returns this mass expressed in `unit`.
    ///
    /// # Range
    ///
    /// As for [`Length::to_in`], this conversion is not range-checked: a mass
    /// near `f64::MAX` expressed in grams overflows to infinity. Use
    /// [`Mass::kilograms`] when the SI value is what is needed.
    #[must_use]
    pub fn to_in(self, unit: MassUnit) -> f64 {
        self.0 / unit.factor_to_si()
    }

    /// Returns this mass in kilograms, the SI base unit.
    #[must_use]
    pub fn kilograms(self) -> f64 {
        self.0
    }
}

/// A mass density in kilograms per cubic metre.
///
/// Invariant: the stored value is finite and strictly positive.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Density(f64);

impl Density {
    /// Builds a density from `value` expressed in `unit`.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsError::NotFinite`] if the value, or the product of the
    /// value and the unit factor, is `NaN` or infinite, and
    /// [`PhysicsError::NotPositive`] if it is zero or negative.
    pub fn from_in(value: f64, unit: DensityUnit) -> Result<Self, PhysicsError> {
        let si = value * unit.factor_to_si();
        validate_positive(si, Quantity::Density)?;
        Ok(Self(si))
    }

    /// Returns this density expressed in `unit`.
    ///
    /// Every density unit is a multiple of the SI unit, so this conversion
    /// divides and cannot overflow; a subnormal density can underflow to zero.
    #[must_use]
    pub fn to_in(self, unit: DensityUnit) -> f64 {
        self.0 / unit.factor_to_si()
    }

    /// Returns this density in kilograms per cubic metre, the SI derived unit.
    #[must_use]
    pub fn kilograms_per_cubic_metre(self) -> f64 {
        self.0
    }
}

/// Units of length accepted by [`Length::from_in`] and [`Length::to_in`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LengthUnit {
    /// Metre (`m`), the SI base unit.
    Metre,
    /// Kilometre (`km`).
    Kilometre,
    /// Centimetre (`cm`).
    Centimetre,
    /// Astronomical unit (`au`), exact (IAU 2012 B2).
    AstronomicalUnit,
    /// Nominal solar radius (`Rsun`), exact (IAU 2015 B3).
    SolarRadius,
    /// Nominal Earth equatorial radius (`Rearth`), exact (IAU 2015 B3).
    EarthEquatorialRadius,
    /// Nominal Jupiter equatorial radius (`Rjup`), exact (IAU 2015 B3).
    JovianEquatorialRadius,
    /// Light-year (`ly`), `c` times one Julian year.
    LightYear,
    /// Parsec (`pc`), `648000/pi` astronomical units.
    Parsec,
}

impl LengthUnit {
    /// Every supported length unit, for iteration by user interfaces.
    pub const ALL: [Self; 9] = [
        Self::Metre,
        Self::Kilometre,
        Self::Centimetre,
        Self::AstronomicalUnit,
        Self::SolarRadius,
        Self::EarthEquatorialRadius,
        Self::JovianEquatorialRadius,
        Self::LightYear,
        Self::Parsec,
    ];

    /// The canonical short symbol, e.g. `"km"`.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Metre => "m",
            Self::Kilometre => "km",
            Self::Centimetre => "cm",
            Self::AstronomicalUnit => "au",
            Self::SolarRadius => "Rsun",
            Self::EarthEquatorialRadius => "Rearth",
            Self::JovianEquatorialRadius => "Rjup",
            Self::LightYear => "ly",
            Self::Parsec => "pc",
        }
    }

    /// The number of metres in one of this unit.
    #[must_use]
    pub const fn factor_to_si(self) -> f64 {
        match self {
            Self::Metre => 1.0,
            Self::Kilometre => 1.0e3,
            Self::Centimetre => 1.0e-2,
            Self::AstronomicalUnit => constants::ASTRONOMICAL_UNIT,
            Self::SolarRadius => constants::NOMINAL_SOLAR_RADIUS,
            Self::EarthEquatorialRadius => constants::NOMINAL_EARTH_EQUATORIAL_RADIUS,
            Self::JovianEquatorialRadius => constants::NOMINAL_JOVIAN_EQUATORIAL_RADIUS,
            Self::LightYear => constants::LIGHT_YEAR,
            Self::Parsec => constants::PARSEC,
        }
    }

    /// Parses a user-supplied token, tolerating case, spaces and underscores.
    ///
    /// Accepts both the canonical symbol and common aliases, so that `"km"`,
    /// `"KM"`, `"r_sun"`, `"rsun"` and `"R_sun"` all resolve.
    #[must_use]
    pub fn from_symbol(token: &str) -> Option<Self> {
        match normalise(token).as_str() {
            "m" | "metre" | "meter" | "metres" | "meters" => Some(Self::Metre),
            "km" | "kilometre" | "kilometer" | "kilometres" | "kilometers" => Some(Self::Kilometre),
            "cm" | "centimetre" | "centimeter" | "centimetres" | "centimeters" => {
                Some(Self::Centimetre)
            }
            "au" | "astronomicalunit" | "astronomicalunits" | "ae" => Some(Self::AstronomicalUnit),
            "rsun" | "solradius" | "solarradius" | "rsolar" => Some(Self::SolarRadius),
            "rearth" | "earthradius" | "rterrestrial" => Some(Self::EarthEquatorialRadius),
            "rjup" | "jovianradius" | "jupiterradius" => Some(Self::JovianEquatorialRadius),
            "ly" | "lightyear" | "lightyears" => Some(Self::LightYear),
            "pc" | "parsec" | "parsecs" => Some(Self::Parsec),
            _ => None,
        }
    }
}

/// Units of mass accepted by [`Mass::from_in`] and [`Mass::to_in`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MassUnit {
    /// Kilogram (`kg`), the SI base unit.
    Kilogram,
    /// Gram (`g`).
    Gram,
    /// Metric tonne (`t`).
    Tonne,
    /// Nominal solar mass (`Msun`), derived from IAU 2015 B3 and `G`.
    SolarMass,
    /// Nominal Earth mass (`Mearth`), derived from IAU 2015 B3 and `G`.
    EarthMass,
    /// Nominal Jupiter mass (`Mjup`), derived from IAU 2015 B3 and `G`.
    JovianMass,
}

impl MassUnit {
    /// Every supported mass unit.
    pub const ALL: [Self; 6] = [
        Self::Kilogram,
        Self::Gram,
        Self::Tonne,
        Self::SolarMass,
        Self::EarthMass,
        Self::JovianMass,
    ];

    /// The canonical short symbol, e.g. `"Msun"`.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Kilogram => "kg",
            Self::Gram => "g",
            Self::Tonne => "t",
            Self::SolarMass => "Msun",
            Self::EarthMass => "Mearth",
            Self::JovianMass => "Mjup",
        }
    }

    /// The number of kilograms in one of this unit.
    #[must_use]
    pub const fn factor_to_si(self) -> f64 {
        match self {
            Self::Kilogram => 1.0,
            Self::Gram => 1.0e-3,
            Self::Tonne => 1.0e3,
            Self::SolarMass => constants::NOMINAL_SOLAR_MASS,
            Self::EarthMass => constants::NOMINAL_EARTH_MASS,
            Self::JovianMass => constants::NOMINAL_JOVIAN_MASS,
        }
    }

    /// Parses a user-supplied token, tolerating case, spaces and underscores.
    #[must_use]
    pub fn from_symbol(token: &str) -> Option<Self> {
        match normalise(token).as_str() {
            "kg" | "kilogram" | "kilograms" => Some(Self::Kilogram),
            "g" | "gram" | "grams" => Some(Self::Gram),
            "t" | "tonne" | "tonnes" | "ton" | "tons" => Some(Self::Tonne),
            "msun" | "solarmass" | "solarmasses" | "solar" => Some(Self::SolarMass),
            "mearth" | "earthmass" | "earthmasses" | "earth" => Some(Self::EarthMass),
            "mjup" | "jupitermass" | "jovianmass" | "jupiter" => Some(Self::JovianMass),
            _ => None,
        }
    }
}

/// Units of density accepted by [`Density::from_in`] and [`Density::to_in`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DensityUnit {
    /// Kilogram per cubic metre (`kg/m3`), the SI derived unit.
    KilogramPerCubicMetre,
    /// Gram per cubic centimetre (`g/cm3`), the common geology unit.
    GramPerCubicCentimetre,
}

impl DensityUnit {
    /// Every supported density unit.
    pub const ALL: [Self; 2] = [Self::KilogramPerCubicMetre, Self::GramPerCubicCentimetre];

    /// The canonical short symbol, e.g. `"kg/m3"`.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::KilogramPerCubicMetre => "kg/m3",
            Self::GramPerCubicCentimetre => "g/cm3",
        }
    }

    /// The number of kilograms per cubic metre in one of this unit.
    #[must_use]
    pub const fn factor_to_si(self) -> f64 {
        match self {
            Self::KilogramPerCubicMetre => 1.0,
            Self::GramPerCubicCentimetre => 1.0e3,
        }
    }

    /// Parses a user-supplied token, tolerating case, spaces and underscores.
    #[must_use]
    pub fn from_symbol(token: &str) -> Option<Self> {
        match normalise(token).as_str() {
            "kg/m3" | "kgm3" | "kgperm3" | "kgpercubicmetre" => Some(Self::KilogramPerCubicMetre),
            "g/cm3" | "gcm3" | "gpercm3" | "gpercubiccentimetre" => {
                Some(Self::GramPerCubicCentimetre)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_positive_and_non_finite() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(Length::from_in(bad, LengthUnit::Metre).is_err(), "{bad}");
            assert!(Mass::from_in(bad, MassUnit::Kilogram).is_err(), "{bad}");
            assert!(
                Density::from_in(bad, DensityUnit::KilogramPerCubicMetre).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn conversions_round_trip() {
        for unit in LengthUnit::ALL {
            let len = Length::from_in(3.5, unit).expect("valid");
            assert!((len.to_in(unit) - 3.5).abs() / 3.5 < 1e-15, "{unit:?}");
            assert!((len.metres() - 3.5 * unit.factor_to_si()).abs() / len.metres() < 1e-15);
        }
        for unit in MassUnit::ALL {
            let mass = Mass::from_in(3.5, unit).expect("valid");
            assert!((mass.to_in(unit) - 3.5).abs() / 3.5 < 1e-15, "{unit:?}");
        }
        for unit in DensityUnit::ALL {
            let rho = Density::from_in(3.5, unit).expect("valid");
            assert!((rho.to_in(unit) - 3.5).abs() / 3.5 < 1e-15, "{unit:?}");
        }
    }

    #[test]
    fn unit_symbols_round_trip() {
        for unit in LengthUnit::ALL {
            assert_eq!(LengthUnit::from_symbol(unit.symbol()), Some(unit));
        }
        for unit in MassUnit::ALL {
            assert_eq!(MassUnit::from_symbol(unit.symbol()), Some(unit));
        }
        for unit in DensityUnit::ALL {
            assert_eq!(DensityUnit::from_symbol(unit.symbol()), Some(unit));
        }
    }

    #[test]
    fn symbol_lookup_is_lenient_about_style() {
        assert_eq!(LengthUnit::from_symbol("KM"), Some(LengthUnit::Kilometre));
        assert_eq!(
            LengthUnit::from_symbol("r_sun"),
            Some(LengthUnit::SolarRadius)
        );
        assert_eq!(
            DensityUnit::from_symbol(" g/cm^3 "),
            Some(DensityUnit::GramPerCubicCentimetre)
        );
        assert_eq!(MassUnit::from_symbol("MSun"), Some(MassUnit::SolarMass));
        assert_eq!(LengthUnit::from_symbol("furlong"), None);
    }

    #[test]
    fn gram_per_cubic_centimetre_is_a_thousand_si() {
        let rho = Density::from_in(1.0, DensityUnit::GramPerCubicCentimetre).expect("valid");
        assert_eq!(rho.kilograms_per_cubic_metre(), 1000.0);
    }

    #[test]
    fn overflow_on_conversion_is_reported_as_not_finite() {
        // 1e308 light-years overflows when multiplied by the ly factor.
        let err = Length::from_in(1.0e308, LengthUnit::LightYear).unwrap_err();
        assert!(matches!(err, PhysicsError::NotFinite { .. }));
    }
}
