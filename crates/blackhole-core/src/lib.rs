//! # blackhole-core — validated SI physics core
//!
//! The scientific foundation of Black Hole Lab: the Schwarzschild radius, its
//! inverse, the compactness ratio, and the mass and critical radius of a
//! homogeneous sphere at an assumed density.
//!
//! The crate is deliberately small and has **no dependencies**. It is pure
//! computation on `f64` behind validated types, so that every consumer — the
//! CLI today, the gravity, lensing, rendering and Python layers later — shares
//! exactly one implementation of the physics and one set of unit conventions.
//!
//! # Conventions
//!
//! * **SI units internally, always.** Metre, kilogram, kilogram per cubic
//!   metre, second. No unit suffix ever appears in a stored number. Conversion
//!   happens at the boundary, through the [`units`] enums.
//! * **Validated quantities.** [`Length`], [`Mass`] and [`Density`] can only be
//!   constructed through fallible constructors and guarantee a finite,
//!   strictly positive value. Invalid input is rejected at the boundary rather
//!   than propagating into the physics.
//! * **No panics, no `NaN`.** Every public function returns
//!   `Result<_, `[`PhysicsError`]`>`. There is no `unwrap`, no `panic!` and no
//!   arithmetic path that can produce `NaN` from a valid input.
//! * **Full `f64` precision.** Results are computed numbers, not measurements.
//!   The physical uncertainty is carried by the constants, dominated by `G`
//!   (relative standard uncertainty `2.2e-5`); see [`constants`].
//!
//! # The two questions this crate answers
//!
//! These are different questions and the API keeps them apart:
//!
//! 1. **How large is the Schwarzschild radius of a given mass?** —
//!    [`schwarzschild_radius`]. Every mass has one; there is no universal
//!    minimum mass for a black hole.
//! 2. **At an assumed constant density, how large must a homogeneous sphere be
//!    for its horizon to reach its own surface?** — [`minimum_radius_at_density`].
//!    This number is a property of the assumed density, not a universal
//!    threshold.
//!
//! # What this crate does not do
//!
//! It does not simulate gravitational collapse, and it does not predict that
//! any object becomes a black hole. Whether a horizon forms depends on the
//! spacetime geometry and on the dynamical state of the matter, none of which
//! appears in these formulae. Newtonian mechanics is in particular not able to
//! represent an event horizon. `docs/physics.md` states the model limits in
//! full, and the CLI prints them with every result.
//!
//! Newtonian N-body dynamics, relativistic ray tracing, gravitational lensing,
//! GPU rendering and Python bindings are **not** part of this crate; they are
//! separate, later milestones. Nothing in this file should be read as claiming
//! otherwise.
//!
//! # Example
//!
//! ```
//! use blackhole_core::{
//!     Density, DensityUnit, Length, LengthUnit, mass_from_radius_density,
//!     minimum_radius_at_density, schwarzschild_radius,
//! };
//!
//! // A one-metre sphere of water.
//! let radius = Length::from_in(1.0, LengthUnit::Metre)?;
//! let density = Density::from_in(1000.0, DensityUnit::KilogramPerCubicMetre)?;
//!
//! let mass = mass_from_radius_density(radius, density)?;
//! let rs = schwarzschild_radius(mass)?;
//! let critical = minimum_radius_at_density(density)?;
//!
//! assert!((mass.kilograms() - 4188.790_204_786_39).abs() < 1e-9);
//! assert!(rs.metres() < 1e-20);
//! assert!((critical.metres() - 4.009_208e11).abs() / critical.metres() < 1e-5);
//! # Ok::<(), blackhole_core::PhysicsError>(())
//! ```
//!
//! # References
//!
//! * Schwarzschild, K. (1916), *Sitzungsber. Preuss. Akad. Wiss. Berlin*,
//!   189–196 — original metric.
//! * Misner, Thorne & Wheeler (1973), *Gravitation* — derivation of `r_s`.
//! * CODATA 2018: Tiesinga et al. (2021), *Rev. Mod. Phys.* **93**, 025010.
//! * IAU 2015 Resolution B3: Prša et al. (2016), *AJ* **152**, 41.

#![forbid(unsafe_code)]

pub mod constants;
pub mod error;
pub mod schwarzschild;
pub mod sphere;
pub mod units;

pub use error::{PhysicsError, Quantity};
pub use schwarzschild::{
    compactness, mass_from_radius_density, mass_from_schwarzschild_radius,
    minimum_radius_at_density, schwarzschild_radius, schwarzschild_radius_from_radius_density,
};
pub use sphere::{mean_density, radius_from_mass_density, sphere_volume};
pub use units::{Density, DensityUnit, Length, LengthUnit, Mass, MassUnit};
