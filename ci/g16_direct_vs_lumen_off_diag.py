#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16.7 诊断）
"""G16.7：Rurix --gi off vs UE Lumen-off。只落 milestones/g16 诊断件，不充 18/18。"""
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g16_p0_lib as g16  # noqa: E402
import g10_exr_lib as exr  # noqa: E402

OUT = g16.ROOT / "milestones" / "g16" / "g16plus_direct_vs_lumen_off.json"
DISP = g16.ROOT / "milestones" / "g16" / "g16_quality_gap_disposition.json"
RURIX_OFF = Path(r"K:\rurix-ext\g15-frames\m_c_prod\cornell-box\tier67\tsr_device\converged.exr")
UE_OFF = g16.UE_LUMEN / "cornell-box" / "off" / ".0031.exr"
GI_ON = g16.ROOT / ".tmp" / "g16plus_gi_probe" / "on" / "cornell-box" / "tier67" / "tsr_device" / "converged.exr"
GI_OFF = g16.ROOT / ".tmp" / "g16plus_gi_probe" / "off" / "cornell-box" / "tier67" / "tsr_device" / "converged.exr"
GI_ON_FALLBACK = g16.ROOT / ".tmp" / "g16plus_gi_probe" / "mega_on" / "cornell-box" / "tier67" / "tsr_device" / "converged.exr"
GI_OFF_FALLBACK = g16.ROOT / ".tmp" / "g16plus_gi_probe" / "mega_off" / "cornell-box" / "tier67" / "tsr_device" / "converged.exr"


def mean_luma(path: Path, end: str) -> float | None:
    if not path.is_file():
        return None
    doc = exr.decode_exr_file(path, end)
    px, n = doc["pixels"], doc["width"] * doc["height"]
    s = 0.0
    for i in range(n):
        s += px[i * 3] * 0.2126 + px[i * 3 + 1] * 0.7152 + px[i * 3 + 2] * 0.0722
    return s / max(n, 1)


def main() -> int:
    ru = mean_luma(RURIX_OFF, "rurix")
    ue = mean_luma(UE_OFF, "ue5")
    energy_ue = None
    indirect = None
    if DISP.is_file():
        for i in (json.loads(DISP.read_text(encoding="utf-8")).get("items") or []):
            if i.get("scene_id") != "cornell-box":
                continue
            for m in i.get("fresh_measured_delta") or []:
                if m.get("metric") == "gi_energy_rel@cornell-box":
                    energy_ue = m.get("a_value")
                if m.get("metric") == "indirect_ssim@cornell-box":
                    indirect = m.get("b_value")
    close = None
    if ru is not None and ue is not None and ue > 1e-8:
        close = abs(ru - ue) / ue
    on_p = GI_ON if GI_ON.is_file() else GI_ON_FALLBACK
    off_p = GI_OFF if GI_OFF.is_file() else GI_OFF_FALLBACK
    gi_on = mean_luma(on_p, "rurix")
    gi_off = mean_luma(off_p, "rurix")
    same_domain = None
    if gi_on is not None and gi_off is not None:
        same_domain = gi_on - gi_off
    doc = {
        "schema_version": 1,
        "wave": "G16.7",
        "generated_utc": datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ"),
        "purpose": "坐实 247× 主因是间接光，不是几何/曝光。不充 18/18。",
        "rurix_gi_off_mean_luma": ru,
        "ue_lumen_off_mean_luma": ue,
        "rel_abs_diff": close,
        "nine_cells_near_if_rel_lt_0_25": bool(close is not None and close < 0.25),
        "domain_note": "raw HDR 均值可能跨显示域；主因仍以 M-b indirect_ssim≈0.05 与 GI on/off 同域差为准。",
        "same_domain_gi_on_minus_off": same_domain,
        "same_domain_gi_on_luma": gi_on,
        "same_domain_gi_off_luma": gi_off,
        "indirect_established": bool(same_domain is not None and same_domain > 1e-3),
        "priority": ["cornell-box 面光反弹", "bistro 填光"],
        "energy_targets_from_m_b": {
            "cornell_energy_ue": energy_ue,
            "cornell_indirect_ssim": indirect,
            "bistro_gi_energy_rel_note": "UE≈3.0 vs Rurix≈0.54（M-b 处置表）",
        },
        "paths": {
            "rurix_off": str(RURIX_OFF),
            "ue_lumen_off": str(UE_OFF),
            "rurix_gi_on": str(on_p),
            "rurix_gi_off": str(off_p),
        },
        "honest": "host 对照可；18 格通过线只认 GPU 生产臂（G11-N3）。",
    }
    OUT.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(doc, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
