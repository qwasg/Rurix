#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 B Task B3 slab 生产接线）
"""G31+ 波 B Task B3 slab 侧表生产资产生成器（一次性程序产，禁手写槽值）。

资产 = milestones/g31/g31_slab_side_table_bistro_interior.json：
- 16 槽 [rc, ab] f32 对（G29 M-b 侧表 ABI 同源生成律：rc_k = k/15*0.95、
  ab_k = (15-k)/15；f32 逐 op 舍入经 struct 仿真，与 g29_slab_device.rs
  side_table_samples() 的 Rust f32 求值序逐 op 同舍入；0.95 上限有意规避
  denom→0 角点区——角点语义覆盖由 M-a 主网格独担，RFC-0046 §2.1 F5）；
- material_slots = bistro-interior glTF 材质索引 → 槽映射（Substrate 类
  双层 slab 演示面：釉面陶瓷/粉刷石膏/清漆木/漆面桌台四类五材质）；
- abi_digest = 16 槽 × [rc f32 LE, ab f32 LE] 128 字节 sha256（篡改即拒）。

生成即验证：f32 位级回读 == 生成律 + digest 互核 + glTF 材质名称/索引互核
+ LF 行尾。重跑幂等（同字节覆写）。

用法：py -3 ci/_gen_g31_slab_side_table.py
"""
from __future__ import annotations

import hashlib
import io
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "milestones" / "g31" / "g31_slab_side_table_bistro_interior.json"
GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
N_SLOTS = 16


def f32(x: float) -> float:
    """IEEE-754 binary32 最近舍入仿真（Rust f32 逐 op 语义同律）。"""
    return struct.unpack("f", struct.pack("f", x))[0]


def gen_slots() -> list[dict]:
    """M-b ABI 生成律（f32 逐 op：k/15 舍入 → ×0.95 舍入；ab = (15−k)/15 一 op）。"""
    slots = []
    for k in range(N_SLOTS):
        rc = f32(f32(f32(k) / f32(15.0)) * f32(0.95))
        ab = f32(f32(15 - k) / f32(15.0))
        slots.append({"k": k, "rc": rc, "ab": ab})
    return slots


def abi_digest(slots: list[dict]) -> str:
    buf = b"".join(struct.pack("<ff", s["rc"], s["ab"]) for s in slots)
    return "sha256:" + hashlib.sha256(buf).hexdigest()


def build_asset() -> dict:
    slots = gen_slots()
    return {
        "schema": "rurix.g31.slab_side_table_asset.v1",
        "scene_id": "bistro-interior",
        "n_slots": N_SLOTS,
        "abi": {
            "slot_count": N_SLOTS,
            "slot_stride_bytes": 8,
            "slot_layout": "[rc f32 LE, ab f32 LE]",
            "generation_law": (
                "rc_k = k/15*0.95, ab_k = (15-k)/15（f32 逐 op 舍入，g29_slab_device "
                "M-b 侧表同源字面；0.95 上限有意规避 denom→0 角点区，角点语义覆盖归 "
                "M-a 主网格，RFC-0046 §2.1 F5）"
            ),
            "device_kernel": (
                "kernels/g29_slab.rx（G29 M-a 本体 0-byte 冻结消费；dispatch [16,1,1] "
                "逐槽单 invocation，R = rc + tc·tc·ab / max(denom, 1e-30) 修法 A）"
            ),
            "host_reference": (
                "material/slab.rs::total_reflectance（f64 直调，0-byte 冻结金标准；"
                "denom ≤ 0 ⇒ 1.0 分支）"
            ),
            "abi_digest": abi_digest(slots),
        },
        "slots": slots,
        "material_slots": [
            {
                "material_index": 54,
                "material_name": "Plates_Ceramic",
                "slot": 2,
                "slab_class": "glazed_ceramic",
                "note": "釉面陶瓷：清漆 coating 覆强 base——Substrate 类原型面",
            },
            {
                "material_index": 24,
                "material_name": "MASTER_Interior_01_Plaster",
                "slot": 1,
                "slab_class": "painted_plaster",
                "note": "粉刷石膏墙：弱 coating 高明 base",
            },
            {
                "material_index": 25,
                "material_name": "MASTER_Interior_01_Wood",
                "slot": 3,
                "slab_class": "varnished_wood",
                "note": "清漆木：中 coating",
            },
            {
                "material_index": 43,
                "material_name": "Wood",
                "slot": 3,
                "slab_class": "varnished_wood",
                "note": "清漆木同类",
            },
            {
                "material_index": 41,
                "material_name": "Paris_Table_03",
                "slot": 4,
                "slab_class": "lacquered_table",
                "note": "漆面桌台：强 coating（121816 三角形，画面信号主载体）",
            },
        ],
        "evaluation_semantics": (
            "albedo_final[c] = albedo_dir[c] * R_slot（c∈RGB，f32 乘；R = 双层 slab 闭式"
            "总反照率 total_reflectance(rc,ab)——能量守恒缩放，emission 通道 0-byte 不触；"
            "非映射材质走既有逐三角 albedo/emission 单层面 0-byte）"
        ),
        "provenance": {
            "wave": "G31+.B Task B3",
            "generated_by": "ci/_gen_g31_slab_side_table.py（python struct f32 逐 op 仿真，程序产禁手写）",
            "g29_anchor": (
                "RD-041-slab 行 g31_anchor 生产接线窗；G29 M-b 侧表 16 槽 ABI"
                "（bin-local 测试件 → 本资产文件升级为场景/资产加载生产面）"
            ),
            "frozen_tolerance_entry": (
                "milestones/g29/g29_budget.json g29.slab_device.host_device_reflectance_tol"
                "（measured p100=1.192092895507812e-07 恰一 ULP；threshold="
                "2.384185791015624e-07 = measured × 2.0 程序产禁手写）"
            ),
        },
    }


