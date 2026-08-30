# -*- coding: utf-8 -*-
"""PNG 侧 limit-15 重烘校验:slots00-13 与 v1 字节一致(确定性),slot14 被替换为平坦。"""
import hashlib
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
V1 = REPO / "artifacts/day_0829_realism/a4_normalmap/baked_normals"
RB = Path(__file__).resolve().parent / "rebake_png_limit15"

m1 = {e["slot"]: e for e in json.load(open(V1 / "manifest.json", encoding="utf-8"))["entries"]}
m2 = {e["slot"]: e for e in json.load(open(RB / "manifest.json", encoding="utf-8"))["entries"]}

fails = []
for s in range(15):
    e1, e2 = m1[s], m2[s]
    b2 = (RB / e2["file"]).read_bytes()
    h2 = "sha256:" + hashlib.sha256(b2).hexdigest()
    assert h2 == e2["png_sha256"], f"slot{s:02d} rebake manifest 自证失败"
    if s == 14:
        print("slot14 v1 png_sha :", e1["png_sha256"])
        print("slot14 rb png_sha :", h2, "(changed:", e1["png_sha256"] != h2, ")")
        print("slot14 rb mean/min/max:", e2["mean_xy"], e2["min_xy"], e2["max_xy"])
        print("slot14 sanitized  :", json.dumps(e2.get("sanitized"), ensure_ascii=False))
        assert e2.get("sanitized") is not None, "slot14 未登记 sanitized"
        assert e2["min_xy"] == [127, 127] and e2["max_xy"] == [127, 127], "slot14 未平坦化"
    else:
        if e1["png_sha256"] != h2:
            fails.append((s, e1["png_sha256"], h2))

if fails:
    for s, a, b in fails:
        print(f"slot{s:02d} PNG DIFF! v1={a} rb={b}")
    raise SystemExit(f"FAIL: {len(fails)} 张非 slot14 PNG 与 v1 不一致")
print("PASS: slots00-13 PNG 与 v1 逐字节一致(sha256),slot14 平坦替换生效")
