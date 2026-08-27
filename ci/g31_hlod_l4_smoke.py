#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C12 HLOD L4 Far Field 档实现批）
"""G31+ 波 C Task C12:HLOD L4 Far Field 档门冒烟(g31.waveC.hlodl4;
G30 承接锚 M98-l4 行「HLOD proxy 追踪 device 腿 + L4 计数器接入」合取两半
兑现;G31_PLUS_COMMERCIAL_RENDERER_TODO §1.2 #25 行;RFC-0044 §4 重判窗程序)。

两半实现面:
- ① device 腿:kernels/g31_hlod_l4_proxy_trace.rx(纯 compute,不消费 TLAS,
  L1 同构面)+ host 镜像 gi::fallback_chain::l4_trace_ray/l4_leg_host(逐字
  同源)+ bin g31_hlod_l4_far_field(device 真跑 + 结构域精确对拍〔hit
  flags/proxy 下标/扫描计数逐像素硬判据〕+ rgb 位级全等〔烘焙辐射度纯数据
  搬运〕+ t 残差信息项〔OpFDiv 2.5 ULP,G7.5b 口径〕)。
- ② L4 计数器接入:L4 槽位真实计数(attempted/proxy 命中/服务像素/扫描耗
  时)+ 可见 proxy 数(下标回读去重)+ 切换次数(至 L4 转移按因分列)+
  覆盖率(hit_rate);三处 fail-closed 入口按锚字面解锁
  (check_l4_trigger=Ready / l4_serve=Ok / 计数非零);半齐保持 fail-closed
  (空 proxy 集 ⇒ NotTriggered/Err/InvalidConfig,空接线冒充判红不冒充)。

判据闭集(milestones/g31/g31_hlod_l4_evidence_schema.json 描述段逐字):
1. host_tests_anchored:gi::fallback_chain 19 单测全绿(14 既有锚定 + L4
   五件新批;cargo test 子进程 rc=0)。
2. g9_gate_maintenance:g9.p0.m98.tracing_fallback_chain 复跑 rc=0 + 最新
   evidence status=pass + checks 14 键全 true(L1-L3 链 0-byte 行为机器
   证明:六档深度带内 + M96 门序锚 + L4 not-triggered 登记维持)。
3. device_leg_parity:结构域精确 + rgb 位级 + t 残差信息项登记。
4. determinism_double_run:device 腿双跑位级 + golden 双跑位级 + device
   腿 golden == host 镜像 golden 全帧位级。
5. l4_counters_wired:L4 计数面真实非空 + transitions_to_l4 == served。
6. unlock_entries:三处入口 Ready/Ok/非零 + 半齐 fail-closed(空集臂)。
7. on_off_relation:on/off digest 必分叉 + ForcedOff 记录 + off_served=0
   + off == legacy 产物位级(截断等价)如实登记。
8. red_arms_detected:静默注入/强关/篡改/空集四 RED 臂子进程独立检出。
9. proxy_coverage_verified:host 参考投影覆盖——逐件 proxy ≥1 命中像素 +
   device 下标列与 host 全等 + golden 四级全消费。
10. frame_ms_measured:L4 纯派发 + 帧管 on/off 各 N 帧真实壁钟如实登记
    (不设通过线,G6 无硬门纪律)。
11. frozen_surfaces_0byte:m98 双 kernel + g9 深度带 + g30 承接锚表 +
    G27_P2 原始锚工作树 0-byte 机核(path_trace.rs 在飞波改动面不在本任务
    闭集,L1-L3 行为保持由判据 2 钉死)。
12. scene_contract_four_tier:canonical 远场契约四级覆盖充分性(golden
    served L1/L2/L3/L4 全 > 0,空转即 RED)。

三态:无 Vulkan loader/设备/SPV → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

evidence 纪律:门 schema 全 const 闭集 = PASS-only 面——PASS 才落
evidence/g31_hlod_l4_<ts>.json(check_schemas 前缀路由 g31_hlod_l4_);
FAIL 诊断件落 .tmp/g31_gates/hlodl4/ 工作区不污染 evidence/ 路由面。
改判登记 = 全齐程序产:PASS 时同批写 milestones/g31/g31_m98_l4_rejudgment
.json(两半清单 + 三入口解锁前后字面 + verdict=rejudged-four-tier-chain;
G27_P2/G30 registry 原始锚 0-byte 不回写;deferred.json 无 M98-l4 条目,
history 追加不适用如实登记)。

用法:
  py -3 ci/g31_hlod_l4_smoke.py --selftest
  py -3 ci/g31_hlod_l4_smoke.py --gate g31.waveC.hlodl4 [--frames 32]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import io
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

GATE_KEY = "g31.waveC.hlodl4"
SUBJECT = "g31_hlod_l4"
WAVE = "G31+.C"
TAG = "g31_hlod_l4"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_hlod_l4_evidence_schema.json"
SCHEMA_ID = "rurix.g31.hlod_l4_smoke_evidence.v1"
HARNESS_SCHEMA_ID = "rurix.g31.hlod_l4_harness.v1"
SCENE = "m98_l4_far_field"
KERNEL = ROOT / "src" / "rurix-render" / "kernels" / "g31_hlod_l4_proxy_trace.rx"
WORK = ROOT / ".tmp" / "g31_gates" / "hlodl4"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "release" / f"g31_hlod_l4_far_field{EXE_SUFFIX}"
RURIXC = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
G9_SMOKE = ROOT / "ci" / "g9_tracing_fallback_chain_smoke.py"
G9_GATE = "g9.p0.m98.tracing_fallback_chain"
G9_SUBJECT = "g9_m98_tracing_fallback_chain"
G9_CHECK_KEYS = [
    "gate_order_m96_passed",
    "host_fallback_chain_tests_anchored",
    "conformance_gi_corpus_anchored",
    "depth_band_provenance_frozen",
    "device_harness_full_pass",
    "device_double_run_bitexact",
    "device_l1_host_parity",
    "device_counters_non_empty",
    "device_force_off_detectable",
    "device_silent_demotion_audit",
    "device_red_arm_submodes_detected",
    "device_l4_not_triggered",
    "device_m96_cross_anchor_band",
    "device_validation_zero",
]
RED_ARMS = ["silent-demotion", "force-off-l4", "tamper-proxy", "empty-proxy"]
FROZEN_M98_KERNELS = [
    "src/rurix-render/kernels/g9_m98_screen_trace.rx",
    "src/rurix-render/kernels/g9_m98_hwrt.rx",
]
FROZEN_G9_BAND = ["milestones/g9/g9_m98_depth_band.json"]
FROZEN_ANCHORS = [
    "milestones/g30/g30_campaign_handover_registry.json",
    "milestones/g27/G27_P2_DECISIONS.md",
]
REJUDGMENT_PATH = ROOT / "milestones" / "g31" / "g31_m98_l4_rejudgment.json"
EXPECTED_TEST_COUNT = 19
EXPECTED_PROXIES = 5

DIGEST_RE = re.compile(r"^[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "host_tests_anchored",
    "g9_gate_maintenance",
    "device_leg_parity",
    "determinism_double_run",
    "l4_counters_wired",
    "unlock_entries",
    "on_off_relation",
    "red_arms_detected",
    "proxy_coverage_verified",
    "frame_ms_measured",
    "frozen_surfaces_0byte",
    "scene_contract_four_tier",
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
# 判读器(selftest 红绿两臂消费面)
# ---------------------------------------------------------------------------


def parity_judge(parity: dict) -> list[str]:
    """③ device/host 对拍判(结构域精确 + rgb 位级硬判据;t 残差信息项健全)。"""
    fails: list[str] = []
    if not isinstance(parity, dict):
        return ["harness parity 非 object"]
    if parity.get("structural_exact") is not True:
        fails.append(f"parity.structural_exact ≠ true: {parity.get('structural_exact')!r}")
    if parity.get("rgb_bitexact") is not True:
        fails.append(f"parity.rgb_bitexact ≠ true: {parity.get('rgb_bitexact')!r}")
    for k in ("t_residual_max_abs", "t_residual_rel_max"):
        v = parity.get(k)
        try:
            f = float(v)
        except (TypeError, ValueError):
            fails.append(f"parity.{k} 非数值: {str(v)[:40]!r}")
            continue
        if not (f == f and f >= 0.0):
            fails.append(f"parity.{k} 非有限非负: {v!r}")
    return fails


def double_run_judge(dr: dict) -> list[str]:
    """④ 双跑位级判(三键全 true)。"""
    fails: list[str] = []
    if not isinstance(dr, dict):
        return ["harness double_run 非 object"]
    for k in ("device_leg_bitexact", "golden_bitexact", "device_golden_eq_host_golden"):
        if dr.get(k) is not True:
            fails.append(f"double_run.{k} ≠ true: {dr.get(k)!r}")
    return fails


def counters_judge(l4: dict) -> list[str]:
    """⑤ L4 计数面判(真实计数 + 切换次数 = served + 覆盖率/可见数闭集)。"""
    fails: list[str] = []
    if not isinstance(l4, dict):
        return ["harness l4 非 object"]
    c = l4.get("counters") or {}
    for k in ("rays_attempted", "rays_hit", "pixels_served", "work_count"):
        v = c.get(k)
        if not isinstance(v, int) or isinstance(v, bool) or v < 1:
            fails.append(f"l4.counters.{k} < 1: {v!r}")
    hr = c.get("hit_rate")
    try:
        hr_f = float(hr)
    except (TypeError, ValueError):
        hr_f = -1.0
    if not (0.0 < hr_f <= 1.0):
        fails.append(f"l4.counters.hit_rate 越 (0,1]: {hr!r}")
    to_l4 = l4.get("transitions_to_l4")
    if not isinstance(to_l4, int) or isinstance(to_l4, bool) or to_l4 < 1:
        fails.append(f"l4.transitions_to_l4 < 1: {to_l4!r}")
    elif to_l4 != c.get("pixels_served"):
        fails.append(f"l4.transitions_to_l4 {to_l4} ≠ pixels_served {c.get('pixels_served')}")
    for k in ("transitions_to_l4_miss", "transitions_to_l4_out_of_range"):
        v = l4.get(k)
        if not isinstance(v, int) or isinstance(v, bool) or v < 0:
            fails.append(f"l4.{k} 非负整数破: {v!r}")
    if l4.get("proxies_visible") != EXPECTED_PROXIES:
        fails.append(f"l4.proxies_visible ≠ {EXPECTED_PROXIES}: {l4.get('proxies_visible')!r}")
    pph = l4.get("per_proxy_hits")
    if not isinstance(pph, list) or len(pph) != EXPECTED_PROXIES or any(
        not isinstance(n, int) or isinstance(n, bool) or n < 1 for n in pph
    ):
        fails.append(f"l4.per_proxy_hits 非 {EXPECTED_PROXIES} 件逐件 ≥1: {pph!r}")
    return fails


def unlock_judge(l4: dict, checks: dict) -> list[str]:
    """⑥ 三处入口解锁判(Ready/Ok/非零 + 半齐 fail-closed 空集臂)。"""
    fails: list[str] = []
    if not isinstance(l4, dict) or not isinstance(checks, dict):
        return ["harness l4/checks 非 object"]
    if l4.get("trigger_ready") is not True:
        fails.append("入口① check_l4_trigger ≠ Ready")
    if l4.get("serve_ok") is not True:
        fails.append("入口② l4_serve ≠ Ok")
    c = l4.get("counters") or {}
    if not (isinstance(c.get("pixels_served"), int) and c.get("pixels_served") > 0):
        fails.append("入口③ L4 计数面非非零")
    if checks.get("empty_proxy_fail_closed") is not True:
        fails.append("半齐保护:空 proxy 集 fail-closed 臂未检出(冒充风险)")
    return fails


def on_off_judge(on_off: dict) -> list[str]:
    """⑦ on/off 对照判(分叉 + ForcedOff + 截断等价位级)。"""
    fails: list[str] = []
    if not isinstance(on_off, dict):
        return ["harness on_off 非 object"]
    for k in ("on_digest", "off_digest", "legacy_digest"):
        d = on_off.get(k)
        if not isinstance(d, str) or not DIGEST_RE.match(d):
            fails.append(f"on_off.{k} 形态非法: {str(d)[:40]!r}")
    if on_off.get("digest_differs") is not True:
        fails.append("on/off digest 未分叉(proxy 贡献未进画面 = 空接线)")
    v = on_off.get("off_forced_off_records")
    if not isinstance(v, int) or isinstance(v, bool) or v < 1:
        fails.append(f"off_forced_off_records < 1: {v!r}")
    if on_off.get("off_l4_served") != 0:
        fails.append(f"off_l4_served ≠ 0: {on_off.get('off_l4_served')!r}")
    if on_off.get("legacy_l4_served") != 0:
        fails.append(f"legacy_l4_served ≠ 0: {on_off.get('legacy_l4_served')!r}")
    if on_off.get("off_eq_legacy_product") is not True:
        fails.append("off ≠ legacy 产物位级(截断语义等价破坏)")
    return fails


def coverage_judge(scene: dict, l4: dict, checks: dict) -> list[str]:
    """⑨ host 参考投影覆盖 + 四级覆盖充分性判。"""
    fails: list[str] = []
    if not isinstance(scene, dict) or not isinstance(l4, dict) or not isinstance(checks, dict):
        return ["harness scene/l4/checks 非 object"]
    if scene.get("name") != SCENE:
        fails.append(f"scene.name ≠ {SCENE}: {scene.get('name')!r}")
    if scene.get("proxies_total") != EXPECTED_PROXIES:
        fails.append(f"scene.proxies_total ≠ {EXPECTED_PROXIES}")
    pph = l4.get("per_proxy_hits")
    if isinstance(pph, list) and pph:
        min_hit = min((n for n in pph if isinstance(n, int) and not isinstance(n, bool)), default=0)
        if min_hit < 1:
            fails.append(f"host 投影覆盖:存在 proxy 命中像素为 0({pph!r})")
    else:
        fails.append(f"per_proxy_hits 形态破: {pph!r}")
    if checks.get("device_host_parity_structural") is not True:
        fails.append("device 下标列与 host 非全等(结构对拍面)")
    if checks.get("level_coverage_all_four_used") is not True:
        fails.append("golden 未真实消费全部四级(覆盖充分性 = 空转 RED)")
    return fails


def frame_ms_sane(*vals: float) -> bool:
    """⑩ frame_ms 登记面健全判:全有限正数(诚实登记非阈门)。"""
    out = []
    for v in vals:
        try:
            f = float(v)
        except (TypeError, ValueError):
            return False
        out.append(f == f and f > 0)
    return all(out)


def g9_maintenance_judge(rc: int, doc: dict | None) -> list[str]:
    """② G9 门维持判:子进程 rc + status=pass + checks 14 键全 true。"""
    fails: list[str] = []
    if rc != 0:
        fails.append(f"g9 复跑子进程 rc={rc} ≠ 0")
    if not isinstance(doc, dict):
        fails.append("g9 最新 evidence 缺失/非 object")
        return fails
    if doc.get("status") != "pass":
        fails.append(f"g9 evidence status ≠ pass: {doc.get('status')!r}")
    checks = doc.get("checks") or {}
    for k in G9_CHECK_KEYS:
        if checks.get(k) is not True:
            fails.append(f"g9 checks.{k} ≠ true: {checks.get(k)!r}")
    return fails


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决:无降级 → None(续跑);有降级 + REQUIRE_REAL → 1(硬红);
    有降级无 REQUIRE_REAL → 0(SKIP 非 PASS 非 FAIL)。"""
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


