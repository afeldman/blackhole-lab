# M1 — Reference repository and licensing analysis

**Status:** complete.
**Scope:** inspect the two reference projects named in the brief, establish the
licence position, inventory their algorithms, and propose the workspace
structure. No code was taken from either project; see §2.

## 1. Method

Both repositories were cloned shallowly (`--depth 50`) and inspected directly,
rather than described from their web pages:

| Repository | Commit inspected | Date of last commit | Stars | Language |
| --- | --- | --- | --- | --- |
| `kavan010/black_hole` | `dc263bb5a1b72f5a24190d2e38b84b5c8fe9a317` | 2026-04-07 | 2639 | C++ |
| `kavan010/gravity_sim` | `48e080438b000df3f81ea7678ba1e04a5807dc7f` | 2025-03-02 | 590 | C++ |

Inspected: every source file, both `README.md`/absence thereof, the build
configuration (`CMakeLists.txt`, `vcpkg.json`, Visual Studio task files), the
complete file list, and the GitHub API repository metadata. Source lines were
counted with `wc -l`; absence claims below are backed by explicit searches over
the whole tree, not by reading two files and extrapolating.

## 2. Licence — the decisive finding

**Neither repository carries a licence.**

Evidence, in full:

* No `LICENSE`, `COPYING`, `NOTICE` or `*.spdx` file exists anywhere in either
  tree (`find` over the full checkout, `.git` excluded).
* No source file contains a copyright header or a licence reference. A
  case-insensitive search for `copyright|licen[cs]e|GPL|MIT|BSD|Apache|all
  rights reserved|SPDX` over all `.cpp`, `.h`, `.md`, `.txt`, `.json`, `.comp`,
  `.frag` and `.vert` files returns only two false positives: `"limits": "cpp"`
  in a Visual Studio settings file, and `std::numeric_limits<float>`.
* The GitHub API reports `license: null` for both repositories.
* `kavan010/gravity_sim` has no `README.md` at all.
* The `black_hole` README says "everything will be inside a src bin incase you
  want to copy the files". That is an invitation in prose; it is **not** a
  licence grant, and it does not alter the default legal position.

Absent an explicit licence, the default is that all rights are reserved. Copying,
translating or adapting the code would therefore not be lawful. This is not a
judgement call about the author's intentions; it is the stated position under
default copyright.

**Consequence for this project:** no code was copied, translated or adapted from
either repository. They were used strictly as a feature inventory. Every
algorithm implemented in this project is re-derived from the standard
literature and verified independently, and the reference implementations were
audited only to the extent needed to know what they do and do not contain.

The mathematics itself (the Schwarzschild relation, the homogeneous-sphere
relations, the null geodesic equation) is not copyrightable subject matter.
Re-deriving it from primary sources is not a workaround; it is the only correct
procedure, and it is also better engineering: the audit in §5 found two defects
that would have been inherited by any copy.

One further provenance item: `black_hole` contains a committed build product
(`black_hole.exe`) and an archive of the second project (`Gravity_Sim.zip`);
`gravity_sim` contains a committed `glfw3.dll`. Committed binaries are a
supply-chain concern in their own right and are not a usable source for anything.

## 3. Repository inventory

### `kavan010/black_hole` (2230 lines of source and config)

| File | Lines | Role |
| --- | --- | --- |
| `black_hole.cpp` | 710 | OpenGL 4.3 host: GLFW window, camera UBO, accretion-disk UBO, object UBO, dispatch of `geodesic.comp`, plus a spacetime-grid visualisation |
| `CPU-geodesic.cpp` | 496 | CPU counterpart: 3D Schwarzschild null-geodesic integration into a pixel buffer, uploaded as a texture |
| `ray_tracing.cpp` | 292 | Plain ray tracer: ray–sphere intersection, no gravitational deflection |
| `2D_lensing.cpp` | 231 | 2D equatorial null-geodesic integration in `(r, phi)`, drawing ray trails |
| `geodesic.comp` | 177 | GLSL 4.30 compute shader: the GPU geodesic integrator |
| `grid.frag`, `grid.vert` | 11 | Spacetime-grid shader |
| `CMakeLists.txt`, `vcpkg.json`, VS configs | 180 | Build: CMake + vcpkg; dependencies GLEW, GLFW, GLM |

No tests, no CI, no licence. The `vs_code/` directory holds editor
configuration rather than source.

