//! Reference data and model assumptions offered by the CLI.
//!
//! # Two categories, never mixed
//!
//! * **Reference data** — a published measurement of a specific object, with
//!   the source named. The numbers may be compared with the literature.
//! * **Model assumption** — a representative or typical value chosen for an
//!   illustrative calculation. It is not a measurement of any particular
//!   sample, and no uncertainty is attached to it here.
//!
//! The classification is carried in the type system (it is a field, not a
//! convention) and printed with every result, because presenting a plausible
//! number as a measurement is the failure mode this project has to avoid.
//!
//! # Where the numbers come from
//!
//! Masses and radii are taken from the NASA NSSDCA planetary fact sheets
//! (NASA Goddard Space Flight Center, sheets last updated 2024) and, for the
//! Galactic Centre black hole, from the GRAVITY Collaboration's interferometric
//! astrometry. Solar and terrestrial unit conversions use the IAU 2015
//! Resolution B3 nominal constants, which are exact by definition — see
//! [`blackhole_core::constants`].
//!
//! Where the fact sheet lists a *volumetric mean radius*, that is the value
//! used; it is the radius that is consistent with `M = (4/3) pi R^3 rho_mean`
//! and therefore the one this model needs. The MEAN density is deliberately
//! **not** copied from the fact sheet: it is derived by the crate from mass and
//! radius, so that the two are guaranteed to be mutually consistent.

use blackhole_core::constants::NOMINAL_SOLAR_MASS;

/// Where a catalogued value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// A published measurement; the payload names the source.
    Reference(&'static str),
    /// An illustrative value; the payload says on what basis it was chosen.
    ModelAssumption(&'static str),
}

impl Provenance {
    /// Short label used in printed output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reference(_) => "reference data",
            Self::ModelAssumption(_) => "model assumption",
        }
    }

    /// The full source or rationale string.
    #[must_use]
    pub const fn detail(self) -> &'static str {
        match self {
            Self::Reference(detail) | Self::ModelAssumption(detail) => detail,
        }
    }
}

/// An astronomical object with a published mass and, usually, a published
/// radius.
#[derive(Debug, Clone, Copy)]
pub struct AstronomicalObject {
    /// Display name.
    pub name: &'static str,
    /// Mass in kilograms.
    pub mass_kg: f64,
    /// Volumetric mean radius in metres, or `None` when the object has no
    /// material surface (a black hole).
    pub radius_m: Option<f64>,
    /// Classification and source.
    pub provenance: Provenance,
    /// Anything the reader must know to avoid over-interpreting the numbers.
    pub note: &'static str,
}

/// A material with a representative density.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    /// Display name.
    pub name: &'static str,
    /// Density in kilograms per cubic metre.
    pub density_kg_m3: f64,
    /// Classification and rationale.
    pub provenance: Provenance,
}

