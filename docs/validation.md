# Validation

What is asserted, against what, with which tolerance, and how to reproduce it.

## Principle

A test that re-implements the formula under test proves only that the author can
write the same expression twice. The tests here therefore use controls that are
independent of the implementation:

1. **Published reference values** — masses, radii and derived quantities taken
   from NASA NSSDCA fact sheets, the IAU 2015 nominal constants and the GRAVITY
   Collaboration's astrometry.
2. **Independent algebraic forms** — the same quantity computed from a different
   rearrangement, e.g. `R_krit` from `sqrt(3c^2/(8 pi G rho))` and from
   `sqrt(3/(4 pi rho kappa))` with `kappa = 2G/c^2`.
3. **Fixed-point and consistency identities** that must hold for any correct
   implementation, and constrain the answer far more tightly than a tolerance
   comparison: `r_s(M(R_krit, rho)) == R_krit`, `compactness(M, r_s(M)) == 1`,
   and the exact round trips `M -> r_s -> M`, `(R, rho) -> M -> rho_bar`, and
   `(M, rho) -> R -> M`.
4. **An independently written external script**,
   `docs/reports/m1_evidence.py`, which recomputes the key identities in Python
   with a different method — including a bisection root find of
   `r_s(M(R)) - R = 0` that never uses the closed form for `R_krit` at all.
   Its recorded output is `docs/reports/m1_evidence.txt`.
5. **Process-level tests** of the built binary, which check what a user or a
   script actually observes: exit statuses, stdout/stderr separation, and
   byte-reproducibility.

## Test inventory

`cargo test --workspace` runs 78 tests:

| Suite | Count | Covers |
| --- | --- | --- |
| `blackhole-core` unit tests | 27 | constants provenance, error messages, unit conversion and validation, sphere geometry, all six physics functions, overflow and underflow bounds, subnormal conditioning |
| `blackhole-core` doctest | 1 | the crate compiles and runs as a library, with no CLI involved |
| `blackhole-cli` unit tests | 36 | lexing and unit resolution, formatting policy, catalogue integrity, report rendering, the full interactive dialogue including mode 4 and invalid UTF-8 |
| `blackhole-cli` integration tests | 14 | the built binary: exit statuses, stdout/stderr split, determinism, closed pipes, invalid UTF-8, index and name selection, interactive and batch paths agreeing on the physics |

## Reference values and tolerances

| Assertion | Reference | Tolerance | Why this tolerance |
| --- | --- | --- | --- |
| `r_s(5.9722e24 kg)` | `8.87e-3 m`, published to three significant figures `[2]` | `1e-3` relative | set from the precision of the published figure, not from this crate's own digits — a tighter tolerance could only pass by reproducing the same closed form, which would test nothing |
| `r_s(1.9884e30 kg)` | `2.953 km`, published to three significant figures `[2]` | `1e-3` relative | as above |
| `mean_density(5.9722e24 kg, 6.371e6 m)` | `5513 kg m^-3` `[3]` | absolute `1.0` | reproduces the sheet value `5513` against a derived `5513.443` |
| `R_krit(1000 kg m^-3)` | `4.009208e11 m` `[1]` | `1e-5` relative | independent Python evaluation, two methods |
| `M(1 m, 1000 kg m^-3)` | `4188.790204786391 kg` | `1e-9` absolute | `(4/3) pi` computed independently |
| `R_krit` self-consistency | `r_s(M(R_krit, rho)) == R_krit` | `1e-12` relative | an identity, so only rounding is permitted |
| `compactness(M, r_s(M))` | exactly `1` | `1e-15` | an identity |
| mass/radius round trip | original mass | `1e-15` relative | exact inverse |
| `R_krit` density scaling | factor `10` for `rho x 100` | `1e-12` relative | an identity of the exponent |

Sources: `[1]` `docs/reports/m1_evidence.py` (this project, independent Python);
`[2]` commonly quoted values for the Earth and the Sun, each stated to three
significant figures; `[3]` NASA NSSDCA Earth Fact Sheet.