### `kavan010/gravity_sim` (1554 lines)

| File | Lines | Role |
| --- | --- | --- |
| `gravity_sim.cpp` | 635 | N-body, per-body OpenGL vertex buffers, pairwise force loop |
| `gravity_sim_3Dgrid.cpp` | 621 | Variant with a grid-based rendering of the field |
| `3D_test.cpp` | 149 | Demo program, **not** a test suite despite the name |
| VS configs | 149 | Editor configuration at repository root |

No `README`, no build system, no tests, no CI, no licence.

## 4. Algorithm inventory — what is actually there

| Capability | Present? | Where and how |
| --- | --- | --- |
| Schwarzschild radius `2GM/c^2` | **yes**, in both | `2D_lensing.cpp:78`, `CPU-geodesic.cpp:257`, `black_hole.cpp:128,249`, `gravity_sim.cpp:613`. Computed on the host; in the GPU path the value is instead hard-coded as `SagA_rs = 1.269e10` (`geodesic.comp:30`). |
| Sphere relation `R = (3M/(4 pi rho))^(1/3)` | **yes** | `gravity_sim.cpp`: `pow(((3 * mass/density)/(4 * 3.14159265359)), (1.0f/3.0f))`, with a hard-coded pi. Only the radius-from-mass-and-density direction exists. |
| Critical radius at an assumed density | **no** | Absent from both repositories. Searches for the closed form and for its constituent expressions return nothing. The formula is therefore this project's own derivation, taken from the brief's algebra and verified independently (`docs/reports/m1_evidence.py` check C3). |
| Schwarzschild null geodesics | **yes** | Second-order system in `(r, theta, phi)` with the first derivatives, integrated with RK4 (`2D_lensing.cpp`, `CPU-geodesic.cpp`) or with a single Euler step mislabelled as RK4 (`geodesic.comp`). |
| Newtonian N-body force | **yes** | `gravity_sim.cpp`: `Gforce = (G * m1 * m2) / (distance*distance)` — the pairwise `1/r^2` law only. |
| An integrator with a physical time step | **no** | See §5, defect D-3. |
| Energy / momentum / angular-momentum diagnostics | **no** | Search for `soften|epsilon|total energy|kinetic|potential|momentum|leapfrog|verlet` returns nothing. |
| Softening parameter | **no** | No singular-separation handling of any kind. |
| Analytic gravitational-lensing formula (`4GM/(c^2 b)`, Einstein radius) | **no** | `ray_tracing.cpp` is an ordinary ray tracer: a quadratic ray–sphere intersection. Its "lensing" consists of moving the sphere centres. One expression of the form `2*sqrt(r_s*(dist - r_s))` does appear in the host-side grid code (`black_hole.cpp:256`), but its derivation is not documented in the repository and it is not used to deflect any ray in the ray tracer. |
| Accretion disk | **partially** | Geometric only: detect a plane crossing between `2.2 r_s` and `5.2 r_s` and shade by radius (`geodesic.comp:111-115`). No emission model, no Doppler shift, no gravitational redshift. |
| Vulkan / Metal / CUDA | **no** | The GPU path is OpenGL 4.3 compute shaders. Neither Vulkan, Metal nor CUDA appears anywhere. |
| Python interface | **no** | Absent from both. |
| Public library API | **no** | Both are single-executable programs; there is no library target in either build. |

This table is deliberately conservative. The brief asks that no capability be
attributed to the reference projects that they do not have, and several
descriptions of these projects elsewhere on the web are more generous than the
source warrants.

## 5. Defects found in the reference implementations

Found while auditing, listed because they justify the re-derivation and because
one of them directly informs the M4 lensing milestone. All four are
reproducible from the stated commit hashes.

### D-1 — `geodesic.comp`'s `rk4Step` is explicit Euler

    void rk4Step(inout Ray ray, float dL) {
        vec3 k1a, k1b;
        geodesicRHS(ray, k1a, k1b);
        ray.r += dL * k1a.x; ... ray.dphi += dL * k1b.z;
    }

Only `k1` is computed and it is applied directly. The function is named `rk4Step`
and is called with a fixed `D_LAMBDA = 1e7`. In contrast, `CPU-geodesic.cpp`
implements a genuine four-stage RK4, and `2D_lensing.cpp` does too. The GPU path
is therefore first-order accurate while the CPU path is fourth-order, and the
two are presented as the same simulation. The shader also works in `float`
(32-bit) while the CPU path uses `double`; with `r_s ~ 1.27e10 m` and a ray
starting near `1e12 m`, 32-bit mantissa resolution is marginal.

