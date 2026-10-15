//! The interactive prompt loop.
//!
//! # Design
//!
//! * The session reads from a generic [`BufRead`] and writes to a generic
//!   [`Write`], so the whole dialogue can be driven from a test with scripted
//!   input and captured output — no process spawning required for unit tests,
//!   and the same code path is exercised by the binary.
//! * Input errors are recovered from: a typo re-prompts for the same field.
//!   Errors that are *semantic* (a physically invalid combination) report the
//!   problem and return to the menu, where the mode can be chosen again.
//! * End of input, and the tokens `q`, `quit` or `exit` at any prompt, end the
//!   session cleanly with a success status. There is no path that panics.

use std::io::{BufRead, Write};

use blackhole_core::{Density, DensityUnit, Length, LengthUnit, Mass, MassUnit};

use crate::catalog;
use crate::input;
use crate::report::Report;

/// How a step of the interactive session ended.
///
/// Internal to this module: [`Session::run`] only ever returns when the session
/// is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// The calculation finished and the user wants another one.
    Continue,
    /// The user quit, or input ended.
    Quit,
}

/// A prompt session over a reader and a writer.
pub struct Session<R: BufRead, W: Write> {
    reader: R,
    writer: W,
}

impl<R: BufRead, W: Write> Session<R, W> {
    /// Creates a session.
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    fn out(&mut self, text: &str) -> std::io::Result<()> {
        writeln!(self.writer, "{text}")?;
        self.writer.flush()
    }

    fn error(&mut self, message: &str) -> std::io::Result<()> {
        self.out(&format!("  input rejected: {message}"))
    }

    /// Reads one line of input. `Ok(None)` means the session should end: input
    /// was exhausted, or the user typed a quit token.
    fn prompt(&mut self, text: &str) -> std::io::Result<Option<String>> {
        self.writer.write_all(text.as_bytes())?;
        self.writer.flush()?;
        let mut bytes = Vec::new();
        if self.reader.read_until(b'\n', &mut bytes)? == 0 {
            self.out("")?;
            return Ok(None);
        }
        // Decode lossily. A byte sequence that is not valid UTF-8 (a stray
        // control byte, a mis-set terminal encoding, a binary file redirected
        // into stdin) must not abort the whole dialogue: it becomes a
        // replacement character, then fails to parse as a number or a unit and
        // is re-prompted like any other input error.
        let line = String::from_utf8_lossy(&bytes).trim().to_string();
        if is_quit(&line) {
            return Ok(None);
        }
        Ok(Some(line))
    }

    /// Prompts until a valid positive length is given.
    fn prompt_length(&mut self, label: &str) -> std::io::Result<Option<Length>> {
        loop {
            let Some(line) = self.prompt(&format!(
                "{label} (value, optional unit such as `6371 km`): "
            ))?
            else {
                return Ok(None);
            };
            let (value, inline) = match input::parse_value_with_optional_unit(&line) {
                Ok(parsed) => parsed,
                Err(message) => {
                    self.error(&message)?;
                    continue;
                }
            };
            // Reject a value that cannot be valid before asking for its unit.
            // Otherwise a typo such as `0` consumes the next line as its unit
            // and desynchronises the rest of the dialogue.
            if let Err(error) = Length::from_in(value, LengthUnit::Metre) {
                self.error(&error.to_string())?;
                continue;
            }
            let prompted = if inline.is_some() {
                None
            } else {
                let symbols = unit_symbols(LengthUnit::ALL.iter().map(|u| LengthUnit::symbol(*u)));
                match self.prompt(&format!("  unit [m] — {symbols}: "))? {
                    Some(answer) => Some(answer),
                    None => return Ok(None),
                }
            };
            let unit = match input::resolve_length_unit(inline.as_deref(), prompted.as_deref()) {
                Ok(unit) => unit,
                Err(message) => {
                    self.error(&message)?;
                    continue;
                }
            };
            match Length::from_in(value, unit) {
                Ok(length) => return Ok(Some(length)),
                Err(error) => self.error(&error.to_string())?,
            }
        }
    }

