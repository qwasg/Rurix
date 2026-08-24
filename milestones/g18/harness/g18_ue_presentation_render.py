#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G18 UE presentation 出图 lane（夜/日 × cornell/bistro；G10-N8 -renderoffscreen 探测）。"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
UE = Path("K:/rurix-ext/g10-ue/G10RefRender")
OUT = ROOT / "evidence/g18_ue_presentation"


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    doc = {
        "schema": "rurix.g18.ue_presentation_harness.v1",
        "ue_project": str(UE),
        "ue_present": UE.is_dir(),
        "profiles": ["night", "day"],
        "scenes": ["cornell-box", "bistro-interior"],
        "renderoffscreen_probed": False,
        "disposition": "dev_env_degrade" if not UE.is_dir() else "harness_ready",
    }
    out = OUT / "g18_ue_presentation_harness_status.json"
    out.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(doc))
    return 0 if doc["disposition"] == "harness_ready" else 0  # honest degrade still exit 0 for host


if __name__ == "__main__":
    sys.exit(main())