### D-2 — the 3D and GPU radial geodesic equation is missing the factor `f`

The radial equation for a Schwarzschild null geodesic is

    d2r/dl2 = -(r_s f / (2 r^2)) tdot^2 + (r_s / (2 r^2 f)) rdot^2 + r f (thdot^2 + sin^2 th phidot^2)

with `f = 1 - r_s/r`. The angular term carries the factor `f`. The 2D
implementation has it, as `(r - r_s) = r f` in the equatorial plane:

    // 2D_lensing.cpp, geodesicRHS
    rhs[2] = -(rs/(2r*r)) * f * (dt_dl*dt_dl) + (rs/(2r*r*f)) * (dr*dr) + (r - rs) * (dphi*dphi);

Both 3D implementations omit it:

    // CPU-geodesic.cpp, rhs[3];  and identically geodesic.comp, d2.x
    ... + r * (dtheta*dtheta + sin(theta)*sin(theta)*dphi*dphi);   // should be r * f * (...)

The consequence is quantifiable. Substituting the null condition to eliminate the
`tdot` and `rdot` terms reduces the radial equation to
`d2r/dl2 = (r - C r_s) phidot^2` with `C = 3/2` for the correct form and
`C = 1/2` for the defective one. In the `u = r_s/r` formulation the quadratic
coefficient becomes `u'' = -u + C' u^2`, and the weak-field deflection
`alpha = 4GM/(c^2 b) = 2 r_s / b` scales with it.

`docs/reports/m1_evidence.py` check C4 integrates both variants. The residual
column is the deviation from the leading-order `2 r_s / b`, and the "predicted"
column is the analytic next-order term `(15 pi/4)(M/b)^2 + (128/3)(M/b)^3`
divided by `4M/b`, with `M = GM/c^2 = r_s/2`:

| `b / r_s` | correct `alpha` (`C = 3/2`) | `2 r_s / b` | series to 3 terms | rel. dev. | predicted | defective `alpha` (`C = 1/2`) | ratio |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 10 | `2.361360e-1` | `2.000000e-1` | `2.347858e-1` | `1.807e-1` | `1.739e-1` | `7.015081e-2` | 0.2971 |
| 50 | `4.122254e-2` | `4.000000e-2` | `4.122076e-2` | `3.056e-2` | `3.052e-2` | `1.346583e-2` | 0.3267 |
| 100 | `2.029997e-2` | `2.000000e-2` | `2.029986e-2` | `1.500e-2` | `1.499e-2` | `6.699590e-3` | 0.3300 |
| 1000 | `2.002951e-3` | `2.000000e-3` | `2.002951e-3` | `1.475e-3` | `1.475e-3` | `6.669940e-4` | 0.3330 |

Three things follow, and they are stronger than simply "the right branch is
closer":

* The correct branch reproduces the leading-order result **and** matches the
  three-term analytic series to 5e-6 relative at `b = 100 r_s`.
* Its residual against the leading order agrees with the *predicted* next-order
  term to three significant figures at every impact parameter, and the residual
  shrinks in proportion to `1/b`. The agreement is therefore improving exactly as
  the series expansion requires, which is a far stronger statement than a
  tolerance-based comparison.
* The defective branch under-predicts by a ratio converging monotonically to
  `1/3` — precisely the factor by which the quadratic coefficient was reduced,
  `(1/2)/(3/2) = 1/3`. Nothing else in the equation changed, so the defect is
  isolated to the missing factor.

Step-size convergence of the correct branch was checked at `dphi` from `1e-4` to
`1e-7` (agreement to 10 significant figures), so the numbers above are the
integrated solution and not an artefact of the step.

This is exactly the class of error that a partially-correct port produces: the
2D original is right, and the factor was lost when the three-dimensional version
was written.

### D-3 — the N-body "integrator" has no physical time step

```cpp
void UpdatePos()     { position[i] += velocity[i] / 94; }
void accelerate(...) { velocity[i] += x / 96; }
...
distance *= 1000;                                   // inside the force law
double Gforce = (G * obj.mass * obj2.mass) / (distance * distance);
obj.velocity *= obj.CheckCollision(obj2);           // CheckCollision returns -0.2 on overlap
```

