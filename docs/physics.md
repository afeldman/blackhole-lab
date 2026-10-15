# Physics

Equations, units, constants, worked values and — most importantly — the limits
of the model. Everything the code computes is stated here, and everything
stated here is computed by the code.

## Scope

This document covers what `blackhole-core` implements: the Schwarzschild radius,
its inverse, the compactness ratio, and the mass, mean density and critical
radius of a homogeneous sphere at an assumed density.

It does **not** cover Newtonian N-body dynamics, relativistic ray tracing,
gravitational lensing or rendering. Those are separate, later milestones; no
result in the current engine depends on them and none of them is claimed.

## Conventions

* **SI base units internally, always.** Metre, kilogram, second. Densities in
  `kg m^-3`. No unit suffix ever appears in a stored number; conversion happens
  at the boundary.
* `M` — mass in kg. `R` — a body's actual (volumetric mean) radius in m.
  `r_s` — Schwarzschild radius in m. `rho` — density in kg m^-3.
* All quantities are finite and strictly positive. Zero is rejected: a zero
  radius, mass or density is the degenerate limit of the model, not a
  representable state.

## Constants

| Symbol | Meaning | Value | Uncertainty | Source |
| --- | --- | --- | --- | --- |
| `G` | Newtonian constant of gravitation | `6.67430e-11 m^3 kg^-1 s^-2` | `2.2e-5` relative | CODATA 2018 |
| `c` | Speed of light in vacuum | `299792458 m s^-1` | exact | SI definition of the metre |
| `2G/c^2` | Mass-to-radius factor | `1.4852e-27 m kg^-1` | `2.2e-5` relative | derived |
| `(GM)_sun^N` | Nominal solar mass parameter | `1.3271244e20 m^3 s^-2` | exact by definition | IAU 2015 B3 |
| `R_sun^N` | Nominal solar radius | `6.957e8 m` | exact by definition | IAU 2015 B3 |
| `(GM)_E^N` | Nominal terrestrial mass parameter | `3.986004e14 m^3 s^-2` | exact by definition | IAU 2015 B3 |
| `R_eE^N` | Nominal Earth equatorial radius | `6.3781e6 m` | exact by definition | IAU 2015 B3 |
| `(GM)_J^N` | Nominal jovian mass parameter | `1.2668653e17 m^3 s^-2` | exact by definition | IAU 2015 B3 |
| `R_eJ^N` | Nominal Jupiter equatorial radius | `7.1492e7 m` | exact by definition | IAU 2015 B3 |
| `au` | Astronomical unit | `1.495978707e11 m` | exact | IAU 2012 B2 |
| `pc` | Parsec | `3.0856776e16 m` | exact | `648000/pi` au |
| `ly` | Light-year | `9.4607304725808e15 m` | exact | `c` times one Julian year |

The IAU *nominal* values above are conversion factors adopted to be exact by
definition (Prša et al. 2016, AJ 152, 41). They are **not** measurements of the
Sun, Earth or Jupiter and must not be quoted as such. Where a solar or
terrestrial mass is needed in kilograms, the code derives it as `(GM)/G` with
`G` stated, which is the form IAU 2015 B3 recommends.

## 1. Schwarzschild radius

    r_s = 2 G M / c^2

The radius of the event horizon of a non-rotating, uncharged (Schwarzschild)
black hole of mass `M` (Schwarzschild 1916; see Misner, Thorne & Wheeler 1973).
Implemented as `schwarzschild_radius`.

**There is no universal minimum mass.** `r_s` is defined and positive for every
positive `M`, however small, and it scales linearly with the mass. Any statement
of the form "an object must be at least this large to become a black hole" is
not a consequence of this relation.

## 2. Inverse relation

    M = r_s c^2 / (2 G)

Implemented as `mass_from_schwarzschild_radius`, and an exact inverse of the
relation above: the round trip returns the input mass to within `f64` rounding
(asserted to `1e-15` relative in the tests).

## 3. Compactness ratio

    compactness = r_s / R = 2 G M / (c^2 R)

Dimensionless. Implemented as `compactness`.

**Naming warning.** In the general-relativity literature "compactness" usually
means `G M / (c^2 R)`, which is exactly **half** this value. Everything in this
project uses `r_s / R`, because that is the ratio the model limits are stated
in and the one the CLI compares. Whenever a compactness number is published or
compared with the literature, state which convention is meant.

Interpretation:

* `r_s / R < 1` — the surface lies outside the horizon *under the stated
  assumptions*.
* `r_s / R >= 1` — the model places the surface at or inside its own horizon.
  The model's premises (a static homogeneous sphere of constant density in flat
  space) are then no longer self-consistent. **This is not a prediction that a
  black hole forms**; it is a statement that the number has left the domain in
  which the model means anything.

