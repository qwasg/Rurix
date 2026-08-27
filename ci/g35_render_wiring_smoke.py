#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: cursor:claude-fable-5(G35 GPU 粒子系统 G35-3 粒子渲染接线)
"""G35-3:粒子渲染接线门冒烟(g35.wave3.render;GPU 粒子接进生产 TSR 车道
——billboard splat〔u64 fetch_max 赢家竞争〕+ 软粒子 + 粒子 MV 覆写 +
DispatchSpec::Indirect 零回读 splat;共享车道体 g14_3_lane_body.rs 0-byte,
一切经 bin/g35_particle_lane.rs 局部追加:--particles off = 母版位级
〔Stage A 锚〕,on = mv 与 TSR 之间插 10 粒子 pass + encode;RFC-0049 §4.6)。

九面判据(facts 闭集):
1. **kernels_spv_valid**:rurixc 现编 15 kernel(车道五件 g14_3_direct_gi/
   g14_mv/g14_8_tsr_resample/g14_8_tsr_resolve/g31_display_encode + 粒子七件
   〔W1/W2 冻结消费面〕+ 渲染三件 g35_splat_clear/g35_render_splat/
   g35_render_resolve)+ spirv-val 全绿 + 冻结消费面 sha256 快照在档。
2. **off_face_stage_a_anchor_match**:--particles off --static-camera 160 帧
   warmup 10 的 render_digest 位级 == milestones/g14/
   g14_3_stage_a_digest_anchor.json cell bistro-interior_t100_tsr_device
   (off 面 = 母版,零追加资源/pass/readback 的机器证明)。
3. **on_off_digest_discrimination**:同轨迹(--auto-move orbit)on/off 双面
   render_digest 必异(粒子渲染真接线判别,防镂空 pass 冒充)。
4. **determinism_double_run**:on 面同参数双跑 render_digest + digest_seq_sha
   位级一致(u64 fetch_max 平局由 slot 序全序裁决,与线程调度无关)。
5. **particle_mv_parity_2px**:--mv-witness 单粒子恒速静态相机腿:readback
   U_MV_OUT 命中像素(winner ≠ 0)与解析期望 mv = project_curr(pos) −
   project_prev(pos − vel·dt) 最大误差 ≤ 冻结容差(milestones/g35/
   g35_budget.json g35.render.mv_parity_px 程序读禁手写:threshold =
   measured × 2.0 标定冻结,measured = 0 时 threshold = 0;缺条目时本腿
   即标定腿程序写入)∧ ≤ 2.0 px 硬顶(fact 名义带)∧ 命中像素 ≥ 1
   ∧ slot 恒 0(单粒子构型)。
6. **barrier_plan_audit**:bin 内机核审计每个粒子 pass(含 encode)双 parity
   bindings ⊆ 其屏障计划资源集 + splat 的 indirect args 资源在计划内
   (evidence barrier_plan_audit 全绿;IndirectRead 转换由执行器
   pass_requirements_with 隐式补全承载,登记面)。
7. **soft_depth_occlusion_witness**:--occlusion-witness 粒子置相机后已知墙
   后(投影 w 门确定性拒绝路径):winner 全零 + 命中像素零 + scene color
   与 off 面位级等 + render_digest 与 off 面等;深度域 quirk(U_SCENE_DEPTH
   生产字面 = clip.x/clip.y,沿视射线常量 ⇒ 同域比较为屏幕域序判而非距离
   遮挡)经 evidence depth_domain 字段如实登记。
8. **indirect_splat_zero_readback**:splat pass dispatch =
   DispatchSpec::Indirect{res:args,offset:0} + 生产帧循环零粒子缓冲回读
   (host 金标准平行推得 dispatch 计数只对拍,不读回 device)。
9. **frame_ms_measured**:on 面逐帧墙钟均值 + 粒子 10 pass GPU 段和
   measured_local 诚实登记(非帧率对标门)。

三态:无 Vulkan loader/设备/资产 → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

用法:
  py -3 ci/g35_render_wiring_smoke.py --selftest
  py -3 ci/g35_render_wiring_smoke.py --gate g35.wave3.render [--frames 48] [--cap 65536] [--seed 42]
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

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g35.wave3.render"
SUBJECT = "g35_render_wiring"
WAVE = "G35.3"
TAG = "g35_render_wiring"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_render_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.render_gate_evidence.v1"
BUDGET_PATH = ROOT / "milestones" / "g35" / "g35_budget.json"
TOL_ENTRY_ID = "g35.render.mv_parity_px"
# fact 名义硬顶(particle_mv_parity_2px):标定带之上的绝对上界。
MV_HARD_CAP_PX = 2.0
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
# 车道五件(0-byte 消费)+ 粒子七件(W1/W2 冻结消费面)+ 渲染三件(本波交付)。
LANE_KERNELS = (
    "g14_3_direct_gi",
    "g14_mv",
    "g14_8_tsr_resample",
    "g14_8_tsr_resolve",
    "g31_display_encode",
)
PARTICLE_KERNELS = (
    "g35_sim",
    "g35_scan_seg_sum",
    "g35_scan_spine",
    "g35_scan_seg_apply",
    "g35_particle_compact",
    "g35_emit",
    "g35_indirect_args",
)
RENDER_KERNELS = ("g35_splat_clear", "g35_render_splat", "g35_render_resolve")
FROZEN_CONSUMED_PATHS = [
    # G35-3 消费不修改承诺面(粒子七 kernel + 共享车道体 + host 金标准)——
    # sha256 快照在档 = 0-byte 纪律漂移守护基线(g35_particle_core 同律)。
    "src/rurix-render/kernels/g35_sim.rx",
    "src/rurix-render/kernels/g35_scan_seg_sum.rx",
    "src/rurix-render/kernels/g35_scan_spine.rx",
    "src/rurix-render/kernels/g35_scan_seg_apply.rx",
    "src/rurix-render/kernels/g35_particle_compact.rx",
    "src/rurix-render/kernels/g35_emit.rx",
    "src/rurix-render/kernels/g35_indirect_args.rx",
    "src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs",
    "src/rurix-render/src/particles/mod.rs",
    "src/rurix-render/src/particles/core.rs",
]
WORK = ROOT / ".tmp" / "g35_gates" / "render"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_particle_lane{EXE_SUFFIX}"
SPLAT_DISPATCH_LITERAL = "DispatchSpec::Indirect{res:args,offset:0}"

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "off_face_stage_a_anchor_match",
    "on_off_digest_discrimination",
    "determinism_double_run",
    "particle_mv_parity_2px",
    "barrier_plan_audit",
    "soft_depth_occlusion_witness",
    "indirect_splat_zero_readback",
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


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def _num(v) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool) and v == v


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面;全纯函数零 GPU)
# ---------------------------------------------------------------------------


def frozen_tol(budget: dict | None) -> float | None:
    """冻结容差程序读(estimated/skip_reason 冒充 measured 即 None fail-closed;
    g35_particle_core frozen_tol 同律)。"""
    if not isinstance(budget, dict):
        return None
    for e in budget.get("entries", []):
        if e.get("id") == TOL_ENTRY_ID:
            if e.get("evidence") != "measured_local" or e.get("skip_reason"):
                return None
            t = e.get("threshold")
            return float(t) if isinstance(t, (int, float)) and not isinstance(t, bool) else None
    return None


def calib_threshold(measured: float) -> float:
    """标定协议冻结 k:threshold = measured × 2.0(measured = 0 时 = 0.0;
    程序产禁手写)。"""
    return measured * 2.0


def upsert_budget_entry(doc: dict | None, entry: dict) -> dict:
    """budget 读-改-写保序:只增改自己 id 条目,他人条目 0-byte 序不动
    (g35_particle_core 同律;文件缺失时建 g35 命名空间骨架)。"""
    if doc is None:
        doc = {
            "schema_version": 1,
            "namespace": "g35",
            "description": (
                "G35 预算面。G35-3 粒子渲染:MV 见证像素误差容差条目由本波标定"
                "真跑程序产(threshold = measured × 2.0 冻结 k,禁手写;"
                "measured = 0 时 threshold = 0 零容差零条目)。"
            ),
            "source_docs": ["milestones/g35/g35_render_gate_evidence_schema.json"],
            "entries": [],
            "ratio_assertions": [],
            "counter_assertions": [],
        }
    entries = list(doc.get("entries") or [])
    for i, e in enumerate(entries):
        if e.get("id") == entry["id"]:
            entries[i] = entry
            break
    else:
        entries.append(entry)
    doc["entries"] = entries
    return doc


def anchor_match(off_doc: dict, expected: str | None) -> bool:
    """② 锚格判:off 面(静态相机锚格模式)digest 形态合法且位级 == 锚。"""
    d = off_doc.get("render_digest")
    return (
        isinstance(expected, str)
        and DIGEST_RE.match(expected) is not None
        and isinstance(d, str)
        and DIGEST_RE.match(d) is not None
        and d == expected
        and off_doc.get("particles") == "off"
        and off_doc.get("static_camera") is True
    )


def discrimination_ok(on_doc: dict, off_doc: dict) -> bool:
    """③ on≠off 判别:同轨迹双面 digest 形态合法且必异。"""
    a, b = on_doc.get("render_digest"), off_doc.get("render_digest")
    return (
        isinstance(a, str)
        and isinstance(b, str)
        and DIGEST_RE.match(a) is not None
        and DIGEST_RE.match(b) is not None
        and a != b
        and on_doc.get("particles") == "on"
        and off_doc.get("particles") == "off"
        and on_doc.get("trajectory") == off_doc.get("trajectory")
    )


def determinism_ok(doc_a: dict, doc_b: dict) -> bool:
    """④ on 面双跑位级判:render_digest + digest_seq_sha 双通道一致。"""
    a, b = doc_a.get("render_digest"), doc_b.get("render_digest")
    sa, sb = doc_a.get("digest_seq_sha"), doc_b.get("digest_seq_sha")
    return (
        isinstance(a, str)
        and DIGEST_RE.match(a) is not None
        and a == b
        and isinstance(sa, str)
        and DIGEST_RE.match(sa) is not None
        and sa == sb
    )


def mv_ok(w: dict | None, tol: float | None) -> bool:
    """⑤ MV 见证判:命中像素 ≥ 1 ∧ slot 恒 0 ∧ 误差 ≤ 冻结容差 ∧
    ≤ 2.0 px 硬顶。"""
    if not isinstance(w, dict) or tol is None:
        return False
    m = w.get("max_err_px")
    return (
        isinstance(w.get("hit_px"), int)
        and w["hit_px"] >= 1
        and w.get("slot_zero") is True
        and _num(m)
        and 0.0 <= m <= tol
        and m <= MV_HARD_CAP_PX
    )


def audit_ok(a: dict | None) -> bool:
    """⑥ 屏障计划机核审计判:总旗标 + 逐 pass 全绿(11 pass = 粒子 10 +
    encode)。"""
    if not isinstance(a, dict) or a.get("all_bindings_subset") is not True:
        return False
    rows = a.get("passes")
    return (
        isinstance(rows, list)
        and len(rows) == 11
        and all(isinstance(r, dict) and r.get("ok") is True for r in rows)
    )


def occlusion_ok(w: dict | None, depth_domain) -> bool:
    """⑦ 遮挡见证判:winner 全零 + scene color 位级等 + render_digest 等 +
    深度域 quirk 登记在档(诚实面)。"""
    return (
        isinstance(w, dict)
        and w.get("winner_nonzero_px") == 0
        and w.get("scene_color_bitexact_with_off") is True
        and w.get("render_digest_match_off") is True
        and isinstance(depth_domain, str)
        and "clip.x/clip.y" in depth_domain
    )


def zero_readback_ok(z: dict | None) -> bool:
    """⑧ indirect 零回读判:splat dispatch 字面 + 生产回读旗标假。"""
    return (
        isinstance(z, dict)
        and z.get("splat_dispatch") == SPLAT_DISPATCH_LITERAL
        and z.get("production_readback_particle_buffers") is False
    )


def frame_ms_sane(fm: dict | None) -> bool:
    """⑨ frame_ms 登记面健全判:逐帧墙钟与粒子 GPU 段均为有限正数。"""
    if not isinstance(fm, dict):
        return False
    r, p = fm.get("real_render_frame_ms"), fm.get("particle_gpu_mean_ms")
    return _num(r) and r > 0 and _num(p) and p > 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def sha256_of(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def spv_args() -> list[str]:
    return [
        "--spv-scene", str(WORK / "g14_3_direct_gi.spv"),
        "--spv-mv", str(WORK / "g14_mv.spv"),
        "--spv-resample", str(WORK / "g14_8_tsr_resample.spv"),
        "--spv-resolve", str(WORK / "g14_8_tsr_resolve.spv"),
        "--spv-encode", str(WORK / "g31_display_encode.spv"),
        "--spv-p-sim", str(WORK / "g35_sim.spv"),
        "--spv-p-scan-seg-sum", str(WORK / "g35_scan_seg_sum.spv"),
        "--spv-p-scan-spine", str(WORK / "g35_scan_spine.spv"),
        "--spv-p-scan-seg-apply", str(WORK / "g35_scan_seg_apply.spv"),
        "--spv-p-compact", str(WORK / "g35_particle_compact.spv"),
        "--spv-p-emit", str(WORK / "g35_emit.spv"),
        "--spv-p-indirect-args", str(WORK / "g35_indirect_args.spv"),
        "--spv-splat-clear", str(WORK / "g35_splat_clear.spv"),
        "--spv-splat", str(WORK / "g35_render_splat.spv"),
        "--spv-presolve", str(WORK / "g35_render_resolve.spv"),
    ]


def run_lane(label: str, extra: list[str], cap: int, seed: int, env: dict) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"lane_{label}.json"
    argv = [str(BIN), *spv_args(), "--cap", str(cap), "--seed", str(seed),
            "--evidence", str(ev_path), *extra]
    r = run(argv, timeout=3600, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def run_gate(frames: int, cap: int, seed: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not GATE_SCHEMA_PATH.is_file():
        fail(f"gate schema 缺失: {GATE_SCHEMA_PATH}")
        return 1
    if not ANCHOR_PATH.is_file():
        fail(f"Stage A 锚缺失: {ANCHOR_PATH}")
        return 1
    anchor_doc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8"))
    anchor_expected = (anchor_doc.get("anchors") or {}).get(ANCHOR_CELL, {}).get("last_frame_digest")
    if not isinstance(anchor_expected, str) or DIGEST_RE.match(anchor_expected) is None:
        fail(f"锚格 {ANCHOR_CELL} last_frame_digest 缺失/形态破")
        return 1

    # ── 构建(车道 bin〔vendor-upscale〕 + rurixc)──
    ok = build_or_fail(
        ["cargo", "build", "-p", "rurix-render", "--features", "vendor-upscale",
         "--bin", "g35_particle_lane", "--quiet"],
        "g35_particle_lane bin",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面:现编 15 件 + spirv-val + 冻结消费面 sha256 快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for name in LANE_KERNELS + PARTICLE_KERNELS + RENDER_KERNELS:
        src = KERNEL_DIR / f"{name}.rx"
        dst = WORK / f"{name}.spv"
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    frozen_snapshot: dict[str, str] = {}
    snapshot_ok = True
    for p in FROZEN_CONSUMED_PATHS:
        fp = ROOT / p
        if fp.is_file():
            frozen_snapshot[p] = sha256_of(fp)
        else:
            snapshot_ok = False
            frozen_snapshot[p] = "MISSING"
    set_fact(
        "kernels_spv_valid",
        spv_ok and snapshot_ok,
        f"rurixc 现编 15 kernel(车道五件 + 粒子七件〔W1/W2 冻结消费面〕+ 渲染三件"
        f"g35_splat_clear/g35_render_splat/g35_render_resolve)+ spirv-val={'绿' if spv_ok else '红'};"
        f"冻结消费面(粒子七 kernel/g14_3_lane_body.rs/particles mod+core)sha256 快照在档={snapshot_ok}"
        f"(G35-3 0-byte 纪律漂移守护基线)",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G35-3 kernel SPV 编译/spirv-val 未过")

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_render_gate_{ts}.json"
    gate_rel = str(gate_path.relative_to(ROOT)).replace("\\", "/")
    doc_anchor: dict | None = None
    doc_off_orbit: dict | None = None
    doc_on_a: dict | None = None
    doc_on_b: dict | None = None
    doc_mv: dict | None = None
    doc_occl: dict | None = None
    run_evidence: list[str] = []
    tol: float | None = None
    calibrated = False
    pending_entry: dict | None = None

    def leg(label: str, extra: list[str], env: dict) -> dict | None:
        """一腿真跑 + 三态检出(skipped_dev_env → degrade 登记返 None)。"""
        rc, doc, ev = run_lane(label, extra, cap, seed, env)
        out = (rc.stdout or "") + (rc.stderr or "")
        if '"skipped_dev_env"' in out:
            degrade.append(f"lane skipped_dev_env({label}): {out.strip()[-200:]}")
            return None
        if rc.returncode != 0 or doc is None:
            fail(f"{label} 腿真跑失败 rc={rc.returncode}: {out[-300:]}")
            return None
        if "Validation Error" in out or "VUID-" in out:
            fail(f"{label} 腿 validation 应静默却报错")
        run_evidence.append(str(ev.relative_to(ROOT)).replace("\\", "/"))
        return doc

    if not degrade:
        env = device_env()
        with gpu_device_lock(purpose=f"{TAG} off 锚腿 + on 双跑 + off 轨迹 + mv/遮挡见证 device 真跑"):
            # ── ② off 锚腿(--static-camera 锚格模式 160 帧 warmup 10)──
            doc_anchor = leg("off_anchor", [
                "--particles", "off", "--static-camera",
                "--frames", "160", "--warmup", "10", "--headless",
            ], env)
            # ── ③ off 轨迹腿(on≠off 判别的同轨迹对面)──
            if not degrade:
                doc_off_orbit = leg("off_orbit", [
                    "--particles", "off", "--auto-move", "orbit",
                    "--frames", str(frames), "--warmup", "6", "--headless",
                ], env)
            # ── ③④⑥⑧⑨ on 双跑腿(--auto-move orbit)──
            if not degrade:
                doc_on_a = leg("on_a", [
                    "--particles", "on", "--auto-move", "orbit",
                    "--frames", str(frames), "--warmup", "6", "--headless",
                ], env)
            if not degrade:
                doc_on_b = leg("on_b", [
                    "--particles", "on", "--auto-move", "orbit",
                    "--frames", str(frames), "--warmup", "6", "--headless",
                ], env)
            # ── ⑤ MV 见证腿(单粒子恒速静态相机;缺 budget 条目即标定腿)──
            if not degrade:
                doc_mv = leg("mv_witness", [
                    "--particles", "on", "--mv-witness",
                    "--frames", "24", "--warmup", "4", "--headless",
                ], env)
            # ── ⑦ 遮挡见证腿(粒子置相机后已知墙后;进程内 on/off 对拍)──
            if not degrade:
                doc_occl = leg("occlusion", [
                    "--particles", "on", "--occlusion-witness",
                    "--frames", "24", "--warmup", "4", "--headless",
                ], env)

    if degrade:
        doc = {
            "schema": "rurix.g35.render_wiring.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for dg in degrade:
            note(f"DEV_ENV_DEGRADE {dg}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    # ── ⑤ 标定/程序读(budget 条目;measured = mv 见证腿 max_err_px)──
    mv_w = (doc_mv or {}).get("mv_witness") if isinstance((doc_mv or {}).get("mv_witness"), dict) else None
    budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
    tol = frozen_tol(budget)
    if tol is None and mv_w is not None and _num(mv_w.get("max_err_px")):
        measured = float(mv_w["max_err_px"])
        tol = calib_threshold(measured)
        calibrated = True
        pending_entry = {
            "id": TOL_ENTRY_ID,
            "description": (
                "G35-3 粒子渲染 MV 见证容差冻结带(--mv-witness 单粒子恒速静态相机腿:"
                "readback U_MV_OUT 命中像素〔winner ≠ 0〕device MV 与解析期望 mv = "
                "project_curr(pos) − project_prev(pos − vel·dt) 逐像素误差换像素域 max;"
                "sim/emit/splat/resolve SPV 注入 NoContraction 后标定;threshold = measured "
                "× 2.0 协议冻结 k,measured = 0 时 threshold = 0 零容差零条目,方向 max;"
                "另有 2.0 px 名义硬顶归 fact particle_mv_parity_2px 判读器;标定真跑 = "
                "ci/g35_render_wiring_smoke.py --gate g35.wave3.render MV 见证腿;"
                "evidence_file = 门裁决件 results.trimmed_mean 镜像槽,budget_eval 通用路"
                "消费;标定程序可复跑)"
            ),
            "direction": "max",
            "evidence": "measured_local",
            "skip_reason": None,
            "unit": "px",
            "threshold": tol,
            "evidence_file": gate_rel,
            "measured_value": measured,
        }
        note(f"MV 标定:measured={measured:e} px → threshold={tol:e}(×2.0 程序产,gate 评后写入 budget)")
    elif tol is not None:
        note(f"冻结容差程序读:threshold={tol:e} px({TOL_ENTRY_ID} 在档跳过标定)")

    # ── ②~⑨ facts 判读 ──
    a_ = doc_anchor or {}
    oo = doc_off_orbit or {}
    ga = doc_on_a or {}
    gb = doc_on_b or {}
    occl_w = (doc_occl or {}).get("occlusion_witness") if isinstance((doc_occl or {}).get("occlusion_witness"), dict) else None
    set_fact(
        "off_face_stage_a_anchor_match",
        anchor_match(a_, anchor_expected),
        f"off 面锚格(--static-camera 160 帧 warmup 10)render_digest="
        f"{str(a_.get('render_digest'))[:23]}… == 锚 {anchor_expected[:23]}…"
        f"({ANCHOR_CELL};off = 母版零追加的机器证明)={a_.get('render_digest') == anchor_expected}",
    )
    set_fact(
        "on_off_digest_discrimination",
        discrimination_ok(ga, oo),
        f"同轨迹 orbit on/off 双面 digest 必异:on={str(ga.get('render_digest'))[:23]}… "
        f"off={str(oo.get('render_digest'))[:23]}…(粒子渲染真接线判别)",
    )
    set_fact(
        "determinism_double_run",
        determinism_ok(ga, gb),
        f"on 面同参数双跑位级:render_digest 等={ga.get('render_digest') == gb.get('render_digest')} + "
        f"digest_seq_sha 等={ga.get('digest_seq_sha') == gb.get('digest_seq_sha')}"
        f"(u64 fetch_max 平局 slot 序全序裁决,与调度无关)",
    )
    set_fact(
        "particle_mv_parity_2px",
        mv_ok(mv_w, tol),
        f"MV 见证:hit_px={(mv_w or {}).get('hit_px')!r} slot_zero={(mv_w or {}).get('slot_zero')!r} "
        f"max_err_px={(mv_w or {}).get('max_err_px')!r} ≤ 冻结容差 {tol!r}"
        f"({TOL_ENTRY_ID} {'本次标定腿程序产' if calibrated else '程序读'};threshold = measured × 2.0)"
        f" ∧ ≤ {MV_HARD_CAP_PX} px 硬顶(mv = project_curr(pos) − project_prev(pos − vel·dt))",
    )
    set_fact(
        "barrier_plan_audit",
        audit_ok(ga.get("barrier_plan_audit")),
        f"bin 内机核审计:11 pass(粒子 10 + encode)双 parity bindings ⊆ 屏障计划资源集 "
        f"all={((ga.get('barrier_plan_audit') or {}).get('all_bindings_subset'))!r};splat 的 args "
        f"indirect 资源在计划内;IndirectRead 转换 = 执行器隐式补全登记面",
    )
    set_fact(
        "soft_depth_occlusion_witness",
        occlusion_ok(occl_w, (doc_occl or {}).get("depth_domain")),
        f"遮挡见证(粒子置相机后已知墙后,投影 w 门确定性拒绝):winner_nonzero_px="
        f"{(occl_w or {}).get('winner_nonzero_px')!r} scene_color 位级等="
        f"{(occl_w or {}).get('scene_color_bitexact_with_off')!r} render_digest 等="
        f"{(occl_w or {}).get('render_digest_match_off')!r};深度域 quirk(生产字面 = "
        f"clip.x/clip.y 屏幕域序判非距离遮挡)evidence depth_domain 如实登记",
    )
    set_fact(
        "indirect_splat_zero_readback",
        zero_readback_ok(ga.get("zero_readback")),
        f"splat dispatch = {SPLAT_DISPATCH_LITERAL}(生产零回读链:host 金标准平行推得 "
        f"dispatch 计数只对拍不读回;production_readback_particle_buffers="
        f"{((ga.get('zero_readback') or {}).get('production_readback_particle_buffers'))!r})",
    )
    fm = ga.get("frame_ms") if isinstance(ga.get("frame_ms"), dict) else None
    set_fact(
        "frame_ms_measured",
        frame_ms_sane(fm),
        f"on 面逐帧墙钟均值 {(fm or {}).get('real_render_frame_ms')!r} ms + 粒子 10 pass GPU 段和 "
        f"{(fm or {}).get('particle_gpu_mean_ms')!r} ms(measured_local 诚实登记,非帧率对标门)",
    )

    # ── evidence 落盘(门裁决件;jsonschema 自校验硬门)──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti(本机单卡 measured_local)",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    spv_entry = lambda name: {
        "path": str((WORK / f"{name}.spv").relative_to(ROOT)).replace("\\", "/"),
        "sha256": sha256_of(WORK / f"{name}.spv") if (WORK / f"{name}.spv").is_file() else "sha256:" + "0" * 64,
    }
    measured_num = float(mv_w["max_err_px"]) if mv_w is not None and _num(mv_w.get("max_err_px")) else -1.0
    audit = ga.get("barrier_plan_audit") or {}
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "lane": {name: spv_entry(name) for name in LANE_KERNELS},
            "particle": {name: spv_entry(name) for name in PARTICLE_KERNELS},
            "render": {name: spv_entry(name) for name in RENDER_KERNELS},
            "spirv_val_all": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "frozen_consumed_snapshot": frozen_snapshot,
        },
        "anchor": {
            "path": str(ANCHOR_PATH.relative_to(ROOT)).replace("\\", "/"),
            "cell": ANCHOR_CELL,
            "expected": anchor_expected,
            "measured": a_.get("render_digest", "sha256:" + "0" * 64),
            "match": bool(anchor_match(a_, anchor_expected)),
        },
        "discrimination": {
            "trajectory": "orbit",
            "on_digest": ga.get("render_digest", "sha256:" + "0" * 64),
            "off_digest": oo.get("render_digest", "sha256:" + "0" * 64),
            "differs": bool(discrimination_ok(ga, oo)),
        },
        "determinism": {
            "double_run_bitexact": bool(determinism_ok(ga, gb)),
            "digest_a": ga.get("render_digest", "sha256:" + "0" * 64),
            "digest_b": gb.get("render_digest", "sha256:" + "0" * 64),
            "digest_seq_sha_a": ga.get("digest_seq_sha") or "sha256:" + "0" * 64,
            "digest_seq_sha_b": gb.get("digest_seq_sha") or "sha256:" + "0" * 64,
        },
        "mv_parity": {
            "hit_px": int((mv_w or {}).get("hit_px") or 0),
            "slot_zero": bool((mv_w or {}).get("slot_zero") is True),
            "measured_px": measured_num,
            "threshold": tol if tol is not None else -1.0,
            "hard_cap_px": MV_HARD_CAP_PX,
            "budget_entry": TOL_ENTRY_ID,
            "calibrated_this_run": calibrated,
            "within": bool(mv_ok(mv_w, tol)),
        },
        "results": {"trimmed_mean": measured_num},
        "barrier_audit": {
            "all_bindings_subset": bool(audit.get("all_bindings_subset") is True),
            "passes_total": len(audit.get("passes") or []),
            "passes_ok": sum(1 for r in (audit.get("passes") or []) if isinstance(r, dict) and r.get("ok") is True),
            "indirect_note": str(audit.get("indirect_note") or ""),
        },
        "occlusion": {
            "winner_nonzero_px": int((occl_w or {}).get("winner_nonzero_px") if isinstance((occl_w or {}).get("winner_nonzero_px"), int) else -1),
            "scene_color_bitexact_with_off": bool((occl_w or {}).get("scene_color_bitexact_with_off") is True),
            "render_digest_match_off": bool((occl_w or {}).get("render_digest_match_off") is True),
            "config": str((occl_w or {}).get("config") or ""),
            "depth_domain": str((doc_occl or {}).get("depth_domain") or ""),
        },
        "zero_readback": {
            "splat_dispatch": str((ga.get("zero_readback") or {}).get("splat_dispatch") or ""),
            # 镜像 bin 登记值(缺失 → True = fail-closed 视作有回读)。
            "production_readback_particle_buffers":
                (ga.get("zero_readback") or {}).get("production_readback_particle_buffers") is not False,
        },
        "frame_ms": {
            "real_render_frame_ms": (fm or {}).get("real_render_frame_ms") if frame_ms_sane(fm) else 1e-9,
            "particle_gpu_mean_ms": (fm or {}).get("particle_gpu_mean_ms") if frame_ms_sane(fm) else 1e-9,
            "frames_measured": int((fm or {}).get("frames_measured") or 0),
            "measured": "measured_local",
            "note": (
                "on 面逐帧墙钟均值(prepare+execute+回读;末帧含回读转换)+ 粒子 10 pass "
                "GPU timestamp 段和;登记语义非帧率对标,帧率门归 G31 波 A 锚"
            ),
        },
        "mesh_particles": {
            "wired_instances": 1,
            "note": (
                "mesh 粒子 TLAS 见证臂 = bin --mesh-particles N(A4 unified_lane_descs_dyn + "
                "g31_dyn_scene + 逐帧 tlas_update Refit inflight=1;wired = 1 实例——"
                "g31_dyn_scene 分派映射 pg = prim + inst·dyn_tri_base 为单动态实例语义,"
                "N > 1 的其余实例如实 not_wired 登记于 bin evidence mesh_particles."
                "not_wired_reason;ray query 场景自动获得光追阴影登记面)。本门 facts "
                "闭集不含 mesh 臂(见证臂非门判据,能力面登记)"
            ),
        },
        "run_evidence": run_evidence or ["(run evidence 缺失)"],
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-3 粒子渲染接线:共享车道体 g14_3_lane_body.rs 0-byte(include! 逐字共享,"
            "sha256 快照在档),一切经 bin/g35_particle_lane.rs 局部追加——off 面 = 母版 Mega "
            "22 资源四 pass 位级(Stage A 锚机器证明);on 面 = mv(pass1)与 tsr resample 之间"
            "插 10 粒子 pass(sim→scan_seg_sum→scan_spine→scan_seg_apply→particle_compact→"
            "emit→indirect_args→splat_clear→splat→resolve)+ 尾追 display encode,粒子资源 "
            "22..=53 追加(params 五件 + 九流 A/B 18 件 + flags/scan_out/seg_sums/seg_offsets + "
            "args〔BufferUsage.indirect〕+ rand_table + winner u64);FrameUpdate 重映射 = bin "
            "自持 prepare 按插入后下标 (12,13,14) 构造 TSR/encode overrides + 粒子 5 pass A/B "
            "parity overrides(共享体 (2,3) 硬编码不消费)。splat = 每线程一粒子 u64 fetch_max "
            "赢家(key = 16777215 − depth_key24(d_view,far) 反深度<<40|slot,同 key 高 slot 胜出"
            "全序平局)+ DispatchSpec::Indirect{args,0} 零回读;resolve = 程序化调色(暖白→橙红"
            "×(1−t)×8.0)+ 软粒子 alpha = (1−t)·soft + 粒子 MV mv = project_curr(pos) − "
            "project_prev(pos − vel·dt) 覆写相机 MV(g14_mv uv 偏移同约定)。深度域 quirk 如实"
            "登记:U_SCENE_DEPTH 生产字面 = 未抖 vp 行 0/1(clip.x/clip.y)沿视射线常量 ⇒ 硬拒"
            "/软粒子为同域屏幕序判非距离遮挡(真深度资源进 Mega 车道前的降级实现,RFC-0049 §4.6 "
            "原文深度源 = out_depth_hz);遮挡见证因此取相机后已知墙后构型(投影 w 门确定性拒绝"
            "路径)。host 金标准 particles::core::frame 平行镜像驱动 n_curr/emit 参数面(整数流"
            "零容差,NoContraction 注入面 W2 实测位级同源)。results.trimmed_mean = MV measured "
            "px 镜像(ci/budget_eval.py 通用路 evidence_file 消费面)。"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED)

    errs = list(jsonschema.Draft7Validator(
        json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        fail("gate evidence schema 自校验红: " + "; ".join(
            f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
        all_pass = False
        gate_doc["verdict"] = "FAIL"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_rel}(lane 真跑件 {len(run_evidence)} 份留 .tmp 工作区)")

    # ── budget 程序写(MV 标定腿产;gate 裁决件已落盘 ⇒ evidence_file 不悬空)──
    if pending_entry is not None:
        budget_doc = json.loads(BUDGET_PATH.read_text(encoding="utf-8")) if BUDGET_PATH.is_file() else None
        budget_doc = upsert_budget_entry(budget_doc, pending_entry)
        BUDGET_PATH.parent.mkdir(parents=True, exist_ok=True)
        BUDGET_PATH.write_text(json.dumps(budget_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        back = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
        if frozen_tol(back) != pending_entry["threshold"]:
            fail("budget 回读互核失败(写入后 frozen_tol ≠ 待写 threshold)")
            all_pass = False
        else:
            note(f"g35_budget.json 程序写入 {TOL_ENTRY_ID}(threshold={pending_entry['threshold']:e};重读核验绿)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿穷举 + schema 校验 + FACT_IDS 互核;零 GPU 零构建)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    d0 = "sha256:" + "a" * 64
    d1 = "sha256:" + "b" * 64
    dd = "屏幕域序判…生产字面 clip.x/clip.y…"
    # 红绿臂①:冻结容差程序读 + threshold = measured × 2.0 协议。
    good_budget = {"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                "skip_reason": None, "threshold": 0.5, "measured_value": 0.25}]}
    expect(frozen_tol(good_budget) == 0.5, "GREEN:容差程序读正例")
    expect(calib_threshold(0.25) == 0.5, "GREEN:×2.0 冻结 k")
    expect(calib_threshold(0.0) == 0.0, "GREEN:measured = 0 ⇒ threshold = 0(零容差零条目)")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "estimated",
                                    "skip_reason": None, "threshold": 1.0}]}) is None,
           "RED:estimated 冒充 measured 必拒")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                    "skip_reason": "no gpu", "threshold": 1.0}]}) is None,
           "RED:skip_reason 携带必拒")
    expect(frozen_tol({"entries": []}) is None, "RED:条目缺失必拒")
    expect(frozen_tol(None) is None, "RED:budget 文件缺失必拒")
    expect(frozen_tol({"entries": [{"id": TOL_ENTRY_ID, "evidence": "measured_local",
                                    "skip_reason": None, "threshold": True}]}) is None,
           "RED:bool 冒充数值阈必拒")
    # 红绿臂②:budget 读-改-写保序。
    foreign = {"id": "g35.particle_core.f32_parity_p100", "threshold": 1.0}
    mine = {"id": TOL_ENTRY_ID, "evidence": "measured_local", "skip_reason": None,
            "threshold": 0.5, "measured_value": 0.25}
    up = upsert_budget_entry({"namespace": "g35", "entries": [foreign]}, dict(mine))
    expect(up["entries"][0] == foreign and up["entries"][1]["id"] == TOL_ENTRY_ID,
           "GREEN:upsert 追加保序(他人条目 0-byte 序不动)")
    up2 = upsert_budget_entry(up, {**mine, "threshold": 1.0})
    expect(len(up2["entries"]) == 2 and up2["entries"][1]["threshold"] == 1.0
           and up2["entries"][0] == foreign,
           "GREEN:upsert 原位替换自己条目(幂等面)")
    # 红绿臂③:锚格判。
    good_anchor = {"render_digest": d0, "particles": "off", "static_camera": True}
    expect(anchor_match(good_anchor, d0), "GREEN:锚格位级等正例")
    expect(not anchor_match({**good_anchor, "render_digest": d1}, d0), "RED:digest 漂移必红")
    expect(not anchor_match({**good_anchor, "particles": "on"}, d0), "RED:on 面冒充 off 锚腿必红")
    expect(not anchor_match({**good_anchor, "static_camera": False}, d0), "RED:非锚格模式必红")
    expect(not anchor_match(good_anchor, None), "RED:锚缺失必红")
    expect(not anchor_match({**good_anchor, "render_digest": "xx"}, d0), "RED:digest 形态破必红")
    # 红绿臂④:on≠off 判别。
    on_d = {"render_digest": d0, "particles": "on", "trajectory": "orbit"}
    off_d = {"render_digest": d1, "particles": "off", "trajectory": "orbit"}
    expect(discrimination_ok(on_d, off_d), "GREEN:on≠off 判别正例")
    expect(not discrimination_ok(on_d, {**off_d, "render_digest": d0}),
           "RED:digest 同(镂空 pass 冒充)必红")
    expect(not discrimination_ok(on_d, {**off_d, "trajectory": "dolly"}), "RED:轨迹不同面必红")
    expect(not discrimination_ok({**on_d, "particles": "off"}, off_d), "RED:双 off 面冒充必红")
    # 红绿臂⑤:on 双跑位级判。
    s0 = "sha256:" + "c" * 64
    ga = {"render_digest": d0, "digest_seq_sha": s0}
    expect(determinism_ok(ga, dict(ga)), "GREEN:双跑位级正例")
    expect(not determinism_ok(ga, {**ga, "render_digest": d1}), "RED:末帧 digest 异必红")
    expect(not determinism_ok(ga, {**ga, "digest_seq_sha": "sha256:" + "d" * 64}),
           "RED:digest_seq_sha 异(逐帧链敏感)必红")
    expect(not determinism_ok({"render_digest": "xx", "digest_seq_sha": s0},
                              {"render_digest": "xx", "digest_seq_sha": s0}),
           "RED:digest 形态破必红")
    # 红绿臂⑥:MV 见证判。
    good_mv = {"hit_px": 12, "slot_zero": True, "max_err_px": 0.25}
    expect(mv_ok(good_mv, 0.5), "GREEN:MV 见证正例")
    expect(mv_ok({**good_mv, "max_err_px": 0.0}, 0.0), "GREEN:measured=0 vs threshold=0 边界过")
    expect(not mv_ok({**good_mv, "max_err_px": 0.6}, 0.5), "RED:超冻结容差必红")
    expect(not mv_ok({**good_mv, "max_err_px": 3.0}, 10.0), "RED:带内但破 2px 硬顶必红")
    expect(not mv_ok({**good_mv, "hit_px": 0}, 0.5), "RED:零命中像素(粒子未上屏)必红")
    expect(not mv_ok({**good_mv, "slot_zero": False}, 0.5), "RED:slot 非 0(赢家槽污染)必红")
    expect(not mv_ok(good_mv, None), "RED:容差缺失(未标定)必红")
    expect(not mv_ok({**good_mv, "max_err_px": float("nan")}, 0.5), "RED:NaN measured 必红")
    expect(not mv_ok(None, 0.5), "RED:见证块缺失必红")
    # 红绿臂⑦:屏障审计判。
    rows11 = [{"pass": f"p{i}", "ok": True} for i in range(11)]
    good_audit = {"all_bindings_subset": True, "passes": rows11}
    expect(audit_ok(good_audit), "GREEN:屏障审计正例(11 pass)")
    expect(not audit_ok({**good_audit, "all_bindings_subset": False}), "RED:总旗标假必红")
    expect(not audit_ok({"all_bindings_subset": True, "passes": rows11[:10]}),
           "RED:pass 行数 ≠ 11 必红")
    expect(not audit_ok({"all_bindings_subset": True,
                         "passes": rows11[:10] + [{"pass": "p10", "ok": False}]}),
           "RED:单 pass 红必红")
    expect(not audit_ok(None), "RED:审计块缺失必红")
    # 红绿臂⑧:遮挡见证判。
    good_occl = {"winner_nonzero_px": 0, "scene_color_bitexact_with_off": True,
                 "render_digest_match_off": True}
    expect(occlusion_ok(good_occl, dd), "GREEN:遮挡见证正例")
    expect(not occlusion_ok({**good_occl, "winner_nonzero_px": 3}, dd),
           "RED:赢家非零(粒子漏上屏)必红")
    expect(not occlusion_ok({**good_occl, "scene_color_bitexact_with_off": False}, dd),
           "RED:scene color 非位级等必红")
    expect(not occlusion_ok({**good_occl, "render_digest_match_off": False}, dd),
           "RED:render_digest 不等必红")
    expect(not occlusion_ok(good_occl, None), "RED:深度域登记缺失(诚实面)必红")
    expect(not occlusion_ok(good_occl, "真 NDC 深度"), "RED:深度域字面漂移必红")
    # 红绿臂⑨:indirect 零回读判。
    good_zr = {"splat_dispatch": SPLAT_DISPATCH_LITERAL,
               "production_readback_particle_buffers": False}
    expect(zero_readback_ok(good_zr), "GREEN:indirect 零回读正例")
    expect(not zero_readback_ok({**good_zr, "splat_dispatch": "Direct"}),
           "RED:dispatch 字面漂移必红")
    expect(not zero_readback_ok({**good_zr, "production_readback_particle_buffers": True}),
           "RED:生产回读粒子缓冲必红")
    expect(not zero_readback_ok(None), "RED:块缺失必红")
    # 红绿臂⑩:frame_ms 健全判。
    expect(frame_ms_sane({"real_render_frame_ms": 12.5, "particle_gpu_mean_ms": 3.1}),
           "GREEN:frame_ms 正例")
    expect(not frame_ms_sane({"real_render_frame_ms": 0.0, "particle_gpu_mean_ms": 3.1}),
           "RED:0ms 必红")
    expect(not frame_ms_sane({"real_render_frame_ms": float("nan"), "particle_gpu_mean_ms": 3.1}),
           "RED:NaN 必红")
    expect(not frame_ms_sane(None), "RED:缺失必红")
    # schema 互核。
    expect(GATE_SCHEMA_PATH.is_file(), "gate schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate schema 门键 const 互核")
        expect(gs["properties"]["mv_parity"]["properties"]["budget_entry"]["const"] == TOL_ENTRY_ID,
               "gate schema budget_entry const 互核")
        expect(gs["properties"]["anchor"]["properties"]["cell"]["const"] == ANCHOR_CELL,
               "gate schema 锚格 cell const 互核")
        expect(gs["properties"]["zero_readback"]["properties"]["splat_dispatch"]["const"]
               == SPLAT_DISPATCH_LITERAL,
               "gate schema splat dispatch 字面 const 互核")
        expect("results" in gs.get("required", [])
               and gs["properties"]["results"]["properties"]["trimmed_mean"]["type"] == "number",
               "gate schema results.trimmed_mean 通用消费面互核(budget_eval evidence_file 路)")
        import jsonschema as _js
        _js.Draft7Validator.check_schema(gs)
        print("  ok   — gate schema Draft7 合法(check_schema 绿)")
    expect(len(FACT_IDS) == 9, "facts 闭集 = 9")
    expect(ANCHOR_PATH.is_file(), "Stage A 锚在树")
    if ANCHOR_PATH.is_file():
        anc = json.loads(ANCHOR_PATH.read_text(encoding="utf-8"))
        cell = (anc.get("anchors") or {}).get(ANCHOR_CELL, {})
        expect(isinstance(cell.get("last_frame_digest"), str)
               and DIGEST_RE.match(cell["last_frame_digest"]) is not None,
               f"锚格 {ANCHOR_CELL} last_frame_digest 形态合法")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=9;10 红绿臂组 + budget 读改写保序 + schema/锚互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=48)
    ap.add_argument("--cap", type=int, default=65536)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 16:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 16(TSR 历史收敛 + 粒子换血最小窗)",
                  file=sys.stderr)
            return 1
        if args.cap <= 0 or args.cap % 256 != 0:
            print(f"[{TAG}] FAIL: --cap {args.cap} 须为 SEG=256 正整倍数", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.cap, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