    /// Prompts until a valid positive mass is given.
    fn prompt_mass(&mut self, label: &str) -> std::io::Result<Option<Mass>> {
        loop {
            let Some(line) = self.prompt(&format!(
                "{label} (value, optional unit such as `1.989e30 kg`): "
            ))?
            else {
                return Ok(None);
            };
            let (value, inline) = match input::parse_value_with_optional_unit(&line) {
                Ok(parsed) => parsed,
                Err(message) => {
                    self.error(&message)?;
                    continue;
                }
            };
            if let Err(error) = Mass::from_in(value, MassUnit::Kilogram) {
                self.error(&error.to_string())?;
                continue;
            }
            let prompted = if inline.is_some() {
                None
            } else {
                let symbols = unit_symbols(MassUnit::ALL.iter().map(|u| MassUnit::symbol(*u)));
                match self.prompt(&format!("  unit [kg] — {symbols}: "))? {
                    Some(answer) => Some(answer),
                    None => return Ok(None),
                }
            };
            let unit = match input::resolve_mass_unit(inline.as_deref(), prompted.as_deref()) {
                Ok(unit) => unit,
                Err(message) => {
                    self.error(&message)?;
                    continue;
                }
            };
            match Mass::from_in(value, unit) {
                Ok(mass) => return Ok(Some(mass)),
                Err(error) => self.error(&error.to_string())?,
            }
        }
    }

    /// Prompts until a valid density is given, returning it with a description
    /// of how it was interpreted.
    fn prompt_density(&mut self) -> std::io::Result<Option<(Density, String)>> {
        loop {
            let Some(line) = self.prompt(&format!(
                "Mean density (kg/m3, e.g. `2.7 g/cm3`, or a material — {}): ",
                input::material_names()
            ))?
            else {
                return Ok(None);
            };
            let (value, basis) = match input::resolve_density(&line) {
                Ok(parsed) => parsed,
                Err(message) => {
                    self.error(&message)?;
                    continue;
                }
            };
            match Density::from_in(value, DensityUnit::KilogramPerCubicMetre) {
                Ok(density) => return Ok(Some((density, basis))),
                Err(error) => self.error(&error.to_string())?,
            }
        }
    }

    /// Prompts for a free-text label, applying a default when left blank.
    fn prompt_label(&mut self) -> std::io::Result<Option<String>> {
        let Some(line) = self.prompt("Label for this object [custom object]: ")? else {
            return Ok(None);
        };
        Ok(Some(if line.is_empty() {
            "custom object".to_string()
        } else {
            line
        }))
    }

    /// Builds, renders and prints a report, or explains why it cannot be built.
    fn present(
        &mut self,
        report: Result<Report, blackhole_core::PhysicsError>,
    ) -> std::io::Result<()> {
        // Start the report on its own line. The prompts above do not end with a
        // newline — they end at whatever the user typed — so without this the
        // first row of the Results block would be glued to the last prompt.
        self.out("")?;
        match report {
            Ok(report) => match report.render() {
                Ok(text) => self.out(&text)?,
                Err(error) => self.error(&error.to_string())?,
            },
            Err(error) => self.error(&error.to_string())?,
        }
        Ok(())
    }