Two of the assertions are deliberately weaker than they look, and are labelled
as such rather than counted as external checks:
`volume_scales_with_the_cube_of_radius` (`V(2R)/V(R) = 8`) and
`critical_radius_scales_as_inverse_sqrt_density` (`R_krit` for `100 rho`) would
hold for any constant in front of `R^3` and `rho^(-1/2)` respectively, so they
constrain the exponent only. The constants themselves are pinned elsewhere:
`sphere_volume` at `R = 1 m` against `4 pi/3`, and `minimum_radius_at_density`
against the independent rearrangement
`sqrt(3/(4 pi rho kappa))` with `kappa = 2G/c^2`.

## Boundary and validation coverage

The following are asserted to be *rejected*, not silently tolerated:

* `NaN`, `+inf`, `-inf`, `0` and negative values for length, mass and density,
  through every constructor of every quantity type.
* Overflow on conversion, e.g. `1e308` light-years.
* Overflow on a derived result: `R^3` for `R = 1e120 m`,
  `mass_from_schwarzschild_radius(Length::from_in(1e308))`, and
  `mass_from_radius_density` at the same radius.
* Underflow to zero: a radius below about `8.4e-109 m` has no representable
  volume, and the failure is reported as `NotRepresentable`, not as `0` or
  `NaN`. The corresponding overflow bound is `3.5e102 m`;
  `minimum_radius_at_density` overflows below a density of
  `5.0e-283 kg m^-3`. Each bound has a test that checks just inside it and just
  outside it, so the documented number and the implemented behaviour cannot
  drift apart.
* Ambiguous numeric input: `1,234.5` mixes decimal separators and is rejected
  rather than guessed at, while `1,5` and `1.5` are both accepted.
* Unknown unit tokens, unknown material names, unknown objects and unknown modes.
* Non-existent catalogue entries cannot be created: the caches are `const`
  tables of raw `f64` and a test walks every row validating it, so a typo fails
  `cargo test` rather than surfacing to a user.

A `NaN` can never escape the core: every public function returns a `Result`, and
there is no `unwrap`, `panic!` or indexing in the crate's **non-test** code — the
test modules use them freely, as tests should. The `forbid(unsafe_code)` lint is
set workspace-wide and enforced by `cargo clippy`.

Two ways a non-finite or zero value can still leave the core are documented
rather than hidden, both confined to inputs no physical model would produce:

* `Length::to_in` and `Mass::to_in` are unvalidated `f64` conversions, so a
  magnitude within a few orders of `f64::MAX` overflows to infinity when
  expressed in a smaller unit. The SI accessors — `metres`, `kilograms`,
  `kilograms_per_cubic_metre` — cannot overflow, and the CLI uses only those for
  display.
* `compactness` can underflow to zero only if the true ratio lies below the
  smallest subnormal. The expression is deliberately ordered `(M / R) * kappa`
  rather than `(M * kappa) / R` to push that boundary as far out as possible: for
  `M = 1e-300 kg` and `R = 1e-6 m` the second ordering underflows to zero while
  the true value `1.4852e-321` is representable, and a test asserts that the
  representable value is what is returned.

## Independent defect check on the reference implementations

The reference repositories named in the brief are not sources of code (see
`docs/reports/M1-analysis.md` for the licence finding), but their algorithms
were audited, and one of their defects is a useful *negative* test of our own
reasoning: the reference 3D and GPU ray-tracing code omits the factor
`f = 1 - r_s/r` in the angular term of the Schwarzschild radial geodesic
equation, which reduces the coefficient of the quadratic term in the
`u = r_s/r` form from `3/2` to `1/2`. `docs/reports/m1_evidence.py` check C4
integrates both variants and shows that the defective one under-predicts the
weak-field deflection `alpha = 4GM/(c^2 b) = 2 r_s / b` by a ratio converging
monotonically to `0.333` as `b` grows, while the correct variant matches both the
leading-order result and the three-term analytic series
`4M/b + (15 pi/4)(M/b)^2 + (128/3)(M/b)^3` to `5e-6` relative at `b = 100 r_s`,
with a residual against the leading order that agrees with the predicted
next-order term to three significant figures and shrinks in proportion to `1/b`.
That is the expected behaviour of the expansion, not a fitted agreement.

