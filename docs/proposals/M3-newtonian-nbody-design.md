# M3 — Newtonian N-body Engine Design Proposal

**Status:** Design proposal pending review  
**Scope:** Independent Rust library API for Newtonian N-body gravity  
**Excluded:** Relativistic dynamics, GPU backends, Python bindings

---

## 1. Executive Summary

This milestone implements a pure-Rust N-body simulation engine as a standalone library (`blackhole-gravity`). It provides:

- State representation for N bodies with mass, position, velocity
- Pairwise Newtonian force evaluation (with softening)
- Numerical integration via Leapfrog/Verlet symplectic scheme
- Diagnostics: total energy, linear momentum, angular momentum
- Deterministic reference problems for testing

**Explicit exclusions:** relativistic N-body dynamics, GPU backends, Python bindings.

---

## 2. State Representation

```rust
pub struct Body {
    pub mass: Mass,           // kg (strictly positive)
    pub position: Vector3<f64>,  // m (SI)
    pub velocity: Vector3<f64>,  // m/s (SI)
    pub id: u64,              // unique identifier
}

pub struct System {
    pub bodies: Vec<Body>,
    pub softening: Option<Length>,  // epsilon for force softening, None for uns softened
    pub bounds: Bounds,              // optional simulation box
}
```

**Units:** SI base units throughout (`kg`, `m`, `s`). No unitless normalization.

---

## 3. Force Evaluation

### 3.1 Pairwise Force Law

For bodies i and j separated by vector r = r_i - r_j:

```rust
F_ij = G * m_i * m_j / |r|³ * (r + epsilon² * unit(r))
```

where `epsilon` is the softening parameter (optional). In uns softened mode, set `epsilon = 0`.

**Constants:** Newton's gravitational constant `G = 6.67430e-11 m³ kg⁻¹ s⁻²`.

### 3.2 Collision and Close-Encounter Policy

- **Softening only:** No hard collision handling. Bodies pass through each other.
- **Minimum separation:** No explicit guard beyond softening (which regularizes the force).
- **No momentum exchange:** Velocities are not modified on "collision".

This is a collisionless N-body engine, suitable for globular clusters or dark matter halos.

---

## 4. Numerical Integration

### 4.1 Integrator Choice: Symplectic Leapfrog (Störmer–Verlet)

The Leapfrog scheme advances position and velocity in staggered half-steps:

```
v_{n+1/2} = v_n + (dt/2) * F(r_n)
r_{n+1}   = r_n + dt * v_{n+1/2}
v_{n+1}   = v_{n+1/2} + (dt/2) * F(r_{n+1})
```

**Properties:**
- Second-order accuracy in time step `dt`
- Symplectic (phase-space volume conserving)
- Time-reversible
- Good long-term energy behavior for bounded orbits

### 4.2 Time Step Selection

Fixed time step recommended for stability and reproducibility:

```rust
pub fn integrate(&self, dt: Duration, steps: u64) -> System;
```

**Guideline:** `dt` should be small enough that the shortest orbital period in the system satisfies `P_min >> dt`.

---

## 5. Diagnostics

### 5.1 Total Energy

```
E = Σ_i (1/2 * m_i * v_i²) + Σ_{i<j} G * m_i * m_j / |r_ij|
```

with softening as above.

### 5.2 Linear Momentum

```
P = Σ_i m_i * v_i
```

Should be conserved (up to numerical precision).

### 5.3 Angular Momentum

```
L = Σ_i m_i * (r_i × v_i)
```

Conserved in the absence of external torques.

### 5.4 Output Format

Diagnostics are reported as floats with consistent formatting for determinism:

```rust
pub struct Diagnostics {
    pub total_energy: Energy,
    pub linear_momentum: Vector3<Momentum>,
    pub angular_momentum: Vector3<AngularMomentum>,
    pub virial_ratio: Option<f64>,  // optional, if center-of-mass is subtracted
}
```

---

## 6. Deterministic Reference Problems

### 6.1 Isolated Two-Body Kepler Orbit

**Parameters:**
- Body A: `m_A = 10 M_sun`, `r_0 = 20 au`, `v_thro = sqrt(G * m_B / r_0)` for parabolic approach, or given `e` for elliptical.
- Body B: `m_B = 1 M_sun`

