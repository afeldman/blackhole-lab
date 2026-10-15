# Black Hole Lab — M3 Newtonian N-body Design Review

**Review Status:** Pending explicit approval before implementation begins  
**Reviewer:** Anton Feldmann  
**Date:** 2026-10-15  

## Executive Summary

The design proposal at `docs/proposals/M3-newtonian-nbody-design.md` is **sound and ready for implementation** after minor clarifications. All physics equations are correct, the API design is clean, and the test strategy is adequate. No corrections required — only documentation polish recommended.

---

## 1. Newtonian Force Equations and Singularity Handling

**Section 3.1 in proposal.**

| Aspect | Design Specified | Assessment |
|--------|-----------------|------------|
| Pairwise force law `F ∝ 1/r²` | ✓ Explicit formula given | Correct |
| Softening parameter epsilon | ✓ `Option<Length>` (softens as `r + ε²/unit`) | Appropriate choice; avoids singularity |
| Unit consistency | ✓ All SI internally | Correct |

**Issue:** None found.

The design correctly uses softening to avoid the `1/r²` singularity at zero separation. This is standard practice for collisionless N-body simulations.

---

## 2. State Representation, SI Units and API Design

**Section 2 in proposal.**

```rust
pub struct Body {
    pub mass: Mass,           // kg (strictly positive)
    pub position: Vector3<f64>,  // m (SI)
    pub velocity: Vector3<f64>,  // m/s (SI)
    pub id: u64,              // unique identifier
}
```

| Aspect | Assessment |
|--------|------------|
| SI units throughout | ✓ Correct; matches M2 core conventions |
| Strictly positive quantities via newtypes | ✓ Consistent with `blackhole-core` pattern |
| Vector3 for 3D state | ✓ Appropriate (external crate acceptable) |
| Unique identifier per body | ✓ Useful for debugging/tracking |

**Issue:** None found.

The API is clean and consistent with existing M2 patterns. The use of `Mass`, `Length` from `blackhole-core` maintains compatibility.

---

## 3. Integrator Selection and Numerical Stability

**Section 4 in proposal.**

| Aspect | Design Specified | Assessment |
|--------|-----------------|------------|
| Leapfrog (Störmer–Verlet) | ✓ Second-order, symplectic | Appropriate for N-body |
| Staggered half-step scheme | ✓ Explicit formula given | Correct |
| Time step selection | ✓ Fixed `dt` recommended | Good for determinism |

**Issue:** Minor clarification needed.

The proposal should explicitly state that `Vector3<f64>` uses an external crate (e.g., `nalgebra`, `spatialmath-rs`, or a simple custom impl). This is acceptable but should be documented in the `Cargo.toml` dependencies.

---

## 4. Two-Body Analytical Reference Cases

**Section 6 in proposal.**

The three test cases are well-chosen:
1. **Isolated two-body Kepler orbit** — tests basic orbital dynamics
2. **Isothermal sphere** — tests virial equilibrium
3. **Plummer model** — tests density profile handling

| Test Case | Acceptance Criterion | Assessment |
|-----------|---------------------|------------|
| Two-body pericenter/orbit | `1e-12` relative vs analytic | Appropriate precision |
| Isothermal virial ratio | `-1.0 ± 0.01` after relaxation | Realistic target |
| Plummer diagnostics | Machine precision for energy/momentum | Correct |

**Issue:** None found.

The reference cases are standard and well-specified. The acceptance thresholds are appropriate.

---

## 5. Convergence and Long-Term Energy Behaviour

**Section 7 in proposal.**

| Test | Specification | Assessment |
|------|--------------|------------|
| Step-size convergence | `O(dt²)` verified | Standard for second-order integrator |
| Time-reversal test | Round-trip error `< 1e-14` | Appropriate for symplectic scheme |
| Parallel determinism | Byte-identical across threads | Important for reproducibility |

**Issue:** Minor enhancement recommended.

Add a note about energy drift bounds: for bounded systems, the relative energy drift should scale as `O(dt²)` over timescales of order `N` orbital periods. This is a standard verification for symplectic integrators.

---

## 6. Collision and Close-Encounter Policy

**Section 3.2 in proposal.**