For reference, the compactness `r_s / R` of a real body is not the quantity the
literature gives as `GM/(c^2R)`: a neutron star with `GM/(c^2R) ~ 0.17` has
`r_s / R ~ 0.34`.

## 4. Homogeneous sphere

    V = (4/3) pi R^3
    M = (4/3) pi R^3 rho
    rho_bar = M / ((4/3) pi R^3)
    R = (3 M / (4 pi rho))^(1/3)

Implemented as `sphere_volume`, `mass_from_radius_density`, `mean_density` and
`radius_from_mass_density`. The sphere volume appears in exactly one place in
the code, so the geometric assumption exists once.

`rho_bar` is the density a *homogeneous* sphere would need in order to have the
given mass and radius. For a body that is not homogeneous it is the mean
density, and not the local density anywhere in particular.

## 5. Critical radius at an assumed density

Set the Schwarzschild radius of a homogeneous sphere equal to its own radius:

    M(R) = (4/3) pi R^3 rho
    r_s(R) = (2G/c^2) M(R) = (8 pi G rho / (3 c^2)) R^3

Requiring `R = r_s(R)`:

    R = (8 pi G rho / (3 c^2)) R^3
    1 = (8 pi G rho / (3 c^2)) R^2
    R_krit = sqrt( 3 c^2 / (8 pi G rho) )

Implemented as `minimum_radius_at_density`. An algebraically equivalent form,
which the tests use as an independent control, follows with `kappa = 2G/c^2`:

    R_krit = sqrt( 3 / (4 pi rho kappa) )

### Reading this number correctly

* `rho` is a **model assumption**, not a property of the object. The result
  scales as `rho^(-1/2)`; a hundredfold denser assumption gives a tenfold
  smaller radius.
* It is the solution of an algebraic identity for a *static, homogeneous,
  constant-density sphere*. It is not a universal threshold, not a minimum size
  for black holes, and not a collapse criterion.
* Nothing here implies that a body larger than some radius must collapse, or
  that a body smaller than `R_krit` cannot.
* The name is deliberately `minimum_radius_at_density` rather than
  "minimum black-hole size": the brief's alternative name
  `critical_radius_under_assumed_density` was considered and dropped as the
  longer of two names for one function. Both describe the same, narrower thing.

## Reference values

Computed by this engine from the catalogued inputs. The input values and their
sources are in `crates/blackhole-cli/src/catalog.rs`; the derivations were
verified independently in `docs/reports/m1_evidence.py`.

| Object | Mass (kg) | R (m) | r_s (m) | r_s / R | R_krit at own mean rho (m) |
| --- | --- | --- | --- | --- | --- |
| Sun | 1.9884e30 | 6.957e8 | 2.953235e3 | 4.244984e-6 | 3.376634e11 |
| Earth | 5.9722e24 | 6.371e6 | 8.870103e-3 | 1.392262e-9 | 1.707446e11 |
| Moon | 7.346e22 | 1.7374e6 | 1.091051e-4 | 6.279794e-11 | 2.192437e11 |
| Jupiter | 1.89813e27 | 6.9911e7 | 2.819164e0 | 4.032503e-8 | 3.481434e11 |
| Sagittarius A* | 8.544197e36 | — | 1.269012e10 | — | — |
| neutron star (representative) | 2.783774e30 | 1.2e4 | 4.134550e3 | 3.445458e-1 | 2.044362e4 |
| 1 m sphere of water | 4.188790e3 | 1.0 | 6.221e-24 | 6.221e-24 | 4.009208e11 |

Cross-checks against published values:

* `r_s(Sun) = 2.953 km`; the commonly quoted figure is 2.95 km.
* `r_s(Earth) = 8.87 mm`; the commonly quoted figure is 8.87 mm.
* `r_s(Jupiter) = 2.82 m`. `r_s(Moon) = 0.109 mm`.
* `r_s(Sgr A*) = 1.269012e10 m` for `4.297e6 Msun`, matching the
  `SagA_rs = 1.269e10` hard-coded in the reference repository's shader to a
  relative deviation of `1.0e-4`.
* Mean densities derived here from mass and radius reproduce the fact-sheet
  values: Earth `5513.443` versus `5513 kg m^-3`, Moon `3343.974` versus
  `3344 kg m^-3`, Jupiter `1326.175` versus `1326 kg m^-3`.

Two derivations deserve a note because they are close but not identical:

* Sun: this engine gives a mean density of `1409.773 kg m^-3`, the NSSDCA sheet
  lists `1408 kg m^-3`. The difference (0.13 %) is consistent with the rounding
  of the tabulated mass to five significant figures and with
  `R_sun^N = 6.957e8 m` being the IAU nominal photospheric radius while the
  sheet's own density is computed from its own quoted radius. Neither value is
  wrong; they answer slightly different questions.
* The `1 m` sphere of water row uses `rho = 1000 kg m^-3` exactly, the nominal
  round value. Liquid water at 4 °C is `999.97 kg m^-3`.

