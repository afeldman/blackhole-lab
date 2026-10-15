# Visualization

## What these images are

| File | What it is |
| --- | --- |
| `assets/blackhole-hero.png` | Rendering of a non-rotating Schwarzschild black hole with a thin accretion disk. 1920 x 1080 (16:9). |
| `assets/blackhole-scientific.png` | The same rendering with the physically meaningful radii and a scale in impact parameter drawn on top, and a provenance caption. |

**Neither image was produced by this project's rendering engine.** The
`blackhole-lensing` and `blackhole-render*` crates do not exist yet: they are
milestones M4 and M5. The images were produced by a standalone Python tool,
`tools/visualization/schwarzschild_raytrace.py`, which depends only on numpy and
Pillow. It adds no GPU dependency and no build surface to the Rust workspace.

They are **conceptual illustrations**, and the annotated variant says so on its
face. They are not output of a validated relativistic radiative-transfer code,
and nothing in the README or the documentation should be read as claiming
otherwise.

## What is actually computed

For every pixel, a null geodesic of the Schwarzschild metric is integrated
numerically in the orbital plane of that ray, with classical RK4:

    d^2u/dphi^2 = -u + (3/2) r_s u^2 ,      u = 1/r        (units: r_s = 1)

This is the exact Schwarzschild light-bending equation. The lensing is
therefore *computed*, not approximated by a deflection formula, and the
consequences are checkable:

* the apparent shadow edge must come out at `b = 3*sqrt(3)*M = (3*sqrt(3)/2) r_s`
  `= 2.5980762 r_s`, which `verify_raytrace.py` measures in the image plane;
* the weak-field deflection must approach `4M/b` with the expected next-order
  term, which the same script checks against the analytic series.

### Geometry and physics used

| Quantity | Value | Basis |
| --- | --- | --- |
| Metric | Schwarzschild (non-rotating, uncharged) | exact; no Kerr, no frame dragging |
| Event horizon | `r_s = 2GM/c^2` | definition |
| Photon sphere | `r = 3GM/c^2 = 1.5 r_s` | analytic circular null orbit |
| Shadow / photon ring | `b_crit = 3*sqrt(3)*GM/c^2 = 2.598076 r_s` | analytic capture boundary |
| Disk inner edge | `r_in = 6GM/c^2 = 3 r_s` | ISCO in Schwarzschild |
| Disk outer edge | `r_out = 14 r_s` | illustrative choice |
| Disk shape | geometrically thin, optically thick, Lambertian | model assumption |
| Radial profile | `T ~ (r_in/r)^(3/4) * (1 - sqrt(r_in/r))^(1/4)` | Shakura-Sunyaev-like |
| Doppler | circular orbit, `beta = sqrt(M/r)`, `g = 1/(gamma(1 - beta.n))` | standard kinematic approximation |
| Redshift | `sqrt(1 - r_s/r)` | static observer |
| Colour | Planck spectrum integrated against CIE 1931 observers | physical chromaticity |
| Camera | `r = 60 r_s`, inclination `82 deg` from the disk normal, `28 deg` horizontal field | illustrative |

The disk rotates prograde about the disk normal; the bright side of the disk in
the image is the approaching side, which is a consequence of the `g^4` beaming
term rather than a decorative choice.

## What is *not* computed

Stated plainly, because these are the things a reader is most likely to assume:

* **No Kerr metric.** No spin, no frame dragging, no ergosphere, no
  spin-dependent shadow asymmetry. The shadow is circular, as it must be for a
  Schwarzschild hole.
* **No general-relativistic radiative transfer.** No Compton scattering, no
  polarised transfer, no returning radiation, no disk self-illumination, no
  radiative coupling between the disk and the plunging region.
* **No GRMHD.** The disk is not evolved; its structure is imposed.
* **No disk thickness.** The disk is a mathematical surface. Its rim is
  therefore a hard edge, which a finite-thickness disk would soften. This is
  visible as a thin bright line where the line of sight grazes the disk plane.
* **The emitted spectrum is rescaled into the visible band.** A real accretion
  disk around a stellar-mass hole radiates predominantly in the UV and X-ray,
  not in visible light. The *relative* spectral shift with radius and with
  orbital velocity is physical; the absolute band is an illustrative choice.
* **The flux and colour ranges are compressed for display.** The physical
  scalings `I ~ T^4` and `T ~ (r_in/r)^(3/4)` span about four orders of
  magnitude across this disk. Rendered directly they clip to white and destroy
  the palette, so the displayed flux and colour use compressing exponents
  (`disk_flux_exponent`, `disk_colour_exponent`). These change no conserved
  quantity and no lensing result; they are a display transform, and the
  commented constants in `SceneConfig` say so.
* **The star field is procedural**, generated from a seeded hash on a direction
  grid. It is not a real sky survey, and its star density and brightness
  distribution are aesthetic choices, not measurements.
* **Pixel directions are treated as coordinate-basis directions** at the camera
  radius. A static-observer local frame differs by
  `sqrt(1 - r_s/r_obs) = 0.9917` at `r_obs = 60 r_s`, i.e. 0.8 %. This affects
  the screen-to-impact-parameter calibration only. It does not enter the
  geodesic integration, and it does not affect checks C1-C4 in
  `docs/reports/viz_evidence.txt`.

