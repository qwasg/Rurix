#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G40 交付清单生成器:把本战役全部交付件的 sha256 登记进 `DELIVERABLES.json`。

分三组,按入库口径:
- `source_in_tree`:仓库源码面(两个 host 模块 / 两 kernel / 展示车道 bin / 门)。
- `campaign_in_tree`:战役目录内入库件(三份 md / tools / 门 evidence)。
- `media_on_disk_not_tracked`:预览 PNG 13 张——仓库全局 `*.png` 使其**留盘不入库**,
  本清单的 sha256 即其入库登记面(day_0902_rain_night / day_0903_water 同律)。

`gate_status` 由 `--gate-status` 给定,默认 `pending`:门
`ci/g40_cloud_smoke.py` 是补齐件,尚未真跑,**不冒充 PASS**。门绿之后重跑本脚本
并传 `--gate-status pass`。

用法:`py -3 artifacts/day_0903_clouds/tools/make_deliverables.py`(在仓库根跑)。
"""

from __future__ import annotations

import argparse
import datetime
import glob
import hashlib
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OUT = os.path.join(ROOT, "artifacts", "day_0903_clouds", "DELIVERABLES.json")

GROUPS = {
    "source_in_tree": [
        "src/rurix-render/src/world/sky.rs",
        "src/rurix-render/src/world/clouds.rs",
        "src/rurix-render/kernels/g40_volumetric_cloud.rx",
        "src/rurix-render/kernels/g40_cloud_encode.rx",
        "src/rurix-render/src/bin/g40_cloud_present.rs",
        "ci/g40_cloud_smoke.py",
    ],
    "campaign_in_tree": [
        "artifacts/day_0903_clouds/*.md",
        "artifacts/day_0903_clouds/tools/*.py",
        "artifacts/day_0903_clouds/evidence/*.json",
    ],
    "media_on_disk_not_tracked": [
        "artifacts/day_0903_clouds/previews/*.png",
    ],
}


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate-status", default="pending", choices=["pending", "pass", "fail"])
    a = ap.parse_args()

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
        "schema": "rurix.g40.deliverables.v1",
        "campaign": "day_0903_clouds",
        "gate": "g40.clouds.present",
        "gate_status": a.gate_status,
        "gate_facts": 7,
        "gate_status_note": (
            "门 ci/g40_cloud_smoke.py 为补齐件,尚未真跑(target-dir 与设备被他役占用);"
            "静态校验绿(ast.parse 无输出、未知门键 rc=2)。跑绿后重跑本脚本传 "
            "--gate-status pass"
        ),
        "upstream": {
            "project": "HanPi Volume Cloud (HPVolumeCloud)",
            "url": "https://github.com/AshenOneArt/HPVolumeCloud",
            "license": "MIT + 署名要求",
            "derivation": "其自身派生自 Unity HDRP 体积云",
            "mode": "clean-room(只取技术方案,不含参考仓库任何源码文本)",
        },
        "sky_calibration": {
            "source": "Poly Haven",
            "license": "CC0-1.0",
            "slug": "Pure Sky",
            "note": "只取太阳高度角/方位角/浊度的标定数值,零二进制资产入库",
        },
        "generated_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "committed": False,
        "commit_note": "未 commit,入库归 owner",
        "binary_policy": (
            "previews/*.png 留盘不入库(仓库全局 *.png 规则);"
            "本清单 sha256 为其入库登记面"
        ),
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