    /// Runs the menu loop until the user quits or input ends.
    pub fn run(&mut self) -> std::io::Result<()> {
        self.out("Black Hole Lab")?;
        self.out("==============")?;
        self.out("")?;
        loop {
            self.out("Choose input mode:")?;
            self.out("  1. Radius and mean density")?;
            self.out("  2. Mass and physical radius")?;
            self.out("  3. Known astronomical object")?;
            self.out("  4. Custom object (choose any two of mass, radius, mean density)")?;
            self.out("  l. List the built-in objects and materials")?;
            self.out("  q. Quit")?;
            self.out("")?;

            let Some(choice) = self.prompt("Mode: ")? else {
                return Ok(());
            };
            let flow = match choice.trim().to_ascii_lowercase().as_str() {
                "1" => self.mode_radius_and_density()?,
                "2" => self.mode_mass_and_radius()?,
                "3" => self.mode_known_object()?,
                "4" => self.mode_custom()?,
                "l" | "list" => {
                    self.out("")?;
                    self.out(&object_listing())?;
                    self.out(&material_listing())?;
                    Flow::Continue
                }
                other => {
                    self.error(&format!("`{other}` is not one of the listed options"))?;
                    Flow::Continue
                }
            };
            if flow == Flow::Quit {
                return Ok(());
            }
        }
    }

    /// Mode 1: radius and mean density are given, the mass is derived.
    fn mode_radius_and_density(&mut self) -> std::io::Result<Flow> {
        let Some(radius) = self.prompt_length("Object radius")? else {
            return Ok(Flow::Quit);
        };
        let Some((density, basis)) = self.prompt_density()? else {
            return Ok(Flow::Quit);
        };
        let Some(label) = self.prompt_label()? else {
            return Ok(Flow::Quit);
        };
        self.present(Report::from_radius_and_density(
            label, radius, density, basis,
        ))?;
        self.after_result()
    }

    /// Mode 2: mass and radius are given, the mean density is derived.
    fn mode_mass_and_radius(&mut self) -> std::io::Result<Flow> {
        let Some(mass) = self.prompt_mass("Object mass")? else {
            return Ok(Flow::Quit);
        };
        let Some(radius) = self.prompt_length("Physical radius")? else {
            return Ok(Flow::Quit);
        };
        let Some(label) = self.prompt_label()? else {
            return Ok(Flow::Quit);
        };
        self.present(Report::from_mass_and_radius(label, mass, radius))?;
        self.after_result()
    }

    /// Mode 3: a catalogued object is selected by name or index.
    fn mode_known_object(&mut self) -> std::io::Result<Flow> {
        self.out("")?;
        self.out(&object_listing())?;
        loop {
            let Some(answer) = self.prompt("Object (name or number): ")? else {
                return Ok(Flow::Quit);
            };
            let object = match answer.trim().parse::<usize>() {
                Ok(index) if index >= 1 => catalog::ASTRONOMICAL_OBJECTS.get(index - 1),
                Ok(_) => None,
                Err(_) => catalog::find_object(&answer),
            };
            match object {
                Some(object) => {
                    self.present(Report::from_object(object))?;
                    return self.after_result();
                }
                None => self.error(&format!(
                    "no catalogued object matches `{}`; try `l` to list them",
                    answer.trim()
                ))?,
            }
        }
    }

    /// Mode 4: the user picks which two of the three quantities to give.
    fn mode_custom(&mut self) -> std::io::Result<Flow> {
        self.out("Which two quantities will you specify?")?;
        self.out("  1. Mass and mean density      (the radius is derived)")?;
        self.out("  2. Mass and physical radius   (the density is derived)")?;
        self.out("  3. Radius and mean density    (the mass is derived)")?;

        let Some(choice) = self.prompt("Combination: ")? else {
            return Ok(Flow::Quit);
        };

        match choice.trim() {
            "1" => {
                let Some(mass) = self.prompt_mass("Object mass")? else {
                    return Ok(Flow::Quit);
                };
                let Some((density, basis)) = self.prompt_density()? else {
                    return Ok(Flow::Quit);
                };
                let Some(label) = self.prompt_label()? else {
                    return Ok(Flow::Quit);
                };
                self.present(Report::from_mass_and_density(label, mass, density, basis))?;
            }
            "2" => {
                let Some(mass) = self.prompt_mass("Object mass")? else {
                    return Ok(Flow::Quit);
                };
                let Some(radius) = self.prompt_length("Physical radius")? else {
                    return Ok(Flow::Quit);
                };
                let Some(label) = self.prompt_label()? else {
                    return Ok(Flow::Quit);
                };
                self.present(Report::from_mass_and_radius(label, mass, radius))?;
            }
            "3" => {
                let Some(radius) = self.prompt_length("Object radius")? else {
                    return Ok(Flow::Quit);
                };
                let Some((density, basis)) = self.prompt_density()? else {
                    return Ok(Flow::Quit);
                };
                let Some(label) = self.prompt_label()? else {
                    return Ok(Flow::Quit);
                };
                self.present(Report::from_radius_and_density(
                    label, radius, density, basis,
                ))?;
            }
            other => self.error(&format!("`{other}` is not one of the listed combinations"))?,
        }
        self.after_result()
    }

