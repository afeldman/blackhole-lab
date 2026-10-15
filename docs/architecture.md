# Architecture

This document records the workspace layout, the responsibility of each crate,
the decisions taken for milestone M2, and the reasoning behind them. It is the
place to look before proposing a structural change.

## Layout

    blackhole/
    ├── Cargo.toml                      virtual workspace manifest
    ├── crates/
    │   ├── blackhole-core/
    │   │   ├── Cargo.toml              no dependencies
    │   │   └── src/
    │   │       ├── lib.rs              crate docs, re-exports, the doctest
    │   │       ├── constants.rs        physical and IAU nominal constants
    │   │       ├── error.rs            PhysicsError, Quantity
    │   │       ├── units.rs            Length, Mass, Density, unit enums
    │   │       ├── sphere.rs           sphere volume, mean density, radius
    │   │       └── schwarzschild.rs    r_s, inverse, compactness, R_krit
    │   └── blackhole-cli/
    │       ├── Cargo.toml              depends on blackhole-core and clap
    │       ├── src/
    │       │   ├── main.rs             argument parsing, dispatch, exit codes
    │       │   ├── catalog.rs          reference data and model assumptions
    │       │   ├── input.rs            number and unit parsing
    │       │   ├── format.rs           number and unit formatting
    │       │   ├── report.rs           computation and rendering
    │       │   └── interactive.rs      the prompt loop
    │       └── tests/cli.rs            process-level tests
    └── docs/

## Crate responsibilities

**`blackhole-core`** owns the physics and nothing else. It knows about SI
quantities, the constants, the Schwarzschild relation and the homogeneous-sphere
model. It does not know that a command-line interface exists: no printing, no
argument parsing, no reference catalogue, no notion of an "object" or a "mode".

**`blackhole-cli`** owns everything presentational and interactive: parsing user
input, unit selection, the reference catalogue, formatting, the dialogue and the
exit statuses. It reimplements no physics — every physical quantity it prints
comes from a `blackhole-core` call. It does form display ratios of its own from
those values (`1 / (r_s/R)` and `R / R_krit`), which is presentation, not a second
implementation of the model.

The dependency direction is one-way: `blackhole-cli → blackhole-core`. Nothing
in core may ever depend on the CLI. That is what makes the core reusable by the
crates planned for M3–M6.

## Crates that do not exist yet

The brief lists `blackhole-gravity`, `blackhole-relativity`,
`blackhole-lensing`, `blackhole-render`, `blackhole-render-wgpu`,
`blackhole-render-cuda` and `blackhole-python`, and also asks to avoid
unnecessary crate splitting while no architectural advantage is demonstrable.

A crate split pays for itself when a module has a *consumer with a different
dependency profile* — a GPU backend needing `wgpu`, a Python extension needing
`pyo3`. Creating seven empty crates now would add seven manifests, seven
compile units and seven chances for the workspace to drift, in exchange for
nothing that can be tested. Each crate is therefore created in the milestone
that first needs it, and `Cargo.toml` members are added at that point.

The planned layout is unchanged; only the timing is deferred.

## Decisions

**D1 — Validated newtypes instead of a dimension crate.**
`Length`, `Mass` and `Density` are newtypes over `f64` in SI base units with
fallible constructors. A dimension library such as `uom` provides *dimensional*
checking but cannot express "positive and finite", which is the invariant this
domain actually needs: `uom` will happily store a negative mass. Since the
brief requires that physically invalid states be unconstructable, the invariant
is owned directly. Cost: about one screen of code. Benefit: the invariant, the
error messages and the absence of a dependency.

**D2 — `blackhole-core` has zero dependencies.**
Every dependency is a supply-chain, build-time and auditability cost. The core
is pure computation on `f64`; nothing it does needs a library. The error type
implements `Display` and `std::error::Error` by hand rather than pulling in
`thiserror`, which for three variants is less code than the dependency's
manifest.

**D3 — Unit conversion through enums, not one constructor per unit.**
`Length::from_in(6371.0, LengthUnit::Kilometre)` rather than
`Length::from_kilometres(6371.0)`. Nine length units times one constructor each
would be nine places for a factor to be wrong; the enum keeps every factor in a
single `match`, which the compiler checks for exhaustiveness. The CLI then maps
a user token to a unit with one look-up and cannot silently omit a unit.

**D4 — Zero is rejected, not clamped.**
All three quantity types require strictly positive values. A zero mass or radius
is the degenerate limit of the model, and a zero density has no finite critical
radius at all. Rejecting zero removes every division-by-zero and `0/0` path from
the physics functions, which is why they never need to re-check their inputs.

**D5 — Errors are coarse and carry the operation separately.**
`PhysicsError` distinguishes *why* a value was rejected (`NotFinite`,
`NotPositive`, `NotRepresentable`), not which function rejected it. Callers that
need to name the operation supply that context. This keeps the enum small and
prevents it from growing one variant per function.