This check matters here because it validates the derivation that the M4 lensing
milestone will depend on, before any lensing code exists.

## Process robustness

Two failure modes specific to running as a program — as opposed to as a library
— are covered by process-level tests. Both were found by adversarial review, not
by design, and both contradicted a claim this document previously made:

* **A closed stdout pipe.** `println!` aborts with a panic and exit status `101`
  when the reader goes away, so `blackhole object Earth | head -1` used to
  terminate with a panic message — neither a usage error nor a physical failure.
  Output is now written through a locked handle and `BrokenPipe` is mapped to
  success, which is the conventional Unix behaviour for a consumer that stopped
  reading. The test pipes a report larger than the pipe buffer into a reader that
  closes after 16 bytes and asserts that nothing panicked and that the status is
  not `101`.
* **Invalid UTF-8 on stdin.** `read_line` returns a decode error, which used to
  end the whole dialogue with exit `1` and an I/O message belonging to no
  documented category. One stray byte should not discard a session, so stdin is
  decoded lossily: the offending line becomes an unrecognised input, is
  re-prompted, and the dialogue continues. The test feeds a lone `0xFF` followed
  by a valid dialogue and asserts that a result is still produced.

The acceptance script asserts on the *content* of every case, not only its exit
status: a case that exits `0` without printing a `Results` block is counted as a
failure, and the script exits non-zero if any case fails. That check was added
after a scripted dialogue was found to have computed nothing while the transcript
recorded it as a success — an exit status alone cannot tell "it worked" from "it
silently did nothing".

## Determinism

* Two runs of the same non-interactive invocation produce byte-identical
  stdout, asserted both in-process (`report::tests`) and across processes
  (`tests/cli.rs`, and section 6 of the acceptance transcript). No hash map
  iteration, no time stamp and no floating-point-order dependence enters the
  output.
* The interactive dialogue and the non-interactive path are asserted to produce
  identical value rows for the same physical input, so the two front ends cannot
  diverge.

## How to reproduce

    cargo fmt --all --check
    cargo clippy --workspace --all-targets
    cargo test --workspace
    python3 docs/reports/m1_evidence.py      # independent physics checks
    bash   docs/reports/m2_cli_checks.sh     # full acceptance transcript

Recorded results at the time of writing, on `aarch64-apple-darwin`:

* `cargo fmt --check`: clean.
* `cargo clippy --workspace --all-targets`: `clippy::all` and
  `clippy::pedantic` enabled workspace-wide, **zero warnings**.
* `cargo test --workspace`: `78 passed, 0 failed`.
* `m2_cli_checks.sh`: **31 cases, 31 passed, 0 failed** — the script computes a
  verdict per case and exits non-zero if any case fails. All four non-interactive
  combinations, four catalogued selections (three by name, one by index) and six
  interactive dialogues produce a `Results` block; thirteen usage cases exit `2`;
  three physics cases and one unrepresentable-result case exit `1`; the
  determinism checks report byte-identical output; and the interactive and batch
  value rows are identical. Full transcript in
  `docs/reports/m2_cli_evidence.txt`.

## Not yet tested

Deliberately deferred, because the code does not exist yet. Listing them here so
that no reader mistakes their absence for a gap in the current coverage:

* N-body conservation of energy, momentum and angular momentum (M3).
* Integrator order of accuracy, symplecticity and energy drift (M3).
* Analytic point-mass lensing against `alpha = 4GM/(c^2 b)` in the engine, and
  Einstein-ring positions (M4).
* Schwarzschild null-geodesic integration, convergence and horizon handling (M4).
* CPU/GPU agreement within stated numerical tolerances (M5).
* Rust/Python parity of the public API (M6).

`docs/numerics.md`, `docs/rendering.md` and `docs/python.md` are created with the
milestones that need them; writing them now would document behaviour that does
not exist.
