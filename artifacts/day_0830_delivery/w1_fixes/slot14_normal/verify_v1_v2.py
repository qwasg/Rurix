# -*- coding: utf-8 -*-
"""v1/v2 法线烘焙件对比校验(day_0830 W1 slot14 修复验收)。

断言面:
  1. 69 张(slot != 14)rgba8bin v1/v2 逐字节相等(sha256 双记账)。
  2. slot14: v1/v2 头 (w,h,mips) 一致;v2 全文件 == 独立构造的平坦参照
     (头 <III>[2048,2048,12] + 12 级 RGBA8 常值 (127,127,128,255),行主序紧凑),
     即内容平坦常值且 mip 布局/尺寸与 v1 完全同构。
  3. manifest_bin.json: 顶层字段(entries 外)逐一相等;69 行 entry 字面一致
     (dict 相等);slot14 行仅 output_sha256 / mip0_rgba8_sha256 变化 + 新增
     sanitized 登记(其余字段一致)。
  4. 目录文件集一致(70 bin + manifest,无多余件)。

产物: verify_output.json;退出码 0 = 全 PASS。
"""
import hashlib
import json
import struct
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
V1 = REPO / "artifacts/day_0829_realism/a4_normalmap/baked_normals_bin"
V2 = REPO / "artifacts/day_0829_realism/a4_normalmap/baked_normals_bin_v2"
OUT = Path(__file__).resolve().parent / "verify_output.json"

SLOT_FIX = 14
FLAT_RGBA = (127, 127, 128, 255)


def sha(b: bytes) -> str:
    return "sha256:" + hashlib.sha256(b).hexdigest()


def expected_flat_blob(w: int, h: int, mips: int) -> bytes:
    """独立构造平坦参照件(不经 pack 链,双源对证)。"""
    assert w == h and (w & (w - 1)) == 0 and mips == w.bit_length()
    blob = bytearray(struct.pack("<III", w, h, mips))
    lw = w
    for _ in range(mips):
        blob.extend(bytes(FLAT_RGBA) * (lw * lw))
        lw = max(1, lw // 2)
    return bytes(blob)


def main() -> int:
    report: dict = {"schema": "rurix.day0830.w1.slot14_normal.verify.v1", "checks": []}
    fails: list[str] = []

    def check(name: str, ok: bool, detail):
        report["checks"].append({"name": name, "pass": bool(ok), "detail": detail})
        if not ok:
            fails.append(name)
        print(("PASS" if ok else "FAIL"), name, "--", detail if isinstance(detail, str) else "")

    # 4. 文件集
    f1 = sorted(p.name for p in V1.iterdir())
    f2 = sorted(p.name for p in V2.iterdir())
    expect = sorted([f"slot{i:02d}.rgba8bin" for i in range(70)] + ["manifest_bin.json"])
    check("fileset_v1", f1 == expect, f"{len(f1)} 件")
    check("fileset_v2", f2 == expect, f"{len(f2)} 件")

    # 1. 69 张逐字节相等
    diff_slots: list[int] = []
    per_slot: list[dict] = []
    for i in range(70):
        n = f"slot{i:02d}.rgba8bin"
        b1 = (V1 / n).read_bytes()
        b2 = (V2 / n).read_bytes()
        s1, s2 = sha(b1), sha(b2)
        equal = b1 == b2
        if not equal:
            diff_slots.append(i)
        per_slot.append({"slot": i, "bytes_v1": len(b1), "bytes_v2": len(b2),
                         "sha_v1": s1, "sha_v2": s2, "byte_equal": equal})
    report["per_slot"] = per_slot
    check("bins_69_byte_equal", diff_slots == [SLOT_FIX],
          f"字节差异槽 = {diff_slots}(预期仅 [14])")

    # 2. slot14 结构与平坦内容
    b1 = (V1 / "slot14.rgba8bin").read_bytes()
    b2 = (V2 / "slot14.rgba8bin").read_bytes()
    h1 = struct.unpack_from("<III", b1, 0)
    h2 = struct.unpack_from("<III", b2, 0)
    check("slot14_header_same", h1 == h2 == (2048, 2048, 12), f"v1={h1} v2={h2}")
    check("slot14_size_same", len(b1) == len(b2) == 22369632, f"v1={len(b1)} v2={len(b2)}")
    exp = expected_flat_blob(*h2)
    check("slot14_v2_flat_exact", b2 == exp,
          f"v2 == 独立构造平坦参照(RGBA 常值 {FLAT_RGBA} 全 12 级);sha={sha(b2)}")
    report["slot14"] = {
        "file": "slot14.rgba8bin", "width": h2[0], "height": h2[1], "mips": h2[2],
        "bytes": len(b2), "sha_v1": sha(b1), "sha_v2": sha(b2),
        "flat_rgba": list(FLAT_RGBA),
    }

    # 3. manifest 对比
    m1 = json.load(open(V1 / "manifest_bin.json", encoding="utf-8"))
    m2 = json.load(open(V2 / "manifest_bin.json", encoding="utf-8"))
    top_diff = [k for k in sorted(set(m1) | set(m2))
                if k != "entries" and m1.get(k) != m2.get(k)]
    check("manifest_toplevel_same", top_diff == [], f"顶层差异键 = {top_diff}")
    e1 = {e["slot"]: e for e in m1["entries"]}
    e2 = {e["slot"]: e for e in m2["entries"]}
    check("manifest_slot_set_same", sorted(e1) == sorted(e2) == list(range(70)), "70 行")
    row_diff = [s for s in range(70) if e1[s] != e2[s]]
    check("manifest_rows_69_identical", row_diff == [SLOT_FIX],
          f"行差异槽 = {row_diff}(预期仅 [14])")
    r1, r2 = e1[SLOT_FIX], e2[SLOT_FIX]
    changed = sorted(k for k in set(r1) | set(r2) if r1.get(k) != r2.get(k))
    check("manifest_slot14_fields", changed == ["mip0_rgba8_sha256", "output_sha256", "sanitized"],
          f"slot14 行变化字段 = {changed}")
    check("manifest_slot14_sha_match_file", r2.get("output_sha256") == sha(b2),
          "manifest output_sha256 == 实文件 sha")
    report["manifest_slot14_diff"] = {
        "changed_fields": changed,
        "output_sha256": {"v1": r1.get("output_sha256"), "v2": r2.get("output_sha256")},
        "mip0_rgba8_sha256": {"v1": r1.get("mip0_rgba8_sha256"), "v2": r2.get("mip0_rgba8_sha256")},
        "sanitized_v2": r2.get("sanitized"),
    }

    report["result"] = "PASS" if not fails else f"FAIL: {fails}"
    OUT.write_text(json.dumps(report, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"\n{report['result']} → {OUT}")
    return 0 if not fails else 1


if __name__ == "__main__":
    sys.exit(main())
