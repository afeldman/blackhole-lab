#!/usr/bin/env python3
"""Schwarzschild black-hole visualization renderer.

This is a *standalone conceptual visualization tool* for the Black Hole Lab
project. It is NOT the project's rendering engine: the M4/M5 lensing and
rendering crates do not exist yet. It is deliberately written in Python with
only numpy and Pillow, so that it adds no GPU dependency and no Rust build
surface to the repository.

What it actually computes
-------------------------
For every pixel, a null geodesic of the Schwarzschild metric is integrated
numerically in the orbital plane of that ray (classical RK4, exact ODE):

    d^2u/dphi^2 = -u + (3/2) r_s u^2 ,      u = 1/r

with r_s = 1 by choice of units. This is the exact Schwarzschild light-bending
equation, not a weak-field approximation. The apparent shadow edge therefore
has to come out at b = 3*sqrt(3)*M = 2.5980762 r_s, which verify_raytrace.py
measures in the produced image as an independent check.

What it does NOT compute
------------------------
* No Kerr (spinning) black hole, no frame dragging. Schwarzschild only.
* No general-relativistic radiative transfer: no Compton scattering, no
  polarized transfer, no returning radiation, no disk self-illumination.
* The accretion disk is geometrically thin, optically thick and Lambertian.
* Doppler beaming and gravitational redshift use the standard circular-orbit
  kinematic approximation in the equatorial plane.
* The emitted spectrum is rescaled into the visible band. A real accretion disk
  at these temperatures radiates predominantly in the UV/X-ray; the *relative*
  spectral shift with radius and with orbital velocity is physical, the absolute
  band is an illustrative choice.
* The star field is procedural, not a real sky survey.

These limitations are restated in docs/visualization.md.
"""

from __future__ import annotations

import argparse
import math
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

# ---------------------------------------------------------------------------
# Physical / numerical constants (units: r_s = 1, c = 1)
# ---------------------------------------------------------------------------

RS = 1.0  # Schwarzschild radius; the length unit of the whole render
M_GEO = RS / 2.0  # geometric mass M = GM/c^2, so that r_s = 2M
R_PHOTON_SPHERE = 3.0 * M_GEO  # 1.5 r_s
B_CRIT = 3.0 * math.sqrt(3.0) * M_GEO  # 3*sqrt(3)*M = 2.598 r_s (shadow edge)
B_CRIT_OVER_RS = B_CRIT / RS  # 2.598076211353316

H_FAR = 0.02  # d(phi) step for r >= R_NEAR_REFINE
H_NEAR = 0.0025  # d(phi) step for 3 <= r < R_NEAR_REFINE
H_CLOSE = 0.0008  # d(phi) step for r < 3 r_s
R_NEAR_REFINE = 8.0
R_ESCAPE_MIN = 2000.0  # integrate this far out before sampling the sky
R_ESCAPE_FACTOR = 64.0  # ... or this multiple of r_obs, whichever is larger
MAX_PHI_FACTOR = 6.0  # swept-angle cap, in units of 2*pi
MAX_STEPS = 400000
STAR_PSF_REFERENCE_WIDTH = 1920  # pixels the PSF sigma is quoted for


@dataclass(frozen=True)
class SceneConfig:
    """Camera, disk geometry and integrator control. Lengths are in units of r_s."""

    r_obs: float = 60.0
    inclination_deg: float = 82.0
    hfov_deg: float = 28.0
    r_in: float = 3.0  # inner disk edge = ISCO = 6M = 3 r_s
    r_out: float = 14.0
    disk_t_max_k: float = 6000.0  # illustrative peak colour temperature
    disk_peak_flux: float = 0.55
    # Display compression. The physical scalings are I ~ T^4 and T ~ (r_in/r)^(3/4)
    # (with the zero-torque inner cut-off); over r = 3..14 r_s that is a dynamic
    # range of ~10^4 in flux and ~2500..6000 K in colour. Rendering those directly
    # through a display transform clips the disk to white and throws the palette
    # away, so the flux exponent and the colour exponent below compress the range
    # for display only. They change no conserved quantity and no lensing result.
    disk_flux_exponent: float = 0.75
    disk_colour_exponent: float = 0.55
    star_density_per_sr: float = 2.0e5
    star_grid: int = 4000
    star_psf_px: float = 0.9
    star_flux_scale: float = 1.6
    nebula_amplitude: float = 0.016
    exposure: float = 1.0
    # Integrator control. Exposed so the verification script can run a
    # step-size and winding-cap convergence study instead of asserting a
    # tolerance that happens to hold for one configuration.
    h_far: float = H_FAR
    h_near: float = H_NEAR
    h_close: float = H_CLOSE
    max_phi_factor: float = MAX_PHI_FACTOR

    def escape_radius(self) -> float:
        """Radius at which a ray is taken to have reached its asymptotic direction."""
        return max(R_ESCAPE_MIN, R_ESCAPE_FACTOR * self.r_obs)


