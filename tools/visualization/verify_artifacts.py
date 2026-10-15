#!/usr/bin/env python3
"""Verify the delivered image artifacts exist, are readable and are 16:9."""

from __future__ import annotations

import hashlib
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = {
    "assets/blackhole-hero.png": (1920, 1080),
    "assets/blackhole-scientific.png": (1920, 1080),
}

failures = 0
for relative, expected in ARTIFACTS.items():
    path = ROOT / relative
    if not path.is_file():
        print(f"MISSING  {relative}")
        failures += 1
        continue
    payload = path.read_bytes()
    digest = hashlib.sha256(payload).hexdigest()
    with Image.open(path) as image:
        size = image.size
        mode = image.mode
        image.verify()
    ratio_ok = abs(size[0] / size[1] - 16 / 9) < 1e-6
    size_ok = size == expected
    status = "OK" if (size_ok and ratio_ok) else "FAIL"
    if status == "FAIL":
        failures += 1
    print(
        f"{status:4s}  {relative}\n"
        f"        {size[0]}x{size[1]} {mode}  {len(payload) / 1e6:.2f} MB  "
        f"16:9 {ratio_ok}  sha256 {digest[:16]}"
    )

print()
print(f"artifacts checked: {len(ARTIFACTS)}, failures: {failures}")
raise SystemExit(1 if failures else 0)
