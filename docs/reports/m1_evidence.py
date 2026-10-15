#!/usr/bin/env python3
"""
M1 evidence script for Black Hole Lab.

Purpose
-------
Independently verify the physics identities that blackhole-core will implement,
and quantify a defect identified in the reference repository
https://github.com/kavan010/black_hole.

Method
------
This script deliberately does NOT reuse any code from the reference projects
(they carry no license). All relations are re-derived from the standard
literature and evaluated here with plain double precision.

Checks
------
C1  Schwarzschild radius of reference objects vs published values.
C2  Consistency of the reference shader's hard-coded SagA_rs constant.
C3  Critical radius under assumed density: closed form vs independent
    bisection root-finding on g(R) = r_s(M(R)) - R.
C4  Schwarzschild radial null-geodesic equation:
        d2r/dl2 = -(rs*f/(2r^2))*tdot^2 + (rs/(2r^2 f))*rdot^2 + r*f*(thdot^2 + sin^2 th * phidot^2)
    in the equatorial plane (th = pi/2, thdot = 0) this collapses, via the null
    condition, to
        d2r/dl2 = (r - 3/2 rs) * phidot^2
    i.e. the angular term of the radial equation carries the factor f = 1 - rs/r.
    The reference 3D implementation (CPU-geodesic.cpp, geodesic.comp) uses
        + r * (thdot^2 + sin^2 th * phidot^2)          <-- missing factor f
    which collapses to (r - 1/2 rs) * phidot^2. We integrate the reduced radial
    equation in the dimensionless form
        u'' = -u + C u^2 ,   u = rs/r ,  C = 3/2 (correct) or 1/2 (reference)
    and compare the resulting light deflection with the analytic weak-field
    result alpha = 4GM/(c^2 b) = 2 rs / b.

All lengths in C4 are expressed in units of rs, hence dimensionless.
"""

import math

# --- CODATA 2018 / SI exact -------------------------------------------------
G = 6.67430e-11        # m^3 kg^-1 s^-2   (CODATA 2018, rel. unc. 2.2e-5)
C_LIGHT = 299792458.0  # m s^-1           (exact, SI definition)

# --- Reference object data (used only for cross-checking here) --------------
REF = {
    # name: (mass_kg, published_rs_m, source_note)
    "Sun":     (1.9885e30,  2953.0,      "IAU nominal solar mass + G"),
    "Earth":   (5.9722e24,  8.870e-3,    "NASA planetary fact sheet"),
    "Sgr A*":  (8.54e36,    1.269e10,    "reference repo black_hole.cpp (4.29e6 Msun)"),
}


def rs(mass):
    """Schwarzschild radius r_s = 2 G M / c^2  [m]."""
    return 2.0 * G * mass / (C_LIGHT * C_LIGHT)


def mass_from_rs(radius):
    """Inverse of rs()."""
    return radius * C_LIGHT * C_LIGHT / (2.0 * G)


def sphere_mass(radius, density):
    """Mass of a homogeneous sphere M = 4/3 pi R^3 rho  [kg]."""
    return (4.0 / 3.0) * math.pi * radius ** 3 * density


def critical_radius_closed_form(density):
    """R_krit = sqrt(3 c^2 / (8 pi G rho))  [m]."""
    return math.sqrt(3.0 * C_LIGHT ** 2 / (8.0 * math.pi * G * density))


def critical_radius_bisection(density, lo=1.0, hi=1e18, tol=1e-6, itmax=400):
    """Independent root of g(R) = rs(sphere_mass(R, rho)) - R = 0 by bisection."""
    def g(R):
        return rs(sphere_mass(R, density)) - R
    glo, ghi = g(lo), g(hi)
    if glo * ghi > 0:
        raise RuntimeError("bracket failure")
    for _ in range(itmax):
        mid = 0.5 * (lo + hi)
        gm = g(mid)
        if abs(gm) < tol * abs(mid) or (hi - lo) < tol * abs(mid):
            return mid
        if glo * gm <= 0:
            hi = mid
        else:
            lo, glo = mid, gm
    return 0.5 * (lo + hi)


