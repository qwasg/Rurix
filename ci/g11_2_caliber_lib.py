#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.2 波）
"""G11.2 口径差对齐波门共享库（milestones/g11/CI_GATES.md §4 M144~M146 + §4A M157 消费面）。

单一事实源面（禁第二份手写）：
- G10.5 锁定契约 digest（机核事实源 = evidence/g10_m130_dual_determinism_contract_
  20260815T233315Z.json 登记值；RXS-0393 L4 转引字面）；
- G11.2 帧区路径（K:/rurix-ext/g11-frames/g11_2/——G10 帧库只读分区隔离）；
- 复跑报告 / 残余口径差登记装载（g11_2_rerun_report.json /
  g11_2_residual_caliber_registry.json，milestones/g11/harness/g11_2_ab_rerun.py 产）；
- HDR 亮度统计（Rec.709 相对亮度，numpy；g10_5_ab_metrics 同口径）；
- 残余登记校验器（RED 臂共用：拟合冒充 / 未对齐口径消费 / 残余未登记判红面）；
- g11_budget.json 字节级纯追加（M138 同纪律：行尾风格随原文件，既有行 0-byte）。
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g10_exr_lib as exr  # noqa: E402

CORPUS = ROOT / "milestones" / "g10" / "corpus"
GAP_REGISTRY = ROOT / "milestones" / "g10" / "g10_gap_registry.json"
BUDGET_PATH = ROOT / "milestones" / "g11" / "g11_budget.json"
REPORT_PATH = ROOT / "milestones" / "g11" / "g11_2_rerun_report.json"
RESIDUAL_PATH = ROOT / "milestones" / "g11" / "g11_2_residual_caliber_registry.json"
FRAMES_G11 = Path(r"K:\rurix-ext\g11-frames\g11_2")
RUST_RELEASE_BIN = ROOT / "target" / "release" / "g10_5_scene_render.exe"
BUILD_SCENES_PY = ROOT / "milestones" / "g10" / "harness" / "ue_python" / "g10_5_build_scenes.py"
SCENE_RENDER_RS = ROOT / "src" / "rurix-asset" / "src" / "bin" / "g10_5_scene_render.rs"
WHITE_HDR = Path(r"K:\rurix-ext\g10-ue\harness_assets\white_2x1.hdr")

# G10.5 锁定契约 digest（evidence/g10_m130_dual_determinism_contract_20260815T233315Z.json）。
LOCKED_DIGEST = {
    "cornell-box": "sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118",
    "bistro-interior": "sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514",
}
LOCKED_DIGEST_JOINT = "sha256:64fd54df6e9be522d6dbb3bec8fac1eb30a0a421c7a5a8185a3452c381178aa4"

SCENES = {
    "cornell-box": {"ev100": 2.0, "res": (512, 512)},
    "bistro-interior": {"ev100": 1.0, "res": (1920, 1080)},
}


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_report() -> dict:
    return load_json(REPORT_PATH)


def load_residual_registry() -> dict:
    return load_json(RESIDUAL_PATH)


def sha256_file(path: Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def hdr_frame(scene_id: str, end: str) -> Path:
    if end == "rurix":
        return FRAMES_G11 / "rurix" / f"{scene_id}.exr"
    return FRAMES_G11 / "ue" / scene_id / ".0000.exr"


def ldr_frame(scene_id: str, end: str) -> Path:
    return FRAMES_G11 / "ldr" / f"{scene_id}_{end}_ldr.exr"


def decode(path: Path, end: str) -> dict:
    return exr.decode_exr(path.read_bytes(), end)


def pixels_of(d: dict) -> np.ndarray:
    return np.asarray(d["pixels"], dtype=np.float64).reshape(d["height"], d["width"], 3)


def lum_stats(arr: np.ndarray) -> dict:
    lum = 0.2126 * arr[..., 0] + 0.7152 * arr[..., 1] + 0.0722 * arr[..., 2]
    flat = np.sort(lum.ravel())
    n = flat.size
    return {
        "median": float(flat[n // 2]),
        "p90": float(flat[int(n * 0.9)]),
        "max": float(flat[-1]),
        "mean": float(flat.mean()),
        "nonzero_ratio": float(np.count_nonzero(flat > 1e-6) / n),
    }


def gap_row(kind_title_prefix: str) -> dict:
    """g10_gap_registry 0-byte 只读消费：按标题前缀取行（C1/C2/C3）。"""
    reg = load_json(GAP_REGISTRY)
    for item in reg["items"]:
        if item["title"].startswith(kind_title_prefix):
            return item
    raise KeyError(f"gap registry 缺行: {kind_title_prefix}")


def contract_digest_rust(scene_id: str) -> str:
    """Rust 第三实现当次重算契约 digest（--contract-digest 真跑）。"""
    p = CORPUS / f"contract_params_{scene_id.replace('-', '_')}.json"
    r = subprocess.run([str(RUST_RELEASE_BIN), "--contract-digest", str(p)],
                       cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"契约 digest 重算失败（{scene_id}）: {r.stderr[-400:]}")
    line = [l for l in r.stdout.splitlines() if "param_digest_rust" in l][-1]
    return "sha256:" + line.split("=")[-1].strip()


def parse_white_hdr(path: Path) -> tuple[list[float], str]:
    """解析 Radiance HDR（白色 cubemap 源资产）：返回 (逐像素 [r,g,b,...], sha256)。
    仅支持非 RLE 平扫面（本资产 2x1 小图；格式违例即 fail-closed）。"""
    raw = path.read_bytes()
    digest = "sha256:" + hashlib.sha256(raw).hexdigest()
    if not raw.startswith(b"#?RADIANCE"):
        raise ValueError("非 Radiance HDR")
    marker = raw.find(b"\n-Y ")
    if marker < 0:
        raise ValueError("缺维度行")
    nl = raw.find(b"\n", marker + 1)
    dims = raw[marker + 1:nl].decode("ascii")
    parts = dims.split()
    h = int(parts[1])
    w = int(parts[3])
    body = raw[nl + 1:]
    need = w * h * 4
    if len(body) < need:
        raise ValueError(f"像素体截断: {len(body)} < {need}")
    px: list[float] = []
    for i in range(w * h):
        r, g, b, e = body[i * 4], body[i * 4 + 1], body[i * 4 + 2], body[i * 4 + 3]
        if e == 0:
            px.extend((0.0, 0.0, 0.0))
            continue
        f = 2.0 ** (e - (128 + 8))
        px.extend((r * f, g * f, b * f))
    return px, digest


def validate_residual_registry(doc: dict) -> list[str]:
    """残余口径差登记校验器（RXS-0392 L4 机核面；RED 臂共用）。"""
    problems: list[str] = []
    if not isinstance(doc, dict):
        return ["登记文档非 object"]
    if doc.get("registry") != "g11_2_residual_caliber_registry":
        problems.append(f"registry 字段漂移: {doc.get('registry')!r}")
    chains = doc.get("aligned_chains")
    if not isinstance(chains, list) or not chains:
        problems.append("aligned_chains 缺失或空（未对齐口径消费复测 delta 即 RED）")
        chains = []
    chain_names = {c.get("chain") for c in chains if isinstance(c, dict)}
    for need in ("sun_color", "sun_lux_to_radiance", "sky_intensity", "exposure_scale"):
        if need not in chain_names:
            problems.append(f"aligned_chains 缺环节: {need}")
    for c in chains:
        if not isinstance(c, dict):
            problems.append("aligned_chains 行非 object")
            continue
        for k in ("chain", "scene_id", "before", "after", "status"):
            if not c.get(k):
                problems.append(f"aligned_chains 行缺字段 {k}: {c.get('chain')!r}")
        if c.get("status") not in ("aligned_fixed", "aligned_verified"):
            problems.append(f"aligned_chains 行 status 非法: {c.get('status')!r}")
    items = doc.get("items")
    if not isinstance(items, list) or not items:
        problems.append("items 缺失或空（残余口径差未登记即 RED）")
        items = []
    anchors_text = ""
    for it in items:
        if not isinstance(it, dict):
            problems.append("items 行非 object")
            continue
        for k in ("residual_id", "chain", "scene_id", "kind", "description", "disposition_anchor", "status"):
            if not it.get(k):
                problems.append(f"items 行缺字段 {k}: {it.get('residual_id')!r}")
        if it.get("kind") != "residual_caliber_diff":
            problems.append(f"items 行 kind 非法: {it.get('kind')!r}")
        if it.get("status") != "registered":
            problems.append(f"items 行 status 非法: {it.get('status')!r}")
        anchors_text += str(it.get("disposition_anchor", ""))
    if "m153" not in anchors_text or "m154" not in anchors_text:
        problems.append("残余登记缺 R3（m153）/ R4（m154）承接锚（残余口径差未登记即 RED）")
    return problems


def validate_budget_entry(entry: dict, p100: float, k: float) -> list[str]:
    """标定条目合法性机核（M138 同字面：手写阈值冒充 / estimated 冒充判红面）。"""
    problems: list[str] = []
    if entry.get("evidence") != "measured_local":
        problems.append(f"{entry.get('id')}: evidence={entry.get('evidence')!r}（estimated 冒充 measured 即 RED）")
    if entry.get("threshold") != p100 * k:
        problems.append(
            f"{entry.get('id')}: threshold={entry.get('threshold')!r} ≠ p100×k={p100 * k!r}（手写阈值冒充标定即 RED）"
        )
    if entry.get("measured_value") != p100:
        problems.append(f"{entry.get('id')}: measured_value ≠ p100")
    ef = entry.get("evidence_file") or ""
    if not ef or not (ROOT / ef).is_file():
        problems.append(f"{entry.get('id')}: evidence_file 不在树: {ef!r}")
    return problems


def append_budget_entries(new_entries: list[dict]) -> list[str]:
    """g11_budget.json 字节级纯追加（M138 同纪律：行尾风格随原文件；已存在同值
    幂等、值漂移即 problems；追加后整体可解析复核）。"""
    budget_text = BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    problems: list[str] = []
    to_add: list[dict] = []
    for entry in new_entries:
        existing = [x for x in budget.get("entries", []) if x.get("id") == entry["id"]]
        if existing:
            ex = existing[0]
            comparable = {k: v for k, v in entry.items() if k != "evidence_file"}
            ex_comparable = {k: v for k, v in ex.items() if k != "evidence_file"}
            if ex_comparable != comparable:
                problems.append(f"{entry['id']} 已在树且值漂移（只追加禁改写）: 在树 {ex} vs 重算 {entry}")
            continue
        to_add.append(entry)
    if problems or not to_add:
        return problems
    nl = "\r\n" if "\r\n" in budget_text else "\n"
    anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
    if anchor not in budget_text:
        return ["g11_budget.json 结构锚缺失（entries 闭合段未找到，拒改写）"]
    frag = ""
    for entry in to_add:
        body = json.dumps(entry, ensure_ascii=False, indent=2)
        body = body.replace("\n", nl)
        body = "    " + body.replace(nl, nl + "    ")
        frag += "," + nl + body
    head, sep, tail = budget_text.partition(anchor)
    budget_text = head + frag + sep + tail
    json.loads(budget_text)
    BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems
