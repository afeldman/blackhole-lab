//! Computing a result and rendering it with its physical interpretation.
//!
//! Every number in the output is computed from the input; there are no
//! placeholders. The limitations block is printed unconditionally, because the
//! misreading it guards against — taking a model output for a physical
//! prediction — does not depend on the input.

use std::fmt::Write as _;

use blackhole_core::{
    Density, Length, Mass, PhysicsError, compactness, mass_from_radius_density, mean_density,
    minimum_radius_at_density, radius_from_mass_density, schwarzschild_radius,
};

use crate::catalog::{AstronomicalObject, Provenance};
use crate::format;

/// The model limitations printed with every result.
///
/// These are not boilerplate. Each entry names a way in which the numbers above
/// it can be misread, and each one is a consequence of the difference between
/// the model (a static, homogeneous sphere in flat space) and the physical
/// situation it evokes.
pub const LIMITATIONS: [&str; 5] = [
    "Every mass has a Schwarzschild radius r_s = 2 G M / c^2, so there is no universal minimum mass for a black hole. A mass and an r_s say nothing on their own about whether a horizon exists.",
    "The density is a model assumption. Compressing a real body changes its density, so the critical radius moves with the assumption; it is not a fixed property of the object.",
    "Comparing a radius with a Schwarzschild radius compares two lengths. It is not a simulation of gravitational collapse.",
    "Whether a horizon actually forms depends on the spacetime geometry, on the state of the matter (pressure, temperature, composition, angular momentum) and on the dynamics of the collapse. None of that is modelled here.",
    "Newtonian mechanics cannot represent an event horizon. N-body dynamics, relativistic ray tracing and gravitational lensing are separate, later milestones and contribute nothing to this result.",
];

/// What is known about the object's extent.
///
/// A body either has a surface — in which case it has both a radius and a mean
/// density — or it has none, as a black hole does. Making that an enum rather
/// than two independent `Option`s means the impossible combination (a radius
/// without a density, or the reverse) cannot be constructed, and no downstream
/// code needs a branch for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// The object has a material surface.
    Surface {
        /// Volumetric mean radius.
        radius: Length,
        /// Mean density under the homogeneous-sphere assumption.
        mean_density: Density,
    },
    /// The object has no material surface; only its Schwarzschild radius is
    /// defined.
    HorizonOnly,
}

/// A fully specified problem together with the quantities that were given
/// rather than derived.
#[derive(Debug, Clone)]
pub struct Report {
    /// Display name of the object.
    pub label: String,
    /// Mass in kilograms.
    pub mass: Mass,
    /// What is known about the object's extent.
    pub shape: Shape,
    /// How the density was specified, echoed back to the user.
    pub density_basis: Option<String>,
    /// Classification of the input values, when they came from the catalog.
    pub provenance: Option<Provenance>,
    /// A caveat attached to the catalog entry.
    pub note: &'static str,
}

impl Report {
    /// Builds a report from a radius and a mean density.
    ///
    /// # Errors
    ///
    /// Propagates [`PhysicsError`] from [`mass_from_radius_density`].
    pub fn from_radius_and_density(
        label: String,
        radius: Length,
        density: Density,
        density_basis: String,
    ) -> Result<Self, PhysicsError> {
        Ok(Self {
            label,
            mass: mass_from_radius_density(radius, density)?,
            shape: Shape::Surface {
                radius,
                mean_density: density,
            },
            density_basis: Some(density_basis),
            provenance: None,
            note: "",
        })
    }

    /// Builds a report from a mass and a physical radius; the mean density is
    /// derived.
    ///
    /// # Errors
    ///
    /// Propagates [`PhysicsError`] from [`mean_density`].
    pub fn from_mass_and_radius(
        label: String,
        mass: Mass,
        radius: Length,
    ) -> Result<Self, PhysicsError> {
        Ok(Self {
            label,
            mass,
            shape: Shape::Surface {
                radius,
                mean_density: mean_density(mass, radius)?,
            },
            density_basis: Some("derived from mass and radius".to_string()),
            provenance: None,
            note: "",
        })
    }