## Uncertainty budget

The results are closed-form, so the only numerical error is floating-point
rounding, at the level of `1e-16` relative. The physical uncertainty comes from
the inputs, and `G` dominates it:

| Quantity | Relative uncertainty | Dominated by |
| --- | --- | --- |
| `r_s = 2GM/c^2` | `2.2e-5` | `G` |
| `M = r_s c^2/(2G)` | `2.2e-5` | `G` |
| `compactness = r_s/R` | `2.2e-5` (+ radius uncertainty) | `G` |
| `M = (4/3) pi R^3 rho` | input-dependent | `R` (cubed) and `rho` |
| `R_krit = sqrt(3c^2/(8 pi G rho))` | `sqrt((1.1e-5)^2 + (delta rho/2rho)^2)` | `G` and `rho` |

`G` is the least precisely known constant used here, by about five orders of
magnitude. Results are nevertheless returned at full `f64` precision: they are
*computed* numbers, not measurements, and rounding them would destroy the exact
algebraic identities the tests rely on. The uncertainty belongs to the physics,
not to the arithmetic.

## Classification of the results

**Analytically exact in exact arithmetic.** All of them. Every function in
`blackhole-core` is a closed-form expression: `r_s`, its inverse, `M(R,rho)`,
`rho_bar(M,R)`, `R(M,rho)`, `compactness` and `R_krit`. There is no iteration,
no discretisation, no solver and no numerical scheme, so there is no
truncation, convergence or stability error to report. The only departure from
the exact value is floating-point rounding.

**Numerically approximated.** Nothing in the engine. (The lensing and N-body
milestones will contain numerical integration; when they do, their error
behaviour belongs in `docs/numerics.md`.)

**Model-contingent, not measured.** `R_krit` in every row of the table above,
and every `rho_bar` derived for a body that is not homogeneous. These are the
outputs of a stated assumption, and they change if the assumption changes.

**Purely illustrative.** The unit hints appended by the CLI (`= 6371.000 km`,
`= 1.000005 Mearth`) are presentation. The material densities are representative
values, not measurements of a sample; the `neutron star (representative)` entry
is labelled a model assumption in the catalogue for exactly this reason. These
are classified in the type system — `Provenance::Reference` versus
`Provenance::ModelAssumption` — and printed with every result.

## Model limits

Stated once, here, in the form the CLI prints with every result:

1. **No universal minimum mass.** Every mass has an `r_s`. A mass and an `r_s`
   say nothing on their own about whether a horizon exists.
2. **The density is an assumption.** Compressing a real body changes its
   density, and `R_krit` follows the assumption.
3. **A comparison of lengths is not a collapse simulation.** `R` versus `r_s`
   compares two lengths computed from a static model.
4. **Horizon formation depends on more than these formulae.** It depends on the
   spacetime geometry, on the state of the matter (pressure, temperature,
   composition, angular momentum) and on the dynamics of the collapse. None of
   that is modelled here.
5. **Newtonian mechanics cannot represent an event horizon.** The N-body
   engine planned for M3 is not a substitute for the relativistic treatment and
   must not be presented as one. Conversely, the Schwarzschild relations here
   are not a substitute for dynamical general relativity.

A related consequence, easy to miss: comparing `R` with `r_s` for an object such
as a star is not physically informative about collapse, because the object's
density is nothing like a homogeneous constant. The comparison is arithmetically
well defined and physically shallow. The homogeneous-sphere model is a useful
first approximation for small, self-gravitating, roughly uniform bodies; it is a
poor description of a star, a planet or an accretion flow, all of which are
radially stratified.

## References

* Schwarzschild, K. (1916), *Sitzungsber. Preuss. Akad. Wiss. Berlin*, 189–196.
* Misner, C. W., Thorne, K. S. & Wheeler, J. A. (1973), *Gravitation*,
  W. H. Freeman.
* Tiesinga, E., Mohr, P. J., Newell, D. B. & Taylor, B. N. (2021), CODATA 2018
  recommended values, *Rev. Mod. Phys.* **93**, 025010.
* Prša, A. et al. (2016), IAU 2015 Resolution B3 nominal conversion constants,
  *AJ* **152**, 41 (arXiv:1510.07674).
* IAU 2012 Resolution B2 — definition of the astronomical unit.
* NASA NSSDCA planetary fact sheets, NASA Goddard Space Flight Center
  (Sun, Earth, Moon and Jupiter sheets, last updated 2024) — catalogued masses
  and volumetric mean radii.
* GRAVITY Collaboration et al. (2022), *A&A* **657**, L12
  (doi:10.1051/0004-6361/202142465) — mass of Sagittarius A * from
  interferometric astrometry of multiple stellar orbits:
  `M = (4.297 +/- 0.012)e6 Msun`.
