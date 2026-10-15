//! Black Hole Lab — command-line front end.
//!
//! Copyright 2026 Black Hole Lab contributors. Licensed under Apache-2.0.
//! See <LICENSE> in the project root for license details.
//!
//! Two entry paths share one code path through the physics:
//!
//! * **interactive** (no subcommand) — a prompted dialogue, for exploration;
//! * **non-interactive** (`calc`, `object`, `list`) — flag or argument driven,
//!   for scripts and automation.
//!
//! Both build the same [`report::Report`] and print the same text, because a
//! scripted run and an interactive run of identical inputs must produce
//! identical output.
//!
//! Exit status: `0` success, `1` a physical/computational failure, `2` a usage
//! error (unknown unit, wrong number of arguments, unknown object).

mod catalog;
mod format;
mod input;
mod interactive;
mod report;

use std::io::{self, BufReader, Write as _};
use std::process::ExitCode;

use blackhole_core::{Density, DensityUnit, Length, LengthUnit, Mass, MassUnit};
use clap::{Args, Parser, Subcommand};

use crate::interactive::Session;
use crate::report::Report;

/// A failure with the exit status it should map to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Failure {
    /// The invocation was wrong; exit `2`.
    Usage(String),
    /// The physics computation failed; exit `1`.
    Physics(String),
}

#[derive(Parser, Debug)]
#[command(
    name = "blackhole",
    version,
    about = "Black Hole Lab: Schwarzschild radius, sphere mass and critical radius",
    long_about = "Black Hole Lab computes, for any object you describe:\n\
                  \n\
                  the Schwarzschild radius r_s = 2 G M / c^2; the compactness \
                  ratio r_s / R; and the critical radius at an assumed mean \
                  density, R_krit = sqrt(3 c^2 / (8 pi G rho)).\n\
                  \n\
                  All input is converted to SI units internally, and the result \
                  is reported together with the model's limitations. This is a \
                  static, homogeneous-sphere model: it does not simulate \
                  gravitational collapse, and it does not predict whether a \
                  horizon forms.\n\
                  \n\
                  Run without a subcommand for the interactive dialogue.",
    after_help = "Examples:\n  \
                  blackhole object Earth\n  \
                  blackhole calc --radius 6371 --radius-unit km --density 5513\n  \
                  blackhole calc --mass 20 --mass-unit Msun --radius 1 --radius-unit Rearth\n  \
                  blackhole calc --radius 1 --radius-unit km --density water\n  \
                  blackhole list"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compute from any two of mass, physical radius and mean density
    Calc(CalcArgs),
    /// Compute for one of the built-in astronomical objects
    Object {
        /// Object name, e.g. Earth, Sun, Moon, Jupiter, "Sagittarius A*"
        name: String,
    },
    /// List the built-in astronomical objects and material densities
    List,
}

#[derive(Args, Debug)]
struct CalcArgs {
    /// Mass
    ///
    /// `allow_negative_numbers` lets a negative mass reach the physics layer and
    /// be reported as "mass must be greater than zero, got -5". It is preferred
    /// over `allow_hyphen_values`, which would also swallow a following flag:
    /// `--mass --radius 1` would consume `--radius` as the value and report a
    /// misleading "unexpected argument '1'".
    #[arg(long, value_name = "VALUE", allow_negative_numbers = true)]
    mass: Option<f64>,
    /// Unit for --mass
    #[arg(long, value_name = "UNIT", default_value = "kg")]
    mass_unit: String,
    /// Physical radius
    ///
    /// Negative values are accepted here for the same reason as for `--mass`.
    #[arg(long, value_name = "VALUE", allow_negative_numbers = true)]
    radius: Option<f64>,
    /// Unit for --radius
    #[arg(long, value_name = "UNIT", default_value = "m")]
    radius_unit: String,
    /// Mean density: a number in kg/m3, `2.7 g/cm3`, or a material name
    ///
    /// Negative values are accepted for the same reason as for `--mass`, so that
    /// `--density -5` is classified as a physical failure rather than as an
    /// unrecognised argument.
    #[arg(long, value_name = "VALUE", allow_negative_numbers = true)]
    density: Option<String>,
    /// Label printed in the report
    #[arg(long, value_name = "TEXT", default_value = "custom object")]
    label: String,
}