    /// Builds a report from a mass and a mean density; the radius is derived.
    ///
    /// # Errors
    ///
    /// Propagates [`PhysicsError`] from [`radius_from_mass_density`].
    pub fn from_mass_and_density(
        label: String,
        mass: Mass,
        density: Density,
        density_basis: String,
    ) -> Result<Self, PhysicsError> {
        Ok(Self {
            label,
            mass,
            shape: Shape::Surface {
                radius: radius_from_mass_density(mass, density)?,
                mean_density: density,
            },
            density_basis: Some(density_basis),
            provenance: None,
            note: "",
        })
    }

    /// Builds a report from a catalogued astronomical object.
    ///
    /// The mean density is derived from the catalogued mass and radius rather
    /// than copied from a table, so that the two are consistent by
    /// construction.
    ///
    /// # Errors
    ///
    /// Returns [`PhysicsError`] if a catalogued value fails validation, which
    /// would indicate a defect in the catalog rather than in the input.
    pub fn from_object(object: &'static AstronomicalObject) -> Result<Self, PhysicsError> {
        let mass = Mass::from_in(object.mass_kg, blackhole_core::MassUnit::Kilogram)?;
        let shape = match object.radius_m {
            Some(metres) => {
                let radius = Length::from_in(metres, blackhole_core::LengthUnit::Metre)?;
                Shape::Surface {
                    radius,
                    mean_density: mean_density(mass, radius)?,
                }
            }
            None => Shape::HorizonOnly,
        };
        Ok(Self {
            label: object.name.to_string(),
            mass,
            shape,
            density_basis: match shape {
                Shape::Surface { .. } => {
                    Some("derived from the catalogued mass and radius".to_string())
                }
                Shape::HorizonOnly => None,
            },
            provenance: Some(object.provenance),
            note: object.note,
        })
    }

    /// Renders the full report: values, interpretation, limitations.
    ///
    /// # Errors
    ///
    /// Propagates [`PhysicsError`] from the physics functions. In practice this
    /// can only happen for inputs whose results are outside the `f64` range.
    pub fn render(&self) -> Result<String, PhysicsError> {
        let r_s = schwarzschild_radius(self.mass)?;
        let (ratio, critical) = match self.shape {
            Shape::Surface {
                radius,
                mean_density,
            } => (
                Some(compactness(self.mass, radius)?),
                Some(minimum_radius_at_density(mean_density)?),
            ),
            Shape::HorizonOnly => (None, None),
        };

        let mut lines: Vec<String> = Vec::new();

        lines.push("Results".to_string());
        lines.push("-".repeat(7));
        lines.push(row("Object", &self.label));
        if let Some(provenance) = self.provenance {
            lines.push(row(
                "Classification",
                &format!("{} — {}", provenance.label(), provenance.detail()),
            ));
        }
        lines.push(row("Mass", &format::mass(self.mass.kilograms())));
        match self.shape {
            Shape::Surface {
                radius,
                mean_density,
            } => {
                lines.push(row("Physical radius R", &format::length(radius.metres())));
                let mut value = format::density(mean_density.kilograms_per_cubic_metre());
                if let Some(basis) = &self.density_basis {
                    write!(value, "  [{basis}]").expect("writing to a String cannot fail");
                }
                lines.push(row("Mean density (sphere model)", &value));
            }
            Shape::HorizonOnly => {
                lines.push(row(
                    "Physical radius R",
                    "not applicable (the object has no material surface)",
                ));
                lines.push(row("Mean density (sphere model)", "not applicable"));
            }
        }
        lines.push(row(
            "Schwarzschild radius r_s",
            &format::length(r_s.metres()),
        ));
        if let Some(ratio) = ratio {
            lines.push(row("Compactness ratio r_s / R", &format::number(ratio)));
            lines.push(row("Ratio R / r_s", &format::number(1.0 / ratio)));
        } else {
            lines.push(row("Compactness ratio r_s / R", "not applicable"));
            lines.push(row("Ratio R / r_s", "not applicable"));
        }
        if let Some(critical) = critical {
            lines.push(row(
                "Critical radius at assumed density",
                &format::length(critical.metres()),
            ));
        } else {
            lines.push(row("Critical radius at assumed density", "not applicable"));
        }

        lines.push(String::new());
        lines.push("Interpretation".to_string());
        lines.push("-".repeat(14));
        lines.extend(self.interpretation(r_s)?);
        if !self.note.is_empty() {
            lines.push(String::new());
            lines.push(row("Note on this entry", self.note));
        }

        lines.push(String::new());
        lines.push("Important limitations".to_string());
        lines.push("-".repeat(21));
        for (index, limitation) in LIMITATIONS.iter().enumerate() {
            for (line_index, wrapped) in wrap(limitation, 74).iter().enumerate() {
                if line_index == 0 {
                    lines.push(format!("  {}. {wrapped}", index + 1));
                } else {
                    lines.push(format!("     {wrapped}"));
                }
            }
            lines.push(String::new());
        }
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }

        Ok(lines.join("\n"))
    }

    /// The prose interpretation of the two comparisons the model supports.
    ///
    /// # Errors
    ///
    /// Propagates [`PhysicsError`] from the physics functions.
    fn interpretation(&self, r_s: Length) -> Result<Vec<String>, PhysicsError> {
        let mut lines = Vec::new();

        match self.shape {
            Shape::Surface {
                radius,
                mean_density,
            } => {
                let compact = compactness(self.mass, radius)?;
                let closing = if radius.metres() > r_s.metres() {
                    "Under the stated assumptions the surface lies outside the \
                     horizon. This compares two lengths of the model; it is not a \
                     prediction about collapse."
                } else {
                    "r_s reaches or exceeds R, so this model places the surface at \
                     or inside its own horizon. Its premises — a static, \
                     homogeneous sphere of constant density in flat space — are \
                     then no longer self-consistent, and no dynamical conclusion \
                     follows from it."
                };
                let text = format!(
                    "1. Schwarzschild comparison. r_s = {} and R = {}, so \
                     r_s / R = {} and R / r_s = {}. {closing}",
                    format::length(r_s.metres()),
                    format::length(radius.metres()),
                    format::number(compact),
                    format::number(1.0 / compact),
                );
                lines.extend(indent(&wrap(&text, 74)));

                let critical = minimum_radius_at_density(mean_density)?;
                let ratio = radius.metres() / critical.metres();
                let closing = if ratio < 1.0 {
                    "This object's radius is therefore smaller than the critical \
                     radius, so the assumed density is far too low for it to be \
                     its own horizon. The ratio follows the assumption and scales \
                     as rho^(-1/2)."
                } else {
                    "At this assumed density a homogeneous sphere of this radius \
                     would have its Schwarzschild radius outside its own surface. \
                     That lies outside the regime a static, homogeneous, \
                     constant-density sphere can describe, so no conclusion about \
                     horizon formation follows from the number."
                };
                let text = format!(
                    "2. Critical radius. At the assumed mean density of {}, a \
                     homogeneous sphere reaches r_s = R at R_krit = {}. The \
                     object's radius is R / R_krit = {}. {closing}",
                    format::density(mean_density.kilograms_per_cubic_metre()),
                    format::length(critical.metres()),
                    format::number(ratio),
                );
                lines.extend(indent(&wrap(&text, 74)));
            }
            Shape::HorizonOnly => {
                let text = format!(
                    "1. Schwarzschild comparison. r_s = {}. No material surface is \
                     defined for this object, so there is no radius to compare r_s \
                     with; r_s is the only length the model provides for it.",
                    format::length(r_s.metres())
                );
                lines.extend(indent(&wrap(&text, 74)));

                let text = "2. Critical radius. Not computed: the assumed-density \
                            model needs a mean density, and this object has no \
                            material surface for one to be defined against.";
                lines.extend(indent(&wrap(text, 74)));
            }
        }

        Ok(lines)
    }
}

/// Formats one aligned `label : value` row.
fn row(label: &str, value: &str) -> String {
    format!("  {label:<38} {value}")
}

/// Prefixes every line with a two-space indent.
fn indent(lines: &[String]) -> Vec<String> {
    lines.iter().map(|line| format!("  {line}")).collect()
}