## Verification

The tool is checked against independent analytic values by
`tools/visualization/verify_raytrace.py`, which records its output to
`docs/reports/viz_evidence.txt`. The checks were written following
`docs/process/scientific-evidence-review.md`: every one compares against an
independent analytic reference, none compares the tool against itself, and
scaling claims are not counted as absolute validation.

| Check | Claim | Result |
| --- | --- | --- |
| C1 | `r = 1.5 r_s` is an exact circular null orbit (`u'' = 0`), and it is unstable | exact identity, residual `0.000e+00` |
| C2 | the first integral `(du/dphi)^2 + u^2(1 - r_s u)` is conserved along integrated geodesics | worst residual `5.1e-8`; converges under step halving |
| C3 | the capture boundary is at `b_crit = 3*sqrt(3)*M` | `2.598076210` vs analytic `2.598076211` r_s, `4.9e-10` relative, monotone in the winding cap |
| C4 | weak-field deflection approaches the analytic series `4M/b + (15pi/4)(M/b)^2 + (128/3)(M/b)^3` | residual falls from `6.9e-4` at `b = 20 r_s` to `1.1e-6` at `b = 500 r_s`, and does not move when the step is halved -> series truncation, not integration error |
| C5 | the shadow edge measured in the *image plane* at the render camera | bisected `2.598076210` r_s, `3.4e-10` relative; the analytic value also lies inside the scanline bracket |
| C6 | the disk inner edge is the ISCO and the radial profile peak is where it should be | `T(r_in) = 0`; peak at `49/36 r_in`, `2.3e-5` relative |

Four defects were found by these checks and fixed; they are recorded in
`docs/reports/viz_evidence.txt` and in the source comments:

1. The escape crossing was recorded one full step past the boundary. In the far
   field a single step crosses several orders of magnitude in radius, so the
   recorded state was unusable and the invariant showed `1e-4` instead of
   `1e-8`. Fixed with cubic Hermite dense output.
2. The bisection that locates the escape crossing had an inverted predicate and
   collapsed to the end of the step. Found by instrumenting the invariant step
   by step rather than by reasoning about it.
3. The outgoing tangent was evaluated as `(dr/dphi) e1 + r e2`, which is the
   tangent only at `phi = 0`. It collapsed every off-axis ray onto nearly the
   same sky direction, magnifying individual stars into large soft blobs, and it
   corrupted the Doppler direction. The correct tangent uses the basis at the
   ray's current `phi`.
4. The CLI's `argparse` defaults duplicated the scene parameters and silently
   overrode every geometry change made in `SceneConfig`.

Two defects were also found in the *checks themselves* and corrected: C4's
reference omitted the finite-radius geometry (a systematic offset growing with
`b`), and C5's tolerance was finer than its own measurement resolution.

## Reproducing the images

    /opt/homebrew/bin/python3.14 tools/visualization/verify_raytrace.py
    /opt/homebrew/bin/python3.14 tools/visualization/schwarzschild_raytrace.py \
        --width 1920 --height 1080 --supersample 2 --out assets/blackhole-hero.png
    /opt/homebrew/bin/python3.14 tools/visualization/annotate.py \
        --in assets/blackhole-hero.png --out assets/blackhole-scientific.png

The render is deterministic: the star field and nebula come from a fixed seed,
and no hash-order or time-dependent quantity enters the output. Verified by two
runs at 960 x 540 producing byte-identical files
(sha256 `3f0edda37f79e59c7eb1af602016efdda617b5f0497a67926e0375067621422d`).
Rendering at 1920 x 1080 with 2x supersampling traces 3840 x 2160 rays and took
119 s on an Apple-silicon laptop; the verification suite takes about 75 s.

## The annotated variant

`assets/blackhole-scientific.png` overlays:

* **the photon ring**, drawn as a solid ring at the *apparent* size of
  `b_crit = 2.598076 r_s`. This is the one radius in the picture that is an
  actual image;
* **the event horizon** `r = r_s` and **the photon sphere** `r = 1.5 r_s`, drawn
  as dashed circles at the angular scale they would subtend *without* lensing
  (`asin(r_s/r_obs)` and `asin(1.5 r_s/r_obs)`), because neither surface is
  directly imaged. The horizon lies inside the shadow, and the photon sphere is
  not a surface at all;
* a **scale in impact parameter** `b`, with a tick at the critical value;
* labels for the disk, its lensed far-side image, and the observer geometry;
* a caption giving the model, the integrations and the limitations.

The distinction between the three radii is the point of the annotated version:
the shadow is `2.6 r_s` in impact parameter while the horizon is at `1 r_s`, so
the dark region is about 2.6 times larger than the horizon would lead one to
expect. Drawing a ring at `r_s` and calling it the horizon would be wrong.

Labels are placed in dark regions and on soft panels; the mathematical labels
carry the same expressions as the table above.

## Provenance of the algorithms

The Schwarzschild geodesic equation, the photon-sphere radius, the critical
impact parameter and the Shakura-Sunyaev profile are standard results, derived
here from the literature rather than copied from any implementation. The
`M1-analysis.md` audit found that the two reference repositories named in the
project brief carry no licence, so no code was taken from them; the same
constraint applies to this tool, which is why the ray tracer is written from the
governing equation and validated against analytic values rather than adapted.