**D6 — Presentation lives in the CLI.**
`blackhole-core` has no `Display` impls for its quantity types and no formatting
policy. The CLI owns the scientific-notation convention, the unit hint ladder
and the line wrapping. A future Python binding or GPU backend must not inherit a
terminal layout from the core.

**D7 — The catalogue stores raw `f64`, validated at load.**
The reference data is a `const` table of `f64` values with their provenance,
not a table of pre-built `Mass`/`Length` values, because the validating
constructors are fallible and cannot run in a `const`. The values are validated
by a test that walks the whole table, so a typo is caught by `cargo test` rather
than when a user selects the entry.

**D8 — `Shape` makes the impossible combination unrepresentable.**
A body either has a surface — both a radius and a mean density — or it has
none, as a black hole does. Modelling that as two independent `Option` fields
would allow "radius without density" and force every consumer to handle it.
The `Shape` enum removes the state and, with it, the unreachable branches that
would otherwise sit in the report code.

**D9 — No serialisation dependency.**
The non-interactive path prints deterministic, line-oriented text. Output is
machine-readable by construction without `serde` or a JSON schema that would
have to be versioned. If structured output is wanted later, that is a deliberate
decision with its own format contract, not an accident of convenience.

**D10 — `clap` for argument parsing, hand-rolled formatting.**
`clap` (derive, `4.5`) removes a class of argument-parsing bugs that the brief's
"no panics on bad input" requirement makes relevant: missing values, `--flag
value` versus `--flag=value`, help generation, unicode. It is the de-facto
standard and is actively maintained. Word wrapping, number formatting and unit
hints are written in-house because they must be byte-reproducible and
inspectable, and a formatting library would make the output a moving target.

**D11 — Edition 2024, MSRV 1.85.**
The toolchain default for a new crate, and nothing in the project needs
compatibility with an older compiler.

**D12 — `allow_negative_numbers` on numeric arguments.**
Without it, `--mass -5` is rejected by the argument parser as an unrecognised
argument instead of reaching the physics layer, which reports the useful message
"mass must be greater than zero, got -5". The first attempt used
`allow_hyphen_values`, which accepts a negative value *and* swallows a following
flag: `--mass --radius 1` consumed `--radius` as the value and reported the
misleading "unexpected argument '1' found" rather than "a value is required for
'--mass <VALUE>'". `allow_negative_numbers` narrows the tolerance to values that
look negative, so both cases now produce accurate diagnostics. `--density` carries
the same attribute, otherwise the identical user error is a usage error for one
quantity and a physical failure for the others. Found by adversarial review, not
by reasoning.

## Error handling strategy

* The core never panics and never returns `NaN`. There is no `unwrap`, no
  `panic!` and no indexing in its non-test code. The two ways a non-finite value
  can still leave it — `to_in` overflow and `compactness` subnormal underflow —
  are documented in `docs/validation.md` rather than hidden.
* Invocation errors are distinguished from physical ones by exit status: `2` for
  a usage error (unknown unit, wrong number of arguments, unknown object), `1`
  for a physical or computational failure. `0` is success.
* For the non-interactive commands, results go to stdout and diagnostics to
  stderr, so a script can consume stdout unconditionally. The interactive
  dialogue is a dialogue: its prompts *and* its diagnostics go to stdout, and it
  reports no non-zero status for ordinary input errors, because recovering from a
  typo is part of the session rather than a failure of the command.
* A closed stdout pipe is not an error. Output is written through a locked handle
  and `BrokenPipe` maps to success, so `blackhole object Earth | head -1` behaves
  like any other Unix filter instead of panicking with status `101`.
* Input is decoded lossily. A byte that is not valid UTF-8 becomes an
  unrecognised input and is re-prompted; it never discards the session.
* In the interactive dialogue, input *typos* re-prompt for the same field, while
  *semantic* errors return to the menu. An invalid value is rejected before its
  unit is requested, so a typo cannot desynchronise the rest of the dialogue.

## Extension points for M3–M6

* The gravity, relativity, lensing and rendering crates consume
  `blackhole-core` for constants and validated quantities; they must not
  reimplement them, and they must not be reachable from `blackhole-core`.
* `PhysicsError` is `#[non_exhaustive]`, so a new variant is not a breaking
  change for downstream crates.
* The N-body engine and the Schwarzschild engine stay separate physical models.
  Nothing in `blackhole-core` may be extended into a Newtonian force law: the
  two must not be able to borrow each other's assumptions by accident.

## Open decisions

* **Licence.** Not chosen. The workspace manifest deliberately omits the
  `license` field; choosing a licence is an authorial decision, not an
  engineering default. Until one is added, all rights are reserved.
* **Version control.** `cargo new` initialised a git repository in the project
  root as a side effect, with no commits. Nothing has been committed. Whether to
  keep it, and what the first commit should contain, is a decision for the
  author.