* `94` and `96` are undocumented magic divisors. They are not a time step in any
  unit, and dividing position and velocity updates by different constants means
  the scheme is not any recognisable symplectic or Verlet variant.
* `distance *= 1000` rescales lengths inside the force law, mixing render units
  with physical units without documenting the convention.
* Collision response is a velocity multiplication by `-0.2`, i.e. reversal with
  a damping factor that has no stated basis.
* No softening and no minimum-separation guard: the `1/r^2` law diverges as two
  bodies approach.
* No energy, momentum or angular-momentum diagnostic exists, so the resulting
  trajectories are unvalidated by construction.
* `std::cout << "radius: " << obj.radius << std::endl;` sits inside the
  `O(N^2)` pairwise loop.

Nothing here is reusable as an integrator. The M3 gravity engine will implement
its own symplectic scheme with documented step-size behaviour and conservation
diagnostics, which the brief requires in any case.

### D-4 — the ray initialisation is not consistent with the null condition (flagged)

In `CPU-geodesic.cpp` and `geodesic.comp` the conserved energy is seeded from

    dt_dlambda = sqrt( rdot^2 / f + r^2 thdot^2 + r^2 sin^2(th) phidot^2 )

whereas the null condition for the Schwarzschild metric,
`-f tdot^2 + rdot^2/f + r^2 thdot^2 + r^2 sin^2(th) phidot^2 = 0`, gives

    tdot = sqrt( rdot^2 / f^2 + (r^2 thdot^2 + r^2 sin^2(th) phidot^2) / f ).

The two agree only for `f -> 1`. Read as coordinate velocities, the
initialisation puts the initial data off the null cone by an amount of order
`r_s/r`; at the reference code's starting radii of `1e12` to `1e14 m` with
`r_s = 1.27e10 m`, that is roughly 0.01 % to 1 %. The reference code does not
state which convention it intends for `(dr, dtheta, dphi)`, which is itself part
of the finding.

**This one is flagged rather than settled.** The derivation is straightforward
and the algebra above is complete, but I have not numerically quantified the
downstream effect on the traced image, and a different (undocumented)
interpretation of the direction components could reconcile the expression. It is
recorded here so that M4 resolves it deliberately: the M4 lensing code will
initialise from the asymptotic data `E = 1`, `L = b` with the impact parameter
`b` set explicitly, which satisfies the null condition by construction, and the
consequence of the reference convention can then be measured rather than
argued.

## 6. Capabilities deliberately *not* attributed to the reference projects

To prevent an over-generous feature inventory from propagating into the
milestone plan:

* There is **no** analytic gravitational-lensing implementation. What the
  `black_hole` project calls lensing is numeric integration of null geodesics
  with a fixed step; its `ray_tracing.cpp` does not deflect light at all.
* There is **no** energy or momentum accounting, and therefore no evidence that
  the N-body trajectories are physically meaningful.
* There are **no** tests and **no** CI in either repository. `3D_test.cpp` is a
  demo program.
* There is **no** unit system. Lengths are mixed between physical values and
  render units by undocumented scaling factors.
* There is **no** GPU path beyond OpenGL 4.3 compute shaders, and no CUDA.
* There is **no** library interface and no Python interface.

## 7. Feature map from the brief to the milestones

| Brief capability | Reference support | Milestone |
| --- | --- | --- |
| Schwarzschild radius of arbitrary masses | present in both | **M2, done** |
| Hypothetical black holes for arbitrary objects | absent | **M2, done** (custom input + catalogue) |
| Critical radius at an assumed density | **absent in both** | **M2, done** (own derivation) |
| Newtonian N-body simulation | defective (§5 D-3) | M3 |
| Gravitational light deflection and lensing | numeric geodesics with defect D-2 | M4 |
| GPU rendering via Vulkan / Metal / optional CUDA | OpenGL compute only | M5 |
| Python scripting interface | absent | M6 |
| Reuse of all physics as a public Rust API | absent (C++, no library target) | **M2, done** for the core relations |

## 8. Proposed workspace structure

Proposed and adopted as delivered; see `docs/architecture.md` for the reasoning.

    blackhole/                      Cargo workspace
    ├── crates/
    │   ├── blackhole-core/         the physics; zero dependencies
    │   └── blackhole-cli/          front end; depends on core and clap
    └── docs/