/// The built-in astronomical objects.
///
/// Order is the display order. The list is intentionally short and every entry
/// is traceable; extending it means adding a row with its source, not guessing.
pub const ASTRONOMICAL_OBJECTS: &[AstronomicalObject] = &[
    AstronomicalObject {
        name: "Sun",
        mass_kg: 1.988_4e30,
        radius_m: Some(6.957e8),
        provenance: Provenance::Reference(
            "NASA NSSDCA Sun Fact Sheet (2024): mass 1.9884e30 kg, volumetric mean radius 695 700 km",
        ),
        note: "Volumetric mean radius; the photospheric radius is used, not the equatorial one.",
    },
    AstronomicalObject {
        name: "Earth",
        mass_kg: 5.972_2e24,
        radius_m: Some(6.371e6),
        provenance: Provenance::Reference(
            "NASA NSSDCA Earth Fact Sheet (2024): mass 5.9722e24 kg, volumetric mean radius 6371.000 km",
        ),
        note: "",
    },
    AstronomicalObject {
        name: "Moon",
        mass_kg: 7.346e22,
        radius_m: Some(1.737_4e6),
        provenance: Provenance::Reference(
            "NASA NSSDCA Moon Fact Sheet (2024): mass 0.07346e24 kg, volumetric mean radius 1737.4 km",
        ),
        note: "",
    },
    AstronomicalObject {
        name: "Jupiter",
        mass_kg: 1.898_13e27,
        radius_m: Some(6.991_1e7),
        provenance: Provenance::Reference(
            "NASA NSSDCA Jupiter Fact Sheet (2024): mass 1.89813e27 kg, volumetric mean radius 69911 km",
        ),
        note: "Volumetric mean radius; the 1-bar equatorial radius is 71 492 km.",
    },
    AstronomicalObject {
        name: "Sagittarius A*",
        mass_kg: 4.297e6 * NOMINAL_SOLAR_MASS,
        radius_m: None,
        provenance: Provenance::Reference(
            "GRAVITY Collaboration et al. (2022), A&A 657, L12, doi:10.1051/0004-6361/202142465: M = (4.297 +/- 0.012)e6 Msun",
        ),
        note: "No material surface exists: the Schwarzschild radius is the only length defined for it.",
    },
    AstronomicalObject {
        name: "neutron star (representative)",
        mass_kg: 1.4 * NOMINAL_SOLAR_MASS,
        radius_m: Some(1.2e4),
        provenance: Provenance::ModelAssumption(
            "Representative values (1.4 Msun, R = 12 km), not a measurement of a specific object",
        ),
        note: "Observed neutron-star radii span roughly 10-13 km and are not sharply constrained.",
    },
];

/// Representative material densities, for use wherever a density is requested.
///
/// These are **model assumptions**. Rock and ice in particular vary widely
/// between samples; the entries name a representative value and the range it
/// sits in.
pub const MATERIALS: &[Material] = &[
    Material {
        name: "water",
        density_kg_m3: 1000.0,
        provenance: Provenance::ModelAssumption(
            "Nominal 1000 kg/m3; liquid water at 4 degC is 999.97 kg/m3",
        ),
    },
    Material {
        name: "ice",
        density_kg_m3: 917.0,
        provenance: Provenance::ModelAssumption(
            "Bubble-free freshwater ice near 0 degC; sea ice is appreciably lower",
        ),
    },
    Material {
        name: "rock",
        density_kg_m3: 2700.0,
        provenance: Provenance::ModelAssumption(
            "Generic crustal rock. Common range 2400-3000 kg/m3; see granite and basalt",
        ),
    },
    Material {
        name: "granite",
        density_kg_m3: 2700.0,
        provenance: Provenance::ModelAssumption(
            "Typical granite; individual samples span roughly 2600-2800 kg/m3",
        ),
    },
    Material {
        name: "basalt",
        density_kg_m3: 2900.0,
        provenance: Provenance::ModelAssumption(
            "Typical basalt; individual samples span roughly 2800-3000 kg/m3",
        ),
    },
    Material {
        name: "iron",
        density_kg_m3: 7874.0,
        provenance: Provenance::ModelAssumption(
            "Pure solid iron at room temperature; liquid iron is slightly less dense",
        ),
    },
    Material {
        name: "air (sea level, 15 degC)",
        density_kg_m3: 1.225,
        provenance: Provenance::ModelAssumption("ISA standard atmosphere at sea level, 15 degC"),
    },
];

/// Normalises a lookup key: lower case, with separators and punctuation
/// removed, so that `"Sagittarius A*"`, `"sgr a*"` and `"sgra"` agree.
fn normalise(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, ' ' | '_' | '-' | '*' | '\t' | '\''))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Looks up an astronomical object by name, tolerating case and punctuation.
#[must_use]
pub fn find_object(name: &str) -> Option<&'static AstronomicalObject> {
    let key = normalise(name);
    let alias: Option<&str> = match key.as_str() {
        "sol" => Some("sun"),
        "luna" => Some("moon"),
        "sgra" | "sagittariusa" | "sgr" | "galacticcenter" | "milkywayblackhole" => {
            Some("sagittarius")
        }
        "neutronstar" | "neutronstarrepresentative" => Some("neutronstar"),
        _ => None,
    };
    let wanted = alias.unwrap_or(key.as_str());
    // Two characters minimum: a single letter is not a name, it is an
    // ambiguity ("s" matches both Sun and Sagittarius A*).
    if wanted.len() < 2 {
        return None;
    }
    ASTRONOMICAL_OBJECTS
        .iter()
        .find(|object| normalise(object.name).starts_with(wanted))
}

