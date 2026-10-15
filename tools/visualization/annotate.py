#!/usr/bin/env python3
"""Scientific annotation pass for the Black Hole Lab visualization.

Draws the physically meaningful radii onto the rendered hero image and labels
them. The geometry is derived from the same Schwarzschild relations the tracer
uses, so the annotated radii cannot drift away from the physics:

* the apparent shadow edge (the photon ring) is at impact parameter
  b_crit = 3*sqrt(3)*M = (3*sqrt(3)/2) r_s = 2.598076 r_s -- NOT at r = r_s;
* the event horizon r = r_s and the photon sphere r = 3r_s/2 are marked at the
  angle they would subtend *without* lensing, because neither is directly
  imaged. The horizon lies hidden inside the shadow.

Everything drawn here is an annotation, not simulation output.
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

B_CRIT_OVER_RS = 3.0 * math.sqrt(3.0) / 2.0  # 2.598076211353316
FONT_REGULAR = "/System/Library/Fonts/Supplemental/Arial.ttf"
FONT_BOLD = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"

INK = (232, 232, 224)
INK_DIM = (168, 172, 178)
ACCENT = (255, 196, 120)
GHOST = (150, 158, 170)
PANEL = (8, 9, 12)
# Populated by label() so main() can verify that no two panels overlap.
LABEL_BOXES: list[tuple[str, list[int]]] = []


def pixel_radius_for_impact_parameter(b: float, r_obs: float, hfov_deg: float, width: int) -> float:
    """Screen radius in pixels whose ray has conserved impact parameter b.

    Inverts 1/b^2 = u0^2 cot^2(theta) + u0^2 (1 - r_s u0) for the launch angle
    theta measured from the inward radial direction, then maps theta through the
    pinhole projection.
    """
    u0 = 1.0 / r_obs
    inverse = 1.0 / (b * b * u0 * u0) - (1.0 - u0)
    if inverse <= 0.0:
        return float("nan")  # not reachable at this camera radius
    tan_theta = 1.0 / math.sqrt(inverse)
    tan_half_fov = math.tan(math.radians(hfov_deg) * 0.5)
    x_ndc = tan_theta / tan_half_fov
    return x_ndc * width * 0.5


def pixel_radius_unlensed(radius_rs: float, r_obs: float, hfov_deg: float, width: int) -> float:
    """Screen radius of a straight-line ray that grazes a sphere of the given radius."""
    sin_theta = min(radius_rs / r_obs, 1.0)
    theta = math.asin(sin_theta)
    tan_half_fov = math.tan(math.radians(hfov_deg) * 0.5)
    return math.tan(theta) / tan_half_fov * width * 0.5


def dashed_circle(draw: ImageDraw.ImageDraw, center, radius, colour, width=1, dash=9, gap=7):
    cx, cy = center
    step = math.radians(1.4)
    arc = 2.0 * math.pi * radius
    dash_rad = 2.0 * math.pi * dash / max(arc, 1.0)
    gap_rad = 2.0 * math.pi * gap / max(arc, 1.0)
    angle = 0.0
    while angle < 2.0 * math.pi:
        end = min(angle + dash_rad, 2.0 * math.pi)
        draw.arc(
            [cx - radius, cy - radius, cx + radius, cy + radius],
            math.degrees(angle) - 90.0,
            math.degrees(end) - 90.0,
            fill=colour,
            width=width,
        )
        angle = end + gap_rad


SUBSCRIPT_S = "\u209b"


def _sub_font(size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(FONT_REGULAR, max(6, int(size * 0.68)))


def rich_width(draw: ImageDraw.ImageDraw, text: str, font) -> float:
    """Width of a string where U+209B is drawn as a real subscript.

    Arial has no glyph for U+209B under Pillow: it renders as a tofu box, and a
    presence probe based on mask size reports a false positive for it. The
    subscript is therefore drawn explicitly, smaller and lowered.
    """
    total = 0.0
    for index, segment in enumerate(text.split(SUBSCRIPT_S)):
        if index:
            total += draw.textlength("s", font=_sub_font(font.size))
        total += draw.textlength(segment, font=font)
    return total


def rich_text(draw: ImageDraw.ImageDraw, xy, text: str, font, fill) -> None:
    x, y = xy
    for index, segment in enumerate(text.split(SUBSCRIPT_S)):
        if index:
            sub = _sub_font(font.size)
            draw.text((x, y + font.size * 0.30), "s", font=sub, fill=fill)
            x += draw.textlength("s", font=sub)
        draw.text((x, y), segment, font=font, fill=fill)
        x += draw.textlength(segment, font=font)


def label(
    draw,
    font,
    text,
    xy,
    anchor,
    fill=INK,
    align_right=False,
    panel_alpha=210,
    pad=9,
):
    """Draw a label on a soft dark panel so it stays legible over any background."""
    lines = text.split("\n")
    widths = [rich_width(draw, line, font) for line in lines]
    line_h = font.size + 5
    box_w = int(max(widths)) + 2 * pad
    box_h = line_h * len(lines) + 2 * pad - 5
    x, y = xy
    if anchor == "lt":
        box = [x, y, x + box_w, y + box_h]
        origin = (x + pad, y + pad)
    elif anchor == "rt":
        box = [x - box_w, y, x, y + box_h]
        origin = (x - box_w + pad, y + pad)
    else:  # "ct"
        box = [x - box_w // 2, y, x + box_w // 2, y + box_h]
        origin = (x - box_w // 2 + pad, y + pad)
    draw.rectangle(box, fill=PANEL + (panel_alpha,))
    draw.rectangle([box[0], box[1], box[0] + 2, box[3]], fill=ACCENT)
    LABEL_BOXES.append((lines[0], box))
    for index, line in enumerate(lines):
        rich_text(
            draw,
            (origin[0], origin[1] + index * line_h),
            line,
            font,
            fill if index == 0 else INK_DIM,
        )
    return box


def leader(draw, start, end, colour=INK_DIM, width=1, dot=True):
    draw.line([start, end], fill=colour, width=width)
    if dot:
        draw.ellipse([end[0] - 2, end[1] - 2, end[0] + 2, end[1] + 2], fill=colour)


def main() -> int:
    parser = argparse.ArgumentParser(description="Annotate the rendered hero image.")
    parser.add_argument("--in", dest="source", default="assets/blackhole-hero.png")
    parser.add_argument("--out", default="assets/blackhole-scientific.png")
    parser.add_argument("--r-obs", type=float, default=60.0)
    parser.add_argument("--inclination", type=float, default=82.0)
    parser.add_argument("--hfov", type=float, default=28.0)
    parser.add_argument("--r-out", type=float, default=14.0)
    args = parser.parse_args()

    source = Image.open(args.source).convert("RGBA")
    width, height = source.size
    # Hold the photograph back so the line work reads.
    base = Image.blend(source, Image.new("RGBA", source.size, (0, 0, 0, 255)), 0.30)
    canvas = base.copy()
    draw = ImageDraw.Draw(canvas, "RGBA")

    cx, cy = width // 2, height // 2
    scale = width / 1920.0  # all radii below are quoted for a 1920-wide render
    f_title = ImageFont.truetype(FONT_BOLD, int(23 * scale))
    f_label = ImageFont.truetype(FONT_REGULAR, int(18 * scale))
    f_small = ImageFont.truetype(FONT_REGULAR, int(15 * scale))
    boxes: list[tuple[str, list[int]]] = []

    r_shadow = pixel_radius_for_impact_parameter(B_CRIT_OVER_RS, args.r_obs, args.hfov, width)
    r_horizon = pixel_radius_unlensed(1.0, args.r_obs, args.hfov, width)
    r_photon = pixel_radius_unlensed(1.5, args.r_obs, args.hfov, width)

    # --- the photon ring: the only one of the three that is an actual image ---
    draw.ellipse(
        [cx - r_shadow, cy - r_shadow, cx + r_shadow, cy + r_shadow],
        outline=ACCENT + (215,),
        width=max(1, int(round(1.6 * scale))),
    )
    # --- horizon and photon sphere at their unlensed angular scale -----------
    dashed_circle(draw, (cx, cy), r_horizon, (255, 120, 110, 190), width=max(1, int(1.2 * scale)))
    dashed_circle(draw, (cx, cy), r_photon, (120, 190, 255, 175), width=max(1, int(1.2 * scale)))

    # --- radial tick scale in units of the impact parameter b ----------------
    y_axis = int(cy + height * 0.5 - 168 * scale)
    draw.line([(cx, y_axis), (cx + r_shadow * 5.1, y_axis)], fill=INK_DIM + (140,), width=1)
    for b in (1.0, 2.0, B_CRIT_OVER_RS, 5.0, 10.0):
        px = pixel_radius_for_impact_parameter(b, args.r_obs, args.hfov, width)
        if not math.isfinite(px) or px > width * 0.5 - 20 * scale:
            continue
        major = abs(b - B_CRIT_OVER_RS) < 1e-9
        half = int((11 if major else 6) * scale)
        draw.line(
            [(cx + px, y_axis - half), (cx + px, y_axis + half)],
            fill=(ACCENT if major else INK_DIM) + (220,),
            width=max(1, int(1.0 * scale)),
        )
        text = "2.60" if major else f"{b:g}"
        draw.text(
            (cx + px - 8 * scale, y_axis + half + 3 * scale),
            text,
            font=f_small,
            fill=(ACCENT if major else INK_DIM),
        )
    draw.text(
        (cx + 4 * scale, y_axis - 22 * scale),
        "impact parameter b  [r\u209b]",
        font=f_small,
        fill=INK_DIM,
    )

    # --- labels --------------------------------------------------------------
    label(
        draw,
        f_title,
        "Schwarzschild black hole \u2014 lensing geometry\n"
        "conceptual illustration, not renderer output",
        (int(46 * scale), int(36 * scale)),
        "lt",
    )
    label(
        draw,
        f_label,
        "shadow edge = photon ring\n"
        f"b = 3\u221a3 GM/c\u00b2 = (3\u221a3/2) r\u209b \u2248 {B_CRIT_OVER_RS:.3f} r\u209b\n"
        "photons with b < b_crit are captured",
        (width - int(46 * scale), int(36 * scale)),
        "rt",
    )
    label(
        draw,
        f_label,
        "r = r\u209b  event horizon\n"
        "r = 3r\u209b/2  photon sphere\n"
        "(dashed, at their unlensed angular\n"
        "scale \u2014 neither surface is imaged;\n"
        "both lie hidden inside the shadow)",
        (int(46 * scale), int(height * 0.30)),
        "lt",
    )
    label(
        draw,
        f_label,
        "accretion disk\n"
        "geometrically thin, optically thick, Lambertian\n"
        f"r_in = 6GM/c\u00b2 = 3 r\u209b (ISCO),  r_out = {args.r_out:g} r\u209b\n"
        "near side seen directly; far side lensed\n"
        "over and under the shadow",
        (int(46 * scale), int(height * 0.70)),
        "lt",
    )
    label(
        draw,
        f_label,
        "lensed image of the far side\n"
        "of the disk, bent beneath the shadow",
        (width - int(46 * scale), int(height * 0.78)),
        "rt",
    )
    label(
        draw,
        f_small,
        f"observer at r = {args.r_obs:g} r\u209b, inclination i = {args.inclination:g}\u00b0 from the disk normal,\n"
        f"field of view {args.hfov:g}\u00b0 horizontal.  Null geodesics integrated with RK4:\n"
        "d\u00b2u/d\u03c6\u00b2 = \u2212u + (3/2) r\u209b u\u00b2,  u = 1/r.  Emission model, colours and star field\n"
        "are illustrative; no GRMHD, radiative transfer, spin or returning radiation.",
        (int(46 * scale), int(height - 100 * scale)),
        "lt",
        panel_alpha=205,
    )

    # --- orientation marker: observer direction and disk plane ---------------
    ox = int(width - 152 * scale)
    oy = int(height * 0.20)
    inc = math.radians(args.inclination)
    disk_dx, disk_dy = math.cos(inc) * 58 * scale, math.sin(inc) * 58 * scale
    draw.line(
        [(ox - disk_dx, oy + disk_dy), (ox + disk_dx, oy - disk_dy)],
        fill=INK_DIM + (200,),
        width=max(1, int(1.2 * scale)),
    )
    tip = (ox + 44 * scale * math.sin(inc), oy + 44 * scale * math.cos(inc))
    draw.line([(ox, oy), tip], fill=INK + (225,), width=max(1, int(1.3 * scale)))
    draw.ellipse(
        [tip[0] - 3.2, tip[1] - 3.2, tip[0] + 3.2, tip[1] + 3.2], fill=INK
    )
    draw.text((ox - 54 * scale, oy + disk_dy + 6 * scale), "disk plane", font=f_small, fill=INK_DIM)
    draw.text((ox + 16 * scale, oy + 40 * scale), "observer", font=f_small, fill=INK)

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(out_path, optimize=True)

    # Layout self-check: overlapping panels are a defect in a scientific figure,
    # so assert it rather than eyeballing it.
    clashes = []
    for i in range(len(LABEL_BOXES)):
        name_a, a = LABEL_BOXES[i]
        for j in range(i + 1, len(LABEL_BOXES)):
            name_b, b = LABEL_BOXES[j]
            if a[0] < b[2] and b[0] < a[2] and a[1] < b[3] and b[1] < a[3]:
                clashes.append(f"{name_a!r} overlaps {name_b!r}")
    offscreen = [
        name
        for name, box in LABEL_BOXES
        if box[0] < 0 or box[1] < 0 or box[2] > width or box[3] > height
    ]

    print(f"wrote {out_path}")
    print(f"  image                     : {width} x {height}")
    print(f"  shadow edge (photon ring) : r = {r_shadow:.1f} px  (b = {B_CRIT_OVER_RS:.6f} r_s)")
    print(f"  horizon, unlensed scale   : r = {r_horizon:.1f} px  (r = 1 r_s)")
    print(f"  photon sphere, unlensed   : r = {r_photon:.1f} px  (r = 1.5 r_s)")
    print(f"  label panels              : {len(LABEL_BOXES)}")
    print(f"  overlapping panels        : {len(clashes)}" + (f"  {clashes}" if clashes else ""))
    print(f"  panels off the canvas     : {len(offscreen)}" + (f"  {offscreen}" if offscreen else ""))
    if clashes or offscreen:
        print("  LAYOUT: FAIL")
        return 1
    print("  LAYOUT: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