def run_gate(frames: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1

    # ── 构建(harness release + rurixc)──
    ok = build_or_fail(
        ["cargo", "build", "--release", "-p", "rurix-render", "--features", "vulkan",
         "--bin", "g31_hlod_l4_far_field", "--quiet"],
        "harness release",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── SPV 面:L4 kernel 现编 + spirv-val ──
    WORK.mkdir(parents=True, exist_ok=True)
    degrade: list[str] = []
    spv = WORK / "g31_hlod_l4_proxy_trace.spv"
    r = run([str(RURIXC), str(KERNEL), "--target", "vulkan", "-o", str(spv)], timeout=1800)
    if r.returncode != 0 or not spv.is_file():
        degrade.append(f"L4 kernel SPV 编译未过: {(r.stdout + r.stderr)[-200:]}")
    else:
        val = run(["spirv-val", str(spv)], timeout=600)
        if val.returncode != 0:
            degrade.append(f"L4 SPV spirv-val 未过: {(val.stdout + val.stderr)[-200:]}")

    # ── ① host 单测锚定(cargo test 子进程;无 device 依赖恒跑)──
    r = run(
        ["cargo", "test", "-p", "rurix-render", "--features", "vulkan", "--lib",
         "gi::fallback_chain", "--quiet"],
        timeout=3600,
    )
    out = (r.stdout or "") + (r.stderr or "")
    m = re.search(r"test result: ok\. (\d+) passed; (\d+) failed", out)
    tests_pass = r.returncode == 0 and m is not None and int(m.group(2)) == 0
    test_count = int(m.group(1)) if m else -1
    set_fact(
        "host_tests_anchored",
        tests_pass and test_count == EXPECTED_TEST_COUNT,
        f"gi::fallback_chain 单测 {test_count}/{EXPECTED_TEST_COUNT} 全绿={tests_pass}"
        "(14 既有锚定 + L4 五件新批:proxy 校验/host 镜像覆盖/选档升级/四级装配审计/打包门卫)",
    )

    # ── ② g9 门维持复跑(子进程自持 gpu_device_lock;RURIX_REQUIRE_REAL 继承
    #       ——三态由该门自裁,rc≠0 即 RED 如实登记)──
    r = run([sys.executable, str(G9_SMOKE), "--gate", G9_GATE], timeout=7200, env=device_env())
    latest = wel.load_latest_evidence(G9_SUBJECT)
    g9_doc = wel.load_json(latest) if latest else None
    g9_ev = str(latest.relative_to(ROOT)).replace("\\", "/") if latest else ""
    g9_fails = g9_maintenance_judge(r.returncode, g9_doc)
    g9_checks_true = 0
    if isinstance(g9_doc, dict):
        g9_checks_true = sum(1 for k in G9_CHECK_KEYS if (g9_doc.get("checks") or {}).get(k) is True)
    set_fact(
        "g9_gate_maintenance",
        not g9_fails,
        f"{G9_GATE} 复跑 rc={r.returncode}(evidence {Path(g9_ev).name if g9_ev else '缺'})"
        f"checks {g9_checks_true}/{len(G9_CHECK_KEYS)} 全 true"
        + ("——L1-L3 链 0-byte 行为机器证明(六档带内+M96 门序锚+L4 登记维持)" if not g9_fails else f";红: {g9_fails[:3]}"),
    )

    # ── dev-env 探针(harness 短跑;skipped_dev_env 即降级登记)──
    env = device_env()
    if not degrade:
        with gpu_device_lock(purpose=f"{TAG} dev-env 探针(harness 短跑)"):
            rp = run(
                [str(BIN), "--spv-l4", str(spv), "--frames", "2",
                 "--evidence", str(WORK / "harness_probe.json")],
                timeout=1800, env=env,
            )
        probe_out = (rp.stdout or "") + (rp.stderr or "")
        if '"state":"skipped_dev_env"' in probe_out:
            degrade.append(f"harness skipped_dev_env: {probe_out.strip()[-200:]}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {
            "schema": "rurix.g31.hlod_l4.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    # ── 渲染腿 + 四 RED 臂(单锁串行;数字全来自真实命令输出)──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    harness_doc: dict | None = None
    red_arm_rcs: dict[str, int] = {}
    legs_ok = True
    with gpu_device_lock(purpose=f"{TAG} harness 主跑 + 四 RED 臂子模式"):
        ev_path = WORK / "harness_main.json"
        rr = run(
            [str(BIN), "--spv-l4", str(spv), "--frames", str(frames),
             "--evidence", str(ev_path)],
            timeout=3600, env=env,
        )
        out = (rr.stdout or "") + (rr.stderr or "")
        if rr.returncode != 0 or not ev_path.is_file() or "G31_HLOD_L4: PASS" not in out:
            fail(f"harness 主跑失败 rc={rr.returncode}: {out[-300:]}")
            legs_ok = False
        if "Validation Error" in out or "VUID-" in out:
            fail("harness validation 应静默却报错")
            legs_ok = False
        if ev_path.is_file():
            try:
                harness_doc = json.loads(ev_path.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                harness_doc = None
        if not isinstance(harness_doc, dict):
            fail("harness evidence 缺失/非 object")
            legs_ok = False
        elif harness_doc.get("schema") != HARNESS_SCHEMA_ID:
            fail(f"harness schema ≠ {HARNESS_SCHEMA_ID}: {harness_doc.get('schema')!r}")
            legs_ok = False
        for arm in RED_ARMS:
            ra = run([str(BIN), "--red-arm", arm], timeout=1800, env=env)
            red_arm_rcs[arm] = ra.returncode
            if ra.returncode != 0:
                fail(f"RED 臂 {arm} 子进程 rc={ra.returncode}: {((ra.stdout or '') + (ra.stderr or ''))[-200:]}")

    # ── ③~⑩ harness evidence 判读 ──
    hd = harness_doc or {}
    hchecks = hd.get("checks") or {}
    parity_doc: dict = {}
    dr_doc: dict = {}
    counters_doc: dict = {}
    unlock_doc: dict = {}
    on_off_doc: dict = {}
    coverage_doc: dict = {}
    frame_ms_doc: dict = {}
    if legs_ok and hd.get("status") == "pass":
        # ③
        parity = hd.get("parity") or {}
        parity_fails = parity_judge(parity)
        parity_doc = {
            "structural_exact": parity.get("structural_exact") is True,
            "rgb_bitexact": parity.get("rgb_bitexact") is True,
            "t_residual_max_abs": float(parity.get("t_residual_max_abs", "nan")),
            "t_residual_rel_max": float(parity.get("t_residual_rel_max", "nan")),
        }
        set_fact(
            "device_leg_parity",
            not parity_fails,
            f"device/host 结构域精确={parity_doc['structural_exact']} rgb 位级={parity_doc['rgb_bitexact']} "
            f"t 残差 max|Δ|={parity_doc['t_residual_max_abs']:.3e} rel={parity_doc['t_residual_rel_max']:.3e}(信息项)"
            + ("" if not parity_fails else f";红 {parity_fails[:2]}"),
        )
        # ④
        dr = hd.get("double_run") or {}
        dr_fails = double_run_judge(dr)
        dr_doc = {
            "device_leg_bitexact": dr.get("device_leg_bitexact") is True,
            "golden_bitexact": dr.get("golden_bitexact") is True,
            "device_golden_eq_host_golden": dr.get("device_golden_eq_host_golden") is True,
        }
        set_fact(
            "determinism_double_run",
            not dr_fails,
            f"device 腿双跑位级={dr_doc['device_leg_bitexact']} golden 双跑位级={dr_doc['golden_bitexact']} "
            f"device golden == host golden={dr_doc['device_golden_eq_host_golden']}"
            + ("" if not dr_fails else f";红 {dr_fails[:2]}"),
        )
        # ⑤
        l4 = hd.get("l4") or {}
        c = l4.get("counters") or {}
        counters_fails = counters_judge(l4)
        counters_doc = {
            "rays_attempted": int(c.get("rays_attempted", 0)),
            "rays_hit": int(c.get("rays_hit", 0)),
            "pixels_served": int(c.get("pixels_served", 0)),
            "work_count": int(c.get("work_count", 0)),
            "hit_rate": float(c.get("hit_rate", "nan")),
            "transitions_to_l4": int(l4.get("transitions_to_l4", 0)),
            "transitions_to_l4_miss": int(l4.get("transitions_to_l4_miss", 0)),
            "transitions_to_l4_out_of_range": int(l4.get("transitions_to_l4_out_of_range", 0)),
            "proxies_visible": int(l4.get("proxies_visible", 0)),
            "proxies_total": EXPECTED_PROXIES,
            "coverage": float(l4.get("coverage", "nan")),
            "per_proxy_hits": [int(n) for n in (l4.get("per_proxy_hits") or [0] * EXPECTED_PROXIES)],
        }
        set_fact(
            "l4_counters_wired",
            not counters_fails,
            f"L4 计数面:attempted={counters_doc['rays_attempted']} hit={counters_doc['rays_hit']} "
            f"served={counters_doc['pixels_served']} work={counters_doc['work_count']} "
            f"hit_rate={counters_doc['hit_rate']:.4f} 切换(至 L4)={counters_doc['transitions_to_l4']}"
            f"(miss={counters_doc['transitions_to_l4_miss']}/oor={counters_doc['transitions_to_l4_out_of_range']}) "
            f"可见 proxy={counters_doc['proxies_visible']}/{EXPECTED_PROXIES} 覆盖率={counters_doc['coverage']:.4f}"
            + ("" if not counters_fails else f";红 {counters_fails[:2]}"),
        )
        # ⑥
        unlock_fails = unlock_judge(l4, hchecks)
        unlock_doc = {
            "trigger_ready": l4.get("trigger_ready") is True,
            "serve_ok": l4.get("serve_ok") is True,
            "counters_non_zero": isinstance(c.get("pixels_served"), int) and c.get("pixels_served") > 0,
            "half_missing_fail_closed": hchecks.get("empty_proxy_fail_closed") is True,
        }
        set_fact(
            "unlock_entries",
            not unlock_fails,
            f"三处 fail-closed 入口解锁:Ready={unlock_doc['trigger_ready']} Ok={unlock_doc['serve_ok']} "
            f"计数非零={unlock_doc['counters_non_zero']};半齐保持 fail-closed(空集臂)={unlock_doc['half_missing_fail_closed']}"
            + ("" if not unlock_fails else f";红 {unlock_fails[:2]}"),
        )
        # ⑦
        on_off = hd.get("on_off") or {}
        on_off_fails = on_off_judge(on_off)
        on_off_doc = {
            "on_digest": on_off.get("on_digest", "") or "0" * 64,
            "off_digest": on_off.get("off_digest", "") or "0" * 64,
            "legacy_digest": on_off.get("legacy_digest", "") or "0" * 64,
            "digest_differs": on_off.get("digest_differs") is True,
            "off_forced_off_records": int(on_off.get("off_forced_off_records", 0)),
            "off_l4_served": int(on_off.get("off_l4_served", -1)),
            "legacy_l4_served": int(on_off.get("legacy_l4_served", -1)),
            "off_eq_legacy_product": on_off.get("off_eq_legacy_product") is True,
            "note": "L4 on(四级链)vs off(L4 强关=L3 截断):digest 必分叉 = proxy 贡献真实进入画面;"
                    "off == legacy(None 旧三级)产物位级 = 截断语义等价;ForcedOff 记录随行",
        }
        set_fact(
            "on_off_relation",
            not on_off_fails,
            f"on/off digest 分叉={on_off_doc['digest_differs']} ForcedOff={on_off_doc['off_forced_off_records']} "
            f"off_served={on_off_doc['off_l4_served']} legacy_served={on_off_doc['legacy_l4_served']} "
            f"off==legacy(截断等价)={on_off_doc['off_eq_legacy_product']}"
            + ("" if not on_off_fails else f";红 {on_off_fails[:2]}"),
        )
        # ⑨
        scene = hd.get("scene") or {}
        coverage_fails = coverage_judge(scene, l4, hchecks)
        pph = counters_doc["per_proxy_hits"]
        coverage_doc = {
            "per_proxy_min_hit": min(pph) if pph else 0,
            "device_indices_equal_host": hchecks.get("device_host_parity_structural") is True,
            "all_levels_served": hchecks.get("level_coverage_all_four_used") is True,
        }
        set_fact(
            "proxy_coverage_verified",
            not coverage_fails,
            f"host 参考投影覆盖:per_proxy_hits={pph} min={coverage_doc['per_proxy_min_hit']} "
            f"device 下标全等={coverage_doc['device_indices_equal_host']} 四级全消费={coverage_doc['all_levels_served']}"
            + ("" if not coverage_fails else f";红 {coverage_fails[:2]}"),
        )
        # ⑩
        fm = hd.get("frame_ms") or {}
        try:
            dispatch_ms = float(fm.get("l4_device_dispatch_ms", "nan"))
            on_ms = float(fm.get("frame_on_ms", "nan"))
            off_ms = float(fm.get("frame_off_ms", "nan"))
        except (TypeError, ValueError):
            dispatch_ms = on_ms = off_ms = float("nan")
        fm_frames = fm.get("frames", 0)
        frame_ms_ok = frame_ms_sane(dispatch_ms, on_ms, off_ms) and fm_frames == frames
        frame_ms_doc = {
            "l4_device_dispatch_ms": dispatch_ms,
            "frame_on_ms": on_ms,
            "frame_off_ms": off_ms,
            "on_over_off": on_ms / off_ms if off_ms > 0 else float("nan"),
            "frames": frames,
            "note": (
                f"canonical 远场契约 64×64 --release 同机同窗 measured_local:L4 纯派发={dispatch_ms:.4f}ms "
                f"帧管 on={on_ms:.4f}ms off={off_ms:.4f}ms(on/off={on_ms / off_ms if off_ms > 0 else float('nan'):.4f};"
                f"各 {frames} 帧;帧管 = GBuffer+host 三腿+L4 镜像+装配 host 壁钟 + kernel 纯派发独立口径);"
                "如实登记不设通过线(G6 无硬门纪律)"
            ),
        }
        set_fact(
            "frame_ms_measured",
            frame_ms_ok,
            f"measured:L4 纯派发={dispatch_ms:.4f}ms 帧管 on={on_ms:.4f}ms off={off_ms:.4f}ms"
            f"(各 {frames} 帧真实命令输出,如实登记)",
        )
        # ⑫
        set_fact(
            "scene_contract_four_tier",
            hchecks.get("level_coverage_all_four_used") is True,
            f"canonical 远场契约({SCENE}):golden served L1/L2/L3/L4 全 > 0"
            f"(四级覆盖充分性,空转即 RED)={hchecks.get('level_coverage_all_four_used')}",
        )
    else:
        fail("harness 主跑/evidence 未绿(判读前置失败)")

    # ── ⑧ 四 RED 臂子进程独立检出 ──
    red_doc = {arm.replace("-", "_"): red_arm_rcs.get(arm) == 0 for arm in RED_ARMS}
    red_ok = all(red_doc.values())
    set_fact(
        "red_arms_detected",
        red_ok,
        "四 RED 臂子进程 rc=0 独立检出:"
        + ",".join(f"{arm}={'检出' if red_arm_rcs.get(arm) == 0 else '失效'}" for arm in RED_ARMS),
    )

    # ── ⑪ 冻结面 0-byte 机核(工作树 porcelain;在飞波改动面不在本任务闭集)──
    u = run(["git", "status", "--porcelain", "--", *FROZEN_M98_KERNELS])
    m98_kernels_clean = not u.stdout.strip()
    u = run(["git", "status", "--porcelain", "--", *FROZEN_G9_BAND])
    g9_band_clean = not u.stdout.strip()
    u = run(["git", "status", "--porcelain", "--", *FROZEN_ANCHORS])
    anchors_clean = not u.stdout.strip()
    frozen_doc = {
        "m98_kernels_0byte": m98_kernels_clean,
        "g9_depth_band_0byte": g9_band_clean,
        "handover_anchors_0byte": anchors_clean,
    }
    set_fact(
        "frozen_surfaces_0byte",
        m98_kernels_clean and g9_band_clean and anchors_clean,
        f"m98 双 kernel 0-byte={m98_kernels_clean};g9 深度带 0-byte={g9_band_clean};"
        f"g30 承接锚表 + G27_P2 原始锚 0-byte={anchors_clean}(原始锚不回写)",
    )

    # ── 门裁决(facts 全绿 + 渲染腿全绿 + FAILURES 空)──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and legs_ok and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti(本机单卡 measured_local)",
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
            "state": "pass" if not g9_fails else "fail",
            "verdict": "PASS" if not g9_fails else "FAIL",
            "checks_all_true": not g9_fails,
            "device_executed": not g9_fails,
            "evidence": g9_ev,
        },
        "host_tests": {
            "fallback_chain_tests_pass": bool(facts["host_tests_anchored"]["status"] == "PASS"),
            "test_count": EXPECTED_TEST_COUNT,
        },
        "device_leg_parity": parity_doc if parity_doc else {
            "structural_exact": False, "rgb_bitexact": False,
            "t_residual_max_abs": -1.0, "t_residual_rel_max": -1.0,
        },
        "determinism_double_run": dr_doc if dr_doc else {
            "device_leg_bitexact": False, "golden_bitexact": False,
            "device_golden_eq_host_golden": False,
        },
        "l4_counters": counters_doc if counters_doc else {
            "rays_attempted": 0, "rays_hit": 0, "pixels_served": 0, "work_count": 0,
            "hit_rate": 0.0, "transitions_to_l4": 0, "transitions_to_l4_miss": 0,
            "transitions_to_l4_out_of_range": 0, "proxies_visible": 0,
            "proxies_total": EXPECTED_PROXIES, "coverage": 0.0,
            "per_proxy_hits": [0] * EXPECTED_PROXIES,
        },
        "unlock_entries": unlock_doc if unlock_doc else {
            "trigger_ready": False, "serve_ok": False,
            "counters_non_zero": False, "half_missing_fail_closed": False,
        },
        "on_off_relation": on_off_doc if on_off_doc else {
            "on_digest": "0" * 64, "off_digest": "0" * 64, "legacy_digest": "0" * 64,
            "digest_differs": False, "off_forced_off_records": 0,
            "off_l4_served": -1, "legacy_l4_served": -1,
            "off_eq_legacy_product": False, "note": "渲染腿未齐备(前置失败)",
        },
        "red_arms": red_doc,
        "proxy_coverage": coverage_doc if coverage_doc else {
            "per_proxy_min_hit": 0, "device_indices_equal_host": False, "all_levels_served": False,
        },
        "frame_ms_measured": frame_ms_doc if frame_ms_doc else {
            "l4_device_dispatch_ms": -1.0, "frame_on_ms": -1.0, "frame_off_ms": -1.0,
            "on_over_off": -1.0, "frames": frames, "note": "渲染腿未齐备(前置失败)",
        },
        "frozen_surfaces": frozen_doc,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C12 HLOD L4 Far Field 档(M98-l4 承接锚两半合取兑现):"
            "①device 腿 = kernels/g31_hlod_l4_proxy_trace.rx 纯 compute(不消费 TLAS,L1 同构面)+ "
            "host 镜像 gi::fallback_chain::l4_trace_ray/l4_leg_host 逐字同源;②L4 计数器接入 = L4 槽位"
            "真实计数 + 可见 proxy 数 + 切换次数 + 覆盖率进 evidence 面,三处 fail-closed 入口按锚字面"
            "解锁(proxy 集装载 ⇒ Ready/Ok/非零;空集 ⇒ NotTriggered/Err/InvalidConfig 半齐不冒充)。"
            "判据:①host 19 单测 ②g9.p0.m98 复跑全绿(L1-L3 链 0-byte 行为机器证明)③device/host "
            "结构域精确对拍 + rgb 位级(t 残差信息项)④device/golden 双跑位级 + device==host golden "
            "⑤L4 计数面真实非空 ⑥三入口解锁 + 半齐 fail-closed ⑦on/off 分叉 + off==legacy 截断等价 "
            f"⑧四 RED 臂独立检出={red_ok} ⑨投影覆盖逐件 ≥1 + 四级全消费 ⑩frame_ms measured"
            f"(L4 纯派发={frame_ms_doc.get('l4_device_dispatch_ms', -1.0):.4f}ms "
            f"帧管 on={frame_ms_doc.get('frame_on_ms', -1.0):.4f}ms off={frame_ms_doc.get('frame_off_ms', -1.0):.4f}ms,"
            "如实登记不设通过线)⑪m98 双 kernel/深度带/承接锚 0-byte ⑫契约四级覆盖充分性。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in fact_rows)}"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED;PASS-only 闭集面)

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_hlod_l4_{ts}.json"
    else:
        # FAIL 诊断件落 .tmp 工作区——PASS-only schema 面,evidence/ 只收门件
        # (fail-closed:evidence/ 无件 = 门未过,不污染 check_schemas 路由面)。
        gate_path = WORK / f"gate_fail_{ts}.json"
    with io.open(gate_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n")
    note(f"evidence: {gate_path.relative_to(ROOT)}(harness 真跑件留 .tmp 工作区)")

    # ── 改判登记(两半全齐 ⇒ 全齐程序产;PASS 才落,FAIL 不冒充)──
    if all_pass:
        rejudgment = {
            "schema": "rurix.g31.m98_l4_rejudgment.v1",
            "subject": "g31_m98_l4_rejudgment",
            "anchor": {
                "source": "milestones/g30/g30_campaign_handover_registry.json M98-l4 行(原始锚 0-byte 不回写)",
                "condition_literal": "HLOD proxy 追踪 device 腿 + L4 计数器接入(合取,两半全齐方改判)",
                "fallback_literal": "维持 L1/L2/L3 三级链 + fail-closed 入口不动(两半未齐时)",
                "prior_state": "G27.3 M-d 重判窗:两半树内实测 0/2(device 腿零实现 + 三处 fail-closed 入口在位)→ 维持三级链",
            },
            "halves": {
                "device_leg": {
                    "status": "implemented",
                    "kernel": "src/rurix-render/kernels/g31_hlod_l4_proxy_trace.rx",
                    "host_mirror": "src/rurix-render/src/gi/fallback_chain.rs(l4_trace_ray/l4_leg_host/l4_axis_inv 逐字同源)",
                    "harness": "src/rurix-render/src/bin/g31_hlod_l4_far_field.rs",
                    "dispatch": "rurix_rt::vk::run_compute 纯 compute(L1 同构面,不消费 TLAS,D2-Q9 射线流纪律)",
                    "parity": gate_doc["device_leg_parity"],
                },
                "l4_counters": {
                    "status": "wired",
                    "selector_surface": "gi::fallback_chain::select_pixel_l4/assemble_l4/audit_l4(L4Leg 接入面;None 委托 = 三级链旧世界位级不变)",
                    "telemetry": gate_doc["l4_counters"],
                },
            },
            "unlock_entries": {
                "check_l4_trigger": "恒 NotTriggered ⇒ proxy 集装载(非空)⇒ Ready{proxies:5};未装载维持 NotTriggered(半齐保护)",
                "l4_serve": "恒 Err(L4InterfaceNotReady) ⇒ proxy 集装载 ⇒ Ok(腿样本);未装载维持 Err(禁静默当绿)",
                "counters_slot": "L4 槽位恒零 ⇒ 启用且装载时真实计数(attempted/hit/served/work);None/强关维持全零显式",
            },
            "verdict": "rejudged-four-tier-chain",
            "verdict_literal": "两半全齐 ⇒ 按承接锚「+」合取字面执行改判:L1/L2/L3 三级链 → L1/L2/L3/L4 四级链(Far Field 档上线);"
                               "L1-L3 链 0-byte 行为由 g9.p0.m98.tracing_fallback_chain 复跑全绿机器钉死"
               ,
            "on_off": gate_doc["on_off_relation"],
            "frame_ms": gate_doc["frame_ms_measured"],
            "red_arms": gate_doc["red_arms"],
            "gate_evidence": str(gate_path.relative_to(ROOT)).replace("\\", "/"),
            "environment": env_info,
            "timestamp": ts,
            "notes": (
                "M98-l4 在 registry/deferred.json 无条目(承接锚链在 G20/G25/G27/G30 campaign handover "
                "registries,各期 P2 表原始锚 0-byte 不回写),deferred.json history 追加不适用如实登记;"
                "本件 = 改判登记唯一新面(锚定 RFC-0044 §4 重判窗程序 + G27.3 M-d evidence 行)。"
                "远场契约场景 = gi::fallback_chain::m98_l4_far_field_scene(构造远场契约,冻结常量);"
                "RXS-0396 世界级辐射缓存 ≠ RXS-0359 L4 Far Field 边界维持(本兑现与 GI 世界缓存无关)。"
            ),
        }
        with io.open(REJUDGMENT_PATH, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(rejudgment, ensure_ascii=False, indent=2) + "\n")
        note(f"rejudgment: {REJUDGMENT_PATH.relative_to(ROOT)}(两半全齐改判登记 = rejudged-four-tier-chain)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿两臂,无 GPU/无构建依赖)
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

    dg = lambda ch: ch * 64  # noqa: E731
    # 红绿臂①:对拍判。
    good_parity = {"structural_exact": True, "rgb_bitexact": True,
                   "t_residual_max_abs": "1.810000e3", "t_residual_rel_max": "3.0e-7"}
    expect(parity_judge(good_parity) == [], "GREEN:parity 正例")
    expect(parity_judge(dict(good_parity, structural_exact=False)), "RED:结构非精确必红")
    expect(parity_judge(dict(good_parity, rgb_bitexact=False)), "RED:rgb 非位级必红")
    expect(parity_judge(dict(good_parity, t_residual_max_abs="zz")), "RED:残差非数值必红")
    expect(parity_judge(dict(good_parity, t_residual_rel_max="-1")), "RED:残差负值必红")
    expect(parity_judge(None), "RED:parity 非 object 必红")
    # 红绿臂②:双跑判。
    good_dr = {"device_leg_bitexact": True, "golden_bitexact": True, "device_golden_eq_host_golden": True}
    expect(double_run_judge(good_dr) == [], "GREEN:双跑正例")
    expect(double_run_judge(dict(good_dr, golden_bitexact=False)), "RED:golden 分叉必红")
    expect(double_run_judge({}), "RED:空 double_run 必红")
    # 红绿臂③:计数面判。
    good_l4 = {
        "trigger_ready": True, "serve_ok": True,
        "counters": {"rays_attempted": 2176, "rays_hit": 1000, "pixels_served": 1813,
                     "work_count": 10880, "hit_rate": "4.595588e-1"},
        "transitions_to_l4": 1813, "transitions_to_l4_miss": 1813, "transitions_to_l4_out_of_range": 0,
        "proxies_visible": 5, "per_proxy_hits": [739, 65, 44, 55, 97], "coverage": "4.595588e-1",
    }
    expect(counters_judge(good_l4) == [], "GREEN:计数面正例")
    bad = json.loads(json.dumps(good_l4))
    bad["counters"]["rays_hit"] = 0
    expect(counters_judge(bad), "RED:零命中(空接线)必红")
    bad = json.loads(json.dumps(good_l4))
    bad["transitions_to_l4"] = 1800
    expect(counters_judge(bad), "RED:切换数 ≠ served 必红")
    bad = json.loads(json.dumps(good_l4))
    bad["proxies_visible"] = 4
    expect(counters_judge(bad), "RED:可见 proxy 缺件必红")
    bad = json.loads(json.dumps(good_l4))
    bad["per_proxy_hits"] = [739, 65, 44, 55, 0]
    expect(counters_judge(bad), "RED:单件零覆盖必红")
    bad = json.loads(json.dumps(good_l4))
    bad["counters"]["hit_rate"] = "1.5"
    expect(counters_judge(bad), "RED:hit_rate 越界必红")
    # 红绿臂④:解锁判。
    good_checks = {"empty_proxy_fail_closed": True}
    expect(unlock_judge(good_l4, good_checks) == [], "GREEN:解锁正例")
    expect(unlock_judge(dict(good_l4, trigger_ready=False), good_checks), "RED:未 Ready 必红")
    expect(unlock_judge(dict(good_l4, serve_ok=False), good_checks), "RED:未 Ok 必红")
    expect(unlock_judge(good_l4, {"empty_proxy_fail_closed": False}), "RED:半齐冒充必红")
    # 红绿臂⑤:on/off 判。
    good_on_off = {
        "on_digest": dg("a"), "off_digest": dg("b"), "legacy_digest": dg("b"),
        "digest_differs": True, "off_forced_off_records": 1813,
        "off_l4_served": 0, "legacy_l4_served": 0, "off_eq_legacy_product": True,
    }
    expect(on_off_judge(good_on_off) == [], "GREEN:on/off 正例")
    expect(on_off_judge(dict(good_on_off, digest_differs=False)), "RED:未分叉必红")
    expect(on_off_judge(dict(good_on_off, off_forced_off_records=0)), "RED:零 ForcedOff 必红")
    expect(on_off_judge(dict(good_on_off, off_l4_served=3)), "RED:off 面有 L4 服务必红")
    expect(on_off_judge(dict(good_on_off, off_eq_legacy_product=False)), "RED:截断不等价必红")
    expect(on_off_judge(dict(good_on_off, on_digest="zz")), "RED:digest 形态非法必红")
    # 红绿臂⑥:覆盖判。
    good_scene = {"name": SCENE, "pixels": 4096, "chain_pixels": 2176, "proxies_total": 5}
    good_cov_checks = {"device_host_parity_structural": True, "level_coverage_all_four_used": True}
    expect(coverage_judge(good_scene, good_l4, good_cov_checks) == [], "GREEN:覆盖正例")
    bad_l4 = json.loads(json.dumps(good_l4))
    bad_l4["per_proxy_hits"] = [739, 65, 0, 55, 97]
    expect(coverage_judge(good_scene, bad_l4, good_cov_checks), "RED:单件零覆盖必红")
    expect(coverage_judge(good_scene, good_l4, dict(good_cov_checks, level_coverage_all_four_used=False)),
           "RED:四级未全消费必红")
    expect(coverage_judge(dict(good_scene, proxies_total=4), good_l4, good_cov_checks),
           "RED:proxy 总数漂移必红")
    # 红绿臂⑦:g9 维持判。
    def g9_doc(status: str = "pass", drop: str | None = None) -> dict:
        checks = {k: True for k in G9_CHECK_KEYS}
        if drop:
            checks.pop(drop, None)
        return {"status": status, "checks": checks}
    expect(g9_maintenance_judge(0, g9_doc()) == [], "GREEN:g9 全绿正例")
    expect(g9_maintenance_judge(1, g9_doc()), "RED:子进程 rc≠0 必红")
    expect(g9_maintenance_judge(0, g9_doc(status="fail")), "RED:status 非 pass 必红")
    expect(g9_maintenance_judge(0, g9_doc(drop="device_l4_not_triggered")), "RED:checks 缺键必红")
    expect(g9_maintenance_judge(0, None), "RED:evidence 缺失必红")
    # 红绿臂⑧:frame_ms/三态判。
    expect(frame_ms_sane(60.6, 3.4, 3.2), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(60.6, 0.0, 3.2), "RED:0ms 必红")
    expect(not frame_ms_sane(60.6, float("nan"), 3.2), "RED:NaN 必红")
    expect(not frame_ms_sane("zz", 3.4, 3.2), "RED:非数值必红")
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
                "gate_maintenance", "host_tests", "device_leg_parity",
                "determinism_double_run", "l4_counters", "unlock_entries",
                "on_off_relation", "red_arms", "proxy_coverage",
                "frame_ms_measured", "frozen_surfaces", "environment",
                "timestamp", "notes",
            ]),
            "schema required 闭集互核(19 字段)",
        )
        lc_req = gs["properties"]["l4_counters"].get("required", [])
        expect(
            all(k in lc_req for k in ("transitions_to_l4", "proxies_visible", "coverage", "per_proxy_hits")),
            "l4_counters required ⊇ 切换次数/可见 proxy 数/覆盖率/逐件命中",
        )
        expect(gs["properties"]["l4_counters"]["properties"]["proxies_total"]["const"] == EXPECTED_PROXIES,
               "proxies_total const=5 互核")
    expect(len(FACT_IDS) == 12, "facts 闭集 = 12")
    expect(len(RED_ARMS) == 4, "RED 臂闭集 = 4")
    expect(len(G9_CHECK_KEYS) == 14, "g9 checks 映射 = 14")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=12;8 红臂组 + 正例组 + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=32)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 8:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 8(schema 下限)", file=sys.stderr)
            return 1
        return run_gate(args.frames)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