/// Looks up a material density by name, tolerating case and punctuation.
#[must_use]
pub fn find_material(name: &str) -> Option<&'static Material> {
    let key = normalise(name);
    // The same two-character minimum as `find_object`. Without it, a single
    // letter silently resolves to whichever material comes first in the table:
    // `i` would give ice rather than iron, which is worse than no match at all.
    if key.len() < 2 {
        return None;
    }
    MATERIALS
        .iter()
        .find(|material| normalise(material.name).starts_with(&key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackhole_core::{Density, DensityUnit, Length, LengthUnit, Mass, MassUnit};

    #[test]
    fn every_catalogued_value_is_a_valid_quantity() {
        // Catches a typo (negative, zero, NaN) in the table at test time rather
        // than at the moment a user selects the entry.
        for object in ASTRONOMICAL_OBJECTS {
            assert!(
                Mass::from_in(object.mass_kg, MassUnit::Kilogram).is_ok(),
                "{} mass {}",
                object.name,
                object.mass_kg
            );
            if let Some(radius) = object.radius_m {
                assert!(
                    Length::from_in(radius, LengthUnit::Metre).is_ok(),
                    "{} radius {}",
                    object.name,
                    radius
                );
            }
            assert!(
                !object.provenance.detail().is_empty(),
                "{} has no provenance",
                object.name
            );
        }
        for material in MATERIALS {
            assert!(
                Density::from_in(material.density_kg_m3, DensityUnit::KilogramPerCubicMetre)
                    .is_ok(),
                "{} density {}",
                material.name,
                material.density_kg_m3
            );
            assert!(
                !material.provenance.detail().is_empty(),
                "{} has no provenance",
                material.name
            );
        }
    }

    #[test]
    fn object_lookup_accepts_aliases_and_style_variants() {
        assert_eq!(find_object("Earth").map(|o| o.name), Some("Earth"));
        assert_eq!(find_object("earth").map(|o| o.name), Some("Earth"));
        assert_eq!(find_object("Sol").map(|o| o.name), Some("Sun"));
        assert_eq!(find_object("Luna").map(|o| o.name), Some("Moon"));
        assert_eq!(
            find_object("Sagittarius A*").map(|o| o.name),
            Some("Sagittarius A*")
        );
        assert_eq!(
            find_object("sgr a*").map(|o| o.name),
            Some("Sagittarius A*")
        );
        assert!(find_object("Vulcan").is_none());
    }

    #[test]
    fn material_lookup_uses_prefixes() {
        assert_eq!(
            find_material("granite").map(|m| m.density_kg_m3),
            Some(2700.0)
        );
        assert_eq!(find_material("WATER").map(|m| m.name), Some("water"));
        assert!(find_material("unobtainium").is_none());
    }

    #[test]
    fn a_single_character_material_name_is_rejected() {
        // Without a two-character minimum, `i` would silently resolve to ice,
        // which precedes iron in the table, so a user asking for iron would get
        // ice. A clear "no match" is the correct answer.
        assert!(find_material("i").is_none());
        assert!(find_material("w").is_none());
        assert!(find_material("r").is_none());
        // Two characters disambiguate every entry in the table.
        assert_eq!(find_material("ir").map(|m| m.name), Some("iron"));
        assert_eq!(find_material("ic").map(|m| m.name), Some("ice"));
    }

    #[test]
    fn only_the_galactic_centre_object_has_no_radius() {
        let without: Vec<&str> = ASTRONOMICAL_OBJECTS
            .iter()
            .filter(|o| o.radius_m.is_none())
            .map(|o| o.name)
            .collect();
        assert_eq!(without, vec!["Sagittarius A*"]);
    }
}