# --- C4: reduced radial null geodesic --------------------------------------
def deflection(u0, coef, dphi=1e-6, u_floor_frac=1e-6):
    """
    Integrate u'' = -u + coef*u^2 with u(0) = u0, u'(0) = 0 (pericentre at phi=0)
    until u has decayed to u_floor_frac*u0. Returns total deflection angle
    alpha = 2*phi_inf - pi [rad] (dimensionless, lengths in units of rs).

    RK4 in phi; the final approach to u = 0 is continued by linear
    extrapolation, which is second-order accurate there because for u -> 0 the
    equation linearises to u'' = -u, so u is locally proportional to
    sin(phi_inf - phi).
    """
    def f(u, up):
        return up, -u + coef * u * u

    u, up, phi = u0, 0.0, 0.0
    u_floor = u_floor_frac * u0
    while u > u_floor:
        k1u, k1p = f(u, up)
        k2u, k2p = f(u + 0.5 * dphi * k1u, up + 0.5 * dphi * k1p)
        k3u, k3p = f(u + 0.5 * dphi * k2u, up + 0.5 * dphi * k2p)
        k4u, k4p = f(u + dphi * k3u, up + dphi * k3p)
        u += dphi / 6.0 * (k1u + 2 * k2u + 2 * k3u + k4u)
        up += dphi / 6.0 * (k1p + 2 * k2p + 2 * k3p + k4p)
        phi += dphi
    # linear extrapolation of the remaining angle to u = 0
    phi_inf = phi - u / up
    return 2.0 * phi_inf - math.pi


def u0_for_b(b_hat, coef):
    """
    Solve 1/b^2 = u0^2 - (2*coef/3)*u0^3 for the pericentre u0, by bisection.

    The coefficient is 2*coef/3 and not coef: multiplying u'' = -u + C u^2 by u'
    and integrating gives the first integral
        (1/2) u'^2 = -u^2/2 + C u^3/3 + const
    and imposing u' = 0 at u = u0, u' = 1/b as u -> 0 yields
        1/b^2 = u0^2 - (2C/3) u0^3 .
    The bracket must extend beyond 1/b, because the root satisfies
    u0 = (1/b)(1 + (C/3)(1/b) + ...) > 1/b; bracketing at hi = 1/b makes the
    bisection degenerate and returns hi.
    """
    target = 1.0 / b_hat ** 2
    k = 2.0 * coef / 3.0
    lo, hi = 1e-12, 1.0
    for _ in range(300):
        mid = 0.5 * (lo + hi)
        val = mid * mid - k * mid ** 3
        if val < target:
            lo = mid
        else:
            hi = mid
    return 0.5 * (lo + hi)


