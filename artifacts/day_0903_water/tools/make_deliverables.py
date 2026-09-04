#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G41 交付清单生成器:把本战役全部交付件的 sha256 登记进 `DELIVERABLES.json`。

分三组,按入库口径:
- `source_in_tree`:仓库源码面(host 模块 / 五 kernel / 两 bin / 门 / RFC)。
- `campaign_in_tree`:战役目录内入库件(三份 md / tools / 冻结带 / 门 evidence /
  mp4 登记件)。
- `media_on_disk_not_tracked`:预览 PNG 与 mp4——仓库全局 `*.png` 与战役
  `.gitignore` 块使其**留盘不入库**,本清单的 sha256 即其入库登记面
  (day_0902_rain_night 同律)。

用法:`py -3 artifacts/day_0903_water/tools/make_deliverables.py`(在仓库根跑)。
"""

from __future__ import annotations

import datetime
import glob
import hashlib
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OUT = os.path.join(ROOT, "artifacts", "day_0903_water", "DELIVERABLES.json")

GROUPS = {
    "source_in_tree": [
        "src/rurix-render/src/world/water_surface.rs",
        "src/rurix-render/kernels/g41_water_*.rx",
        "src/rurix-render/src/bin/g41_water_present.rs",
        "src/rurix-render/src/bin/g41_water_probe.rs",
        "ci/g41_water_smoke.py",
        "rfcs/0050-water-surface-rendering.md",
    ],
    "campaign_in_tree": [
        "artifacts/day_0903_water/*.md",
        "artifacts/day_0903_water/tools/*.py",
        "artifacts/day_0903_water/g41_wave_band.json",
        "artifacts/day_0903_water/evidence/*.json",
        "artifacts/day_0903_water/lagoon_orbit.mp4.json",
    ],
    "media_on_disk_not_tracked": [
        "artifacts/day_0903_water/previews/*.png",
        "artifacts/day_0903_water/lagoon_orbit.mp4",
    ],
}


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    os.chdir(ROOT)
    groups = {}
    total = 0
    for name, patterns in GROUPS.items():
        files = []
        for pat in patterns:
            for p in sorted(glob.glob(pat)):
                if os.path.basename(p) == "DELIVERABLES.json":
                    continue  # 自身不入自身清单
                files.append(
                    {
                        "path": p.replace(os.sep, "/"),
                        "bytes": os.path.getsize(p),
                        "sha256": sha256(p),
                    }
                )
        groups[name] = {"count": len(files), "files": files}
        total += len(files)

    out = {
        "schema": "rurix.g41.deliverables.v1",
        "campaign": "day_0903_water",
        "gate": "g41.water.surface",
        "gate_status": "pass",
        "gate_facts": 11,
        "rfc": "RFC-0050 (Draft)",
        "generated_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "committed": False,
        "commit_note": "未 commit,入库归 owner",
        "binary_policy": (
            "previews/*.png 与 *.mp4 留盘不入库(仓库全局 *.png + 战役 .gitignore 块);"
            "本清单 sha256 为其入库登记面"
        ),
        "env_asset": {
            "source": "Poly Haven",
            "license": "CC0-1.0",
            "slug": "lakeside_sunrise",
            "res": "1k",
            "cache": "K:/rurix_g10_cache/polyhaven-env/lakeside_sunrise/",
            "note": "HDR 与烘焙 LUT 留缓存根不入 git;完整 sha256 见该目录 *.skylut.json",
        },
        "file_count": total,
        "groups": groups,
    }
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)
        f.write("\n")
    for name, g in groups.items():
        print(f"{name}: {g['count']}")
    print(f"total: {total} → {os.path.relpath(OUT, ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
