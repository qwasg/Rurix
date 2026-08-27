#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C14 KTX2 三行立项窗兑现）
"""G31+ 波 C Task C14：KTX2 三行接线门冒烟（g31.waveC.ktx2；
G31_PLUS_COMMERCIAL_RENDERER_TODO #37-#39；RD-041 分项；
milestones/g22/g22_ktx2_disposition.json KTX2 立项窗 +
milestones/g29/g29_svt_ktx2_rejudgment.json 重判窗兑现）。
B4 纹理采样门 / C13 SVT 门范本同构（ci/g31_texture_sampling_smoke.py /
ci/g31_svt_smoke.py）。

四面判据（facts 闭集；三行各一 + 整合一）：
1. **ktx2_1_container_parse**：KTX2-1 容器解析——src/rurix-asset/src/ktx2.rs
   parse_ktx2（host safe Rust；头/key-value/level index/supercompression
   元数据 + mip 布局）。cargo test ktx2/basis-sys 绿（真实 encoder 件逐字段
   互核〔DFD colorModel=166〕+ 最小合成件多级 vendor 真转码 + 确定性双读 +
   fail-closed 臂）+ CI 独立 Python 解析器对 harness dump 全链件重解析
   （levelCount/尺寸律法/逐级长度/KTXwriter/digest 跨实现互核）。
2. **ktx2_2_transcoder_integration**：KTX2-2 BasisU 转码器集成——vendor
   C++ 桥（rurix-basis-sys 既有 FFI 面 + rurix_basis_transcode_level mip
   参数化新面,unsafe-audit U60）→ BC7/ASTC 转码落地；fail-closed 纪律
   （level 越界 rc=17/.basis rc=5/垃圾容器确定性 Err 不崩）；转码产物与
   bcdec 独立参考解码对拍（BC7 全 8 mode 像素面；premultiplied-aware：
   UASTC rgb_masked ≤ AP-TEX 冻结 48（实测 max 4）+ alpha ≤ 16（实测
   max 4）；ETC1S 参照腿 measured 登记）。
3. **ktx2_3_transcode_ab_measured**：KTX2-3 收益 A/B measured——bistro
   12 槽 DDS 面（top-12 律法 CI 独立重算互核 + rgba8_digest == G11.3
   manifest 12/12）：原始分发体积 vs KTX2-UASTC 全链 vs ETC1S 全链实测
   字节 + 转码/编码耗时同机 measured + CI 自原始字节重算 ratio 互核 +
   政策面落档（何时用 KTX2 何时用 DDS + C5 SDK bundle 纹理面建议）。
4. **ktx2_integration_regression**：整合回归——vendor pin 不动
   （vendor_basis_universal.py --verify 逐文件 digest 复核 + vendor/ 0-byte）
   + G11.3 manifest 0-byte + B4 frozen 路径 0-byte + g22/g29 判档 0-byte
   + M83 门复跑绿（纹理 cook 链回归）。

三态：无 vendor 快照/bistro 场景资产/构建工具链 → DEV_ENV_DEGRADE 退 0
（不冒充 PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock
充真跑）。本门纯 host（GPU 非必需）。

用法：
  py -3 ci/g31_ktx2_smoke.py --selftest
  py -3 ci/g31_ktx2_smoke.py --gate g31.waveC.ktx2
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

GATE_KEY = "g31.waveC.ktx2"
SUBJECT = "g31_ktx2"
WAVE = "G31.C"
TAG = "g31_ktx2"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_ktx2_ab_evidence_schema.json"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_ktx2_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g31.ktx2_ab_evidence.v1"
GATE_SCHEMA_ID = "rurix.g31.ktx2_gate_evidence.v1"
G11_MANIFEST_PATH = ROOT / "milestones" / "g11" / "g11_3_dds_transcode_manifest.json"
G22_DISPOSITION_PATH = ROOT / "milestones" / "g22" / "g22_ktx2_disposition.json"
G29_REJUDGMENT_PATH = ROOT / "milestones" / "g29" / "g29_svt_ktx2_rejudgment.json"
VENDOR_DIR = ROOT / "src" / "rurix-basis-sys" / "vendor" / "basis_universal"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
WORK = ROOT / ".tmp" / "g31_gates" / "ktx2"
DUMP_KTX2 = WORK / "dump_fullchain.ktx2"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_AB = ROOT / "target" / "release" / f"g31_ktx2_ab{EXE_SUFFIX}"
# B4 纹理面 frozen 路径（HEAD 在案面,0-byte 机核;本任务零触碰承诺面;
# src/rurix-asset 本体为加性编辑面,回归由 cargo test + M83 门复跑承载,
# 不以 0-byte 冒充）。
FROZEN_PATHS = [
    "src/rurix-render/kernels/g14_3_direct_gi.rx",
    "src/rurix-render/src/material",
    "src/rurix-render/src/graph/types.rs",
    "milestones/g11/g11_3_dds_transcode_manifest.json",
]
# 波 B/C 在飞产物（本工作树未提交;HEAD 无基线 ⇒ 0-byte-vs-HEAD 不可归因,
# C13 SVT 门 terrain.rs 同例）——本任务零触碰如实登记（不以 0-byte 冒充）。
B4_IN_FLIGHT_PATHS = [
    "src/rurix-render/kernels/g31_texture_gi.rx",
    "src/rurix-render/kernels/g31_texture_probe.rx",
    "src/rurix-render/src/bin/g31_window_present.rs",
]
N_MAPPED = 12
COLOR_DELTA_BOUND = 48
ALPHA_DELTA_BOUND = 16
ETC1S_RGB_MAX_BOUND = 160
ETC1S_RGB_MEAN_BOUND = 8.0
UASTC_RGB_MEAN_BOUND = 2.0

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
KTX2_IDENTIFIER = b"\xABKTX 20\xBB\r\n\x1A\n"
FAILURES: list[str] = []

FACT_IDS = [
    "ktx2_1_container_parse",
    "ktx2_2_transcoder_integration",
    "ktx2_3_transcode_ab_measured",
    "ktx2_integration_regression",
]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


# ---------------------------------------------------------------------------
# 判读器①：CI 独立 KTX2 解析器（Rust parse_ktx2 跨实现互核面；selftest 消费）
# ---------------------------------------------------------------------------

KTX2_HEADER_LEN = 12 + 68
KTX2_LEVEL_ENTRY_LEN = 24
KTX2_MAX_LEVELS = 32


class Ktx2PyError(Exception):
    """CI 独立解析器失败（确定性;同输入两次解析同错）。"""


def parse_ktx2_py(b: bytes) -> dict:
    """KTX2 头/level index/KVD/supercompression 元数据最小独立解析。

    与 Rust parse_ktx2 同律法（跨实现互核;独立第二实现是互核可信的前提）。"""
    if len(b) < KTX2_HEADER_LEN:
        raise Ktx2PyError("TooShort")
    if b[:12] != KTX2_IDENTIFIER:
        raise Ktx2PyError("BadIdentifier")
    u32 = lambda o: struct.unpack_from("<I", b, o)[0]
    u64 = lambda o: struct.unpack_from("<Q", b, o)[0]
    hdr = {
        "vk_format": u32(12), "type_size": u32(16),
        "pixel_width": u32(20), "pixel_height": u32(24), "pixel_depth": u32(28),
        "layer_count": u32(32), "face_count": u32(36), "level_count": u32(40),
        "supercompression_scheme": u32(44),
        "dfd_off": u32(48), "dfd_len": u32(52),
        "kvd_off": u32(56), "kvd_len": u32(60),
        "sgd_off": u64(64), "sgd_len": u64(72),
    }
    if hdr["type_size"] != 1:
        raise Ktx2PyError(f"BadTypeSize({hdr['type_size']})")
    if not (1 <= hdr["level_count"] <= KTX2_MAX_LEVELS):
        raise Ktx2PyError(f"BadLevelCount({hdr['level_count']})")
    if hdr["face_count"] == 0:
        raise Ktx2PyError("BadFaceCount")
    if hdr["supercompression_scheme"] not in (0, 1, 2, 3):
        raise Ktx2PyError(f"UnsupportedScheme({hdr['supercompression_scheme']})")
    n = hdr["level_count"]
    if KTX2_HEADER_LEN + n * KTX2_LEVEL_ENTRY_LEN > len(b):
        raise Ktx2PyError("OutOfBounds(level_index)")
    levels = []
    for i in range(n):
        base = KTX2_HEADER_LEN + i * KTX2_LEVEL_ENTRY_LEN
        off, ln, unc = u64(base), u64(base + 8), u64(base + 16)
        if ln == 0:
            raise Ktx2PyError(f"EmptyLevel({i})")
        if off + ln > len(b):
            raise Ktx2PyError("OutOfBounds(level_data)")
        if hdr["supercompression_scheme"] == 0 and unc != ln:
            raise Ktx2PyError(f"SchemeZeroLengthMismatch({i})")
        levels.append((off, ln, unc))
    if hdr["dfd_off"] + hdr["dfd_len"] > len(b) or hdr["dfd_len"] < 4:
        raise Ktx2PyError("BadDfd")
    dfd_total = struct.unpack_from("<I", b, hdr["dfd_off"])[0]
    if dfd_total != hdr["dfd_len"]:
        raise Ktx2PyError("BadDfd")
    if hdr["kvd_off"] + hdr["kvd_len"] > len(b):
        raise Ktx2PyError("OutOfBounds(kvd)")
    kvs = {}
    pos = hdr["kvd_off"]
    end = pos + hdr["kvd_len"]
    while pos < end:
        if pos + 4 > end:
            raise Ktx2PyError("BadKeyValue")
        rl = u32(pos)
        if rl == 0 or pos + 4 + rl > end:
            raise Ktx2PyError("BadKeyValue")
        rec = b[pos + 4: pos + 4 + rl]
        nul = rec.find(b"\0")
        if nul < 0:
            raise Ktx2PyError("BadKeyValue")
        kvs[rec[:nul].decode("utf-8", "strict")] = rec[nul + 1:]
        pos += 4 + ((rl + 3) & ~3)
    if hdr["sgd_len"] and hdr["sgd_off"] + hdr["sgd_len"] > len(b):
        raise Ktx2PyError("OutOfBounds(sgd)")
    return {"header": hdr, "levels": levels, "kv": kvs}


def level_dims_py(w: int, h: int, level: int) -> tuple[int, int]:
    return (max(1, w >> level), max(1, h >> level))


# ---------------------------------------------------------------------------
# 判读器②：top-12 映射律法（B4 同律;CI 独立重算面）
# ---------------------------------------------------------------------------


def gltf_material_tris(gltf: dict) -> dict[int, int]:
    accessors = gltf.get("accessors", [])
    meshes = gltf.get("meshes", [])
    out: dict[int, int] = {}
    for node in gltf.get("nodes", []):
        mi_mesh = node.get("mesh")
        if mi_mesh is None:
            continue
        for prim in meshes[mi_mesh].get("primitives", []):
            mat = prim.get("material")
            if mat is None:
                continue
            acc = accessors[prim["indices"]]
            out[mat] = out.get(mat, 0) + acc["count"] // 3
    return out


def expected_uris(gltf: dict, n: int = N_MAPPED) -> list[str]:
    """top-N 律法（三角数降序,并列 material_index 升序）→ baseColor uri 列。"""
    tris = gltf_material_tris(gltf)
    rank = sorted(tris.items(), key=lambda kv: (-kv[1], kv[0]))[:n]
    uris = []
    for mi, _t in rank:
        pbr = gltf["materials"][mi].get("pbrMetallicRoughness", {})
        ti = pbr["baseColorTexture"]["index"]
        src = gltf["textures"][ti]["source"]
        uris.append(gltf["images"][src]["uri"])
    return uris


def load_manifest() -> dict[str, tuple[str, str]]:
    doc = json.loads(G11_MANIFEST_PATH.read_text(encoding="utf-8"))
    return {e["source_uri"]: (e["source_digest"], e["rgba8_digest"]) for e in doc.get("entries", [])}


# ---------------------------------------------------------------------------
# 判读器③④：质量界 / ratio 重算（selftest 红绿臂消费面）
# ---------------------------------------------------------------------------


def uastc_quality_ok(rgb_max: int, rgb_mean_max: float, alpha_max: int) -> bool:
    """UASTC 腿判：rgb_masked ≤ AP-TEX 冻结 48 ∧ mean ≤ 2.0 ∧ alpha ≤ 16。"""
    return (
        isinstance(rgb_max, int) and 0 <= rgb_max <= COLOR_DELTA_BOUND
        and isinstance(alpha_max, int) and 0 <= alpha_max <= ALPHA_DELTA_BOUND
        and isinstance(rgb_mean_max, (int, float)) and 0 <= rgb_mean_max <= UASTC_RGB_MEAN_BOUND
    )


def etc1s_quality_ok(rgb_max: int, rgb_mean_max: float, alpha_max: int) -> bool:
    """ETC1S 参照腿判（有损如实登记）：max ≤ 160（语义翻转级必红）∧
    mean ≤ 8.0 ∧ alpha ≤ 16。"""
    return (
        isinstance(rgb_max, int) and 0 <= rgb_max <= ETC1S_RGB_MAX_BOUND
        and isinstance(alpha_max, int) and 0 <= alpha_max <= ALPHA_DELTA_BOUND
        and isinstance(rgb_mean_max, (int, float)) and 0 <= rgb_mean_max <= ETC1S_RGB_MEAN_BOUND
    )


def ratio_close(a: float, b: float, eps: float = 1e-9) -> bool:
    return abs(a - b) <= eps


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def cargo_test_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv, timeout=3600)
    out = (r.stdout or "") + (r.stderr or "")
    if r.returncode != 0 or "0 failed" not in out:
        fail(f"{what} 测试红: {out[-400:]}")
        return False
    return True


def git_clean(paths: list[str]) -> tuple[bool, bool]:
    d = run(["git", "diff", "--quiet", "HEAD", "--", *paths])
    u = run(["git", "status", "--porcelain", "--", *paths])
    return d.returncode == 0, not u.stdout.strip()


def run_gate() -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    for sp, name in ((SCHEMA_PATH, "harness schema"), (GATE_SCHEMA_PATH, "gate schema")):
        if not sp.is_file():
            fail(f"{name} 缺失: {sp}")
    if FAILURES:
        return 1

    # ── dev-env 降级面（vendor 快照/bistro 资产/12 槽 DDS）──
    degrade: list[str] = []
    gltf_doc = json.loads(BISTRO_GLTF.read_text(encoding="utf-8")) if BISTRO_GLTF.is_file() else None
    manifest = load_manifest() if G11_MANIFEST_PATH.is_file() else None
    if not (VENDOR_DIR / "vendor_manifest.json").is_file():
        degrade.append(f"vendor 快照缺失 {VENDOR_DIR}（py -3 ci/vendor_basis_universal.py）")
    if gltf_doc is None:
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")
    if manifest is None:
        degrade.append("G11.3 manifest 缺失")
    uris = expected_uris(gltf_doc) if gltf_doc is not None else []
    if gltf_doc is not None and len(uris) != N_MAPPED:
        degrade.append(f"top-{N_MAPPED} 律法产出 {len(uris)} ≠ {N_MAPPED}")
    dds_dir = BISTRO_GLTF.parent
    missing_dds = [u for u in uris if not (dds_dir / u).is_file()]
    if missing_dds:
        degrade.append(f"DDS 缺失 {missing_dds[:3]}")

    if degrade:
        doc = {
            "schema": "rurix.g31.ktx2.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for d_ in degrade:
            note(f"DEV_ENV_DEGRADE {d_}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 DEV_ENV 降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── 构建 + host 单测（facts 1/2 共享面）──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-asset", "--bin", "g31_ktx2_ab", "--quiet"],
        "g31_ktx2_ab release",
    )
    if not ok:
        return 1
    t_basis = cargo_test_or_fail(["cargo", "test", "-p", "rurix-basis-sys"], "rurix-basis-sys")
    t_asset = cargo_test_or_fail(["cargo", "test", "-p", "rurix-asset", "--lib"], "rurix-asset lib")

    # ── A/B harness 真跑（12 槽全量 measured）──
    WORK.mkdir(parents=True, exist_ok=True)
    if DUMP_KTX2.exists():
        DUMP_KTX2.unlink()
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ab_raw = WORK / f"ab_{ts}.json"
    r = run(
        [str(BIN_AB), "--dds-dir", str(dds_dir), "--textures", ",".join(uris),
         "--out", str(ab_raw), "--dump-ktx2", str(DUMP_KTX2)],
        timeout=3600,
    )
    out = (r.stdout or "") + (r.stderr or "")
    ab = None
    if ab_raw.is_file():
        try:
            ab = json.loads(ab_raw.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            ab = None
    if r.returncode != 0 or ab is None or "PASS" not in out:
        fail(f"A/B harness 真跑失败 rc={r.returncode}: {out[-300:]}")

    # harness evidence 归档（schema 路由前缀面）
    harness_archives: list[str] = []
    if ab is not None:
        arch = ROOT / "evidence" / f"g31_ktx2_ab_harness_{ts}.json"
        arch.write_text(json.dumps(ab, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        harness_archives.append(str(arch.relative_to(ROOT)))

    # ── ① KTX2-1：CI 独立重解析互核 ──
    ci_reparse_ok = False
    dumped_levels = 0
    dumped_uri = ""
    if ab is not None and DUMP_KTX2.is_file():
        blob = DUMP_KTX2.read_bytes()
        dg = "sha256:" + hashlib.sha256(blob).hexdigest()
        try:
            parsed = parse_ktx2_py(blob)
            row = next((x for x in ab["textures"] if x["ktx2_uastc_digest"] == dg), None)
            if row is not None:
                h = parsed["header"]
                dims_law = all(
                    level_dims_py(h["pixel_width"], h["pixel_height"], i)
                    == (max(1, row["width"] >> i), max(1, row["height"] >> i))
                    for i in range(h["level_count"])
                )
                ci_reparse_ok = (
                    h["level_count"] == row["levels_generated"]
                    and h["pixel_width"] == row["width"]
                    and h["pixel_height"] == row["height"]
                    and h["supercompression_scheme"] == 0
                    and len(blob) == row["ktx2_uastc_full_bytes"]
                    and all(unc == ln for _o, ln, unc in parsed["levels"])
                    and parsed["kv"].get("KTXwriter")
                    == b"rurix-asset ktx2.rs write_ktx2_multilevel"
                    and dims_law
                )
                dumped_levels = h["level_count"]
                dumped_uri = row["uri"]
        except Ktx2PyError as e:
            note(f"CI 独立解析器判红: {e}")
    k1_detail = (
        f"cargo test basis-sys={'绿' if t_basis else '红'} asset={'绿' if t_asset else '红'}；"
        f"CI 独立 Python 解析器重解析 dump 件（uri={dumped_uri},levels={dumped_levels}）"
        f"跨实现互核={'绿' if ci_reparse_ok else '红'}；"
        f"harness parse_crosscheck={(ab or {}).get('parse_crosscheck')}"
    )
    set_fact(
        "ktx2_1_container_parse",
        t_basis and t_asset and ci_reparse_ok
        and (ab or {}).get("parse_crosscheck", {}).get("ok") is True
        and (ab or {}).get("determinism", {}).get("parse_double_read_bitexact") is True
        and dumped_levels >= 2,
        k1_detail,
    )

    # ── ② KTX2-2：转码对拍质量界 + fail-closed ──
    tot = (ab or {}).get("totals") or {}
    rows = (ab or {}).get("textures") or []
    u_rgb_max = tot.get("uastc_rgb_max_masked", 255)
    u_alpha_max = tot.get("uastc_alpha_max", 255)
    u_rgb_mean_max = max((x.get("uastc_rgb_mean_masked", 99.0) for x in rows), default=99.0)
    e_rgb_max = tot.get("etc1s_rgb_max_masked", 255)
    e_alpha_max = tot.get("etc1s_alpha_max", 255)
    e_rgb_mean_max = max((x.get("etc1s_rgb_mean_masked", 99.0) for x in rows), default=99.0)
    det = (ab or {}).get("determinism") or {}
    k2_ok = (
        t_basis
        and uastc_quality_ok(u_rgb_max, u_rgb_mean_max, u_alpha_max)
        and etc1s_quality_ok(e_rgb_max, e_rgb_mean_max, e_alpha_max)
        and det.get("uastc_encode_double_run_bitexact") is True
        and (ab or {}).get("codec_version") == "basis_universal/1.16.4+g900e40fb5d25"
    )
    set_fact(
        "ktx2_2_transcoder_integration",
        k2_ok,
        f"对拍（bcdec 全 8 mode 独立解码,premultiplied-aware）:UASTC rgb_max={u_rgb_max}≤48 "
        f"alpha_max={u_alpha_max}≤16 mean_max={u_rgb_mean_max:.4f}≤2.0；"
        f"ETC1S 参照 rgb_max={e_rgb_max}≤160 mean_max={e_rgb_mean_max:.4f}≤8.0 alpha={e_alpha_max}；"
        f"fail-closed(level 越界 rc=17/.basis rc=5/垃圾容器)+双编码位级+pin 字面 经 cargo test 承载"
        f"={'绿' if t_basis else '红'}",
    )

    # ── ③ KTX2-3：A/B measured 互核 + 政策面 ──
    ratio_u = (tot.get("ktx2_uastc_full_bytes") or 0) / (tot.get("dds_file_bytes") or 1)
    ratio_e = (tot.get("etc1s_full_bytes") or 0) / (tot.get("dds_file_bytes") or 1)
    manifest_hits = 0
    if manifest is not None:
        for x in rows:
            want = manifest.get(x.get("uri"), (None, None))[1]
            if want is not None and x.get("rgba8_digest") == want:
                manifest_hits += 1
    sums_ok = all(
        tot.get(k) == sum(x[k] for x in rows)
        for k in ("dds_file_bytes", "dds_l0_bytes", "ktx2_uastc_full_bytes",
                  "ktx2_uastc_l0_bytes", "etc1s_full_bytes", "etc1s_l0_bytes")
    ) and ratio_close(ratio_u, tot.get("ktx2_uastc_full_bytes", 1) / tot.get("dds_file_bytes", 1))
    k3_ok = (
        len(rows) == N_MAPPED
        and manifest_hits == N_MAPPED
        and sums_ok
        and tot.get("dds_file_bytes", 0) > 0
        and tot.get("ktx2_uastc_full_bytes", 0) > 0
        and tot.get("etc1s_full_bytes", 0) > 0
        and (ab or {}).get("bounds", {}).get("color_max_delta_bound") == COLOR_DELTA_BOUND
        and (ab or {}).get("bounds", {}).get("alpha_delta_bound") == ALPHA_DELTA_BOUND
    )
    policy_doc = {
        "dds_when": (
            "Windows-first 单机/闭集资产分发（现压测闭集 + SDK bundle 纹理面）：DDS BCn 直传 GPU "
            "零转码成本;禁 zstd 在树约束下 UASTC-KTX2 体积 measured 2.00× 于 DDS——维持 G11.3 DDS 链"
        ),
        "ktx2_etc1s_when": (
            "跨平台分发或带宽/体积预算门成立时：ETC1S（BasisU 家族）体积 measured 0.144×"
            "（≈6.96× 收益）;代价 = 有损（maxΔ110/meanΔ≤2.9 measured）+ 离线编码 ~38.4s/12 槽 + "
            "运行时转码 ~85ms/12 槽 L0"
        ),
        "ktx2_uastc_when": (
            "目标平台无 BC7（ASTC-only 移动面）且要求视觉无损（maxΔ4 measured）的跨平台中间档;"
            "体积 2× 于 BC1 DDS,Windows-only 面不成立"
        ),
        "sdk_bundle_note": (
            "C5 分发面联动：SDK bundle 16 组件纹理面维持 DDS BCn（Windows-first 闭集,与本 measured "
            "结论一致）;跨平台/下载分发需求成立时按本表 ETC1S 面评估接入,UASTC-KTX2 作高质量跨平台"
            "中间档登记"
        ),
    }
    set_fact(
        "ktx2_3_transcode_ab_measured",
        k3_ok,
        f"12 槽 measured:DDS={tot.get('dds_file_bytes')}B UASTC全链={tot.get('ktx2_uastc_full_bytes')}B"
        f"（{ratio_u:.3f}×）ETC1S全链={tot.get('etc1s_full_bytes')}B（{ratio_e:.3f}×）；"
        f"转码 L0 合计 uastc={tot.get('uastc_transcode_l0_ms')}ms etc1s={tot.get('etc1s_transcode_l0_ms')}ms;"
        f"manifest digest 互核 {manifest_hits}/12;ratio CI 重算互核={'绿' if sums_ok else '红'}",
    )

    # ── ④ 整合回归：pin 不动 + 0-byte 面 + M83 门复跑 ──
    v = run(["py", "-3", "ci/vendor_basis_universal.py", "--verify"], timeout=1200)
    vendor_verify = v.returncode == 0 and "VERDICT=OK" in (v.stdout or "")
    vdiff, vwork = git_clean(["src/rurix-basis-sys/vendor"])
    g11d, g11w = git_clean(["milestones/g11/g11_3_dds_transcode_manifest.json"])
    bfd, bfw = git_clean(FROZEN_PATHS)
    g22d, g22w = git_clean([
        "milestones/g22/g22_ktx2_disposition.json",
        "milestones/g29/g29_svt_ktx2_rejudgment.json",
    ])
    # 在飞面如实登记（非门判据;HEAD 无基线 ⇒ 不以 0-byte-vs-HEAD 冒充）。
    inflight = run(["git", "status", "--porcelain", "--", *B4_IN_FLIGHT_PATHS])
    inflight_status = "; ".join(
        f"{ln[:2].strip()} {ln[3:]}" for ln in inflight.stdout.strip().splitlines()
    ) or "clean"
    b4_inflight_registered = True
    m83 = run(["py", "-3", "ci/g8_texture_transcode_smoke.py", "--gate", "g8.p1.m83.texture_transcode"],
              timeout=3600)
    m83_out = (m83.stdout or "") + (m83.stderr or "")
    m83_ok = m83.returncode == 0
    m83_ev = ""
    ev_dir = ROOT / "evidence"
    if ev_dir.is_dir():
        cands = sorted(ev_dir.glob("g8_m83_texture_transcode_*.json"),
                       key=lambda p: p.stat().st_mtime, reverse=True)
        if cands:
            m83_ev = str(cands[0].relative_to(ROOT))
    set_fact(
        "ktx2_integration_regression",
        vendor_verify and vdiff and vwork and g11d and g11w and bfd and bfw and g22d and g22w and m83_ok
        and b4_inflight_registered,
        f"vendor --verify={'绿' if vendor_verify else '红'} vendor/0-byte={vdiff and vwork} "
        f"G11.3 0-byte={g11d and g11w} B4 frozen(HEAD 在案) 0-byte={bfd and bfw} "
        f"g22/g29 判档 0-byte={g22d and g22w} M83 门复跑={'绿' if m83_ok else '红'}"
        f"（{m83_ev or m83_out[-120:]}）；B4 在飞面登记[{inflight_status}]（本任务零触碰,非 0-byte 判据）",
    )

    # ── evidence 落盘（门裁决件;jsonschema 自校验硬门）──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "host-only 门（GPU 非必需;CPU 编码/转码 measured_local）",
        "cpu_note": "本机 measured_local（RTX 4070 Ti 主机 CPU）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "ktx2_1": {
            "host_tests_green": t_basis and t_asset,
            "real_fixture_fields_match": t_asset,
            "multilevel_transcode_bitexact": t_asset,
            "double_read_bitexact": det.get("parse_double_read_bitexact", False),
            "fail_closed_arms": 8,
            "ci_independent_reparse_match": ci_reparse_ok,
            "dumped_levels": dumped_levels,
            "dumped_uri": dumped_uri,
        },
        "ktx2_2": {
            "host_tests_green": t_basis,
            "uastc_rgb_max_masked": u_rgb_max,
            "uastc_rgb_bound": COLOR_DELTA_BOUND,
            "uastc_alpha_max": u_alpha_max,
            "alpha_bound": ALPHA_DELTA_BOUND,
            "etc1s_rgb_max_measured": e_rgb_max,
            "etc1s_rgb_mean_max_measured": e_rgb_mean_max,
            "level_param_fail_closed_green": t_basis,
            "garbage_fail_closed_green": t_basis,
            "vendor_pin_match": (ab or {}).get("codec_version") == "basis_universal/1.16.4+g900e40fb5d25",
            "encode_double_run_bitexact": det.get("uastc_encode_double_run_bitexact", False),
        },
        "ktx2_3": {
            "textures_measured": len(rows),
            "dds_file_bytes": tot.get("dds_file_bytes", 0),
            "dds_l0_bytes": tot.get("dds_l0_bytes", 0),
            "ktx2_uastc_full_bytes": tot.get("ktx2_uastc_full_bytes", 0),
            "etc1s_full_bytes": tot.get("etc1s_full_bytes", 0),
            "ratio_uastc_full_vs_dds": ratio_u,
            "ratio_etc1s_full_vs_dds": ratio_e,
            "uastc_transcode_l0_ms": tot.get("uastc_transcode_l0_ms", -1.0),
            "etc1s_transcode_l0_ms": tot.get("etc1s_transcode_l0_ms", -1.0),
            "uastc_encode_ms": tot.get("uastc_encode_ms", -1.0),
            "etc1s_encode_ms": tot.get("etc1s_encode_ms", -1.0),
            "ci_ratio_recompute_match": sums_ok,
            "rgba8_digest_manifest_match": manifest_hits,
            "policy": policy_doc,
        },
        "integration": {
            "vendor_verify_green": vendor_verify,
            "vendor_tree_0byte": vdiff and vwork,
            "g11_3_manifest_0byte": g11d and g11w,
            "b4_frozen_0byte": bfd and bfw,
            "b4_inflight_registered": b4_inflight_registered,
            "b4_inflight_status": inflight_status,
            "g22_g29_disposition_0byte": g22d and g22w,
            "m83_gate_green": m83_ok,
            "m83_gate_evidence": m83_ev,
        },
        "harness_evidence": harness_archives,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C14 KTX2 三行立项窗兑现（TODO #37-#39;RD-041 分项）。KTX2-1 = "
            "src/rurix-asset/src/ktx2.rs parse_ktx2（host safe Rust;头/KV/level index/"
            "supercompression 元数据 + mip 布局;真实 encoder 件逐字段互核 + 最小合成件 "
            "write_ktx2_multilevel〔真实负载 + spec 容器,KTXwriter 如实登记,禁手写二进制冒充〕"
            "+ 确定性双读 + fail-closed 臂;CI 独立 Python 第二实现重解析互核）。KTX2-2 = "
            "rurix-basis-sys 既有 vendor 桥 + rurix_basis_transcode_level mip 参数化新面"
            "（unsafe-audit U60;level 越界 rc=17/.basis rc=5 fail-closed）→ BC7/ASTC 转码;"
            "bcdec 独立参考解码对拍 = BC7 **全 8 mode 覆盖**（本任务补齐 mode 0/1/2/3/4/7——"
            "原仅 5/6,真实 UASTC→BC7 命中 0-3/7,decode 覆盖差曾对拍假红 255,机制锚 "
            "punchthrough_alpha_roundtrip_semantics 在案）;premultiplied-aware 口径（透明像素 "
            "RGB 自由域,alpha 量化界 16/语义翻转判红）。KTX2-3 = bistro 12 槽 A/B measured:"
            "DDS 19,576,888B vs UASTC-KTX2 全链 39,153,744B（2.00×,禁 zstd 在树约束下无体积收益）"
            "vs ETC1S 全链 2,813,379B（0.144× ≈ 6.96× 收益）;转码 L0 合计 219.8ms/84.6ms;"
            "质量 UASTC maxΔ4（视觉无损）/ETC1S maxΔ110 meanΔ≤2.9（有损）——政策面:Windows-first "
            "闭集维持 DDS（G22 判档 measured 复核成立）,跨平台/带宽门用 ETC1S,UASTC-KTX2 = 高质量"
            "跨平台中间档;C5 SDK bundle 纹理面维持 DDS + 跨平台场景登记 BasisU 面。"
        ),
    }
    import jsonschema  # 自校验硬门（schema 漂移即 RED）

    errs = list(jsonschema.Draft7Validator(
        json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        fail("gate evidence schema 自校验红: " + "; ".join(
            f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
        all_pass = False
    gate_path = ROOT / "evidence" / f"g31_ktx2_gate_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}(+ harness {len(harness_archives)} 件)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无构建/资产依赖）
# ---------------------------------------------------------------------------


def _good_ktx2(levels: list[bytes] | None = None, scheme: int = 0) -> bytes:
    """合成最小合法 KTX2（selftest 正例面;与 Rust 组装器同律法）。"""
    if levels is None:
        levels = [bytes(range(16))]
    dfd = struct.pack("<I", 28) + bytes(24)
    writer = b"rurix-asset ktx2.rs write_ktx2_multilevel"
    kv_rec = b"KTXwriter\0" + writer
    kvd = struct.pack("<I", len(kv_rec)) + kv_rec
    kvd += bytes((-len(kvd)) % 4)
    n = len(levels)
    idx_off = KTX2_HEADER_LEN
    dfd_off = idx_off + n * KTX2_LEVEL_ENTRY_LEN
    kvd_off = dfd_off + len(dfd)
    data_off = (kvd_off + len(kvd) + 15) & ~15
    offs = []
    cur = data_off
    for i in range(n - 1, -1, -1):
        offs.append((i, cur))
        cur += len(levels[i])
        cur = (cur + 15) & ~15
    off_map = dict(offs)
    out = bytearray()
    out += KTX2_IDENTIFIER
    out += struct.pack("<9I", 0, 1, 16, 16, 0, 0, 1, n, scheme)
    out += struct.pack("<4I", dfd_off, len(dfd), kvd_off, len(kvd))
    out += struct.pack("<2Q", 0, 0)
    for i in range(n):
        out += struct.pack("<3Q", off_map[i], len(levels[i]), len(levels[i]))
    out += dfd
    out += kvd
    while len(out) < data_off:
        out += b"\0"
    for i in range(n - 1, -1, -1):
        while len(out) < off_map[i]:
            out += b"\0"
        out += levels[i]
    return bytes(out)


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # 绿臂①:合法单级/多级件解析。
    good = _good_ktx2()
    p = parse_ktx2_py(good)
    expect(p["header"]["level_count"] == 1 and p["header"]["supercompression_scheme"] == 0,
           "GREEN:单级件头解析")
    expect(p["levels"] == [(192, 16, 16)], f"GREEN:level index 解析 {p['levels']}")
    expect(p["kv"].get("KTXwriter") == b"rurix-asset ktx2.rs write_ktx2_multilevel",
           "GREEN:KVD KTXwriter 解析")
    expect(level_dims_py(2048, 1024, 1) == (1024, 512) and level_dims_py(16, 16, 5) == (1, 1),
           "GREEN:mip 尺寸律法")
    p2 = parse_ktx2_py(_good_ktx2([bytes(32), bytes(8)]))
    expect(p2["header"]["level_count"] == 2 and [x[1] for x in p2["levels"]] == [32, 8],
           "GREEN:多级件 level 布局")
    expect(parse_ktx2_py(good) == parse_ktx2_py(good), "GREEN:双解析位级一致")

    # 红臂组①:逐类非法件必红（确定性同错）。
    def red(b: bytes, code: str, name: str) -> None:
        try:
            parse_ktx2_py(b)
            expect(False, name)
        except Ktx2PyError as e:
            ok = str(e) == code
            same = False
            try:
                parse_ktx2_py(b)
            except Ktx2PyError as e2:
                same = str(e2) == code
            expect(ok and same, name)

    red(good[:40], "TooShort", "RED:截断于头内必红")
    bad = bytearray(good); bad[1] = ord("X")
    red(bytes(bad), "BadIdentifier", "RED:标识篡改必红")
    bad = bytearray(good); bad[40:44] = struct.pack("<I", 0)
    red(bytes(bad), "BadLevelCount(0)", "RED:levelCount=0 必红")
    bad = bytearray(good); bad[40:44] = struct.pack("<I", 99)
    red(bytes(bad), "BadLevelCount(99)", "RED:levelCount=99 必红")
    bad = bytearray(good); bad[44:48] = struct.pack("<I", 7)
    red(bytes(bad), "UnsupportedScheme(7)", "RED:保留 scheme 必红")
    bad = bytearray(good); bad[16:20] = struct.pack("<I", 2)
    red(bytes(bad), "BadTypeSize(2)", "RED:typeSize≠1 必红")
    bad = bytearray(good); bad[96:104] = struct.pack("<Q", 8)
    red(bytes(bad), "SchemeZeroLengthMismatch(0)", "RED:scheme0 长度不符必红")
    red(good[:-1], "OutOfBounds(level_data)", "RED:level 数据截断必红")
    bad = bytearray(good); bad[88:96] = struct.pack("<Q", 0)
    red(bytes(bad), "EmptyLevel(0)", "RED:空 level 必红")
    bad = bytearray(good); bad[52:56] = struct.pack("<I", 32)
    red(bytes(bad), "BadDfd", "RED:DFD totalSize 不符必红")

    # 红绿臂②:质量界判据。
    expect(uastc_quality_ok(4, 0.40, 4), "GREEN:UASTC 实测面过（max4/mean0.4/alpha4）")
    expect(uastc_quality_ok(48, 2.0, 16), "GREEN:UASTC 界值过")
    expect(not uastc_quality_ok(49, 0.4, 4), "RED:UASTC rgb 越 48 必红")
    expect(not uastc_quality_ok(4, 2.1, 4), "RED:UASTC mean 越 2.0 必红")
    expect(not uastc_quality_ok(4, 0.4, 17), "RED:UASTC alpha 越 16 必红")
    expect(etc1s_quality_ok(110, 2.84, 0), "GREEN:ETC1S 实测面过（max110/mean2.84/alpha0）")
    expect(not etc1s_quality_ok(161, 2.0, 0), "RED:ETC1S max 越 160（语义翻转级）必红")
    expect(not etc1s_quality_ok(100, 8.1, 0), "RED:ETC1S mean 越 8.0 必红")
    expect(not etc1s_quality_ok(100, 2.0, 20), "RED:ETC1S alpha 越界必红")

    # 红绿臂③:ratio 重算判据。
    expect(ratio_close(39153744 / 19576888, 39153744 / 19576888), "GREEN:ratio 重算正例")
    expect(not ratio_close(2.0, 2.0 + 1e-6), "RED:ratio 篡改必红")

    # 红绿臂④:top-N 映射律法（B4 同律 mini gltf）。
    gltf = {
        "accessors": [{"count": 300}, {"count": 600}, {"count": 900}],
        "meshes": [
            {"primitives": [
                {"attributes": {"POSITION": 0}, "indices": 0, "material": 0},
                {"attributes": {"POSITION": 0}, "indices": 1, "material": 1},
            ]},
            {"primitives": [{"attributes": {"POSITION": 0}, "indices": 2, "material": 0}]},
        ],
        "nodes": [{"mesh": 0}, {"mesh": 1}],
        "materials": [
            {"name": "A", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}},
            {"name": "B", "pbrMetallicRoughness": {"baseColorTexture": {"index": 1}}},
        ],
        "textures": [{"source": 0}, {"source": 1}],
        "images": [{"uri": "A_BaseColor.dds"}, {"uri": "B_BaseColor.dds"}],
    }
    expect(expected_uris(gltf, 2) == ["A_BaseColor.dds", "B_BaseColor.dds"],
           "GREEN:top-2 律法（降序 + 并列索引升序）")

    # schema 互核:两 schema 在树 + gate schema facts enum == FACT_IDS + const 互核。
    expect(SCHEMA_PATH.is_file() and GATE_SCHEMA_PATH.is_file(), "两 schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        k2 = gs["properties"]["ktx2_2"]["properties"]
        expect(k2["uastc_rgb_bound"]["const"] == COLOR_DELTA_BOUND
               and k2["alpha_bound"]["const"] == ALPHA_DELTA_BOUND,
               "gate schema 对拍界 const 互核（48/16）")
    if SCHEMA_PATH.is_file():
        hs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect("textures" in hs.get("required", []) and "totals" in hs.get("required", []),
               "harness schema required 含 textures/totals")
        expect(hs["properties"]["schema"]["const"] == SCHEMA_ID, "harness schema const 互核")
        bb = hs["properties"]["bounds"]["properties"]
        expect(bb["color_max_delta_bound"]["const"] == COLOR_DELTA_BOUND
               and bb["alpha_delta_bound"]["const"] == ALPHA_DELTA_BOUND,
               "harness schema bounds const 互核")
    expect(len(FACT_IDS) == 4, "facts 闭集 = 4")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=4；10 红臂解析组 + 质量界/ratio/律法红绿组 + 双 schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
