#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 B Task B1 HZB 遮挡剔除生产接线收尾段）
"""G31+ 波 B Task B1：HZB 遮挡剔除生产接线门冒烟（g31.waveB.hzb；
G30 承接锚 G27 行「生产接线窗」+ RFC-0044 §5.8 两阶段闭环第二段兑现；
G31_PLUS_COMMERCIAL_RENDERER_TODO §1.2 #6 行）。

接线面（src/rurix-render/src/bin/g31_window_present.rs --hzb <off|on> 闭集,
与 --fg/--slab-table 互斥、须 --tier 100）：bistro 逐 mesh 节点 BLAS 分解
（1186 实例）+ 双 TLAS（初剔表 0 相机射线 / 全量表 1 阴影射线）+ 帧内金字塔
轮换（上帧金字塔初剔 → 本帧真深度重建 → 上帧被剔集重测 = 两阶段闭环第二段）
+ g27_hzb_reduce/g27_hzb_test 两 kernel 本体 0-byte 冻结消费（bin 侧
NoContraction 注入 = mips 位级全等关键,SPV 文件 0-byte）。

判据闭集（milestones/g31/g31_hzb_wiring_evidence_schema.json 描述段逐字）：
1. **g27_gate_maintenance**：G27 门维持复跑全绿——子进程
   ci/g27_hzb_device_kernel_smoke.py --gate g27.p0.m_a.hzb_device_kernel
   rc=0 + host_section_pass=true + 六 facts（mips 位级全等/800 rect 判定序列
   全等/零假阳性/双跑位级/tamper RED/冻结面 0-byte）全 PASS。
2. **wired_parity_probe**：接线态对拍——orbit probe 腿 evidence hzb.parity
   三块：车道平铺金字塔/判定 digest 与 host 金标准位级全等 + false_positives=0
   （harness 已 fail-fast 硬门,本门登记面复核）。
3. **culling_neutrality**：剔除像素中性门（可见集一致性）——--hzb on 双跑
   vs RURIX_HZB_ALL_VISIBLE=1 全集渲染实验臂 digest_seq 逐帧位级一致
   （mismatch_count=0;剔除不改变可见像素）。
4. **determinism_double_run**：生产双跑位级一致（on_a vs on_b digest_seq
   逐帧全等;登记 notes,schema 闭集外内部硬门）。
5. **on_off_relation**：on/off 关系如实登记——分解/双 TLAS 结构 ULP 噪声,
   位级全等结构上不可达,mismatch 计数登记不设通过线（剔除中性由 ③ 钉死）。
6. **frame_ms_measured**：measured 对照——bistro 1080p 静态相机 --hzb
   off/on 各 ≥100 帧 real_render_frame_ms 真实数字（同机同窗 --hidden
   release;如实登记不设通过线,G6 无硬门纪律）。
7. **off_static_anchor_zero_drift**：off 面静态锚零漂移——off = 生产车道
   0-byte ⇒ g14_3_pipeline_perf canonical 160 帧 warmup 10 bistro-interior/
   t100/tsr_device 末帧 digest == milestones/g14/g14_3_stage_a_digest_anchor
   .json 在案锚（锚在案复用非新订,program_produced_first_run=false）。
8. **frozen_surfaces_0byte**：geometry 三文件（hzb/cull/visbuffer.rs）vs
   g26-closed + g27 两 kernel（g27_hzb_reduce/g27_hzb_test.rx）vs
   g27-closed 三处 0-byte 机核（提交面 + 工作树双面）。
9. **occlusion_culling_active**：剔除真实发生——orbit on 腿 occlusion
   计数 tested_p1≥1 且 occluded_p1≥1（零剔除即空接线冒充判红）+
   visible_mean>0。

三态：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0（不冒充
PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock 充真跑）。

evidence 纪律：门 schema 全 const 闭集 = PASS-only 面——PASS 才落
evidence/g31_hzb_wiring_<ts>.json（check_schemas 前缀路由 g31_hzb_wiring_）；
FAIL 诊断件落 .tmp/g31_gates/hzb/ 工作区不污染 evidence/ 路由面
（fail-closed：evidence/ 无件 = 门未过,不冒充）。harness 真跑件
（rurix.g31.hzb_wiring_evidence.v1 字面）无注册 schema,全留 .tmp 工作区,
数字经门裁决件蒸馏登记。

用法：
  py -3 ci/g31_hzb_wiring_smoke.py --selftest
  py -3 ci/g31_hzb_wiring_smoke.py --gate g31.waveB.hzb [--frames 100] [--warmup 10]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveB.hzb"
SUBJECT = "g31_hzb_wiring"
WAVE = "G31+.B"
TAG = "g31_hzb"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_hzb_wiring_evidence_schema.json"
SCHEMA_ID = "rurix.g31.hzb_wiring_smoke_evidence.v1"
HARNESS_SCHEMA_ID = "rurix.g31.hzb_wiring_evidence.v1"
ANCHOR_PATH = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"
ANCHOR_CELL = "bistro-interior_t100_tsr_device"
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
HZB_KERNELS = (
    "g31_hzb_primary.rx",
    "g31_hzb_shade.rx",
    "g31_hzb_pack.rx",
    "g27_hzb_reduce.rx",
    "g27_hzb_test.rx",
)
WORK = ROOT / ".tmp" / "g31_gates" / "hzb"
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
LANE_SPVS = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_PRESENT = ROOT / "target" / "release" / f"g31_window_present{EXE_SUFFIX}"
BIN_BENCH = ROOT / "target" / "release" / f"g14_3_pipeline_perf{EXE_SUFFIX}"
G27_SMOKE = ROOT / "ci" / "g27_hzb_device_kernel_smoke.py"
G27_GATE = "g27.p0.m_a.hzb_device_kernel"
G27_SUBJECT = "g27_m_a_hzb_device_kernel"
FROZEN_GEOM_BASE = "g26-closed"
FROZEN_GEOM_PATHS = [
    "src/rurix-render/src/geometry/hzb.rs",
    "src/rurix-render/src/geometry/cull.rs",
    "src/rurix-render/src/geometry/visbuffer.rs",
]
FROZEN_KERNEL_BASE = "g27-closed"
FROZEN_KERNEL_PATHS = [
    "src/rurix-render/kernels/g27_hzb_reduce.rx",
    "src/rurix-render/kernels/g27_hzb_test.rx",
]
SCENE = "bistro-interior"
TRAJECTORY = "orbit"
ORBIT_FRAMES = 8  # schema culling_neutrality frames ≥8
ORBIT_WARMUP = 2  # schema warmup ≥2;seq_len = 8+2 = 10 ≥ 10
ANCHOR_FRAMES = 160  # g14 Stage A 锚 canonical 帧数（锚收割口径）
ANCHOR_WARMUP = 10

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

# G27 维持腿 extra_facts id → 门 schema gate_maintenance 字段映射（闭集）。
G27_FACT_MAP = {
    "mips_bitexact_all_levels": "mips_bitexact",
    "rect_verdict_sequence_equal_800x2": "verdict_sequence_equal",
    "zero_false_positive_vs_exact": "zero_false_positive",
    "device_double_run_bitexact": "device_double_run_bitexact",
    "tamper_red_arm_detected": "tamper_red_arm_detected",
    "geometry_frozen_0byte": "geometry_frozen_0byte",
}

FACT_IDS = [
    "g27_gate_maintenance",
    "wired_parity_probe",
    "culling_neutrality",
    "determinism_double_run",
    "on_off_relation",
    "frame_ms_measured",
    "off_static_anchor_zero_drift",
    "frozen_surfaces_0byte",
    "occlusion_culling_active",
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


# ---------------------------------------------------------------------------
# 判读器（selftest 红绿两臂消费面）
# ---------------------------------------------------------------------------


def seqs_bitexact(a: list, b: list) -> bool:
    """同轨迹双臂 digest_seq 逐帧位级一致判据（非空 + 等长 + 逐项全等）。"""
    return len(a) == len(b) and len(a) > 0 and all(x == y for x, y in zip(a, b))


def seq_mismatch_count(a: list, b: list) -> int:
    """on/off 逐帧 mismatch 计数（长度不齐 = -1 拒判）。"""
    if len(a) != len(b):
        return -1
    return sum(1 for x, y in zip(a, b) if x != y)


def parity_judge(parity: dict) -> list[str]:
    """② 接线态对拍判（harness hzb.parity 块;返回失败串列表,空 = 绿）。"""
    fails: list[str] = []
    if not isinstance(parity, dict):
        return ["hzb.parity 非 object"]
    for k in ("mips_bitexact", "verdict_sequence_equal", "pyramid_digest_equal_host", "verdict_digest_equal_host"):
        if parity.get(k) is not True:
            fails.append(f"hzb.parity.{k} ≠ true: {parity.get(k)!r}")
    if parity.get("false_positives") != 0:
        fails.append(f"hzb.parity.false_positives ≠ 0: {parity.get('false_positives')!r}")
    for k in ("pyramid_digest", "verdict_digest"):
        d = parity.get(k)
        if not isinstance(d, str) or not DIGEST_RE.match(d):
            fails.append(f"hzb.parity.{k} 形态非法: {str(d)[:40]!r}")
    return fails


def occlusion_judge(occ: dict) -> list[str]:
    """⑨ 剔除真实发生判（hzb.occlusion 块;occluded_p1≥1 = 空接线冒充硬红）。"""
    fails: list[str] = []
    if not isinstance(occ, dict):
        return ["hzb.occlusion 非 object"]
    for k in ("tested_p1", "occluded_p1"):
        v = occ.get(k)
        if not isinstance(v, int) or isinstance(v, bool) or v < 1:
            fails.append(f"hzb.occlusion.{k} < 1（剔除未真实发生）: {v!r}")
    for k in ("offscreen", "retested_p2", "flipped_p2", "closure_frames",
              "closure_extra_submits", "closure_full_fallbacks"):
        v = occ.get(k)
        if not isinstance(v, int) or isinstance(v, bool) or v < 0:
            fails.append(f"hzb.occlusion.{k} 非负整数破: {v!r}")
    vm = occ.get("visible_mean")
    if not isinstance(vm, (int, float)) or isinstance(vm, bool) or not vm > 0:
        fails.append(f"hzb.occlusion.visible_mean 非正: {vm!r}")
    return fails


def g27_maintenance_judge(rc: int, doc: dict | None) -> tuple[dict[str, bool], list[str]]:
    """① G27 门维持判：子进程 rc + host_section_pass + 六 facts 映射全 PASS。

    返回 (六布尔映射, 失败串列表);任一 facts 缺失/非 PASS 即红（维持门
    不许残缺冒充）。
    """
    booleans = {v: False for v in G27_FACT_MAP.values()}
    fails: list[str] = []
    if rc != 0:
        fails.append(f"G27 复跑子进程 rc={rc} ≠ 0")
    if not isinstance(doc, dict):
        fails.append("G27 evidence 缺失/非 object")
        return booleans, fails
    if doc.get("host_section_pass") is not True:
        fails.append(f"G27 host_section_pass ≠ true: {doc.get('host_section_pass')!r}")
    by_id = {f.get("id"): f for f in (doc.get("extra_facts") or []) if isinstance(f, dict)}
    for fid, key in G27_FACT_MAP.items():
        f = by_id.get(fid)
        if f is None:
            fails.append(f"G27 extra_facts 缺 {fid}")
            continue
        booleans[key] = f.get("status") == "PASS"
        if not booleans[key]:
            fails.append(f"G27 fact {fid} 非 PASS: {f.get('status')!r}")
    return booleans, fails


def anchor_zero_drift(fresh: str | None, anchor: str | None) -> bool:
    """⑦ Stage A 锚零漂移判：新鲜复跑 digest == 在案锚（双非空且全等）。"""
    return (
        isinstance(fresh, str) and isinstance(anchor, str)
        and DIGEST_RE.match(fresh) is not None and DIGEST_RE.match(anchor) is not None
        and fresh == anchor
    )


def frame_ms_sane(*vals: float) -> bool:
    """⑥ frame_ms 登记面健全判：全有限正数（诚实登记非阈门）。"""
    return all(isinstance(v, (int, float)) and not isinstance(v, bool) and v == v and v > 0 for v in vals)


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决：无降级 → None（续跑）;有降级 + REQUIRE_REAL → 1（硬红）;
    有降级无 REQUIRE_REAL → 0（SKIP 非 PASS 非 FAIL）。"""
    if not degrade:
        return None
    return 1 if require_real else 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def run_present(
    label: str,
    frames: int,
    warmup: int,
    hzb_on: bool,
    trajectory: str | None,
    env: dict,
    spv_overrides: dict[str, Path] | None = None,
    timeout: int = 3600,
) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"harness_{label}.json"
    argv = [
        str(BIN_PRESENT),
        "--frames", str(frames),
        "--warmup", str(warmup),
        "--hidden",
        "--quality", "off",  # W4 默认翻转免疫:hzb 诊断臂 + off 基线腿显式 off（DEFAULT_FLIP_PLAN §2.5）
        "--evidence", str(ev_path),
    ]
    if trajectory is not None:
        argv += ["--auto-move", trajectory]
    argv += ["--hzb", "on" if hzb_on else "off"]
    if hzb_on and spv_overrides:
        for flag, p in spv_overrides.items():
            argv += [flag, str(p)]
    r = run(argv, timeout=timeout, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def harness_common_judge(
    doc: dict, frames: int, warmup: int, trajectory: str | None, label: str
) -> list[str]:
    """harness evidence 公共判。trajectory 非 None = 轨迹腿（digest_seq 全字段
    硬判——A3/HZB 轨迹面逐帧 digest_seq 字面）;trajectory None = 静态测量腿
    （A1 面 digest_seq 不在 schema;HZB 静态面 digest_seq 亦空——逐帧 digest
    仅 --auto-move 面填充,harness 帧循环 auto_move.is_some() 分支字面;静态腿
    只判 frame_ms 测量面 + 在档字段,不强加轨迹面字段闭集）。"""
    fails: list[str] = []
    total = frames + warmup
    if trajectory is not None:
        if doc.get("frames_completed") != total:
            fails.append(f"{label}: frames_completed {doc.get('frames_completed')} ≠ {total}")
        if doc.get("exit_reason") != "frames_done":
            fails.append(f"{label}: exit_reason ≠ frames_done: {doc.get('exit_reason')!r}")
        if doc.get("trajectory") != trajectory:
            fails.append(f"{label}: trajectory ≠ {trajectory}: {doc.get('trajectory')!r}")
        seq = doc.get("digest_seq")
        if not isinstance(seq, list) or len(seq) != total or any(
            not isinstance(x, str) or not DIGEST_RE.match(x) for x in seq
        ):
            fails.append(f"{label}: digest_seq 形态/长度破（≠{total}）")
        if doc.get("digest") != (seq[-1] if isinstance(seq, list) and seq else None):
            fails.append(f"{label}: digest ≠ digest_seq 末项")
    else:
        # 静态测量腿：在档字段判（A1 面无 frames_completed/trajectory;HZB 面有
        # 且 trajectory="static"——出现即判,不强制补齐）。
        if doc.get("frames") != frames or doc.get("warmup") != warmup:
            fails.append(f"{label}: frames/warmup 不符: {doc.get('frames')}/{doc.get('warmup')}")
        if "frames_completed" in doc and doc.get("frames_completed") != total:
            fails.append(f"{label}: frames_completed {doc.get('frames_completed')} ≠ {total}")
        if "exit_reason" in doc and doc.get("exit_reason") != "frames_done":
            fails.append(f"{label}: exit_reason ≠ frames_done: {doc.get('exit_reason')!r}")
        if "trajectory" in doc and doc.get("trajectory") != "static":
            fails.append(f"{label}: trajectory ≠ static: {doc.get('trajectory')!r}")
        d = doc.get("digest")
        if not isinstance(d, str) or not DIGEST_RE.match(d):
            fails.append(f"{label}: digest 形态非法: {str(d)[:40]!r}")
    rr = doc.get("real_render_frame_ms")
    if not isinstance(rr, (int, float)) or isinstance(rr, bool) or not rr > 0:
        fails.append(f"{label}: real_render_frame_ms 非正: {rr!r}")
    if doc.get("render_includes_forced_readback") is not True:
        fails.append(f"{label}: render_includes_forced_readback ≠ true")
    if (doc.get("contracts") or {}).get("consistency") != "pass":
        fails.append(f"{label}: contracts.consistency ≠ pass")
    return fails


def run_gate(frames: int, warmup: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:180]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1

    # ── 构建（release 双 bin + rurixc debug SPV 面）──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-render", "--features", "vendor-upscale",
         "--bin", "g31_window_present", "--bin", "g14_3_pipeline_perf", "--quiet"],
        "harness release",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── SPV 面：HZB kernel 五件现编 + spirv-val（g27 两件本体 0-byte 冻结消费;
    #    NoContraction = bin 侧加载期注入,SPV 文件 0-byte 不动）──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_paths: dict[str, Path] = {}
    spv_flags: dict[str, Path] = {}
    degrade: list[str] = []
    flag_of = {
        "g31_hzb_primary.rx": "--spv-hzb-primary",
        "g31_hzb_shade.rx": "--spv-hzb-shade",
        "g31_hzb_pack.rx": "--spv-hzb-pack",
        "g27_hzb_reduce.rx": "--spv-hzb-reduce",
        "g27_hzb_test.rx": "--spv-hzb-test",
    }
    for kern in HZB_KERNELS:
        src = KERNEL_DIR / kern
        spv = WORK / kern.replace(".rx", ".spv")
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(spv)], timeout=1800)
        spv_ok = r.returncode == 0 and spv.is_file()
        if spv_ok:
            val = run(["spirv-val", str(spv)], timeout=600)
            spv_ok = val.returncode == 0
        if not spv_ok:
            degrade.append(f"{kern} SPV 编译/spirv-val 未过: {(r.stdout + r.stderr)[-200:]}")
        else:
            spv_paths[kern] = spv
            spv_flags[flag_of[kern]] = spv
    missing_lane = [f for f in LANE_SPVS if not (SPV_DIR / f).is_file()]
    if missing_lane:
        degrade.append(f"车道 SPV 缺失 {missing_lane}")
    if not BISTRO_GLTF.is_file():
        degrade.append(f"bistro gltf 缺失 {BISTRO_GLTF}")

    # ── ① G27 门维持复跑（子进程自持 gpu_device_lock;RURIX_REQUIRE_REAL 继承
    #       ——三态由该门自裁,rc≠0 即 RED 如实登记）──
    g27_ev_path_str = ""
    g27_booleans = {v: False for v in G27_FACT_MAP.values()}
    r = run([sys.executable, str(G27_SMOKE), "--gate", G27_GATE], timeout=7200, env=device_env())
    latest = wel.load_latest_evidence(G27_SUBJECT)
    g27_doc = wel.load_json(latest) if latest else None
    g27_ev_path_str = str(latest.relative_to(ROOT)).replace("\\", "/") if latest else ""
    g27_booleans, g27_fails = g27_maintenance_judge(r.returncode, g27_doc)
    set_fact(
        "g27_gate_maintenance",
        not g27_fails,
        f"{G27_GATE} 接线态复跑 rc={r.returncode}（evidence {Path(g27_ev_path_str).name if g27_ev_path_str else '缺'}）"
        + ("六 facts 全 PASS" if not g27_fails else f"红: {g27_fails[:3]}"),
    )

    # ── dev-env 探针（hzb off 短跑;skipped_dev_env 即降级登记）──
    env = device_env()
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} dev-env 探针（hzb off 短跑）"):
            rp, _, _ = run_present("probe", 2, 1, False, None, env, timeout=1200)
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {
            "schema": "rurix.g31.hzb_wiring.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── 渲染腿 + Stage A 锚格（单锁串行;数字全来自真实命令输出）──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    orbit_docs: dict[str, dict] = {}
    static_docs: dict[str, dict] = {}
    parity_doc: dict = {}
    occlusion_doc: dict = {}
    frame_ms_doc: dict = {}
    anchor_doc: dict = {}
    legs_ok = True
    with gpu_device_lock(purpose=f"{TAG} orbit 四腿 + 静态 measured 两腿 + Stage A 锚格 bench"):
        # ②③④⑤ orbit 四腿：on_a（probe 对拍 + 剔除计数）/ on_b（双跑位级）/
        # on_allvis（RURIX_HZB_ALL_VISIBLE=1 实验臂）/ off（on/off 关系登记）。
        orbit_legs = [
            ("on_a", True, False),
            ("on_b", True, False),
            ("on_allvis", True, True),
            ("off", False, False),
        ]
        for label, hzb_on, allvis in orbit_legs:
            leg_env = dict(env)
            if allvis:
                leg_env["RURIX_HZB_ALL_VISIBLE"] = "1"
            rr, doc, _ = run_present(
                label, ORBIT_FRAMES, ORBIT_WARMUP, hzb_on, TRAJECTORY, leg_env,
                spv_overrides=spv_flags if hzb_on else None,
            )
            out = (rr.stdout or "") + (rr.stderr or "")
            if rr.returncode != 0 or doc is None or "[g31_window_present]: PASS" not in out:
                fail(f"{label} 真跑失败 rc={rr.returncode}: {out[-300:]}")
                legs_ok = False
                continue
            if "Validation Error" in out or "VUID-" in out:
                fail(f"{label} validation 应静默却报错")
                legs_ok = False
            j = harness_common_judge(doc, ORBIT_FRAMES, ORBIT_WARMUP, TRAJECTORY, label)
            for m in j:
                fail(m)
            legs_ok &= not j
            if hzb_on:
                if doc.get("schema") != HARNESS_SCHEMA_ID:
                    fail(f"{label}: schema ≠ {HARNESS_SCHEMA_ID}: {doc.get('schema')!r}")
                    legs_ok = False
                hzb_block = doc.get("hzb") or {}
                if hzb_block.get("mode") != "on":
                    fail(f"{label}: hzb.mode ≠ on")
                    legs_ok = False
            orbit_docs[label] = doc
        # ⑥ 静态 measured 两腿（off/on 各 ≥100 帧 real_render_frame_ms）。
        for label, hzb_on in (("static_off", False), ("static_on", True)):
            rr, doc, _ = run_present(
                label, frames, warmup, hzb_on, None, env,
                spv_overrides=spv_flags if hzb_on else None,
            )
            out = (rr.stdout or "") + (rr.stderr or "")
            if rr.returncode != 0 or doc is None or "[g31_window_present]: PASS" not in out:
                fail(f"{label} 真跑失败 rc={rr.returncode}: {out[-300:]}")
                legs_ok = False
                continue
            if "Validation Error" in out or "VUID-" in out:
                fail(f"{label} validation 应静默却报错")
                legs_ok = False
            j = harness_common_judge(doc, frames, warmup, None, label)
            for m in j:
                fail(m)
            legs_ok &= not j
            static_docs[label] = doc
        # ⑦ Stage A 锚格（canonical 160 帧;off = 生产车道 0-byte 机器证明）。
        bench_root = WORK / "anchor_bench"
        rb = run(
            [str(BIN_BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
             "--backend", "tsr_device", "--frames", str(ANCHOR_FRAMES), "--warmup", str(ANCHOR_WARMUP),
             "--out-root", str(bench_root)],
            timeout=3600, env=env,
        )
        receipt = bench_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
        fresh = None
        if rb.returncode == 0 and receipt.is_file():
            fresh = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
        anchors = json.loads(ANCHOR_PATH.read_text(encoding="utf-8")).get("anchors") or {}
        anchor_dg = (anchors.get(ANCHOR_CELL) or {}).get("last_frame_digest")
        anchor_doc = {
            "cell": ANCHOR_CELL,
            "fresh_digest": fresh,
            "anchor_digest": anchor_dg,
            "match": anchor_zero_drift(fresh, anchor_dg),
            "frames": ANCHOR_FRAMES,
            "warmup": ANCHOR_WARMUP,
        }
        set_fact(
            "off_static_anchor_zero_drift",
            anchor_doc["match"],
            f"Stage A 锚格 {ANCHOR_CELL}:fresh {str(fresh)[:23]}… vs 在案 {str(anchor_dg)[:23]}… "
            f"{'位级 MATCH（off = 生产车道 0-byte 机器证明）' if anchor_doc['match'] else 'DRIFT（RED）'}",
        )

    # ── ② 接线态对拍（on_a probe 帧 hzb.parity 三块）──
    if orbit_docs.get("on_a"):
        on_a = orbit_docs["on_a"]
        parity = (on_a.get("hzb") or {}).get("parity") or {}
        parity_fails = parity_judge(parity)
        parity_doc = {
            "mips_bitexact": parity.get("mips_bitexact") is True,
            "verdict_sequence_equal": parity.get("verdict_sequence_equal") is True,
            "false_positives": parity.get("false_positives", -1),
            "pyramid_digest_equal_host": parity.get("pyramid_digest_equal_host") is True,
            "verdict_digest_equal_host": parity.get("verdict_digest_equal_host") is True,
            "pyramid_digest": parity.get("pyramid_digest", ""),
            "verdict_digest": parity.get("verdict_digest", ""),
        }
        set_fact(
            "wired_parity_probe",
            not parity_fails,
            f"probe 帧 hzb.parity:mips 位级={parity_doc['mips_bitexact']} "
            f"verdict 全等={parity_doc['verdict_sequence_equal']} fp={parity_doc['false_positives']} "
            f"pyramid/verdict digest vs host 互核={parity_doc['pyramid_digest_equal_host']}/"
            f"{parity_doc['verdict_digest_equal_host']}（n_rects={parity.get('n_rects')}）"
            + ("" if not parity_fails else f"；红 {parity_fails[:2]}"),
        )
        # ── ⑨ 剔除真实发生（on_a occlusion 计数）──
        occ = (on_a.get("hzb") or {}).get("occlusion") or {}
        occ_fails = occlusion_judge(occ)
        occlusion_doc = {
            "tested_p1": occ.get("tested_p1", 0),
            "occluded_p1": occ.get("occluded_p1", 0),
            "offscreen": occ.get("offscreen", 0),
            "retested_p2": occ.get("retested_p2", 0),
            "flipped_p2": occ.get("flipped_p2", 0),
            "closure_frames": occ.get("closure_frames", 0),
            "closure_extra_submits": occ.get("closure_extra_submits", 0),
            "closure_full_fallbacks": occ.get("closure_full_fallbacks", 0),
            "visible_mean": occ.get("visible_mean", 0.0),
        }
        set_fact(
            "occlusion_culling_active",
            not occ_fails,
            f"剔除真实发生:tested_p1={occlusion_doc['tested_p1']} occluded_p1={occlusion_doc['occluded_p1']}"
            f" offscreen={occlusion_doc['offscreen']} retested_p2={occlusion_doc['retested_p2']}"
            f" flipped_p2={occlusion_doc['flipped_p2']} 闭环(帧/额外提交/全量兜底)="
            f"{occlusion_doc['closure_frames']}/{occlusion_doc['closure_extra_submits']}/"
            f"{occlusion_doc['closure_full_fallbacks']} visible_mean={occlusion_doc['visible_mean']:.2f}"
            + ("" if not occ_fails else f"；红 {occ_fails[:2]}"),
        )

    # ── ③④⑤ digest 序列三判（orbit 四腿齐备才判）──
    on_off_doc: dict = {}
    neutral_bit = False
    neutral_mismatch = -1
    double_run_bit = False
    if all(k in orbit_docs for k in ("on_a", "on_b", "on_allvis", "off")):
        seq_on = orbit_docs["on_a"].get("digest_seq", [])
        seq_on_b = orbit_docs["on_b"].get("digest_seq", [])
        seq_allvis = orbit_docs["on_allvis"].get("digest_seq", [])
        seq_off = orbit_docs["off"].get("digest_seq", [])
        # ③ 剔除像素中性门（可见集一致性硬判）
        neutral_bit = seqs_bitexact(seq_on, seq_allvis)
        neutral_mismatch = seq_mismatch_count(seq_on, seq_allvis)
        set_fact(
            "culling_neutrality",
            neutral_bit and neutral_mismatch == 0,
            f"--hzb on vs RURIX_HZB_ALL_VISIBLE=1 全集渲染臂 digest_seq 逐帧位级一致={neutral_bit}"
            f"（mismatch={neutral_mismatch}/{len(seq_on)};剔除不改变可见像素 = 可见集一致性结构判据）",
        )
        # ④ 生产确定性双跑位级（内部硬门,notes 登记）
        double_run_bit = seqs_bitexact(seq_on, seq_on_b)
        set_fact(
            "determinism_double_run",
            double_run_bit,
            f"on_a vs on_b 同参双跑 digest_seq 逐帧位级一致={double_run_bit}（seq_len={len(seq_on)}）",
        )
        # ⑤ on/off 关系如实登记（不设通过线;digo 全等结构上不可达）
        on_off_mismatch = seq_mismatch_count(seq_on, seq_off)
        on_off_doc = {
            "on_digest": orbit_docs["on_a"].get("digest", ""),
            "off_digest": orbit_docs["off"].get("digest", ""),
            "digest_seq_bitexact": on_off_mismatch == 0,
            "mismatch_frames": on_off_mismatch if on_off_mismatch >= 0 else len(seq_on),
            "seq_len": len(seq_on),
            "structural_note": (
                "分解/双 TLAS 结构 ULP 噪声 ⇒ on/off digest_seq 位级全等结构上不可达,"
                "mismatch 计数如实登记不设通过线;剔除像素中性由 culling_neutrality"
                "（on vs ALL_VISIBLE=1 位级一致）钉死"
            ),
        }
        relation_ok = (
            DIGEST_RE.match(on_off_doc["on_digest"] or "") is not None
            and DIGEST_RE.match(on_off_doc["off_digest"] or "") is not None
            and on_off_doc["seq_len"] >= 10 and on_off_mismatch >= 0
        )
        set_fact(
            "on_off_relation",
            relation_ok,
            f"on/off digest_seq mismatch={on_off_doc['mismatch_frames']}/{on_off_doc['seq_len']} 帧"
            f"（位级全等={on_off_doc['digest_seq_bitexact']},结构 ULP 噪声预期 false;如实登记不设通过线）",
        )

    # ── ⑥ on/off frame_ms measured（静态两腿）──
    if all(k in static_docs for k in ("static_off", "static_on")):
        off_mean = static_docs["static_off"]["real_render_frame_ms"]
        on_mean = static_docs["static_on"]["real_render_frame_ms"]
        stats_on = (static_docs["static_on"].get("stats") or {})
        frame_ms_doc = {
            "off_mean": off_mean,
            "on_mean": on_mean,
            "on_over_off": on_mean / off_mean,
            "frames": frames,
            "warmup": warmup,
            "note": (
                f"bistro-interior 1080p 静态相机 --hidden release 同机同窗 measured_local:"
                f"off={off_mean:.4f}ms on={on_mean:.4f}ms（on/off={on_mean / off_mean:.4f};"
                f"各 {frames} 帧 warmup {warmup};hzb_gpu_ms={stats_on.get('hzb_gpu_ms', 0.0):.4f} "
                f"scene_gpu_ms={stats_on.get('scene_gpu_ms', 0.0):.4f} "
                f"closure_extra_gpu_ms={stats_on.get('closure_extra_gpu_ms', 0.0):.4f} "
                f"hzb_host_ms={stats_on.get('hzb_host_ms', 0.0):.4f}）;如实登记不设通过线"
                "（G6 无硬门纪律）"
            ),
        }
        set_fact(
            "frame_ms_measured",
            frame_ms_sane(off_mean, on_mean) and frames >= 100,
            f"measured:off={off_mean:.4f}ms on={on_mean:.4f}ms（on/off={on_mean / off_mean:.4f}）"
            f"各 {frames} 帧 warmup {warmup}（真实命令输出,如实登记）",
        )

    # ── ⑧ 冻结面 0-byte 机核（git 提交面 + 工作树双面）──
    frozen_geom_ok = frozen_kern_ok = worktree_geom_ok = worktree_kern_ok = False
    d = run(["git", "diff", "--quiet", FROZEN_GEOM_BASE, "--", *FROZEN_GEOM_PATHS])
    frozen_geom_ok = d.returncode == 0
    u = run(["git", "status", "--porcelain", "--", *FROZEN_GEOM_PATHS])
    worktree_geom_ok = not u.stdout.strip()
    d = run(["git", "diff", "--quiet", FROZEN_KERNEL_BASE, "--", *FROZEN_KERNEL_PATHS])
    frozen_kern_ok = d.returncode == 0
    u = run(["git", "status", "--porcelain", "--", *FROZEN_KERNEL_PATHS])
    worktree_kern_ok = not u.stdout.strip()
    set_fact(
        "frozen_surfaces_0byte",
        frozen_geom_ok and worktree_geom_ok and frozen_kern_ok and worktree_kern_ok,
        f"geometry 三文件 vs {FROZEN_GEOM_BASE} 0-byte={frozen_geom_ok}（工作树干净={worktree_geom_ok}）"
        f"；g27 两 kernel vs {FROZEN_KERNEL_BASE} 0-byte={frozen_kern_ok}（工作树干净={worktree_kern_ok}）",
    )

    # ── 门裁决（facts 全绿 + 渲染腿全绿 + FAILURES 空）──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and legs_ok and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "scene_id": SCENE,
        "gate_maintenance": {
            "state": "pass" if not g27_fails else "fail",
            "verdict": "PASS" if not g27_fails else "FAIL",
            **g27_booleans,
            "evidence": g27_ev_path_str,
        },
        "wired_parity": {
            "mips_bitexact": bool(parity_doc.get("mips_bitexact")),
            "verdict_sequence_equal": bool(parity_doc.get("verdict_sequence_equal")),
            "false_positives": int(parity_doc.get("false_positives", -1)),
            "pyramid_digest_equal_host": bool(parity_doc.get("pyramid_digest_equal_host")),
            "verdict_digest_equal_host": bool(parity_doc.get("verdict_digest_equal_host")),
            "pyramid_digest": parity_doc.get("pyramid_digest", "") or ("sha256:" + "0" * 64),
            "verdict_digest": parity_doc.get("verdict_digest", "") or ("sha256:" + "0" * 64),
        },
        "culling_neutrality": {
            "trajectory": TRAJECTORY,
            "frames": ORBIT_FRAMES,
            "warmup": ORBIT_WARMUP,
            "seq_len": ORBIT_FRAMES + ORBIT_WARMUP,
            "on_vs_all_visible_seq_bitexact": bool(neutral_bit),
            "mismatch_count": int(neutral_mismatch) if neutral_mismatch >= 0 else -1,
        },
        "on_off_relation": on_off_doc if on_off_doc else {
            "on_digest": "sha256:" + "0" * 64,
            "off_digest": "sha256:" + "0" * 64,
            "digest_seq_bitexact": False,
            "mismatch_frames": -1,
            "seq_len": 0,
            "structural_note": "orbit 腿未齐备（前置失败）",
        },
        "occlusion": {
            **{k: int(occlusion_doc.get(k, 0)) for k in (
                "tested_p1", "occluded_p1", "offscreen", "retested_p2", "flipped_p2",
                "closure_frames", "closure_extra_submits", "closure_full_fallbacks")},
            "visible_mean": float(occlusion_doc.get("visible_mean", 0.0)),
            "culling_active": bool(occlusion_doc.get("occluded_p1", 0) >= 1
                                   and occlusion_doc.get("tested_p1", 0) >= 1),
        },
        "frame_ms_compare": frame_ms_doc if frame_ms_doc else {
            "off_mean": -1.0, "on_mean": -1.0, "on_over_off": -1.0,
            "frames": frames, "warmup": warmup, "note": "静态腿未齐备（前置失败）",
        },
        "off_static_anchor": {
            "path": "milestones/g14/g14_3_stage_a_digest_anchor.json",
            "digest": anchor_doc.get("fresh_digest") or ("sha256:" + "0" * 64),
            "zero_drift": bool(anchor_doc.get("match")),
            "program_produced_first_run": False,
        },
        "frozen_surfaces": {
            "geometry_files_0byte_vs_g26_closed": bool(frozen_geom_ok and worktree_geom_ok),
            "g27_kernels_0byte_vs_g27_closed": bool(frozen_kern_ok and worktree_kern_ok),
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 B Task B1 HZB 遮挡剔除生产接线（两阶段闭环第二段）：bistro 逐 mesh 节点 "
            "BLAS 分解（1186 实例）+ 双 TLAS（初剔表 0 相机射线/全量表 1 阴影射线）+ 帧内金字塔 "
            "轮换（上帧金字塔初剔 → 本帧真深度〔depth_hz = g31_hzb_shade ④b 段 vp 行 2/3 真 ZO NDC〕"
            "重建 → 上帧被剔集重测）+ 误剔闭环重渲（≤4 迭代未收敛全掩码兜底 = 精确收敛）；"
            "g27_hzb_reduce/g27_hzb_test 本体 0-byte 冻结消费（bin 侧 NoContraction 注入,"
            "SPV 文件 0-byte）。判据：①G27 门维持复跑（六 facts 全 PASS）②接线态 probe 对拍"
            "（pyramid/verdict digest vs host 金标准位级全等 + fp=0）③剔除像素中性门"
            "（on vs RURIX_HZB_ALL_VISIBLE=1 digest_seq 逐帧位级一致,mismatch=0）④生产双跑位级"
            f"={double_run_bit}（schema 闭集外内部硬门,本字段登记）⑤on/off 关系如实登记"
            "（结构 ULP 噪声,mismatch 计数不设通过线）⑥on/off frame_ms measured"
            f"（off={frame_ms_doc.get('off_mean', -1.0):.4f}ms on={frame_ms_doc.get('on_mean', -1.0):.4f}ms,"
            "如实登记不设通过线）⑦off 面 Stage A 锚零漂移（锚在案复用非新订,"
            "program_produced_first_run=false）⑧geometry 三文件 vs g26-closed + g27 两 kernel "
            "vs g27-closed 0-byte 机核 ⑨剔除真实发生（occluded_p1≥1,零剔除即空接线冒充判红）。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in fact_rows)}"
        ),
    }
    import jsonschema  # 自校验硬门（schema 漂移即 RED;PASS-only 闭集面）

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_hzb_wiring_{ts}.json"
    else:
        # FAIL 诊断件落 .tmp 工作区——PASS-only schema 面,evidence/ 只收门件
        # （fail-closed：evidence/ 无件 = 门未过,不污染 check_schemas 路由面）。
        gate_path = WORK / f"gate_fail_{ts}.json"
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}（harness 真跑件 {len(orbit_docs) + len(static_docs)} 件留 .tmp 工作区）")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂,无 GPU/无构建依赖）
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

    dg = lambda ch: "sha256:" + ch * 64  # noqa: E731
    # 红绿臂①:digest 序列判。
    expect(seqs_bitexact(["a", "b"], ["a", "b"]), "GREEN:双跑位级正例")
    expect(not seqs_bitexact(["a", "b"], ["a", "x"]), "RED:双跑漂移必红")
    expect(not seqs_bitexact(["a"], ["a", "b"]), "RED:长度不齐必红")
    expect(not seqs_bitexact([], []), "RED:空序列必红")
    expect(seq_mismatch_count(["a", "b", "c"], ["a", "x", "c"]) == 1, "GREEN:mismatch 计数正例")
    expect(seq_mismatch_count(["a", "b"], ["a", "b"]) == 0, "GREEN:零 mismatch 正例")
    expect(seq_mismatch_count(["a"], ["a", "b"]) == -1, "RED:长度不齐 -1 拒判")
    # 红绿臂②:接线态对拍判。
    good_parity = {
        "mips_bitexact": True, "verdict_sequence_equal": True, "false_positives": 0,
        "pyramid_digest_equal_host": True, "verdict_digest_equal_host": True,
        "pyramid_digest": dg("a"), "verdict_digest": dg("b"),
    }
    expect(parity_judge(good_parity) == [], "GREEN:parity 正例")
    bad = dict(good_parity, mips_bitexact=False)
    expect(parity_judge(bad), "RED:mips 非位级必红")
    bad = dict(good_parity, verdict_sequence_equal=False)
    expect(parity_judge(bad), "RED:verdict 序列异必红")
    bad = dict(good_parity, false_positives=1)
    expect(parity_judge(bad), "RED:假阳性 1 必红")
    bad = dict(good_parity, pyramid_digest_equal_host=False)
    expect(parity_judge(bad), "RED:pyramid digest vs host 不符必红")
    bad = dict(good_parity, verdict_digest="sha256:zz")
    expect(parity_judge(bad), "RED:digest 形态非法必红")
    bad = {k: v for k, v in good_parity.items() if k != "mips_bitexact"}
    expect(parity_judge(bad), "RED:缺字段必红")
    expect(parity_judge(None), "RED:parity 非 object 必红")
    # 红绿臂③:剔除真实发生判。
    good_occ = {
        "tested_p1": 9000, "occluded_p1": 120, "offscreen": 300, "retested_p2": 118,
        "flipped_p2": 2, "closure_frames": 1, "closure_extra_submits": 1,
        "closure_full_fallbacks": 0, "visible_mean": 700.5,
    }
    expect(occlusion_judge(good_occ) == [], "GREEN:剔除活跃正例")
    bad = dict(good_occ, occluded_p1=0)
    expect(occlusion_judge(bad), "RED:零剔除（空接线冒充）必红")
    bad = dict(good_occ, tested_p1=0)
    expect(occlusion_judge(bad), "RED:零测试必红")
    bad = dict(good_occ, visible_mean=0.0)
    expect(occlusion_judge(bad), "RED:visible_mean 零必红")
    bad = dict(good_occ, offscreen=-1)
    expect(occlusion_judge(bad), "RED:负计数必红")
    expect(occlusion_judge({}), "RED:空 occlusion 必红")
    # 红绿臂④:G27 门维持判。
    def g27_doc(status: str = "PASS", drop: str | None = None, hsp: bool = True) -> dict:
        facts = [{"id": fid, "status": status, "detail": "x"} for fid in G27_FACT_MAP]
        if drop:
            facts = [f for f in facts if f["id"] != drop]
        return {"host_section_pass": hsp, "extra_facts": facts}
    boo, fl = g27_maintenance_judge(0, g27_doc())
    expect(not fl and all(boo.values()), "GREEN:G27 全绿正例")
    boo, fl = g27_maintenance_judge(0, g27_doc(status="FAIL"))
    expect(fl and not any(boo.values()), "RED:单 facts FAIL 必红")
    _, fl = g27_maintenance_judge(0, g27_doc(drop="tamper_red_arm_detected"))
    expect(bool(fl), "RED:facts 缺失必红")
    _, fl = g27_maintenance_judge(0, g27_doc(hsp=False))
    expect(bool(fl), "RED:host_section_pass false 必红")
    _, fl = g27_maintenance_judge(1, g27_doc())
    expect(bool(fl), "RED:子进程 rc≠0 必红")
    _, fl = g27_maintenance_judge(0, None)
    expect(bool(fl), "RED:evidence 缺失必红")
    # 红绿臂⑤:锚/frame_ms/三态判。
    expect(anchor_zero_drift(dg("a"), dg("a")), "GREEN:锚零漂移正例")
    expect(not anchor_zero_drift(dg("a"), dg("b")), "RED:锚漂移必红")
    expect(not anchor_zero_drift(None, dg("a")), "RED:fresh 缺失必红")
    expect(not anchor_zero_drift("sha256:zz", dg("a")), "RED:digest 形态非法必红")
    expect(frame_ms_sane(3.5, 3.6), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(3.5, 0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(3.5, float("nan")), "RED:NaN 必红")
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    # schema 互核:在树 + 关键 const/required 逐字。
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(gs["properties"]["scene_id"]["const"] == SCENE, "scene const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "scene_id",
                "gate_maintenance", "wired_parity", "culling_neutrality",
                "on_off_relation", "occlusion", "frame_ms_compare",
                "off_static_anchor", "frozen_surfaces", "environment",
                "timestamp", "notes",
            ]),
            "schema required 闭集互核（16 字段）",
        )
        gm_req = gs["properties"]["gate_maintenance"].get("required", [])
        expect(
            all(v in gm_req for v in G27_FACT_MAP.values()),
            "gate_maintenance required ⊇ G27 六 facts 映射值",
        )
    expect(len(FACT_IDS) == 9, "facts 闭集 = 9")
    expect(len(G27_FACT_MAP) == 6, "G27 facts 映射 = 6")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=9；5 红臂组 + 正例组 + schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=100)
    ap.add_argument("--warmup", type=int, default=10)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if args.frames < 100:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 100（on/off frame_ms 对照下限）", file=sys.stderr)
            return 1
        if args.warmup < 1:
            print(f"[{TAG}] FAIL: --warmup {args.warmup} < 1（schema 下限）", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