/// Naive but deterministic greedy word wrap at `width` columns.
///
/// Deliberately not a dependency: the output must be byte-reproducible and the
/// wrapping rule must be inspectable.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackhole_core::{DensityUnit, LengthUnit};

    fn earth() -> Report {
        let object = crate::catalog::find_object("Earth").expect("catalogued");
        Report::from_object(object).expect("valid")
    }

    /// The rendered report with whitespace collapsed, so that prose assertions
    /// are not defeated by the line wrapping.
    fn flattened(report: &Report) -> String {
        report
            .render()
            .expect("renders")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn report_contains_only_computed_values_and_no_placeholders() {
        let text = earth().render().expect("renders");
        assert!(text.contains("Results"));
        // Reference values independently computed in
        // docs/reports/m1_evidence.py (check C1): r_s(5.9722e24 kg) = 8.870103e-3 m.
        assert!(text.contains("5.972200e24 kg"));
        assert!(text.contains("6.371000e6 m (= 6.371000e3 km)"));
        assert!(text.contains("8.870103e-3 m (= 8.870103e0 mm)"));
        // The limitation block is present in full.
        for limitation in LIMITATIONS {
            let first_words = limitation
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            assert!(
                text.contains(&first_words),
                "missing limitation: {limitation}"
            );
        }
    }

    #[test]
    fn rendering_is_byte_reproducible() {
        let a = earth().render().expect("renders");
        let b = earth().render().expect("renders");
        assert_eq!(a, b);
    }

    #[test]
    fn no_radius_object_states_not_applicable_explicitly() {
        let object = crate::catalog::find_object("Sagittarius A*").expect("catalogued");
        let report = Report::from_object(object).expect("valid");
        assert_eq!(report.shape, Shape::HorizonOnly);
        let text = report.render().expect("renders");
        assert!(text.contains("not applicable (the object has no material surface)"));
        assert!(flattened(&report).contains("no radius to compare r_s with"));
        assert!(!text.contains("placeholder"));
    }

    #[test]
    fn water_sphere_report_is_self_consistent() {
        let radius = Length::from_in(1.0, LengthUnit::Metre).expect("valid");
        let density = Density::from_in(1000.0, DensityUnit::KilogramPerCubicMetre).expect("valid");
        let report = Report::from_radius_and_density(
            "water sphere".to_string(),
            radius,
            density,
            "1000 kg/m3".to_string(),
        )
        .expect("valid");
        let text = report.render().expect("renders");
        // Reference values independently computed in docs/reports/m1_evidence.py:
        // 4/3 pi (1 m)^3 * 1000 kg/m3 = 4188.790204786391 kg (check C1), and
        // R_krit(1000 kg/m3) = 4.009208e11 m (check C3, two independent methods).
        assert!(text.contains("4.188790e3 kg"));
        assert!(text.contains("4.009208e11 m"));
    }

    #[test]
    fn a_realistic_neutron_star_is_not_compact_enough_by_this_criterion() {
        // 1.4 Msun inside 12 km: rho = 3.8459e17 kg/m3, so R_krit = 2.0444e4 m
        // and r_s / R = 0.3445. A real neutron star therefore lies *inside* the
        // regime where the homogeneous-sphere model is meaningful, but the
        // critical radius at its own mean density is still larger than the
        // star: the model does not put it at its own horizon.
        let object = crate::catalog::find_object("neutron star").expect("catalogued");
        let report = Report::from_object(object).expect("valid");
        let text = report.render().expect("renders");
        assert!(text.contains("3.845932e17 kg/m3"), "{text}");
        assert!(text.contains("2.044362e4 m"), "{text}");
        assert!(text.contains("3.445458e-1"), "{text}");
        assert!(flattened(&report).contains("assumed density is far too low"));
    }

    #[test]
    fn a_body_inside_its_own_horizon_is_reported_as_out_of_regime() {
        // 1e31 kg within 1 m: r_s = 1.4852e4 m > R, so compactness > 1 and the
        // model's own premises (a static homogeneous sphere in flat space) have
        // broken down. Both prose branches must say so instead of implying a
        // physical prediction.
        let mass = Mass::from_in(1.0e31, blackhole_core::MassUnit::Kilogram).expect("valid");
        let radius = Length::from_in(1.0, LengthUnit::Metre).expect("valid");
        let report = Report::from_mass_and_radius("compact test body".to_string(), mass, radius)
            .expect("valid");
        let flat = flattened(&report);
        assert!(flat.contains("no longer self-consistent"), "{flat}");
        assert!(
            flat.contains("outside the regime a static, homogeneous"),
            "{flat}"
        );
        assert!(!flat.contains("assumed density is far too low"), "{flat}");
    }

    #[test]
    fn wrapping_is_lossless_in_words() {
        let text = "one two three four five six seven eight nine ten eleven twelve";
        let wrapped = wrap(text, 12);
        assert_eq!(wrapped.join(" "), text);
        assert!(wrapped.iter().all(|line| line.len() <= 12));
    }
}