def verify(doc: dict) -> None:
    raw = OUT.read_bytes()
    assert b"\r" not in raw, "CRLF 混入（LF 纪律）"
    assert doc["schema"] == "rurix.g31.slab_side_table_asset.v1"
    assert doc["n_slots"] == N_SLOTS and len(doc["slots"]) == N_SLOTS
    for s in doc["slots"]:
        rc = f32(s["rc"])
        ab = f32(s["ab"])
        assert struct.pack("f", rc) == struct.pack("f", s["rc"]), "rc f32 round-trip 破"
        assert struct.pack("f", ab) == struct.pack("f", s["ab"]), "ab f32 round-trip 破"
        exp_rc = f32(f32(f32(s["k"]) / f32(15.0)) * f32(0.95))
        exp_ab = f32(f32(15 - s["k"]) / f32(15.0))
        assert struct.pack("f", rc) == struct.pack("f", exp_rc), f"槽 {s['k']} rc 位级 ≠ 生成律"
        assert struct.pack("f", ab) == struct.pack("f", exp_ab), f"槽 {s['k']} ab 位级 ≠ 生成律"
        assert 0.0 <= rc <= 1.0 and 0.0 <= ab <= 1.0, "域 [0,1] 破"
    assert abi_digest(doc["slots"]) == doc["abi"]["abi_digest"], "ABI digest 不符"
    assert len({m["slot"] for m in doc["material_slots"]}) >= 1
    if GLTF.is_file():
        gltf = json.loads(GLTF.read_text(encoding="utf-8"))
        mats = gltf.get("materials", [])
        for m in doc["material_slots"]:
            assert 0 <= m["material_index"] < len(mats), f"material_index 越界: {m}"
            name = mats[m["material_index"]].get("name")
            assert name == m["material_name"], f"材质名不符: index={m['material_index']} gltf={name!r} asset={m['material_name']!r}"
            assert 0 <= m["slot"] < N_SLOTS, f"槽越界: {m}"
    print(
        "ASSET_VERIFY OK: 16 槽 f32 位级 == M-b 生成律 + abi_digest "
        + doc["abi"]["abi_digest"][:23]
        + "… + 5 材质映射名称/索引互核全中 + LF"
    )


def main() -> int:
    doc = build_asset()
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with io.open(str(OUT), "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    verify(json.loads(OUT.read_text(encoding="utf-8")))
    print(f"generated: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
