# Black Hole Lab

A Rust astrophysics engine for black-hole physics, gravitational lensing and
rendering. This repository currently contains **milestones M1 and M2**: the
validated SI physics core and its command-line front end.

    blackhole/
    ├── Cargo.toml                  Cargo workspace
    ├── crates/
    │   ├── blackhole-core/         validated physics: r_s, sphere mass, critical radius
    │   └── blackhole-cli/          interactive and non-interactive front end
    └── docs/
        ├── physics.md              equations, units, model limits, worked values
        ├── architecture.md         crate layout, decisions, dependency rationale
        ├── validation.md           test strategy, reference values, tolerances
        └── reports/                analysis and execution evidence
            ├── M1-analysis.md      reference-repository and licence analysis
            ├── m1_evidence.py      independent verification of the physics
            ├── m1_evidence.txt     its recorded output
            ├── m2_cli_checks.sh    acceptance transcript script
            └── m2_cli_evidence.txt its recorded output

## What it computes

Two different questions, deliberately kept apart:

**A. How large is the Schwarzschild radius of a given mass?**

    r_s = 2 G M / c^2

Every mass has one. There is **no universal minimum mass for a black hole**,
and `r_s` on its own says nothing about whether a horizon exists.

**B. At an assumed constant density, how large must a homogeneous sphere be for
its horizon to reach its own surface?**

    R_krit = sqrt( 3 c^2 / (8 pi G rho) )

This is a property of the *assumed density*, not a universal threshold. It is
not a general minimum size for black holes, and it is not a collapse criterion.

## Quick start

    cargo build --release
    cargo run -p blackhole-cli -- object Earth
    cargo run -p blackhole-cli -- list

Interactive dialogue (the default when no subcommand is given):

    cargo run -p blackhole-cli

Non-interactive, for scripts. Specify exactly two of mass, radius and mean
density; the third is derived:

    blackhole calc --radius 6371 --radius-unit km --density 5513
    blackhole calc --mass 1 --mass-unit Msun --density granite
    blackhole calc --mass 20 --mass-unit Mearth --radius 1 --radius-unit Rearth
    blackhole calc --radius 1 --radius-unit km --density water --label "water ball"
    blackhole object "sgr a*"

Exit status: `0` success, `1` a physical/computational failure, `2` a usage
error. For the subcommands, results go to stdout and diagnostics to stderr; the
interactive dialogue prints its prompts and its diagnostics to stdout, since it is
a dialogue. The same arguments always produce byte-identical output, a closed
stdout pipe is not treated as a failure, and no input — however malformed —
causes a panic.

Units accepted include `m`, `km`, `cm`, `au`, `Rsun`, `Rearth`, `Rjup`, `ly`,
`pc` for lengths; `kg`, `g`, `t`, `Msun`, `Mearth`, `Mjup` for masses; and
`kg/m3` or `g/cm3` for densities, plus the built-in material names (`water`,
`ice`, `rock`, `granite`, `basalt`, `iron`, `air`).

## Library use

The physics is independent of the CLI:

```rust
use blackhole_core::{
    Density, DensityUnit, Length, LengthUnit, mass_from_radius_density,
    minimum_radius_at_density, schwarzschild_radius,
};

let radius = Length::from_in(1.0, LengthUnit::Metre)?;
let density = Density::from_in(1000.0, DensityUnit::KilogramPerCubicMetre)?;

let mass = mass_from_radius_density(radius, density)?;
let rs = schwarzschild_radius(mass)?;
let critical = minimum_radius_at_density(density)?;
# Ok::<(), blackhole_core::PhysicsError>(())
```

`blackhole-core` has **no dependencies**. Quantities are validated newtypes:
`Length`, `Mass` and `Density` can only be built through fallible constructors
and guarantee a finite, strictly positive value, so the physics functions never
see an invalid input and never return `NaN`.

## Verification

    cargo fmt --all --check
    cargo clippy --workspace --all-targets    # pedantic, zero warnings
    cargo test --workspace
    bash docs/reports/m2_cli_checks.sh        # self-checking acceptance transcript

The acceptance script computes a PASS/FAIL verdict per case from the case's
*content*, not only its exit status, and exits non-zero if any case fails: a case
that exits `0` without printing a `Results` block counts as a failure.

The physics is checked against published reference values and against
independent algebraic and numerical controls, not only against itself. See
`docs/validation.md` for what is asserted and with which tolerance, and
`docs/reports/m1_evidence.txt` for the independent computations.

## Model limits

The CLI prints these with every result, and they are not boilerplate:

* A mass and its Schwarzschild radius say nothing on their own about whether a
  horizon exists; there is no universal minimum mass for a black hole.
* The density is a model assumption. Compressing a real body changes it, and
  `R_krit` scales as `rho^(-1/2)`, so the number moves with the assumption.
* Comparing a radius with a Schwarzschild radius compares two lengths. It is not
  a simulation of gravitational collapse.
* Whether a horizon forms depends on the spacetime geometry, on the state of the
  matter (pressure, temperature, composition, angular momentum) and on the
  dynamics of the collapse. None of that is modelled here.
* Newtonian mechanics cannot represent an event horizon. N-body dynamics,
  relativistic ray tracing and gravitational lensing are separate, later
  milestones and contribute nothing to the current result.

## Status

| Milestone | Scope | State |
| --- | --- | --- |
| M1 | Reference-repository and licence analysis, workspace, architecture | done |
| M2 | Physics core, CLI, tests, physics documentation | done |
| M3 | Newtonian N-body engine (`blackhole-gravity`) | not started |
| M4 | CPU gravitational lensing and relativistic ray tracing | not started |
| M5 | GPU backends (wgpu: Vulkan/Metal; optional CUDA) | not started |
| M6 | Python bindings and `examples/lensing.py` | not started |

`docs/architecture.md` records which crates exist, which are planned, and why
the split is deferred until each module has a consumer.

## Provenance of the reference projects

The two repositories named in the project brief were analysed and are **not**
used as a source of code:

* <https://github.com/kavan010/black_hole> — C++/OpenGL, 2639 stars
* <https://github.com/kavan010/gravity_sim> — C++/OpenGL, 590 stars

**Neither repository carries a licence.** There is no `LICENSE`/`COPYING` file,
no copyright header in any source file, and GitHub reports `license: null` for
both. Absent a licence, the default is that all rights are reserved, so no code
was copied, translated or adapted from either project. They were used only as a
feature inventory. The algorithms implemented here are re-derived from the
standard literature and verified independently;
`docs/reports/M1-analysis.md` records what the reference implementations do,
including two defects found in them, and `docs/reports/m1_evidence.*` gives the
numerical evidence.

## Licence

**Not yet chosen.** Selecting a licence for this work is an authorial decision,
so the workspace manifest deliberately omits the `license` field rather than
defaulting to one. Until a licence is added, the same all-rights-reserved
default applies to this repository.