    /// Offers to loop without restarting the program.
    fn after_result(&mut self) -> std::io::Result<Flow> {
        self.out("")?;
        let Some(answer) = self.prompt("Another calculation? [Y/n]: ")? else {
            return Ok(Flow::Quit);
        };
        if matches!(answer.trim().to_ascii_lowercase().as_str(), "n" | "no") {
            return Ok(Flow::Quit);
        }
        self.out("")?;
        Ok(Flow::Continue)
    }
}

/// The tokens that end a session at any prompt.
fn is_quit(line: &str) -> bool {
    matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "q" | "quit" | "exit"
    )
}

/// Joins unit symbols for a prompt.
fn unit_symbols(symbols: impl Iterator<Item = &'static str>) -> String {
    symbols.collect::<Vec<_>>().join(", ")
}

/// A numbered listing of the catalogued objects, with provenance.
#[must_use]
pub fn object_listing() -> String {
    let mut lines = vec!["Known astronomical objects:".to_string()];
    for (index, object) in catalog::ASTRONOMICAL_OBJECTS.iter().enumerate() {
        lines.push(format!(
            "  {}. {:<32} {:>14} kg   [{}]",
            index + 1,
            object.name,
            crate::format::number(object.mass_kg),
            object.provenance.label()
        ));
        lines.push(format!("     {}", object.provenance.detail()));
    }
    lines.join("\n")
}