**Acceptance:** The pericenter distance and orbital period computed numerically must match the analytic Kepler formulas to within `1e-12` relative.

### 6.2 Isothermal Sphere Equilibrium

**Parameters:**
- Density profile: `rho(r) = rho_0 / sqrt(1 + (r/r_c)²)` with `rho_0 = 0.1 M_sun/pc³`, `r_c = 0.5 pc`.
- Velocity dispersion: derived from hydrostatic equilibrium.

**Acceptance:** The virial ratio `2T/|U|` should be within `-1.0 ± 0.01` after the system relaxes for `10^4` steps.

### 6.3 Plummer Model

**Parameters:**
- Mass: `M = 1 M_sun`, scale radius: `a = 1 pc`.
- Density: `rho(r) = (3M/(4π a³)) / (1 + r²/a²)^{5/2}`.

**Acceptance:** Total energy, linear and angular momentum are computed to machine precision; orbital elements of tracer particles match analytic predictions.

---

## 7. Numerical Convergence Tests

For each reference problem:

1. **Step-size convergence:** Run with `dt`, `dt/2`, `dt/4`; verify that the total energy drift scales as `O(dt²)`.
2. **Time-reversal test:** Advance for N steps, reverse velocities, advance N more; final state should return to initial within round-trip error.
3. **Parallel determinism:** Run with two threads using deterministic reduction; results must be bitwise identical.

---

## 8. Public API and Physics Documentation

### 8.1 Library Interface

```rust
pub mod blackhole_gravity {
    pub struct Body;
    pub struct System;
    pub fn new_system(bodies: Vec<Body>, softening: Option<f64>) -> System;
    pub fn integrate(&self, dt: Duration, steps: u64) -> System;
    pub fn diagnostics(&self) -> Diagnostics;
}
```

### 8.2 Physics Documentation

`docs/src/blackhole_gravity/physics.md` will contain:

- Derivation of the pairwise force law (with softening derivation)
- Leapfrog stability analysis (region in `(dt * omega)` plane)
- Conservation properties and expected drift bounds
- Reference values for each test case

---

## 9. Implementation Plan

### Phase D1 — Core Library

1. Implement `Body`, `System`, vector math primitives
2. Implement pairwise force loop with softening option
3. Implement Leapfrog integrator

**Deliverable:** `crates/blackhole-gravity/Cargo.toml` and `src/lib.rs` compile without warnings.

### Phase D2 — Diagnostics and Tests

1. Implement energy/momentum/angular momentum diagnostics
2. Implement the three reference problems
3. Add convergence tests

**Deliverable:** `cargo test -p blackhole-gravity` passes all 20+ tests.

### Phase D3 — Documentation

1. Write physics documentation (`docs/src/blackhole_gravity/physics.md`)
2. Document API with examples

**Deliverable:** README and docs complete, no doc lints.

---

## 10. Acceptance Criteria

| Criterion | Threshold |
|-----------|-----------|
| Two-body orbital period vs analytic | `1e-12` relative |
| Virial ratio (isothermal sphere) | `-1.0 ± 0.01` after relaxation |
| Energy drift (step-size scaling) | `O(dt²)` verified via convergence test |
| Time-reversal round trip error | `< 1e-14` for small `dt` |
| Parallel determinism | Byte-identical output across threads |
| Clippy warnings (`pedantic`) | Zero |
| Doc tests | All pass |

---

## 11. Open Questions

1. **Softening parameter:** Should it be a fixed fraction of typical separation, or user-specified? → **Recommendation:** user-specified `Option<f64>`, default `None` (uns softened).
2. **Thread safety:** Single-threaded by default; add `Arc<S>` + `Mutex` for parallel integration if needed in M5.
3. **Output format:** JSON, CSV, or plain-text diagnostics? → **Recommendation:** plain-text with fixed-precision floats for determinism; binary format deferred to M6.

---

## 12. Dependencies on Other Milestones

- **M2 physics core:** Provides `Mass`, `Length`, `Density` newtypes and constants (`G`, `c`).
- **No dependency on:** `-relativity`, `-lensing`, `-render*`, `-python`.

---

**Review requested:** Pending approval from the author before implementation begins.
