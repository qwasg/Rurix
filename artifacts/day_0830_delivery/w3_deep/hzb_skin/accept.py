#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude（G37 W3 hzb_skin——HZB×蒙皮同车道合并面验收臂）
"""G37 W3 hzb_skin 合并臂验收（gate 字面 g37.wave3.hzb_skin;G36 W4-W5 留窗
「HZB×蒙皮同车道（新 kernel 合并面）」兑现件的 GPU 验收脚本）。

**判据 = 蒙皮动核验（skin 门口径）∧ HZB 金字塔/判定/剔除（hzb 门口径）∧
双跑位级 ∧ 与单开臂各自口径不降级**——本脚本为工作区验收臂（artifacts/ 件,
非 ci/ 门;门脚本扩合并臂归提案面,见同目录 REPORT.md §提案）。

七判据闭集（facts）:
1. kernels_spv_valid    —— rurixc 现编七件（g34_unified_primary_skin〔G37 W3
   加性合并主射线〕/ g34_unified_primary〔单开臂对照〕/ g34_unified_shade /
   g31_hzb_pack / g27_hzb_reduce / g27_hzb_test / g31_skin / g34_unified_mv
   ——实为八件,primary 双形态并列〕+ spirv-val 全绿 + 母版四件 0-byte
   （g34_unified_primary/gi_skin/mv/g31_skin .rx vs 本波前快照 sha 相等——
   新 kernel 新文件纪律的机核）。
2. merged_skin_caliber  —— 合并腿 evidence skin 块过 skin 门口径:
   ① vertex_parity.all_bitexact（全核验帧 max_abs == 0.0 位级）
   ② verify_frames 全帧 pass + 位置核验（质心/AABB/计数）
   ③ MV 三类（类 3 中位差 + 窗级真动 + 类 1 静态区 + 类 2 刚性激活 ≥1 帧）。
3. merged_hzb_caliber   —— 合并腿 evidence hzb 块过 hzb 门口径:
   parity 三面（mips_bitexact + verdict_equal + false_positives==0 + digest
   互核）+ occluded_p1 ≥ 1（零剔除即空接线冒充判红）+ 计数非负。
4. culling_pixel_neutral—— 合并腿 vs RURIX_HZB_ALL_VISIBLE=1 全集渲染实验臂
   digest_seq 逐帧位级一致（剔除不改变可见像素——两阶段闭环正确性结构判据;
   蒙皮角色/动态尾槽恒可见,中性门在合并面同字面成立）。
5. determinism_double_run—— 合并腿双跑 digest_seq 逐帧位级 + render_digest 一致。
6. single_open_no_degrade—— 同机同窗复跑 --hzb on 单开腿 + --skin 单开腿,
   各自以其在案门口径判读（hzb: parity 三面 + occluded_p1≥1;skin: 蒙皮三面
   聚合旗标）——合并面落地后单开臂口径不降级的机器证明。
7. frame_ms_measured    —— 合并腿 real_render_frame_ms vs 两单开腿如实登记
   不设通过线（G6 无硬门纪律,measured_local）。

三态：无 Vulkan/设备/资产/SPV 编译失败 → DEV_ENV_DEGRADE 退 0（不冒充 PASS）;
RURIX_REQUIRE_REAL=1 翻硬 FAIL。产物落本目录 accept_result_<ts>.json
（工作区件,不进 evidence/ 路由——门裁决件归 ci/ 提案面）。

用法：
  py -3 artifacts/day_0830_delivery/w3_deep/hzb_skin/accept.py --selftest
  py -3 artifacts/day_0830_delivery/w3_deep/hzb_skin/accept.py --run [--frames 64] [--warmup 10]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent.parent.parent  # artifacts/day_0830_delivery/w3_deep/hzb_skin → 仓根
GATE_KEY = "g37.wave3.hzb_skin"
TAG = "g37_hzb_skin_accept"
MERGED_SCHEMA_ID = "rurix.g37.hzb_skin_unified_evidence.v1"
HZB_SCHEMA_ID = "rurix.g34.hzb_unified_evidence.v1"
SKIN_SCHEMA_ID = "rurix.g34.skin_unified_evidence.v1"
HZB_GATE_KEY = "g34.wave2.hzb"
SKIN_GATE_KEY = "g34.wave2.skin"
SLAB_ASSET = ROOT / "milestones" / "g31" / "g31_slab_side_table_bistro_interior.json"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
WORK = ROOT / ".tmp" / "g34_gates" / "hzb_skin"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_FULL = ROOT / "target" / "release" / f"g34_full_lane{EXE_SUFFIX}"
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

# rurixc 现编八件（合并腿七件消费 + 单开对照臂 primary;g27 两件/pack/skin/mv
# 本体 0-byte 冻结消费——bin 侧 NoContraction 注入,SPV 文件不回写）。
KERNEL_SPECS = (
    ("g34_unified_primary_skin.rx", "g34_unified_primary_skin.spv"),
    ("g34_unified_primary.rx", "g34_unified_primary.spv"),
    ("g34_unified_shade.rx", "g34_unified_shade.spv"),
    ("g31_hzb_pack.rx", "g31_hzb_pack.spv"),
    ("g27_hzb_reduce.rx", "g27_hzb_reduce.spv"),
    ("g27_hzb_test.rx", "g27_hzb_test.spv"),
    ("g31_skin.rx", "g31_skin.spv"),
    ("g34_unified_mv.rx", "g34_unified_mv.spv"),
    ("g34_unified_gi_skin.rx", "g34_unified_gi_skin.spv"),
    ("g34_unified_gi.rx", "g34_unified_gi.spv"),
)
# 母版 0-byte 机核（本波前快照 sha256——新 kernel 新文件纪律;快照值取自
# G37 W3 实装当日工作树,后续漂移 = 母版被改 = 红）。
MOTHER_FROZEN_SHA = {
    # 值 "SNAPSHOT_AT_RUN" 哨兵 = 首跑时落账本目录 mother_sha.json 后比对
    # （工作区验收臂两段式:首跑登记,复跑机核——避免手写 sha 冒充）。
    "src/rurix-render/kernels/g34_unified_primary.rx": None,
    "src/rurix-render/kernels/g34_unified_gi_skin.rx": None,
    "src/rurix-render/kernels/g34_unified_mv.rx": None,
    "src/rurix-render/kernels/g31_skin.rx": None,
}
MOTHER_SHA_FILE = HERE / "mother_sha.json"

FACT_IDS = [
    "kernels_spv_valid",
    "merged_skin_caliber",
    "merged_hzb_caliber",
    "culling_pixel_neutral",
    "determinism_double_run",
    "single_open_no_degrade",
    "frame_ms_measured",
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
# 判读器（--selftest 红绿两臂消费面;纯 CPU 零构建零 GPU）
# ---------------------------------------------------------------------------


def seqs_bitexact(a: list, b: list) -> bool:
    """双臂 digest_seq 逐帧位级一致判据（非空 + 等长 + 逐项全等）。"""
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


def _nonneg_int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool) and v >= 0


def _pos_int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool) and v >= 1


def skin_caliber_judge(sk) -> list[str]:
    """skin 门口径判（ci/g34_skin_unified_smoke.py 三 facts 蒸馏同判据:
    ① vertex_parity 位级 ② verify_frames 全 pass + all_pass 聚合 ③ 窗级
    真动门 + 类 2 激活帧 ≥1;缺键即红 fail-closed）。"""
    if not isinstance(sk, dict):
        return ["skin 块非 object"]
    fails: list[str] = []
    vp = sk.get("vertex_parity")
    if not isinstance(vp, dict) or vp.get("all_bitexact") is not True:
        fails.append(f"skin.vertex_parity.all_bitexact ≠ true: {vp!r}"[:160])
    rows = sk.get("verify_frames")
    if not isinstance(rows, list) or not rows:
        fails.append("skin.verify_frames 空/缺失（零核验帧即空接线冒充）")
        rows = []
    for r in rows:
        if not isinstance(r, dict) or r.get("pass") is not True:
            fails.append(f"skin.verify_frames 帧 {r.get('frame') if isinstance(r, dict) else '?'} 未过")
            break
        if r.get("vertex_max_abs") != 0.0:
            fails.append(f"skin 帧 {r.get('frame')} vertex_max_abs ≠ 0.0（① 位级门破）")
            break
    if sk.get("all_pass") is not True:
        fails.append(f"skin.all_pass ≠ true: {sk.get('all_pass')!r}")
    mg = sk.get("motion_gate")
    if not isinstance(mg, dict):
        fails.append("skin.motion_gate 缺失")
    else:
        hm = mg.get("host_motion_max_px")
        th = mg.get("threshold_px")
        if not (isinstance(hm, (int, float)) and isinstance(th, (int, float)) and hm >= th):
            fails.append(f"skin 窗级真动门破: host_motion_max={hm!r} < threshold={th!r}")
    gap = sk.get("mv_gap")
    if not isinstance(gap, dict) or not _pos_int(gap.get("rigid_active_frames")):
        fails.append(f"skin.mv_gap.rigid_active_frames < 1（类 2 臂零激活）: {gap!r}"[:160])
    return fails


def hzb_caliber_judge(hz) -> list[str]:
    """hzb 门口径判（ci/g34_hzb_unified_smoke.py parity_judge + counts_
    effective 蒸馏同判据;缺键即红 fail-closed）。"""
    if not isinstance(hz, dict):
        return ["hzb 块非 object"]
    fails: list[str] = []
    p = hz.get("parity")
    if not isinstance(p, dict):
        fails.append("hzb.parity 非 object（probe 对拍未成）")
    else:
        if p.get("mips_bitexact") is not True:
            fails.append(f"hzb.parity.mips_bitexact ≠ true: {p.get('mips_bitexact')!r}")
        if p.get("verdict_equal") is not True:
            fails.append(f"hzb.parity.verdict_equal ≠ true: {p.get('verdict_equal')!r}")
        if p.get("false_positives") != 0:
            fails.append(f"hzb.parity.false_positives ≠ 0: {p.get('false_positives')!r}")
        for k in ("pyramid_digest", "verdict_digest"):
            d = p.get(k)
            hd = p.get("host_" + k)
            if not isinstance(d, str) or not DIGEST_RE.match(d) or d != hd:
                fails.append(f"hzb.parity.{k} vs host 互核破")
    if not _pos_int(hz.get("occluded_p1")):
        fails.append(f"hzb.occluded_p1 < 1（零剔除即空接线冒充）: {hz.get('occluded_p1')!r}")
    if not _pos_int(hz.get("tested")):
        fails.append(f"hzb.tested < 1: {hz.get('tested')!r}")
    for k in ("flipped_p2", "closure_extra_submits", "closure_full_fallback_frames"):
        if not _nonneg_int(hz.get(k)):
            fails.append(f"hzb.{k} 非负整数破: {hz.get(k)!r}")
    if not _pos_int(hz.get("instances")) or not _pos_int(hz.get("mips")):
        fails.append("hzb.instances/mips < 1")
    return fails


def merged_leg_judge(doc: dict, frames: int, warmup: int, label: str, expect_allvis: bool) -> list[str]:
    """合并腿公共判（schema/gate 字面 + 帧完成 + digest 序列形态 + 特性旗标
    五真 + allvis 标记;深判归 skin/hzb 口径两判器）。"""
    fails: list[str] = []
    if doc.get("schema") != MERGED_SCHEMA_ID:
        fails.append(f"{label}: schema ≠ {MERGED_SCHEMA_ID}: {doc.get('schema')!r}")
    if doc.get("gate") != GATE_KEY:
        fails.append(f"{label}: gate ≠ {GATE_KEY}: {doc.get('gate')!r}")
    total = frames + warmup
    if doc.get("frames_completed") != total:
        fails.append(f"{label}: frames_completed {doc.get('frames_completed')} ≠ {total}")
    if doc.get("exit_reason") != "frames_done":
        fails.append(f"{label}: exit_reason ≠ frames_done: {doc.get('exit_reason')!r}")
    seq = doc.get("digest_seq")
    if not isinstance(seq, list) or len(seq) != total or any(
        not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq
    ):
        fails.append(f"{label}: digest_seq 形态/长度破（≠{total}）")
    if not isinstance(doc.get("render_digest"), str) or not DIGEST_RE.match(doc["render_digest"]):
        fails.append(f"{label}: render_digest 形态破")
    ft = doc.get("features")
    for k in ("textures", "slab", "dyn", "hzb", "skin"):
        if not isinstance(ft, dict) or ft.get(k) is not True:
            fails.append(f"{label}: features.{k} ≠ true（五特性同开字面破）")
            break
    rr = doc.get("real_render_frame_ms")
    if not isinstance(rr, (int, float)) or isinstance(rr, bool) or not rr > 0:
        fails.append(f"{label}: real_render_frame_ms 非正: {rr!r}")
    hz = doc.get("hzb")
    if isinstance(hz, dict):
        if hz.get("all_visible_arm") is not expect_allvis:
            fails.append(f"{label}: hzb.all_visible_arm ≠ {expect_allvis}（实验臂标记面破）")
    else:
        fails.append(f"{label}: hzb 块缺失")
    if doc.get("host_parity") is not None:
        fails.append(f"{label}: host_parity 非 null（合并腿诚实登记面破——对拍 = probe 三面 + 蒙皮①臂承载）")
    return fails


def single_hzb_judge(doc: dict, label: str) -> list[str]:
    """--hzb on 单开腿不降级判（其在案门口径:schema/gate 字面 + hzb 口径）。"""
    fails: list[str] = []
    if doc.get("schema") != HZB_SCHEMA_ID:
        fails.append(f"{label}: schema ≠ {HZB_SCHEMA_ID}")
    if doc.get("gate") != HZB_GATE_KEY:
        fails.append(f"{label}: gate ≠ {HZB_GATE_KEY}")
    fails += [f"{label}: {m}" for m in hzb_caliber_judge(doc.get("hzb"))]
    return fails


def single_skin_judge(doc: dict, label: str) -> list[str]:
    """--skin 单开腿不降级判（其在案门口径:schema/gate 字面 + skin 口径）。"""
    fails: list[str] = []
    if doc.get("schema") != SKIN_SCHEMA_ID:
        fails.append(f"{label}: schema ≠ {SKIN_SCHEMA_ID}")
    if doc.get("gate") != SKIN_GATE_KEY:
        fails.append(f"{label}: gate ≠ {SKIN_GATE_KEY}")
    fails += [f"{label}: {m}" for m in skin_caliber_judge(doc.get("skin"))]
    return fails


def frame_ms_sane(*vals: float) -> bool:
    return all(isinstance(v, (int, float)) and not isinstance(v, bool) and v == v and v > 0 for v in vals)


# ---------------------------------------------------------------------------
# GPU 腿（--run;禁在本子任务会话跑——主 agent GPU 验收面）
# ---------------------------------------------------------------------------


def leg_argv(label: str, frames: int, warmup: int, mode: str) -> list[str]:
    """harness 调用契约（mode ∈ merged|hzb|skin）:
    merged = --hzb on --skin on 同开（合并臂;kernels 全走 WORK 现编件）;
    hzb    = --hzb on 单开（不降级对照;primary 走单开件）;
    skin   = --skin on 单开（不降级对照;gi_skin/mv/skin 走 WORK 现编件）。
    """
    argv: list[str] = [str(BIN_FULL)]
    if mode in ("merged", "hzb"):
        argv += ["--hzb", "on"]
    if mode in ("merged", "skin"):
        argv += ["--skin", "on"]
    argv += [
        "--full", "--slab-table", str(SLAB_ASSET),
        "--frames", str(frames), "--warmup", str(warmup),
        "--auto-move", "orbit", "--tier", "100", "--hidden",
    ]
    if mode in ("merged", "hzb"):
        argv += [
            "--spv-hzb-shade", str(WORK / "g34_unified_shade.spv"),
            "--spv-hzb-pack", str(WORK / "g31_hzb_pack.spv"),
            "--spv-hzb-reduce", str(WORK / "g27_hzb_reduce.spv"),
            "--spv-hzb-test", str(WORK / "g27_hzb_test.spv"),
        ]
    if mode == "merged":
        argv += [
            "--spv-hzbskin-primary", str(WORK / "g34_unified_primary_skin.spv"),
            "--spv-skin", str(WORK / "g31_skin.spv"),
            "--spv-skin-mv", str(WORK / "g34_unified_mv.spv"),
        ]
    if mode == "hzb":
        argv += ["--spv-hzb-primary", str(WORK / "g34_unified_primary.spv")]
    if mode == "skin":
        argv += [
            "--spv-skin", str(WORK / "g31_skin.spv"),
            "--spv-skin-scene", str(WORK / "g34_unified_gi_skin.spv"),
            "--spv-skin-mv", str(WORK / "g34_unified_mv.spv"),
        ]
    argv += ["--evidence", str(WORK / f"leg_{label}.json")]
    return argv


def run_leg(label: str, frames: int, warmup: int, mode: str, env: dict) -> tuple[subprocess.CompletedProcess, dict | None]:
    r = run(leg_argv(label, frames, warmup, mode), env=env)
    ev_path = WORK / f"leg_{label}.json"
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc


def run_accept(frames: int, warmup: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    sys.path.insert(0, str(ROOT / "ci"))
    from gpu_device_lock import gpu_device_lock  # noqa: E402（GPU 段独有依赖）

    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    # ── 构建（release harness + rurixc）──
    for what, argv in (
        ("harness release", ["cargo", "build", "--release", "-p", "rurix-render",
                             "--features", "vendor-upscale", "--bin", "g34_full_lane", "--quiet"]),
        ("rurixc", ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend",
                    "--bin", "rurixc", "--quiet"]),
    ):
        r = run(argv)
        if r.returncode != 0:
            fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
            return 1

    # ── ① kernel SPV 面:现编十件 + spirv-val + 母版 0-byte 机核（两段式
    #    快照:首跑落账 mother_sha.json,复跑比对——新 kernel 新文件纪律）──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for src_name, dst_name in KERNEL_SPECS:
        src = KERNEL_DIR / src_name
        dst = WORK / dst_name
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src_name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst_name}: {(val.stdout + val.stderr)[-200:]}")
    cur_sha = {
        p: "sha256:" + hashlib.sha256((ROOT / p).read_bytes()).hexdigest()
        for p in MOTHER_FROZEN_SHA
        if (ROOT / p).is_file()
    }
    mother_ok = len(cur_sha) == len(MOTHER_FROZEN_SHA)
    if MOTHER_SHA_FILE.is_file():
        frozen = json.loads(MOTHER_SHA_FILE.read_text(encoding="utf-8"))
        drift = [p for p, s in cur_sha.items() if frozen.get(p) != s]
        if drift:
            mother_ok = False
            note(f"母版 0-byte 机核红:漂移 {drift}")
    else:
        MOTHER_SHA_FILE.write_text(json.dumps(cur_sha, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        note(f"母版快照首跑落账 → {MOTHER_SHA_FILE.name}（复跑起为 0-byte 机核基线）")
    set_fact(
        "kernels_spv_valid",
        spv_ok and mother_ok,
        f"rurixc 现编 {len(KERNEL_SPECS)} 件 + spirv-val={'绿' if spv_ok else '红'}；"
        f"母版四件（primary/gi_skin/mv/g31_skin .rx）0-byte 机核={'绿' if mother_ok else '红'}"
        "（两段式快照:首跑落账 mother_sha.json,复跑比对——新 kernel 新文件纪律）",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("kernel SPV 编译/spirv-val 未过")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")
    if not SLAB_ASSET.is_file():
        degrade.append(f"slab 侧表资产缺失 {SLAB_ASSET}")
    for f_ in ("g14_mv.spv", "g14_8_tsr_resample.spv", "g14_8_tsr_resolve.spv",
               "g31_display_encode.spv", "g29_slab.spv"):
        if not (ROOT / ".tmp" / "g14_gates" / "m_c" / f_).is_file():
            degrade.append(f"车道 SPV 缺失 {f_}")
    if not (ROOT / ".tmp" / "g31_gates" / "texture" / "g31_texture_probe.spv").is_file():
        degrade.append("纹理探针 SPV 缺失")

    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    leg_docs: dict[str, dict] = {}
    leg_ok = True
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} 五腿（merged_a/b + allvis + hzb/skin 单开）"):
            legs = [
                # label, mode, allvis_env, pass_marker
                ("merged_a", "merged", False, "[hzb_skin] PASS"),
                ("merged_b", "merged", False, "[hzb_skin] PASS"),
                ("allvis", "merged", True, "[hzb_skin] PASS"),
                ("hzb_single", "hzb", False, "[hzb] PASS"),
                ("skin_single", "skin", False, "[skin] PASS"),
            ]
            for label, mode, allvis, marker in legs:
                leg_env = dict(env)
                if allvis:
                    leg_env["RURIX_HZB_ALL_VISIBLE"] = "1"
                r, doc = run_leg(label, frames, warmup, mode, leg_env)
                out = (r.stdout or "") + (r.stderr or "")
                if r.returncode != 0 or doc is None or marker not in out:
                    fail(f"{label} 真跑失败 rc={r.returncode}: {out[-300:]}")
                    leg_ok = False
                    continue
                if "Validation Error" in out or "VUID-" in out:
                    fail(f"{label} validation 应静默却报错")
                    leg_ok = False
                leg_docs[label] = doc

    if degrade:
        doc = {"schema": "rurix.g37.hzb_skin_accept.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── ②③ 合并腿双口径 ──
    ma = leg_docs.get("merged_a") or {}
    for m in merged_leg_judge(ma, frames, warmup, "merged_a", expect_allvis=False):
        fail(m)
        leg_ok = False
    sk_fails = skin_caliber_judge(ma.get("skin"))
    set_fact(
        "merged_skin_caliber",
        leg_ok and not sk_fails,
        "合并腿 skin 门口径:① 逐顶点位级 ② 位置/逐帧门 ③ MV 三类 + 窗级真动 + 类2激活"
        + ("" if not sk_fails else f"；红 {sk_fails[:2]}"),
    )
    hz_fails = hzb_caliber_judge(ma.get("hzb"))
    set_fact(
        "merged_hzb_caliber",
        leg_ok and not hz_fails,
        "合并腿 hzb 门口径:parity 三面（mips 位级/判定逐字节/零假阳性 + digest 互核）"
        f"+ occluded_p1={((ma.get('hzb') or {}).get('occluded_p1'))!r} ≥1"
        + ("" if not hz_fails else f"；红 {hz_fails[:2]}"),
    )
    # ── ④ 剔除像素中性 ──
    seq_a = ma.get("digest_seq", [])
    seq_av = (leg_docs.get("allvis") or {}).get("digest_seq", [])
    av_fails = merged_leg_judge(leg_docs.get("allvis") or {}, frames, warmup, "allvis", expect_allvis=True)
    for m in av_fails:
        fail(m)
    neutral = seqs_bitexact(seq_a, seq_av)
    set_fact(
        "culling_pixel_neutral",
        leg_ok and neutral and not av_fails,
        f"合并腿 vs RURIX_HZB_ALL_VISIBLE=1 全集渲染实验臂 digest_seq 逐帧位级一致={neutral}"
        f"（{len(seq_a)} 帧;剔除不改变可见像素——蒙皮/动态尾槽恒可见下中性门同字面成立）",
    )
    # ── ⑤ 确定性双跑 ──
    mb = leg_docs.get("merged_b") or {}
    mb_fails = merged_leg_judge(mb, frames, warmup, "merged_b", expect_allvis=False)
    for m in mb_fails:
        fail(m)
    bit = seqs_bitexact(seq_a, mb.get("digest_seq", []))
    rd_eq = ma.get("render_digest") == mb.get("render_digest")
    set_fact(
        "determinism_double_run",
        leg_ok and bit and rd_eq and not mb_fails,
        f"合并腿双跑 digest_seq 位级一致={bit}（{len(seq_a)} 帧）render_digest 一致={rd_eq}",
    )
    # ── ⑥ 单开臂不降级 ──
    hz_single_fails = single_hzb_judge(leg_docs.get("hzb_single") or {}, "hzb_single")
    sk_single_fails = single_skin_judge(leg_docs.get("skin_single") or {}, "skin_single")
    for m in hz_single_fails + sk_single_fails:
        fail(m)
    set_fact(
        "single_open_no_degrade",
        leg_ok and not hz_single_fails and not sk_single_fails,
        "同机同窗复跑 --hzb on 单开（parity 三面 + occluded_p1≥1）+ --skin 单开"
        "（蒙皮三面聚合旗标）各自在案口径全绿——合并面落地后单开臂不降级机器证明"
        + ("" if not (hz_single_fails or sk_single_fails) else f"；红 {(hz_single_fails + sk_single_fails)[:2]}"),
    )
    # ── ⑦ frame_ms 对照（不设通过线;G6 无硬门纪律）──
    m_ms = ma.get("real_render_frame_ms", -1.0)
    h_ms = (leg_docs.get("hzb_single") or {}).get("real_render_frame_ms", -1.0)
    s_ms = (leg_docs.get("skin_single") or {}).get("real_render_frame_ms", -1.0)
    ms_ok = frame_ms_sane(m_ms, h_ms, s_ms)
    set_fact(
        "frame_ms_measured",
        leg_ok and ms_ok,
        f"frame_ms measured_local:merged={m_ms:.4f}ms hzb_single={h_ms:.4f}ms "
        f"skin_single={s_ms:.4f}ms（如实登记不设通过线,G6 无硬门纪律）",
    )

    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and leg_ok and not FAILURES
    result = {
        "schema": "rurix.g37.hzb_skin_accept_result.v1",
        "gate": GATE_KEY,
        "verdict": "PASS" if all_pass else "FAIL",
        "facts": fact_rows,
        "frames": frames,
        "warmup": warmup,
        "legs": {k: str((WORK / f"leg_{k}.json").relative_to(ROOT)) for k in leg_docs},
        "timestamp": ts,
        "note": (
            "G37 W3 hzb_skin 合并臂验收（工作区件;门脚本扩合并臂归提案面见 REPORT.md）:"
            "判据 = skin 门口径 ∧ hzb 门口径 ∧ 剔除像素中性 ∧ 双跑位级 ∧ 单开臂不降级 "
            "∧ frame_ms 如实登记。harness 真跑件留 .tmp/g34_gates/hzb_skin/。"
        ),
    }
    out_path = HERE / f"accept_result_{ts}.json"
    out_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"result → {out_path}")
    note(f"ACCEPT {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂;纯 CPU 零构建零 GPU 零仓外读取）
# ---------------------------------------------------------------------------


def _dg(ch: str) -> str:
    return "sha256:" + ch * 64


def _good_skin_block() -> dict:
    return {
        "vertex_parity": {"frames": 5, "max_abs_max": 0.0, "all_bitexact": True},
        "verify_frames": [
            {"frame": 10, "pass": True, "vertex_max_abs": 0.0},
            {"frame": 20, "pass": True, "vertex_max_abs": 0.0},
        ],
        "all_pass": True,
        "motion_gate": {"host_motion_max_px": 2.5, "threshold_px": 1.0},
        "mv_gap": {"rigid_active_frames": 3, "class2_delta_max_px": 0.4},
    }


def _good_hzb_block() -> dict:
    return {
        "instances": 1187,
        "mips": 11,
        "tested": 9000,
        "occluded_p1": 120,
        "flipped_p2": 2,
        "closure_extra_submits": 1,
        "closure_full_fallback_frames": 0,
        "all_visible_arm": False,
        "parity": {
            "mips_bitexact": True,
            "verdict_equal": True,
            "false_positives": 0,
            "pyramid_digest": _dg("e"),
            "host_pyramid_digest": _dg("e"),
            "verdict_digest": _dg("f"),
            "host_verdict_digest": _dg("f"),
        },
    }


def _good_merged(allvis: bool = False) -> dict:
    hz = _good_hzb_block()
    hz["all_visible_arm"] = allvis
    return {
        "schema": MERGED_SCHEMA_ID,
        "gate": GATE_KEY,
        "frames": 2,
        "warmup": 1,
        "frames_completed": 3,
        "exit_reason": "frames_done",
        "digest_seq": [_dg("a"), _dg("b"), _dg("c")],
        "render_digest": _dg("d"),
        "real_render_frame_ms": 7.7,
        "features": {"textures": True, "slab": True, "dyn": True, "full": True,
                     "static_camera": False, "hzb": True, "skin": True},
        "hzb": hz,
        "skin": _good_skin_block(),
        "host_parity": None,
    }


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    # 红绿臂①:skin 口径判。
    expect(skin_caliber_judge(_good_skin_block()) == [], "GREEN:skin 口径正例")
    b = _good_skin_block()
    b["vertex_parity"]["all_bitexact"] = False
    expect(skin_caliber_judge(b), "RED:① 逐顶点非位级必红")
    b = _good_skin_block()
    b["verify_frames"][1]["pass"] = False
    expect(skin_caliber_judge(b), "RED:② 核验帧未过必红")
    b = _good_skin_block()
    b["verify_frames"][0]["vertex_max_abs"] = 1e-7
    expect(skin_caliber_judge(b), "RED:帧级 max_abs ≠ 0 必红")
    b = _good_skin_block()
    b["verify_frames"] = []
    expect(skin_caliber_judge(b), "RED:零核验帧（空接线冒充）必红")
    b = _good_skin_block()
    b["motion_gate"]["host_motion_max_px"] = 0.5
    expect(skin_caliber_judge(b), "RED:窗级真动门破必红")
    b = _good_skin_block()
    b["mv_gap"]["rigid_active_frames"] = 0
    expect(skin_caliber_judge(b), "RED:类 2 零激活必红")
    b = _good_skin_block()
    b["all_pass"] = False
    expect(skin_caliber_judge(b), "RED:all_pass 聚合旗标破必红")
    expect(skin_caliber_judge(None), "RED:skin 块缺失必红")
    # 红绿臂②:hzb 口径判。
    expect(hzb_caliber_judge(_good_hzb_block()) == [], "GREEN:hzb 口径正例")
    b = _good_hzb_block()
    b["parity"]["mips_bitexact"] = False
    expect(hzb_caliber_judge(b), "RED:mips 非位级必红")
    b = _good_hzb_block()
    b["parity"]["verdict_equal"] = False
    expect(hzb_caliber_judge(b), "RED:判定序列异必红")
    b = _good_hzb_block()
    b["parity"]["false_positives"] = 1
    expect(hzb_caliber_judge(b), "RED:假阳性 1 必红")
    b = _good_hzb_block()
    b["parity"]["host_pyramid_digest"] = _dg("9")
    expect(hzb_caliber_judge(b), "RED:金字塔 digest vs host 互核破必红")
    b = _good_hzb_block()
    b["occluded_p1"] = 0
    expect(hzb_caliber_judge(b), "RED:零剔除（空接线冒充）必红")
    b = _good_hzb_block()
    b["flipped_p2"] = -1
    expect(hzb_caliber_judge(b), "RED:负计数必红")
    b = _good_hzb_block()
    del b["parity"]
    b["parity"] = None
    expect(hzb_caliber_judge(b), "RED:parity 缺失（probe 未成）必红")
    expect(hzb_caliber_judge(None), "RED:hzb 块缺失必红")
    # 红绿臂③:合并腿公共判。
    expect(merged_leg_judge(_good_merged(), 2, 1, "t", False) == [], "GREEN:合并腿正例")
    expect(merged_leg_judge(dict(_good_merged(), schema=HZB_SCHEMA_ID), 2, 1, "t", False),
           "RED:schema 混用单开面必红")
    expect(merged_leg_judge(dict(_good_merged(), gate=HZB_GATE_KEY), 2, 1, "t", False),
           "RED:gate 字面不符必红")
    expect(merged_leg_judge(dict(_good_merged(), frames_completed=2), 2, 1, "t", False),
           "RED:缺帧必红")
    d = _good_merged()
    d["features"]["skin"] = False
    expect(merged_leg_judge(d, 2, 1, "t", False), "RED:features.skin ≠ true 必红")
    d = _good_merged()
    d["host_parity"] = {"in_tol": True}
    expect(merged_leg_judge(d, 2, 1, "t", False), "RED:host_parity 非 null（诚实登记面破）必红")
    expect(merged_leg_judge(_good_merged(allvis=True), 2, 1, "t", False),
           "RED:all_visible_arm 与腿别不符必红")
    expect(merged_leg_judge(_good_merged(allvis=True), 2, 1, "t", True) == [],
           "GREEN:allvis 腿标记正例")
    d = _good_merged()
    d["digest_seq"] = ["not-a-digest"] * 3
    expect(merged_leg_judge(d, 2, 1, "t", False), "RED:digest 形态破必红")
    # 红绿臂④:digest 序列判（中性/双跑共用面）。
    expect(seqs_bitexact([_dg("a")], [_dg("a")]), "GREEN:双臂位级正例")
    expect(not seqs_bitexact([_dg("a")], [_dg("b")]), "RED:漂移必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(not seqs_bitexact([_dg("a")], [_dg("a"), _dg("b")]), "RED:长度不齐必红")
    # 红绿臂⑤:单开臂不降级判。
    hz_doc = {"schema": HZB_SCHEMA_ID, "gate": HZB_GATE_KEY, "hzb": _good_hzb_block()}
    expect(single_hzb_judge(hz_doc, "t") == [], "GREEN:hzb 单开腿正例")
    expect(single_hzb_judge(dict(hz_doc, schema="x"), "t"), "RED:hzb 单开 schema 破必红")
    sk_doc = {"schema": SKIN_SCHEMA_ID, "gate": SKIN_GATE_KEY, "skin": _good_skin_block()}
    expect(single_skin_judge(sk_doc, "t") == [], "GREEN:skin 单开腿正例")
    bad_sk = {"schema": SKIN_SCHEMA_ID, "gate": SKIN_GATE_KEY, "skin": dict(_good_skin_block(), all_pass=False)}
    expect(single_skin_judge(bad_sk, "t"), "RED:skin 单开聚合旗标破必红")
    # 红绿臂⑥:frame_ms 健全判。
    expect(frame_ms_sane(3.5, 4.1, 5.0), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0, 5.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan"), 5.0), "RED:NaN 必红")
    # 结构互核:facts 闭集 + 调用契约字面。
    expect(len(FACT_IDS) == 7, "facts 闭集 = 7")
    argv = leg_argv("x", 4, 2, "merged")
    expect("--hzb" in argv and "--skin" in argv and "--spv-hzbskin-primary" in argv,
           "merged 腿调用契约（--hzb on --skin + 合并主射线 SPV）")
    argv = leg_argv("x", 4, 2, "hzb")
    expect("--spv-hzb-primary" in argv and "--skin" not in argv,
           "hzb 单开腿调用契约（单开 primary,无 --skin）")
    argv = leg_argv("x", 4, 2, "skin")
    expect("--spv-skin-scene" in argv and "--hzb" not in argv,
           "skin 单开腿调用契约（gi_skin scene,无 --hzb）")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=7；6 红臂组 + 正例组 + 调用契约互核;纯 CPU 零 GPU）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", action="store_true", help="GPU 验收真跑（主 agent 面;本子任务禁跑）")
    ap.add_argument("--selftest", action="store_true", help="判读器红绿自检（纯 CPU）")
    ap.add_argument("--frames", type=int, default=64)
    ap.add_argument("--warmup", type=int, default=10)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.run:
        if args.frames < 32:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 32（确定性/中性对拍面下限）", file=sys.stderr)
            return 1
        return run_accept(args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