/// A listing of the built-in material densities.
#[must_use]
pub fn material_listing() -> String {
    let mut lines = vec![String::new(), "Built-in material densities:".to_string()];
    for material in catalog::MATERIALS {
        lines.push(format!(
            "  {:<28} {:>12} kg/m3   [{}]",
            material.name,
            crate::format::number(material.density_kg_m3),
            material.provenance.label()
        ));
        lines.push(format!("     {}", material.provenance.detail()));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(script: &str) -> String {
        let mut session = Session::new(script.as_bytes(), Vec::new());
        session
            .run()
            .expect("the session must not fail on valid input");
        String::from_utf8(session.writer).expect("utf-8 output")
    }

    #[test]
    fn empty_input_exits_cleanly() {
        let output = session("");
        assert!(output.contains("Black Hole Lab"));
        assert!(output.contains("Choose input mode"));
    }

    #[test]
    fn mode_two_computes_and_reports_the_earth() {
        // Choose mode 2, give mass 5.9722e24 kg and radius 6.371e6 m, label,
        // then decline another calculation.
        let output = session("2\n5.9722e24\nkg\n6.371e6\nm\nEarth-ish\nn\n");
        assert!(output.contains("Schwarzschild radius r_s"));
        // r_s(5.9722e24 kg) = 8.870103e-3 m, per docs/reports/m1_evidence.txt C1.
        assert!(output.contains("8.870103e-3 m"));
        assert!(output.contains("Important limitations"));
    }

    #[test]
    fn mode_one_accepts_a_material_name_and_an_inline_unit() {
        let output = session("1\n1 km\nwater\nball of water\nq\n");
        // 4/3 pi (1000 m)^3 * 1000 kg/m3 = 4.188790e12 kg
        assert!(output.contains("4.188790e12 kg"), "{output}");
        // R_krit(1000 kg/m3) = 4.009208e11 m
        assert!(output.contains("4.009208e11 m"), "{output}");
    }

    #[test]
    fn mode_three_accepts_a_name_and_an_index() {
        let by_name = session("3\nEarth\nn\n");
        assert!(by_name.contains("reference data"));
        let by_index = session("3\n2\nn\n");
        assert!(by_index.contains("reference data"));
        // Sgr A* has no surface and must say so rather than invent a radius.
        let sgr = session("3\nsgr a*\nn\n");
        assert!(sgr.contains("no material surface"), "{sgr}");
    }

    #[test]
    fn mode_four_prompts_the_label_after_the_quantities() {
        // The label is asked for last, as in modes 1-3. If it were asked for
        // before the quantities, the `1` intended as the mass would be consumed
        // as the label and nothing would be computed.
        let output = session("4\n1\n1\nMsun\ngranite\ngranite sphere\nn\n");
        assert!(!output.contains("input rejected"), "{output}");
        assert!(output.contains("granite sphere"), "{output}");
        assert!(output.contains("Mean density (sphere model)"), "{output}");
        assert!(output.contains("Schwarzschild radius r_s"), "{output}");
    }

    #[test]
    fn invalid_utf8_on_the_dialogue_input_is_recovered_from() {
        // A lone 0xFF is not valid UTF-8. It must be treated as an unrecognised
        // mode rather than aborting the session.
        let mut bytes: Vec<u8> = vec![0xFF, 0xFE, b'\n'];
        bytes.extend_from_slice(b"3\nEarth\nn\n");
        let mut session = Session::new(bytes.as_slice(), Vec::new());
        session
            .run()
            .expect("the session must survive invalid UTF-8");
        let output = String::from_utf8(session.writer).expect("utf-8 output");
        assert!(output.contains("input rejected"), "{output}");
        assert!(output.contains("Schwarzschild radius r_s"), "{output}");
    }

    #[test]
    fn an_invalid_value_is_rejected_before_its_unit_is_requested() {
        // `0` cannot become a length, so it must be rejected immediately. If it
        // were not, the next line `2e3` would be read as its unit and the whole
        // remaining dialogue would be shifted by one line.
        let output = session("2\n1e3\nkg\n0\n2e3\nm\nprobe\nn\n");
        assert!(
            output.contains("length must be greater than zero, got 0"),
            "{output}"
        );
        assert!(output.contains("2.000000e3 m"), "{output}");
    }

    #[test]
    fn typo_in_a_number_reprompts_instead_of_aborting() {
        let output = session("2\ntwelve\n5.9722e24\nkg\n6.371e6\nm\nx\nn\n");
        assert!(output.contains("input rejected"));
        assert!(output.contains("8.870103e-3 m"));
    }

    #[test]
    fn unknown_mode_and_unknown_object_are_reported_not_fatal() {
        let output = session("9\n3\nVulcan\nEarth\nn\n");
        assert!(output.contains("is not one of the listed options"));
        assert!(output.contains("no catalogued object matches"));
        assert!(output.contains("Earth"));
    }

    #[test]
    fn physically_invalid_input_returns_to_the_menu() {
        // A zero radius is rejected by the core, not by the prompt.
        let output = session("2\n5.9722e24\nkg\n0\nm\nn\n");
        assert!(output.contains("input rejected"));
        assert!(output.contains("length must be greater than zero"));
    }

    #[test]
    fn the_listing_command_shows_both_catalogues() {
        let output = session("l\nq\n");
        assert!(output.contains("Known astronomical objects"));
        assert!(output.contains("Built-in material densities"));
        assert!(output.contains("granite"));
    }
}