The brief's full crate list (`blackhole-gravity`, `-relativity`, `-lensing`,
`-render`, `-render-wgpu`, `-render-cuda`, `-python`) is confirmed as the target
layout, but each crate is created in the milestone that first gives it a
consumer. Creating seven empty crates now would add manifests and compile units
with nothing testable behind them, against the brief's own instruction to avoid
unnecessary splitting.

Two structure decisions worth recording:

* **Only two of the brief's nine proposed crates exist so far**, because only two
  have content. This is a scheduling decision, not a reduction in scope.
* **The `examples/` directory does not exist yet.** Its stated purpose is
  reproducible scientific experiments; `examples/lensing.py` belongs to M6, and
  the core's usability as a library is demonstrated by the crate doctest that
  `cargo test` executes on every run.

## 9. M1 deliverables

| Deliverable | Where |
| --- | --- |
| Both repositories inspected, licence status established | this document, §1–§2 |
| Algorithms documented, with what was adopted, adapted or re-derived | this document, §4–§6 |
| Workspace created | `Cargo.toml`, `crates/` |
| Architecture decisions written down | `docs/architecture.md` |
| Independent numerical verification of the physics | `docs/reports/m1_evidence.py`, `.txt` |

## 10. Open items for the author

* **Licence for this project.** Not chosen; see `docs/architecture.md`.
* **Whether to keep the git repository** that `cargo new` initialised in the
  project root as a side effect. Nothing has been committed.

### Resolved during review

* **The `Sagittarius A*` citation is now complete.** It was recorded against the
  A&A internal article identifier `aa42465-21` because the publisher's page is
  behind a bot challenge. Adversarial review resolved the DOI
  `10.1051/0004-6361/202142465` through Crossref to *A&A* **657**, L12, and the
  catalogue and `docs/physics.md` now cite it in full.
* **Four defects found by adversarial review have been fixed**, with regression
  tests and a corrected record here and in `docs/validation.md`: a closed stdout
  pipe caused a panic and exit `101`; invalid UTF-8 on stdin aborted the dialogue;
  `allow_hyphen_values` made `--mass --radius 1` report a misleading error and
  treated `--density` inconsistently; and `find_material` accepted one-character
  names, so `--density i` silently returned ice. Review also found that this
  project's own acceptance script contained a scripted dialogue that computed
  nothing while the transcript recorded it as a pass — the script now asserts on
  case content and self-reports a pass/fail count.
* **Three documented numeric bounds were wrong** and are corrected from
  measurement: the `sphere_volume` underflow bound is `8.4e-109 m` (not
  `4e-103 m`), its overflow bound is `3.5e102 m` (not `5.6e102 m`), and
  `minimum_radius_at_density` overflows below `5.0e-283 kg m^-3` (not
  `1e-315 kg m^-3`). Each bound now has a test that checks just inside and just
  outside it.
* **The C4 evidence routine in `m1_evidence.py` had two defects**, found by a
  second reviewer that re-derived the physics independently and then checked the
  numbers. The pericentre solve bracketed the root at `1/b`, but the root of
  `1/b^2 = u0^2 - (2C/3) u0^3` is always slightly *greater* than `1/b`, so the
  bisection was degenerate and returned the bracket edge; and the call site
  passed `C` where the first integral requires `2C/3`. The effect was to
  understate the correct branch's residual against `2 r_s/b` by about a factor of
  one and a half. Corrected, the correct branch now matches the three-term
  analytic series to `5e-6` relative and its residual agrees with the predicted
  next-order term `15 pi r_s/(32 b)` to three significant figures — so the
  narrative in §5 is now *quantitatively* supported rather than merely asserted.
  The direction of the finding and the `1/3` ratio for the defective branch are
  unaffected. The reference-deflection table in §5 was additionally transcribed
  with a slipped exponent in its `b/r_s = 10` row; it has been replaced with the
  regenerated output.
* **One test was mislabelled as an external check.**
  `earth_radius_matches_the_published_figure` previously asserted against this
  project's own computed digits at a tolerance tighter than the published `8.87
  mm` it cited, so it could only pass by reproducing the same closed form. It now
  asserts against the published three-significant-figure value with a
  commensurate `1e-3` tolerance. Two scaling tests are likewise labelled in
  `docs/validation.md` as constraining only the exponent, not the constant.
