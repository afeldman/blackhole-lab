//! Parsing of user-supplied numbers and unit tokens.
//!
//! Kept separate from the prompts so that the parsers can be tested directly,
//! and so that the interactive and non-interactive paths share exactly one
//! implementation of "what does this input mean".

use blackhole_core::{DensityUnit, LengthUnit, MassUnit};

use crate::catalog;

/// Parses a plain decimal or scientific-notation number.
///
/// A decimal comma is accepted for convenience, but only when it is
/// unambiguous: a string containing both `,` and `.` is rejected rather than
/// guessed at, because `1,234` means different things in different locales.
pub fn parse_number(raw: &str) -> Result<f64, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("no value given".to_string());
    }
    let has_comma = trimmed.contains(',');
    let has_point = trimmed.contains('.');
    let normalised = if has_comma && has_point {
        return Err(format!(
            "`{trimmed}` mixes `,` and `.`; write the decimal separator only once"
        ));
    } else if has_comma {
        trimmed.replace(',', ".")
    } else {
        trimmed.to_string()
    };
    normalised
        .parse::<f64>()
        .map_err(|_| format!("`{trimmed}` is not a number"))
}

/// Parses `"<number> [unit]"`, returning the value and the unit token if one
/// was given inline.
pub fn parse_value_with_optional_unit(raw: &str) -> Result<(f64, Option<String>), String> {
    let trimmed = raw.trim();
    let mut parts = trimmed.split_whitespace();
    let value = parts
        .next()
        .ok_or_else(|| "no value given".to_string())
        .and_then(parse_number)?;
    let unit = match parts.next() {
        Some(token) if parts.next().is_none() => Some(token.to_string()),
        Some(_) => return Err(format!("`{trimmed}` has more than one unit token")),
        None => None,
    };
    Ok((value, unit))
}

/// Resolves a length unit from an optional inline token, the dedicated unit
/// prompt answer, or the default.
pub fn resolve_length_unit(
    inline: Option<&str>,
    prompted: Option<&str>,
) -> Result<LengthUnit, String> {
    for token in [inline, prompted].into_iter().flatten() {
        if token.trim().is_empty() {
            continue;
        }
        return LengthUnit::from_symbol(token).ok_or_else(|| {
            format!(
                "unknown length unit `{}`; supported: {}",
                token.trim(),
                unit_list(LengthUnit::ALL.iter().map(|u| u.symbol()))
            )
        });
    }
    Ok(LengthUnit::Metre)
}

/// Resolves a mass unit from an optional inline token, the dedicated unit
/// prompt answer, or the default.
pub fn resolve_mass_unit(inline: Option<&str>, prompted: Option<&str>) -> Result<MassUnit, String> {
    for token in [inline, prompted].into_iter().flatten() {
        if token.trim().is_empty() {
            continue;
        }
        return MassUnit::from_symbol(token).ok_or_else(|| {
            format!(
                "unknown mass unit `{}`; supported: {}",
                token.trim(),
                unit_list(MassUnit::ALL.iter().map(|u| u.symbol()))
            )
        });
    }
    Ok(MassUnit::Kilogram)
}

/// Resolves a density specification: either a material name or
/// `"<number> [unit]"`.
///
/// Returns the density in kg/m3 and a human-readable description of what was
/// interpreted, so the report can echo the assumption back to the user.
pub fn resolve_density(raw: &str) -> Result<(f64, String), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("no density given".to_string());
    }
    if let Some(material) = catalog::find_material(trimmed) {
        return Ok((
            material.density_kg_m3,
            format!(
                "{} ({}: {})",
                material.name,
                material.provenance.label(),
                material.provenance.detail()
            ),
        ));
    }
    let (value, unit_token) = parse_value_with_optional_unit(trimmed).map_err(|message| {
        // The token was neither a known material nor a parsable number, so say
        // both instead of reporting only the parse failure.
        format!("{message}; known materials: {}", material_names())
    })?;
    let unit = match unit_token.as_deref() {
        Some(token) => DensityUnit::from_symbol(token).ok_or_else(|| {
            format!(
                "unknown density unit `{token}`; supported: {}. Known materials: {}",
                unit_list(DensityUnit::ALL.iter().map(|u| u.symbol())),
                material_names()
            )
        })?,
        None => DensityUnit::KilogramPerCubicMetre,
    };
    let si = value * unit.factor_to_si();
    Ok((si, format!("{value} {}", unit.symbol())))
}

/// Comma-separated list of the built-in material names.
#[must_use]
pub fn material_names() -> String {
    catalog::MATERIALS
        .iter()
        .map(|material| material.name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Joins unit symbols for an error message.
fn unit_list<'a>(symbols: impl Iterator<Item = &'a str>) -> String {
    symbols.collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_parse_in_common_notations() {
        assert_eq!(parse_number("1000").unwrap(), 1000.0);
        assert_eq!(parse_number(" 1.989e30 ").unwrap(), 1.989e30);
        assert_eq!(parse_number("1E-3").unwrap(), 1e-3);
        assert_eq!(parse_number("-2.5").unwrap(), -2.5);
        assert_eq!(parse_number("1,5").unwrap(), 1.5);
    }

    #[test]
    fn ambiguous_or_invalid_numbers_are_rejected() {
        assert!(parse_number("").is_err());
        assert!(parse_number("1,234.5").is_err());
        assert!(parse_number("twelve").is_err());
        assert!(parse_number("1e").is_err());
    }

    #[test]
    fn value_and_unit_split_correctly() {
        assert_eq!(
            parse_value_with_optional_unit("6371 km").unwrap(),
            (6371.0, Some("km".to_string()))
        );
        assert_eq!(
            parse_value_with_optional_unit("6371").unwrap(),
            (6371.0, None)
        );
        assert!(parse_value_with_optional_unit("6371 km s").is_err());
    }

    #[test]
    fn units_default_to_si_and_reject_nonsense() {
        assert_eq!(resolve_length_unit(None, None).unwrap(), LengthUnit::Metre);
        assert_eq!(
            resolve_length_unit(Some("km"), None).unwrap(),
            LengthUnit::Kilometre
        );
        assert_eq!(
            resolve_length_unit(None, Some("au")).unwrap(),
            LengthUnit::AstronomicalUnit
        );
        // An inline token wins over the prompted one.
        assert_eq!(
            resolve_length_unit(Some("cm"), Some("km")).unwrap(),
            LengthUnit::Centimetre
        );
        assert!(resolve_length_unit(Some("furlong"), None).is_err());
        assert_eq!(resolve_mass_unit(None, None).unwrap(), MassUnit::Kilogram);
        assert!(resolve_mass_unit(Some("stone"), None).is_err());
    }

    #[test]
    fn density_accepts_material_names_and_units() {
        let (water, description) = resolve_density("water").unwrap();
        assert_eq!(water, 1000.0);
        assert!(description.contains("model assumption"));

        let (si, _) = resolve_density("2700").unwrap();
        assert_eq!(si, 2700.0);

        let (cgs, _) = resolve_density("2.7 g/cm3").unwrap();
        assert!((cgs - 2700.0).abs() < 1e-9);

        assert!(resolve_density("unobtainium").is_err());
        assert!(resolve_density("").is_err());
    }
}