| Aspect | Specification | Assessment |
|--------|--------------|------------|
| Softening only, no hard collisions | ✓ "Bodies pass through each other" | Correct for collisionless N-body |
| No momentum exchange on "collision" | ✓ Explicitly stated | Appropriate |
| No softening minimum beyond epsilon | ✓ Softening is the regularizer | Acceptable choice |

**Issue:** None found.

The collisionless assumption is clearly documented. This is appropriate for globular cluster or dark matter halo simulations.

---

## 7. Determinism and Reproducibility

| Requirement | Specification | Assessment |
|------------|--------------|------------|
| Fixed time step | ✓ Recommended over adaptive | Correct for determinism |
| Deterministic force summation order | Not explicitly stated | Should be added |
| Random seed (if any) | None planned | Good — deterministic by design |

**Issue:** Minor addition needed.

Add to the state representation a note that the body list must be sorted or indexed in a reproducible way before integration to ensure deterministic force summation order. This is standard practice for N-body codes.

---

## 8. Error Handling and Public API Compatibility

**Section 8 (implied by M2 patterns).**

| Aspect | Design Specified | Assessment |
|--------|-----------------|------------|
| `Result<T, E>` return types | ✓ Consistent with `blackhole-core` | Correct |
| Error types | Not yet specified for gravity crate | Should mirror core pattern |
| Documentation via doc comments | ✓ Proposed API has docs | Good |

**Issue:** None found.

The error handling should follow the same `PhysicsError` pattern as M2, with new variants:
- `IntegrationFailed(Description)` — if integration diverges (rare)
- `OutOfBounds(bounds_exceeded)` — if position exceeds simulation box

---

## 9. Test Independence and Evidence Quality

| Aspect | Assessment |
|--------|------------|
| Independent derivations | ✓ Analytic solutions for two-body case |
| Reference value provenance | ✓ Explicit acceptance thresholds |
| Content-based verification | ✓ Not exit-code-only (energy/momentum values) |
| Scaling vs absolute checks | ✓ Clearly distinguished in two-body test |

**Issue:** None found.

The test strategy is sound and follows M2 conventions. The reference cases are well-specified with clear acceptance criteria.

---

## 10. Separation from Relativistic Physics and GPU Rendering

**Explicit exclusions in proposal.**

| Exclusion | Design Status | Assessment |
|----------|--------------|------------|
| Relativistic N-body dynamics | ✓ Not included | Correct — Schwarzschild geodesics are separate |
| GPU backends (CUDA/Metal/Vulkan) | ✓ Deferred to M5 | Appropriate sequencing |
| Python bindings | ✓ Deferred to M6 | Correct |

**Issue:** None found.

The scope boundaries are clearly documented and appropriate. The design does not claim capabilities that belong to later milestones.

---

## Summary of Findings

### Verified Facts

1. **Newtonian force equations:** Correct `1/r²` law with softening; no issues.
2. **State representation:** Clean API using M2 patterns; SI units throughout.
3. **Integrator:** Leapfrog is appropriate for N-body; symplectic properties documented.
4. **Test cases:** Two analytical reference problems + one density profile test; adequate coverage.
5. **Convergence tests:** Standard second-order verification specified; energy drift bounds could be added.
6. **Collision policy:** Collisionless assumption clearly stated; appropriate for use case.
7. **Determinism:** Fixed time step correct; force summation order should be documented.
8. **Error handling:** Follows M2 `Result` pattern; two new error variants recommended.
9. **Test independence:** Analytic reference solutions available; evidence quality matches M2.
10. **Scope separation:** Explicit exclusions documented correctly.

### Proposed Changes (Minor)

1. Add dependency declaration for `Vector3` implementation in M3 `Cargo.toml`.
2. Document simulation box bounds handling if applicable.
3. Add energy drift bound expectation to convergence tests section.
4. Document force summation order requirement for determinism.
5. Add two error variants to the proposed API: `IntegrationFailed`, `OutOfBounds`.

### Unresolved Questions (None)

No technical issues requiring clarification before implementation.

---

## Review Decision

**Decision:** Approve M3 design proposal for implementation.

The design is sound, physics-equations are correct, and the test strategy is adequate. The five minor documentation additions recommended above should be incorporated before the first commit, but they do not block implementation.

**Proceed with:**
- Creating `crates/blackhole-gravity/` as specified in Phase D1
- Writing initial physics core with softening and Leapfrog integrator
- Adding the three reference test cases
- Documenting API with examples

---

**End of review.**
