# Scientific Evidence Review Protocol

**Status:** Repository-documented procedure (skill creation blocked by 60-char description limit)  
**Last reviewed:** 2026-10-15  
**Owner:** Black Hole Lab contributors, Licensed Apache-2.0  

## Purpose

This document defines the review protocol for scientific claims in this project. It is intended for use as a skill (`scientific-evidence-review`) but is also documented here because the skill creation mechanism cannot accommodate a description longer than 60 characters. The full procedure should be consulted whenever reviewing:

- Independent derivations for central scientific claims
- Numerical integration coefficients (e.g., geodesic integrals)
- Bisection brackets and root-finding procedures
- Initial conditions and boundary conditions
- External reference values with justified tolerances
- Scaling tests vs absolute-constant validation
- Reproducible evidence artifacts
- Traceable records of original claims, defects, corrections, verification

## Core Principles

### 1. Independent Verification Required

Every central scientific claim must be independently derived or recomputed before accepting its correctness. A test that re-implements the same formula only proves consistency of implementation, not physical correctness.

Use controls that are independent of the implementation:
- Published reference values with their stated precision
- Independent algebraic rearrangements
- Fixed-point identities that must hold for any correct implementation
- Independently written scripts in a different language
- Numerical integration against analytical solutions

### 2. Scaling vs Absolute Constants

Distinguish two classes of tests:

| Type | Constrains | Tolerance rationale |
|------|------------|--------------------|
| Scaling test | Exponent only (e.g., `R_krit ∝ rho^(-1/2)`) | Weak; holds for any constant. Not an external check. |
| Absolute check | Full value against reference or identity | Strong; pins the actual constant. External check if against published value. |

Scaling tests constrain the functional form but not the prefactor. Never count them as external validation of a specific physical result.

### 3. Reference Values Must Include Stated Precision

A published value "8.87 mm" has three significant figures. The tolerance must be commensurate with this precision, not tighter:
- Use relative tolerance ~1e-3 for three-significant-figure data
- Tighter tolerances only pass if reproducing the same closed form — which tests nothing.

### 4. Content-Based Acceptance

An acceptance script must check the *content* of each case, not only its exit status. A case that exits `0` without printing expected content counts as a failure.

Example pattern:
```bash
echo "exit status: $status   expected: results   verdict: $verdict"
if [ "$verdict" = "PASS" ]; then
  passed=$((passed + 1))
else
  failed=$((failed + 1))
  echo "!! FAILED CASE: $description"
fi
```

### 5. Traceable Record of Corrections

When a defect is found:
1. Record the original claim as stated
2. Document what was wrong (bug, mislabeling, incorrect tolerance)
3. Show the correction
4. Verify the corrected version independently
5. Update documentation to reflect the corrected narrative

**Never amend history.** Create a new commit with the fix and document it separately. This protocol was violated in the M1/M2 review when C4 evidence code was corrected without a separate commit — see `docs/reports/m1_evidence.py` revision history for example.

### 6. Bisection Bracket Validation

When using bisection root finding:
- The bracket `[a, b]` must satisfy `f(a) * f(b) < 0`. A degenerate bracket returns the edge and gives no numerical information.
- Verify that the computed root lies within the expected physical range.

### 7. Numerical Integration Coefficients

For geodesic integrals or similar physics:
- Verify differential equation coefficients against the governing equation
- Check that missing factors (e.g., `f = 1 - r_s/r` in Schwarzschild geodesics) are included
- Compare against known analytical solutions or series expansions
- Document convergence behavior with step size

### 8. External Reference Must Be Primary, Not Secondary Summary

Prefer peer-reviewed results, mission documentation, and primary data sources. Do not cite a secondary summary when the primary result is available.

## Procedure

### Step 1: Identify the Claim to Review

What physical quantity or relationship is being asserted?
- Write down the claimed formula or numerical value
- Note where it comes from (code line, documentation section, test case)

### Step 2: Independent Derivation

Derive or compute the same quantity independently:
- Use different algebraic rearrangements if possible
- Or use a different computational method (Python vs Rust, bisection vs closed form)
- Document each step of your derivation

### Step 3: Check Boundary/Initial Conditions

Verify that initial/boundary conditions are physically correct and consistent with the model:
- For geodesic integrals: check energy-angular momentum conservation, null condition satisfaction
- For N-body: check force law at large/small separations, softening, collision handling
- For any root finding: check bracket validity `f(a)*f(b) < 0`

### Step 4: Validate Against Reference Values

If available:
- Get the reference value from a primary source
- Note its stated precision
- Set tolerance commensurate with that precision

### Step 5: Distinguish Scaling vs Absolute

Ask: does this test constrain only the exponent, or also the prefactor?
- If scaling-only (e.g., doubling radius doubles volume as `8`), label it as "scaling test" and do not count as external validation
- If it pins the absolute value against a reference, it is an external check

### Step 6: Reproduce Evidence Artifacts

Generate new evidence artifacts:
```bash
cargo build --release -p blackhole-core
python3 docs/reports/m1_evidence.py > docs/reports/m1_evidence.txt.new
cat docs/reports/m1_evidence.txt.new >> docs/reports/m2_evidence_run1.txt
```

### Step 7: Document Findings

Create a brief report:
- Original claim (with citation)
- What was checked
- Method of independent verification
- Result: PASS/FAIL with quantitative deviation if applicable
- Any defects discovered
- Corrections made
- Updated tolerance or reference if needed

## Known Pitfalls from M1/M2 Review

| Defect | Category | Prevention |
|--------|----------|------------|
| Incorrect bisection bracket returning edge instead of root | Root-finding | Check `f(a)*f(b)<0`; verify against independent method |
| Missing factor `f=1-r_s/r` in radial geodesic equation | Physics derivation | Derive from governing equation; compare to series expansion |
| Tolerance tighter than published precision (e.g., 8.87 mm cited with 1e-15 tolerance) | Reference handling | Match tolerance to stated significant figures |
| Scaling test mislabeled as external check | Classification | Explicitly label scaling tests |
| Defect found but history rewritten (amend/rebase) | Git hygiene | New commit with fix, never amend/rebase |

## Related Skills

- `brainstorming`: before any creative or investigative work
- `test-driven-development`: when implementing fixes that change behavior
- `using-superpowers`: always invoke relevant skills first