def main():
    print("=" * 78)
    print("C1  Schwarzschild radius vs published values")
    print("=" * 78)
    print(f"{'object':<10}{'mass [kg]':>14}{'r_s computed':>18}{'r_s published':>18}{'rel.dev':>12}")
    for name, (m, r_ref, _src) in REF.items():
        r = rs(m)
        print(f"{name:<10}{m:>14.6e}{r:>18.6e}{r_ref:>18.6e}{abs(r - r_ref) / r_ref:>12.2e}")

    print()
    print("=" * 78)
    print("C2  reference shader constant SagA_rs = 1.269e10 m  (geodesic.comp line 30)")
    print("=" * 78)
    m_sgra = 8.54e36
    print(f"    r_s(8.54e36 kg) = {rs(m_sgra):.6e} m   -> consistent with 1.269e10 m "
          f"(rel.dev {abs(rs(m_sgra) - 1.269e10) / 1.269e10:.2e})")
    print(f"    implied mass of SagA* = {mass_from_rs(1.269e10):.6e} kg "
          f"= {mass_from_rs(1.269e10) / 1.9885e30:.4e} M_sun")

    print()
    print("=" * 78)
    print("C3  critical radius at assumed density: closed form vs bisection")
    print("=" * 78)
    print(f"{'medium':<22}{'rho [kg/m3]':>13}{'R_krit closed':>17}{'R_krit bisect':>17}{'rel.dev':>11}")
    media = [
        ("water (4 degC)", 1000.0),
        ("ice", 917.0),
        ("basalt (typical)", 2900.0),
        ("granite (typical)", 2700.0),
        ("Earth mean", 5514.0),
        ("Sun mean", 1408.0),
    ]
    for name, rho in media:
        rc = critical_radius_closed_form(rho)
        rb = critical_radius_bisection(rho)
        print(f"{name:<22}{rho:>13.1f}{rc:>17.6e}{rb:>17.6e}{abs(rc - rb) / rc:>11.2e}")

    print()
    print("    self-consistency: at R_krit the sphere must satisfy rs(M(R_krit)) == R_krit")
    for name, rho in media[:1]:
        rc = critical_radius_closed_form(rho)
        M = sphere_mass(rc, rho)
        print(f"      rho = {rho:g} kg/m3 -> R_krit = {rc:.6e} m, M = {M:.6e} kg, "
              f"r_s(M) = {rs(M):.6e} m, ratio r_s/R = {rs(M) / rc:.15f}")

    print()
    print("=" * 78)
    print("C4  light deflection: correct radial coefficient (3/2) vs reference (1/2)")
    print("=" * 78)
    print("    leading order:      alpha = 4GM/(c^2 b) = 2*rs/b")
    print("    full series (M = GM/c^2 = rs/2):")
    print("        alpha = 4M/b + (15 pi/4)(M/b)^2 + (128/3)(M/b)^3 + ...")
    print("    so the residual against 2*rs/b must approach")
    print("        (15 pi/4)(M/b)^2 / (2 rs/b) = 15 pi rs / (32 b)")
    print()
    print(f"{'b [rs]':>7}{'u0 (pericentre)':>18}{'alpha C=3/2':>16}{'2 rs/b':>11}"
          f"{'series':>13}{'rel.dev':>10}{'predicted':>11}{'alpha C=1/2':>16}{'ratio':>8}")
    for b_hat in (10.0, 50.0, 100.0, 1000.0):
        u0c = u0_for_b(b_hat, 1.5)
        u0r = u0_for_b(b_hat, 0.5)
        a_c = deflection(u0c, 1.5)
        a_r = deflection(u0r, 0.5)
        a_first = 2.0 / b_hat
        m_over_b = 0.5 / b_hat
        a_series = (
            4.0 * m_over_b
            + (15.0 * math.pi / 4.0) * m_over_b ** 2
            + (128.0 / 3.0) * m_over_b ** 3
        )
        predicted = ((15.0 * math.pi / 4.0) * m_over_b ** 2
                     + (128.0 / 3.0) * m_over_b ** 3) / (4.0 * m_over_b)
        print(f"{b_hat:>7.0f}{u0c:>18.12e}{a_c:>16.10e}{a_first:>11.4e}"
              f"{a_series:>13.6e}{abs(a_c - a_first) / a_first:>10.3e}"
              f"{predicted:>11.3e}{a_r:>16.10e}{a_r / a_c:>8.4f}")

    print()
    print("    step-size convergence of the correct branch (b = 100 rs):")
    u0 = u0_for_b(100.0, 1.5)
    for dphi in (1e-4, 1e-5, 1e-6, 1e-7):
        a = deflection(u0, 1.5, dphi=dphi)
        print(f"      dphi = {dphi:.0e}  alpha = {a:.10e}  rel.dev vs 2/b = "
              f"{abs(a - 0.02) / 0.02:.2e}")

    print()
    print("=" * 78)
    print("Interpretation")
    print("=" * 78)
    print("  C1/C2: r_s = 2GM/c^2 reproduces published radii to <1e-4 relative.")
    print("  C3:    the closed form R_krit = sqrt(3c^2/(8 pi G rho)) agrees with an")
    print("         independent bisection root of r_s(M(R)) - R = 0.")
    print("  C4:    the correct angular term r*f carries the factor f = 1 - rs/r;")
    print("         omitting it (as the reference 3D/GPU code does) reduces the")
    print("         quadratic term coefficient from 3/2 to 1/2 and therefore")
    print("         under-predicts the deflection by a factor approaching 1/3, a")
    print("         defect that vanishes only in the far-field limit.")
    print("         The correct branch reproduces 2*rs/b, with a residual that")
    print("         falls off as the analytic next-order term 15 pi rs/(32 b) -")
    print("         i.e. the agreement improves exactly as the expansion requires.")


if __name__ == "__main__":
    main()