# ---------------------------------------------------------------------------
# Blackbody colour: Planck spectrum integrated against CIE 1931 observers
# ---------------------------------------------------------------------------

_WAVELENGTH_NM = np.arange(380.0, 781.0, 1.0)


def _cie_xyz_bar(wl_nm: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """CIE 1931 2-deg colour matching functions (Wyman/Sloan/Shirley 2013 fits)."""

    def gauss(mu: float, s1: float, s2: float) -> np.ndarray:
        sigma = np.where(wl_nm < mu, s1, s2)
        return np.exp(-0.5 * ((wl_nm - mu) / sigma) ** 2)

    x_bar = (
        1.056 * gauss(599.8, 37.9, 31.0)
        + 0.362 * gauss(442.0, 16.0, 26.7)
        - 0.065 * gauss(501.1, 20.4, 26.2)
    )
    y_bar = 0.821 * gauss(568.8, 46.9, 40.5) + 0.286 * gauss(530.9, 16.3, 31.1)
    z_bar = 1.217 * gauss(437.0, 11.8, 36.0) + 0.681 * gauss(459.0, 26.0, 13.8)
    return x_bar, y_bar, z_bar


def _planck_spectral_radiance(wl_m: np.ndarray, temperature_k: float) -> np.ndarray:
    """Planck spectral radiance, arbitrary scale."""
    h = 6.626_070_15e-34
    c = 2.997_924_58e8
    k_b = 1.380_649e-23
    exponent = np.clip(h * c / (wl_m * k_b * temperature_k), 1e-12, 700.0)
    return (2.0 * h * c**2 / wl_m**5) / np.expm1(exponent)


_XYZ_TO_RGB = np.array(
    [
        [3.2406, -1.5372, -0.4986],
        [-0.9689, 1.8758, 0.0415],
        [0.0557, -0.2040, 1.0570],
    ]
)


def _blackbody_lut() -> tuple[np.ndarray, np.ndarray]:
    """Precompute linear-sRGB chromaticity of a blackbody over a temperature grid."""
    temperatures = np.logspace(math.log10(600.0), math.log10(40000.0), 96)
    x_bar, y_bar, z_bar = _cie_xyz_bar(_WAVELENGTH_NM)
    wl_m = _WAVELENGTH_NM * 1e-9
    chromaticities = np.empty((temperatures.size, 3))
    for index, temperature in enumerate(temperatures):
        spectrum = _planck_spectral_radiance(wl_m, float(temperature))
        xyz = np.array(
            [
                float(np.sum(spectrum * x_bar)),
                float(np.sum(spectrum * y_bar)),
                float(np.sum(spectrum * z_bar)),
            ]
        )
        rgb = _XYZ_TO_RGB @ (xyz / xyz[1])  # normalise to Y = 1
        peak = float(rgb.max())
        chromaticities[index] = rgb / peak if peak > 0.0 else rgb
    return temperatures, chromaticities


_LUT_TEMPERATURES, _LUT_CHROMATICITY = _blackbody_lut()


def blackbody_rgb(temperature_k: np.ndarray) -> np.ndarray:
    """Linear-sRGB chromaticity (peak channel = 1) for a blackbody temperature."""
    t = np.clip(temperature_k, float(_LUT_TEMPERATURES[0]), float(_LUT_TEMPERATURES[-1]))
    grid = np.log10(_LUT_TEMPERATURES)
    position = np.interp(np.log10(t), grid, np.arange(grid.size))
    lower = np.floor(position).astype(np.intp)
    upper = np.minimum(lower + 1, grid.size - 1)
    frac = (position - lower)[..., None]
    rgb = _LUT_CHROMATICITY[lower] * (1.0 - frac) + _LUT_CHROMATICITY[upper] * frac
    return np.clip(rgb, 0.0, None)


# ---------------------------------------------------------------------------
# Procedural background sky: stars on a hash grid + smooth nebula
# ---------------------------------------------------------------------------

def _hash01(ix: np.ndarray, iy: np.ndarray, iz: np.ndarray, salt: int) -> np.ndarray:
    """Deterministic 64-bit integer hash mapped to [0, 1)."""
    u64 = np.uint64
    h = u64(0x9E3779B97F4A7C15) * (ix.astype(u64) + u64(salt))
    h ^= u64(0xBF58476D1CE4E5B9) * (iy.astype(u64) + u64(3 * salt + 1))
    h ^= u64(0x94D049BB133111EB) * (iz.astype(u64) + u64(7 * salt + 2))
    h ^= h >> u64(30)
    h *= u64(0xBF58476D1CE4E5B9)
    h ^= h >> u64(27)
    h *= u64(0x94D049BB133111EB)
    h ^= h >> u64(31)
    return (h >> u64(11)).astype(np.float64) * (1.0 / 9007199254740992.0)


class Sky:
    """Procedural star field and nebula, sampled by unit direction."""

    def __init__(self, config: SceneConfig, seed: int = 20261015) -> None:
        self.config = config
        rng = np.random.default_rng(seed)
        self._nebula_k = rng.normal(size=(48, 3))
        self._nebula_k /= np.linalg.norm(self._nebula_k, axis=1, keepdims=True)
        self._nebula_k *= rng.uniform(1.0, 9.0, size=(48, 1))
        self._nebula_phase = rng.uniform(0.0, 2.0 * math.pi, size=48)
        self._nebula_weight = rng.normal(size=48) * np.exp(-np.arange(48) / 14.0)
        band_normal = rng.normal(size=3)
        self._band_normal = band_normal / np.linalg.norm(band_normal)
        self._band_width = 0.55
        cells_per_sr = 0.25 * config.star_grid**2
        self._star_exist_p = float(
            np.clip(config.star_density_per_sr / cells_per_sr, 0.0, 1.0)
        )
        self.grid = config.star_grid
        # Fixed angular PSF: quoted per output pixel of the reference width.
        self.sigma_ang = (
            config.star_psf_px * math.radians(config.hfov_deg) / STAR_PSF_REFERENCE_WIDTH
        )

    def sample(self, direction: np.ndarray) -> np.ndarray:
        direction = direction / np.maximum(
            np.linalg.norm(direction, axis=-1, keepdims=True), 1e-300
        )
        return self._nebula(direction) + self._stars(direction)

    def _nebula(self, direction: np.ndarray) -> np.ndarray:
        phase = direction @ self._nebula_k.T
        field = np.tanh((np.cos(phase + self._nebula_phase) @ self._nebula_weight) * 0.55)
        band = np.exp(-0.5 * ((direction @ self._band_normal) / self._band_width) ** 2)
        luminance = self.config.nebula_amplitude * (0.35 + 0.65 * (0.5 + 0.5 * field)) * (
            0.30 + 0.70 * band
        )
        tint = np.empty(direction.shape, dtype=np.float64)
        tint[..., 0] = 0.85 + 0.25 * band
        tint[..., 1] = 0.90 + 0.18 * band
        tint[..., 2] = 1.00
        return tint * luminance[..., None]

    def _stars(self, direction: np.ndarray) -> np.ndarray:
        grid = self.grid
        sigma_ang = self.sigma_ang
        cutoff = 3.2 * sigma_ang
        out = np.zeros(direction.shape, dtype=np.float64)
        position = (direction * 0.5 + 0.5) * grid
        base = np.floor(position - 0.5).astype(np.int64)

        for dx in (0, 1):
            for dy in (0, 1):
                for dz in (0, 1):
                    ix = base[:, 0] + dx
                    iy = base[:, 1] + dy
                    iz = base[:, 2] + dz
                    exists = _hash01(ix, iy, iz, 11) < self._star_exist_p
                    if not exists.any():
                        continue
                    rows = np.flatnonzero(exists)
                    cx, cy, cz = ix[rows], iy[rows], iz[rows]
                    star = np.stack(
                        [
                            (cx + _hash01(cx, cy, cz, 23)) / grid * 2.0 - 1.0,
                            (cy + _hash01(cx, cy, cz, 29)) / grid * 2.0 - 1.0,
                            (cz + _hash01(cx, cy, cz, 31)) / grid * 2.0 - 1.0,
                        ],
                        axis=-1,
                    )
                    norm = np.linalg.norm(star, axis=-1, keepdims=True)
                    on_sphere = norm[:, 0] > 1e-6
                    if not on_sphere.any():
                        continue
                    rows = rows[on_sphere]
                    cx, cy, cz = cx[on_sphere], cy[on_sphere], cz[on_sphere]
                    star = star[on_sphere] / norm[on_sphere]
                    cos_angle = np.clip(np.sum(star * direction[rows], axis=-1), -1.0, 1.0)
                    angle = np.arccos(cos_angle)
                    near = angle < cutoff
                    if not near.any():
                        continue
                    rows = rows[near]
                    cx, cy, cz = cx[near], cy[near], cz[near]
                    angle = angle[near]
                    flux = _hash01(cx, cy, cz, 37) ** 8
                    rare = _hash01(cx, cy, cz, 41) < 0.015
                    flux = flux * np.where(rare, 8.0, 1.0)
                    temperature = 2800.0 + 11000.0 * _hash01(cx, cy, cz, 43) ** 1.6
                    sigma = sigma_ang * (1.0 + 0.55 * np.log1p(flux))
                    profile = np.exp(-0.5 * (angle / sigma) ** 2)
                    colour = blackbody_rgb(temperature)
                    out[rows] += colour * (profile * flux * self.config.star_flux_scale)[:, None]
        return out


def _hermite(u0: np.ndarray, du0: np.ndarray, u1: np.ndarray, du1: np.ndarray, h, t):
    """Cubic Hermite value and derivative of u at parameter t in [0, 1].

    Used as dense output to locate a crossing inside a step. Linear
    interpolation of u is not enough: the escape step can cross seven orders of
    magnitude in radius, and the derivative error of a linear fit there is
    O(h^2 |u'''|), which is large compared with the invariant at the crossing.
    Found by the C2 check (residual flat at 1e-10 along the path, jumping to
    9e-5 on the escape step alone).
    """
    t = np.asarray(t, dtype=np.float64)
    t2 = t * t
    t3 = t2 * t
    value = (
        (2.0 * t3 - 3.0 * t2 + 1.0) * u0
        + (t3 - 2.0 * t2 + t) * h * du0
        + (-2.0 * t3 + 3.0 * t2) * u1
        + (t3 - t2) * h * du1
    )
    derivative = (
        (6.0 * t2 - 6.0 * t) * u0
        + (3.0 * t2 - 4.0 * t + 1.0) * h * du0
        + (-6.0 * t2 + 6.0 * t) * u1
        + (3.0 * t2 - 2.0 * t) * h * du1
    ) / h
    return value, derivative


# ---------------------------------------------------------------------------
# Geometry helpers
# ---------------------------------------------------------------------------

def _camera_basis(inclination_deg: float) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    """Return (position, forward, right, up) for the camera look-at the origin."""
    inc = math.radians(inclination_deg)
    position = np.array([math.sin(inc), 0.0, math.cos(inc)], dtype=np.float64)
    forward = -position
    right = np.array([0.0, 1.0, 0.0])
    up = np.cross(right, forward)
    up /= np.linalg.norm(up)
    return position, forward, right, up


def _disk_profile(radius: np.ndarray, r_in: float) -> np.ndarray:
    """Shakura-Sunyaev-like radial temperature profile (not normalised)."""
    ratio = r_in / np.maximum(radius, 1e-12)
    shape = np.clip(1.0 - np.sqrt(np.clip(ratio, 0.0, 1.0)), 0.0, None) ** 0.25
    return shape * np.clip(ratio, 0.0, None) ** 0.75


def _profile_peak(r_in: float) -> float:
    grid = np.linspace(r_in * 1.000001, r_in * 40.0, 400000)
    return float(_disk_profile(grid, r_in).max())


# ---------------------------------------------------------------------------
# Ray tracing
# ---------------------------------------------------------------------------

def trace_chunk(
    directions: np.ndarray,
    config: SceneConfig,
    sky: Sky,
    profile_peak: float,
    diagnostics: bool = False,
):
    """Trace one block of rays and return linear-RGB radiance per ray.

    With ``diagnostics=True`` returns ``(radiance, info)`` where ``info`` records
    the termination state of every ray (escape angle, impact parameter, disk
    hit), so that the verification script exercises this exact code path rather
    than a second implementation of the integrator.
    """
    count = directions.shape[0]
    position, _, _, _ = _camera_basis(config.inclination_deg)
    camera = position * config.r_obs
    e1 = camera / np.linalg.norm(camera)  # outward radial direction at the camera
    r_obs = config.r_obs
    u0 = 1.0 / r_obs

    proj = directions @ e1
    lateral = directions - proj[:, None] * e1[None, :]
    lateral_norm = np.linalg.norm(lateral, axis=-1)
    degenerate = lateral_norm < 1e-9
    e2 = lateral / np.maximum(lateral_norm, 1e-300)[:, None]

    # Full-size result records.
    escaped = np.zeros(count, dtype=bool)
    esc_u = np.zeros(count)
    esc_du = np.zeros(count)
    esc_phi = np.zeros(count)
    disk_hit = np.zeros(count, dtype=bool)
    disk_radius = np.zeros(count)
    disk_phi = np.zeros(count)
    disk_u = np.zeros(count)
    disk_du = np.zeros(count)

    # Exactly radial rays: outward -> sky, inward -> horizon (b ~ 0 << b_crit).
    if degenerate.any():
        outward = degenerate & (proj >= 0.0)
        inward = degenerate & (proj < 0.0)
        escaped[outward] = True
        esc_u[outward] = u0
        esc_du[outward] = -1.0
        esc_phi[outward] = 0.0
        _ = inward  # captured: no radiance is recorded

    # Compactable working set.
    work = np.flatnonzero(~degenerate)
    if work.size == 0:
        radiance = np.zeros((count, 3))
        if diagnostics:
            return radiance, {
                "escaped": escaped,
                "captured": ~escaped,
                "esc_phi": esc_phi,
                "esc_u": esc_u,
                "esc_du": esc_du,
                "disk_hit": disk_hit,
                "disk_radius": disk_radius,
                "disk_phi": disk_phi,
                "disk_u": disk_u,
                "disk_du": disk_du,
            }
        return radiance

    u = np.full(work.size, u0)
    du = -u0 * proj[work] / np.maximum(lateral_norm[work], 1e-300)
    phi = np.zeros(work.size)
    r_prev = np.full(work.size, r_obs)
    z_prev = np.full(work.size, r_obs * e1[2])
    e2w = e2[work]
    active = np.ones(work.size, dtype=bool)
    steps = 0

    while active.any() and steps < MAX_STEPS:
        steps += 1
        radius = 1.0 / u
        h = np.where(
            radius < 3.0,
            config.h_close,
            np.where(radius < R_NEAR_REFINE, config.h_near, config.h_far),
        )

        k1u, k1d = du, -u + 1.5 * u**2
        a_u, a_d = u + 0.5 * h * k1u, du + 0.5 * h * k1d
        k2u, k2d = a_d, -a_u + 1.5 * a_u**2
        b_u, b_d = u + 0.5 * h * k2u, du + 0.5 * h * k2d
        k3u, k3d = b_d, -b_u + 1.5 * b_u**2
        c_u, c_d = u + h * k3u, du + h * k3d
        k4u, k4d = c_d, -c_u + 1.5 * c_u**2

        # u_raw is kept unclamped: the escape crossing is located by
        # interpolating between u and u_raw, and clamping first would move it.
        u_raw = u + (h / 6.0) * (k1u + 2.0 * k2u + 2.0 * k3u + k4u)
        du_raw = du + (h / 6.0) * (k1d + 2.0 * k2d + 2.0 * k3d + k4d)
        u_new = np.maximum(u_raw, 1e-300)
        du_new = du_raw
        phi_new = phi + h
        r_new = 1.0 / u_new
        z_new = r_new * (np.cos(phi_new) * e1[2] + np.sin(phi_new) * e2w[:, 2])

        # --- First crossing of the equatorial plane inside the annulus -----
        crossing = active & (z_prev * z_new < 0.0)
        if crossing.any():
            hit = np.flatnonzero(crossing)
            denominator = z_prev[hit] - z_new[hit]
            frac = z_prev[hit] / np.where(np.abs(denominator) < 1e-300, 1e-300, denominator)
            r_cross = r_prev[hit] + frac * (r_new[hit] - r_prev[hit])
            inside = (r_cross >= config.r_in) & (r_cross <= config.r_out)
            fresh = hit[inside][~disk_hit[work[hit[inside]]]]
            if fresh.size:
                rows = work[fresh]
                disk_hit[rows] = True
                f = frac[np.searchsorted(hit, fresh)]
                u_c, du_c = _hermite(
                    u[fresh], du[fresh], u_raw[fresh], du_raw[fresh], h[fresh], f
                )
                disk_radius[rows] = 1.0 / np.maximum(u_c, 1e-300)
                disk_phi[rows] = phi[fresh] + f * h[fresh]
                disk_u[rows] = u_c
                disk_du[rows] = du_c
                active[fresh] = False  # an optically thick disk occludes

        # --- Termination: horizon ------------------------------------------
        into_horizon = active & (u_new >= 1.0)
        active[into_horizon] = False

        # --- Termination: escaped past r_escape -----------------------------
        # The crossing of r = r_escape is located to sub-step accuracy by
        # interpolating in u. Without this, the recorded sweep angle is wrong by
        # up to one full step, and in the far field a single step of size h can
        # overshoot u = 1/r_escape by orders of magnitude (u is then ~0, and
        # both the invariant check and the outgoing direction become ill
        # conditioned). This was found by the C2/C4 checks in verify_raytrace.
        u_escape = 1.0 / config.escape_radius()
        leaving = active & (u_raw < u_escape) & (du_raw < 0.0)
        if leaving.any():
            rows = work[leaving]
            sub_u0, sub_du0 = u[leaving], du[leaving]
            sub_u1, sub_du1 = u_raw[leaving], du_raw[leaving]
            sub_h = h[leaving]
            # Dense output: solve the Hermite cubic for u = u_escape, then take
            # its analytic derivative there rather than interpolating linearly.
            # u decreases across this step, so u(mid) < u_escape means the root
            # lies to the LEFT of mid.
            lo_t = np.zeros(sub_u0.size)
            hi_t = np.ones(sub_u0.size)
            for _ in range(60):
                mid_t = 0.5 * (lo_t + hi_t)
                value, _ = _hermite(sub_u0, sub_du0, sub_u1, sub_du1, sub_h, mid_t)
                below = value < u_escape
                hi_t = np.where(below, mid_t, hi_t)
                lo_t = np.where(below, lo_t, mid_t)
            t_star = 0.5 * (lo_t + hi_t)
            _, du_star = _hermite(sub_u0, sub_du0, sub_u1, sub_du1, sub_h, t_star)
            escaped[rows] = True
            esc_u[rows] = u_escape
            esc_du[rows] = du_star
            esc_phi[rows] = phi[leaving] + t_star * sub_h
            active[leaving] = False

        # --- Termination: swept-angle cap (thin near-critical annulus) ------
        capped = active & (phi_new >= config.max_phi_factor * 2.0 * math.pi)
        active[capped] = False

        u, du, phi, r_prev, z_prev = u_new, du_new, phi_new, r_new, z_new
        if not active.all():
            keep = np.flatnonzero(active)
            if keep.size == 0:
                break
            work = work[keep]
            u, du, phi = u[keep], du[keep], phi[keep]
            r_prev, z_prev = r_prev[keep], z_prev[keep]
            e2w = e2w[keep]
            active = np.ones(keep.size, dtype=bool)

    # -----------------------------------------------------------------------
    # Shading
    # -----------------------------------------------------------------------
    radiance = np.zeros((count, 3))

    if escaped.any():
        rows = np.flatnonzero(escaped)
        uu = np.maximum(esc_u[rows], 1e-300)
        rr = 1.0 / uu
        dr_dphi = -esc_du[rows] / uu**2
        phi_e = esc_phi[rows]
        # Tangent in the orbital plane. The basis must be the one at the ray's
        # current phi, not the fixed (e1, e2):
        #     dx/dphi = (dr/dphi) e_r + r e_phi,
        #     e_r = cos(phi) e1 + sin(phi) e2,  e_phi = -sin(phi) e1 + cos(phi) e2.
        # An earlier version used dr_dphi*e1 + r*e2, which is the tangent only at
        # phi = 0; it collapsed every off-axis ray onto nearly the same sky
        # direction and magnified single stars into large soft blobs.
        cos_phi = np.cos(phi_e)
        sin_phi = np.sin(phi_e)
        outgoing = (
            (dr_dphi * cos_phi - rr * sin_phi)[:, None] * e1[None, :]
            + (dr_dphi * sin_phi + rr * cos_phi)[:, None] * e2[rows]
        )
        outgoing /= np.maximum(np.linalg.norm(outgoing, axis=-1, keepdims=True), 1e-300)
        # The observed photon travelled opposite to our outward integration.
        radiance[rows] = sky.sample(-outgoing)

    if disk_hit.any():
        rows = np.flatnonzero(disk_hit)
        r_c = disk_radius[rows]
        phi_c = disk_phi[rows]
        u_c = np.maximum(disk_u[rows], 1e-300)
        rr = 1.0 / u_c
        dr_dphi = -disk_du[rows] / u_c**2
        radial = np.cos(phi_c)[:, None] * e1[None, :]
        tangential = np.sin(phi_c)[:, None] * e2[rows]
        xvec = rr[:, None] * (radial + tangential)
        planar = np.maximum(
            np.linalg.norm(xvec[:, :2], axis=-1), 1e-300
        )
        beta = np.sqrt(M_GEO / np.maximum(r_c, RS))
        vel_dir = np.empty((rows.size, 3))
        vel_dir[:, 0] = -xvec[:, 1] / planar
        vel_dir[:, 1] = xvec[:, 0] / planar
        vel_dir[:, 2] = 0.0
        outgoing = (
            (dr_dphi * np.cos(phi_c) - rr * np.sin(phi_c))[:, None] * e1[None, :]
            + (dr_dphi * np.sin(phi_c) + rr * np.cos(phi_c))[:, None] * e2[rows]
        )
        outgoing /= np.maximum(np.linalg.norm(outgoing, axis=-1, keepdims=True), 1e-300)
        to_observer = -outgoing
        beta_dot_n = beta * np.sum(vel_dir * to_observer, axis=-1)
        gamma = 1.0 / np.sqrt(np.maximum(1.0 - beta**2, 1e-12))
        doppler = 1.0 / (gamma * np.maximum(1.0 - beta_dot_n, 1e-6))
        grav = np.sqrt(np.maximum(1.0 - RS / r_c, 1e-12))
        g_total = doppler * grav
        profile = _disk_profile(r_c, config.r_in) / profile_peak
        temperature = (
            np.clip(g_total, 0.05, None)
            * config.disk_t_max_k
            * profile ** config.disk_colour_exponent
        )
        flux = config.disk_peak_flux * (
            profile**4 * g_total**4
        ) ** config.disk_flux_exponent
        radiance[rows] = blackbody_rgb(temperature) * flux[:, None]

    if diagnostics:
        return radiance, {
            "escaped": escaped,
            "captured": (~escaped) & (~disk_hit),
            "esc_phi": esc_phi,
            "esc_u": esc_u,
            "esc_du": esc_du,
            "disk_hit": disk_hit,
            "disk_radius": disk_radius,
            "disk_phi": disk_phi,
            "disk_u": disk_u,
            "disk_du": disk_du,
        }
    return radiance


# ---------------------------------------------------------------------------
# Rendering driver
# ---------------------------------------------------------------------------

def render(
    width: int,
    height: int,
    config: SceneConfig,
    supersample: int = 2,
    seed: int = 20261015,
    progress: bool = True,
) -> np.ndarray:
    """Render the scene and return an 8-bit sRGB image array."""
    sky = Sky(config, seed=seed)
    profile_peak = _profile_peak(config.r_in)
    trace_w, trace_h = width * supersample, height * supersample

    x_ndc = (np.arange(trace_w) + 0.5) / trace_w * 2.0 - 1.0
    y_ndc = 1.0 - (np.arange(trace_h) + 0.5) / trace_h * 2.0
    tan_h = math.tan(math.radians(config.hfov_deg) * 0.5)
    tan_v = tan_h * (trace_h / trace_w)

    _, forward, right, up = _camera_basis(config.inclination_deg)
    dir_x = np.outer(np.ones(trace_h), x_ndc * tan_h)
    dir_y = np.outer(y_ndc * tan_v, np.ones(trace_w))

    linear = np.empty((trace_h, trace_w, 3))
    block_rows = max(1, min(64, trace_h))
    started = time.time()
    for start in range(0, trace_h, block_rows):
        stop = min(start + block_rows, trace_h)
        dx = dir_x[start:stop].ravel()
        dy = dir_y[start:stop].ravel()
        directions = (
            forward[None, :] + dx[:, None] * right[None, :] + dy[:, None] * up[None, :]
        )
        directions /= np.linalg.norm(directions, axis=-1, keepdims=True)
        linear[start:stop] = trace_chunk(directions, config, sky, profile_peak).reshape(
            stop - start, trace_w, 3
        )
        if progress:
            done = stop / trace_h
            elapsed = time.time() - started
            print(
                f"  rows {stop:5d}/{trace_h}  {done * 100:5.1f}%  "
                f"{elapsed:6.1f}s elapsed, ~{elapsed / done - elapsed:6.1f}s left",
                flush=True,
            )

    return _tonemap(linear * config.exposure, supersample)


def _tonemap(linear: np.ndarray, supersample: int) -> np.ndarray:
    """Bloom, ACES-like tone mapping, sRGB encode, supersample resolve."""
    luminance = linear @ np.array([0.2126, 0.7152, 0.0722])
    excess = np.clip(luminance - 0.75, 0.0, None)
    scale = np.where(luminance > 1e-9, excess / np.maximum(luminance, 1e-9), 0.0)
    bright = linear * scale[..., None]
    small = Image.fromarray(np.clip(bright * 255.0, 0, 255).astype(np.uint8)).resize(
        (max(1, linear.shape[1] // 4), max(1, linear.shape[0] // 4)), Image.BILINEAR
    )
    bloom = (
        np.asarray(
            small.filter(ImageFilter.GaussianBlur(radius=2.5)).resize(
                (linear.shape[1], linear.shape[0]), Image.BILINEAR
            ),
            dtype=np.float64,
        )
        / 255.0
    )
    x = np.clip(linear + 0.25 * bloom, 0.0, None)

    a, b, c, d, e = 2.51, 0.03, 2.43, 0.59, 0.14
    mapped = (x * (a * x + b)) / (x * (c * x + d) + e)
    encoded = np.where(
        mapped <= 0.0031308,
        mapped * 12.92,
        1.055 * np.power(np.clip(mapped, 0.0, None), 1.0 / 2.4) - 0.055,
    )
    image = Image.fromarray(np.clip(encoded * 255.0 + 0.5, 0, 255).astype(np.uint8))
    if supersample > 1:
        image = image.resize(
            (image.width // supersample, image.height // supersample), Image.LANCZOS
        )
    return np.asarray(image)


def main() -> int:
    defaults = SceneConfig()
    parser = argparse.ArgumentParser(description="Schwarzschild black-hole renderer.")
    parser.add_argument("--width", type=int, default=1920)
    parser.add_argument("--height", type=int, default=1080)
    parser.add_argument("--supersample", type=int, default=2)
    parser.add_argument("--seed", type=int, default=20261015)
    parser.add_argument("--out", type=str, default="assets/blackhole-hero.png")
    # Defaults are taken from SceneConfig so the dataclass stays the single
    # source of truth for the scene (a duplicated literal here silently
    # overrode every geometry change made in the dataclass).
    parser.add_argument("--r-obs", type=float, default=defaults.r_obs)
    parser.add_argument("--inclination", type=float, default=defaults.inclination_deg)
    parser.add_argument("--hfov", type=float, default=defaults.hfov_deg)
    parser.add_argument("--r-out", type=float, default=defaults.r_out)
    parser.add_argument("--exposure", type=float, default=defaults.exposure)
    args = parser.parse_args()

    config = SceneConfig(
        r_obs=args.r_obs,
        inclination_deg=args.inclination,
        hfov_deg=args.hfov,
        r_out=args.r_out,
        exposure=args.exposure,
    )
    print(f"tracing {args.width * args.supersample} x {args.height * args.supersample} rays")
    started = time.time()
    image = render(
        args.width, args.height, config, supersample=args.supersample, seed=args.seed
    )
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(image).save(out_path, optimize=True)
    print(f"wrote {out_path}  ({time.time() - started:.1f}s)")
    print(f"predicted shadow edge at b = {B_CRIT_OVER_RS:.6f} r_s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