/// Writes a report to stdout, treating a closed pipe as a normal end of output.
///
/// `println!` panics when the reader has gone away, so
/// `blackhole object Earth | head -1` would abort with status 101 — neither a
/// physical failure nor a usage error, and a violation of the documented exit
/// contract. Mapping `BrokenPipe` to success gives the conventional Unix
/// behaviour for a consumer that stopped reading early.
fn emit(text: &str) -> Result<(), io::Error> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match writeln!(handle, "{text}") {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other,
    }
}

/// [`emit`] plus the exit status: success, or a reported I/O failure.
fn emit_then_status(text: &str) -> ExitCode {
    match emit(text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => run_interactive(),
        Some(Command::Calc(args)) => run_calc(&args),
        Some(Command::Object { name }) => run_object(&name),
        Some(Command::List) => {
            let listings = format!(
                "{}\n{}",
                interactive::object_listing(),
                interactive::material_listing()
            );
            emit_then_status(&listings)
        }
    }
}

/// Runs the interactive dialogue on stdin/stdout.
fn run_interactive() -> ExitCode {
    let stdin = io::stdin();
    let mut session = Session::new(BufReader::new(stdin.lock()), io::stdout());
    match session.run() {
        Ok(()) => ExitCode::SUCCESS,
        // The reader went away mid-dialogue; that is not an error.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

/// Runs a non-interactive calculation.
fn run_calc(args: &CalcArgs) -> ExitCode {
    match build_calc(args) {
        Ok(report) => match report.render() {
            Ok(text) => emit_then_status(&text),
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::from(1)
            }
        },
        Err(Failure::Usage(message)) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
        Err(Failure::Physics(message)) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

/// Validates the arguments and builds the report.
fn build_calc(args: &CalcArgs) -> Result<Report, Failure> {
    let provided = [
        args.mass.is_some(),
        args.radius.is_some(),
        args.density.is_some(),
    ]
    .iter()
    .filter(|given| **given)
    .count();
    if provided != 2 {
        return Err(Failure::Usage(format!(
            "specify exactly two of --mass, --radius and --density (got {provided}); \
             the third quantity is derived from the sphere model"
        )));
    }

    let mass = match args.mass {
        Some(value) => {
            let unit = MassUnit::from_symbol(&args.mass_unit).ok_or_else(|| {
                Failure::Usage(format!(
                    "unknown mass unit `{}`; supported: {}",
                    args.mass_unit,
                    joined(MassUnit::ALL.iter().map(|u| u.symbol()))
                ))
            })?;
            Some(Mass::from_in(value, unit).map_err(|e| Failure::Physics(e.to_string()))?)
        }
        None => None,
    };

    let radius = match args.radius {
        Some(value) => {
            let unit = LengthUnit::from_symbol(&args.radius_unit).ok_or_else(|| {
                Failure::Usage(format!(
                    "unknown length unit `{}`; supported: {}",
                    args.radius_unit,
                    joined(LengthUnit::ALL.iter().map(|u| u.symbol()))
                ))
            })?;
            Some(Length::from_in(value, unit).map_err(|e| Failure::Physics(e.to_string()))?)
        }
        None => None,
    };

    let density = match &args.density {
        Some(raw) => {
            let (si, basis) = input::resolve_density(raw).map_err(Failure::Usage)?;
            let density = Density::from_in(si, DensityUnit::KilogramPerCubicMetre)
                .map_err(|e| Failure::Physics(e.to_string()))?;
            Some((density, basis))
        }
        None => None,
    };

    let report = match (mass, radius, density) {
        (Some(mass), Some(radius), None) => {
            Report::from_mass_and_radius(args.label.clone(), mass, radius)
        }
        (Some(mass), None, Some((density, basis))) => {
            Report::from_mass_and_density(args.label.clone(), mass, density, basis)
        }
        (None, Some(radius), Some((density, basis))) => {
            Report::from_radius_and_density(args.label.clone(), radius, density, basis)
        }
        _ => {
            return Err(Failure::Usage(
                "internal error: unexpected argument combination".to_string(),
            ));
        }
    };
    report.map_err(|error| Failure::Physics(error.to_string()))
}

/// Runs a calculation for a catalogued object.
fn run_object(name: &str) -> ExitCode {
    // A 1-based index is accepted here as well, so that this subcommand and the
    // interactive mode-3 prompt accept exactly the same tokens.
    let object = match name.trim().parse::<usize>() {
        Ok(index) if index >= 1 => catalog::ASTRONOMICAL_OBJECTS.get(index - 1),
        Ok(_) => None,
        Err(_) => catalog::find_object(name),
    };
    let Some(object) = object else {
        eprintln!("error: no catalogued object matches `{name}`");
        eprintln!(
            "       available: {}",
            catalog::ASTRONOMICAL_OBJECTS
                .iter()
                .enumerate()
                .map(|(index, object)| format!("{}. {}", index + 1, object.name))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return ExitCode::from(2);
    };
    match Report::from_object(object).and_then(|report| report.render()) {
        Ok(text) => emit_then_status(&text),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

/// Joins symbols for a usage message.
fn joined(symbols: impl Iterator<Item = &'static str>) -> String {
    symbols.collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fully defaulted argument set; tests override individual fields.
    fn template() -> CalcArgs {
        CalcArgs {
            mass: None,
            mass_unit: "kg".to_string(),
            radius: None,
            radius_unit: "m".to_string(),
            density: None,
            label: "test".to_string(),
        }
    }

    #[test]
    fn exactly_two_of_three_is_enforced() {
        let only_one = CalcArgs {
            mass: Some(1.0),
            ..template()
        };
        assert!(matches!(build_calc(&only_one), Err(Failure::Usage(_))));

        let all_three = CalcArgs {
            mass: Some(1.0),
            radius: Some(1.0),
            density: Some("1000".to_string()),
            ..template()
        };
        assert!(matches!(build_calc(&all_three), Err(Failure::Usage(_))));
    }

    #[test]
    fn unknown_units_are_usage_errors_not_panics() {
        let bad_unit = CalcArgs {
            mass: Some(1.0),
            mass_unit: "stone".to_string(),
            radius: Some(1.0),
            ..template()
        };
        assert!(matches!(build_calc(&bad_unit), Err(Failure::Usage(_))));

        let bad_density = CalcArgs {
            radius: Some(1.0),
            density: Some("unobtainium".to_string()),
            ..template()
        };
        assert!(matches!(build_calc(&bad_density), Err(Failure::Usage(_))));
    }

    #[test]
    fn physically_invalid_values_are_physics_failures() {
        let negative = CalcArgs {
            mass: Some(-1.0),
            radius: Some(1.0),
            ..template()
        };
        assert!(matches!(build_calc(&negative), Err(Failure::Physics(_))));
    }

    #[test]
    fn every_valid_combination_builds_a_report() {
        let mass_radius = CalcArgs {
            mass: Some(5.972_2e24),
            radius: Some(6.371e6),
            ..template()
        };
        let mass_density = CalcArgs {
            mass: Some(5.972_2e24),
            density: Some("5513".to_string()),
            ..template()
        };
        let radius_density = CalcArgs {
            radius: Some(1.0),
            density: Some("water".to_string()),
            ..template()
        };
        for args in [mass_radius, mass_density, radius_density] {
            let report = build_calc(&args).expect("valid combination");
            assert!(report.render().is_ok());
        }
    }
}
