#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude(G35 GPU 粒子系统 G35-8 作者面与 SDK)
"""G35-8:数据驱动作者面与 SDK 门冒烟(g35.wave8.authoring;声明式 emitter
资产〔JSON v1 十字段闭集,RFC-0049 §3 冻结〕fail-closed 解析 + 参数化映射
〔EmitterAsset::to_desc → particles::core::EmitterDesc + emit_count_at 曲线
求值〕+ 热重载〔纯参数面变化,池/pid/帧钟连续,下一帧生效〕+ SDK C ABI 加性
〔RFC-0049 §4.11 冻结四签名 rxsdk_particles_*,实现层 src/rurix-renderer-sdk;
v1 SDK 粒子臂 = host 臂,device 车道接线与用户面 MINOR 1.1.0 归收口批,
apps/g31-renderer-sdk/API_VERSIONING.md §6 登记〕——host 金标准 =
src/rurix-render/src/particles/{emitter_asset,core}.rs,probe =
src/rurix-render/src/bin/g35_authoring_probe.rs 纯 host 全链 64 帧)。

八面判据(facts 闭集):
1. **asset_schema_fail_closed**:十种非法资产(缺字段/多字段/类型错×2/
   闭集外枚举×3/嵌套缺字段/域违约×2)逐一 probe typed 退出码 3 +
   AUTHORING_ASSET_ERR kind 闭集 token 精确匹配(禁默认值兜底;kind 闭集 =
   Json/NotObject/MissingField/UnknownField/Type/EnumOutOfSet/Domain)。
2. **curve_eval_deterministic**:emit_curve 求值(const 恒值取整/step 阶梯
   查表)库实现 vs probe 独立参考实现逐帧互核 + 双求值确定(scan.rs 双实现
   互核先例)。
3. **hot_reload_semantics**:--reload-at 32 --asset2 重载 = 纯参数面变化——
   重载轨迹 digest ≠ 无重载基线(生效)+ 重载帧 emit == asset2 曲线该帧
   求值且 ≠ asset1 同帧求值(下一帧生效)+ 旧粒子当前 gravity 下冻结运算序
   单步重放 bitwise 全等(不瞬移,边界核验样本量 ≥ 1)。
4. **pid_continuity_across_reload**:每帧无重复 pid + 幸存段 ⊆ 上帧集 +
   新发射段 == [pid_base, pid_base+emit) 精确区间 + 跨重载 pid_base 单调
   不重置;发射钳制 rejected 如实登记(RFC-0049 §4.4 F7)。
5. **sdk_abi_surface_frozen**:RFC-0049 §4.11 冻结四签名源级字面在档
   (参数名/类型序/返回精确正则)+ cdylib 符号面 4 新 + 9 既有全在
   (dumpbin /exports 可用则 dumpbin,否则 cargo-test-source 路如实登记)+
   ci/stable_snapshot.py --check 绿(用户面 sdk.rx 导出集/ABI 版本 1.0.0
   0-byte 机器证明——加性纪律;用户面薄转发 + MINOR bump 待收口批)。
6. **sdk_handle_fail_closed**:cargo test -p rurix-renderer-sdk --features
   sdk-device 全绿 + 粒子面两单测 ok 在案(悬空句柄 ST_HANDLE/空指针与
   资产违例与 key 闭集外 ST_INPUT/会话回收连带悬空/tick 精确账目)。
7. **determinism_double_run**:同 seed 同资产全链双跑轨迹 digest 位级一致
   (digest = 逐帧 n‖pid‖8 f32 流 bits‖args 链式 sha256)。
8. **red_arm_effective**:--red-arm field-tamper 资产字段篡改必检出双面——
   值篡改(gravity_y/vel_base 内存面)digest 必异 + 文本注入闭集外字段
   解析必 typed Err。

三态(形骸保留,评注):本门**纯 host**,无 kernels_spv_valid/frame_ms 面,
--gate 不占 GPU 锁,host 面永可跑——degrade 集常空;唯一可能登记项 = SDK
单测 vulkan loader 缺席跳过面(仅 loader 探测零 GPU 工作)。降级时
RURIX_REQUIRE_REAL=1 翻硬 FAIL,否则 SKIP 退 0 不冒充 PASS(三态同族一致)。

用法:
  py -3 ci/g35_authoring_smoke.py --selftest
  py -3 ci/g35_authoring_smoke.py --gate g35.wave8.authoring [--frames 64] [--cap 2048] [--seed 42]
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

GATE_KEY = "g35.wave8.authoring"
SUBJECT = "g35_authoring"
WAVE = "G35.8"
TAG = "g35_authoring"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_authoring_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.authoring_gate_evidence.v1"
SDK_LIB_RS = ROOT / "src" / "rurix-renderer-sdk" / "src" / "lib.rs"
WORK = ROOT / ".tmp" / "g35_gates" / "authoring"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_authoring_probe{EXE_SUFFIX}"
SDK_DLL = ROOT / "target" / "debug" / "rurix_renderer_sdk.dll"
# dumpbin 定位(ci/g31_renderer_sdk_smoke.py MSVC pin 同源;缺席走
# cargo-test-source 路如实登记)。
MSVC_BIN = Path(r"C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.44.35207/bin/Hostx64/x64")

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "asset_schema_fail_closed",
    "curve_eval_deterministic",
    "hot_reload_semantics",
    "pid_continuity_across_reload",
    "sdk_abi_surface_frozen",
    "sdk_handle_fail_closed",
    "determinism_double_run",
    "red_arm_effective",
]

# v1 十字段闭集(RFC-0049 §3 冻结;emitter_asset.rs FIELDS 同一字面)。
FIELD_CLOSED_SET = [
    "name", "pos", "spread", "vel_base", "vel_spread",
    "life_base", "gravity_y", "emit_curve", "render", "blend",
]

# RFC-0049 §4.11 冻结四签名(源级字面正则;实现层 lib.rs 消费面——
# usize == size_t,u64 句柄,i32 状态码)。
FROZEN_SIG_RES = {
    "rxsdk_particles_emitter_create": (
        r'pub extern "C" fn rxsdk_particles_emitter_create\(\s*sys:\s*u64,\s*'
        r'desc_json:\s*\*const u8,\s*len:\s*usize,\s*out:\s*\*mut u64,?\s*\)\s*->\s*i32'
    ),
    "rxsdk_particles_emitter_set_param": (
        r'pub extern "C" fn rxsdk_particles_emitter_set_param\(\s*h:\s*u64,\s*'
        r'key:\s*\*const u8,\s*klen:\s*usize,\s*value:\s*f32,?\s*\)\s*->\s*i32'
    ),
    "rxsdk_particles_emitter_destroy": (
        r'pub extern "C" fn rxsdk_particles_emitter_destroy\(\s*h:\s*u64,?\s*\)\s*->\s*i32'
    ),
    "rxsdk_particles_stats": (
        r'pub extern "C" fn rxsdk_particles_stats\(\s*sys:\s*u64,\s*out:\s*\*mut u64,?\s*\)\s*->\s*i32'
    ),
}
PARTICLES_SYMBOLS = list(FROZEN_SIG_RES.keys())
# 既有 9 rxsdk_* 导出闭集(加性 0-byte 面;apps/g31-renderer-sdk 纪律)。
EXISTING_RXSDK_SYMBOLS = [
    "rxsdk_abi_version",
    "rxsdk_caps_probe",
    "rxsdk_renderer_create",
    "rxsdk_renderer_destroy",
    "rxsdk_renderer_load_scene",
    "rxsdk_renderer_set_camera",
    "rxsdk_renderer_set_exposure_ev100",
    "rxsdk_renderer_render_frame",
    "rxsdk_renderer_present",
]
PARTICLES_TEST_NAMES = [
    "particles_handle_and_input_fail_closed",
    "particles_lifecycle_tick_and_stats",
]

# 十种非法资产(case, 变换, 期望 kind)——emitter_asset.rs 单测闭集同族,
# 经 probe CLI typed 退出码路真跑。
BASE_ASSET = {
    "name": "closed_set_probe",
    "pos": [0.0, 1.0, -0.5],
    "spread": [0.4, 0.2, 0.4],
    "vel_base": [0.0, 3.0, 0.0],
    "vel_spread": [1.0, 0.5, 1.0],
    "life_base": 1.2,
    "gravity_y": -9.8,
    "emit_curve": {"kind": "const", "value": 24.7},
    "render": "billboard",
    "blend": "alpha",
}


def illegal_cases() -> list[tuple[str, dict, str]]:
    """(case 名, 资产 dict, 期望 kind)十例闭集(纯函数,selftest 消费)。"""
    def mk(**kv) -> dict:
        d = json.loads(json.dumps(BASE_ASSET))
        for k, v in kv.items():
            if v is None:
                d.pop(k, None)
            else:
                d[k] = v
        return d

    return [
        ("missing_life_base", mk(life_base=None), "MissingField"),
        ("unknown_field_drag", mk(drag=0.1), "UnknownField"),
        ("type_pos_scalar", mk(pos=1.0), "Type"),
        ("type_name_number", mk(name=42), "Type"),
        ("enum_render_out", mk(render="sprite"), "EnumOutOfSet"),
        ("enum_blend_out", mk(blend="multiply"), "EnumOutOfSet"),
        ("enum_curve_kind_out", mk(emit_curve={"kind": "ramp", "value": 1.0}), "EnumOutOfSet"),
        ("curve_const_missing_value", mk(emit_curve={"kind": "const"}), "MissingField"),
        ("step_len_mismatch", mk(emit_curve={"kind": "step", "frames": [0, 8], "values": [1.0]}), "Domain"),
        ("step_not_increasing", mk(emit_curve={"kind": "step", "frames": [0, 8, 8], "values": [1.0, 2.0, 3.0]}), "Domain"),
    ]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                          encoding="utf-8", errors="replace", timeout=timeout, env=env)


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面;全纯函数零构建零 GPU)
# ---------------------------------------------------------------------------


def _b(v) -> bool:
    return v is True


def asset_err_kind(stderr: str) -> str | None:
    """probe typed 退出 token 解析(AUTHORING_ASSET_ERR kind=<K>)。"""
    m = re.search(r"AUTHORING_ASSET_ERR kind=(\w+)", stderr or "")
    return m.group(1) if m else None


def fail_closed_ok(cases: list[dict]) -> bool:
    """① 十例闭集全过:exit==3 + kind 精确匹配(空集/缺例必红)。"""
    return (
        len(cases) == 10
        and all(
            c.get("ok") is True
            and c.get("exit_code") == 3
            and c.get("kind_seen") == c.get("kind_expected")
            for c in cases
        )
    )


def curve_ok(doc: dict) -> bool:
    """② 曲线求值互核判:旗标 + 双臂样本非空。"""
    return (
        _b(doc.get("curve_crosscheck_ok"))
        and bool(doc.get("curve_samples_a"))
        and bool(doc.get("curve_samples_b"))
    )


def hot_reload_ok(doc: dict) -> bool:
    """③ 热重载语义判:生效 + 下一帧生效 + 不瞬移 + 边界样本量 + digest
    形态合法且必异(自相矛盾拒)。"""
    base, rel = doc.get("digest_baseline"), doc.get("digest_a")
    return (
        _b(doc.get("reload_effective_digest_diff"))
        and _b(doc.get("reload_next_frame_effective"))
        and _b(doc.get("old_particles_continuous"))
        and isinstance(doc.get("boundary_survivors_checked"), int)
        and doc["boundary_survivors_checked"] >= 1
        and isinstance(base, str)
        and isinstance(rel, str)
        and DIGEST_RE.match(base) is not None
        and DIGEST_RE.match(rel) is not None
        and base != rel
    )


def pid_ok(doc: dict) -> bool:
    """④ pid 连续判:三旗标 + 发行水位非零。"""
    return (
        _b(doc.get("pid_unique"))
        and _b(doc.get("pid_survivor_subset"))
        and _b(doc.get("pid_emit_range_exact"))
        and isinstance(doc.get("pids_issued"), int)
        and doc["pids_issued"] >= 1
    )


def sdk_surface_ok(sig_ok: bool, symbols_ok: bool, snapshot_exit: int) -> bool:
    """⑤ SDK 面判:冻结签名源级 + 符号面 + stable 快照 0 漂移三合取。"""
    return sig_ok is True and symbols_ok is True and snapshot_exit == 0


def sdk_tests_ok(test_exit: int, out: str) -> bool:
    """⑥ SDK 单测判:cargo test 退 0 + 粒子面两单测 ok 在案。"""
    return test_exit == 0 and all(
        re.search(rf"test sdk::tests::{re.escape(n)} \.\.\. ok", out or "") for n in PARTICLES_TEST_NAMES
    )


def determinism_ok(doc: dict) -> bool:
    """⑦ 双跑位级判:旗标 + digest 形态 + a == b 互核。"""
    a, b = doc.get("digest_a"), doc.get("digest_b")
    return (
        _b(doc.get("double_run_bitexact"))
        and isinstance(a, str)
        and isinstance(b, str)
        and DIGEST_RE.match(a) is not None
        and a == b
    )


def red_ok(doc: dict) -> bool:
    """⑧ RED 臂判:field-tamper 双面(值篡改 digest 必异 + schema 篡改
    typed 检出)。"""
    g, r = doc.get("digest_green"), doc.get("digest_red")
    return (
        doc.get("arm") == "field-tamper"
        and _b(doc.get("detected"))
        and _b(doc.get("schema_tamper_detected"))
        and doc.get("schema_tamper_kind") == "UnknownField"
        and isinstance(g, str)
        and isinstance(r, str)
        and DIGEST_RE.match(g) is not None
        and DIGEST_RE.match(r) is not None
        and g != r
    )


def frozen_sigs_in_source(text: str) -> tuple[bool, list[str]]:
    """冻结四签名源级机核(参数名/类型序/返回精确;缺一即红)。"""
    missing = [n for n, pat in FROZEN_SIG_RES.items() if re.search(pat, text or "", re.DOTALL) is None]
    return (not missing, missing)


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    """三态裁决形骸(本门纯 host,degrade 常空):无降级 → None(续跑);
    降级 + REQUIRE_REAL → 1(硬红);降级无 REQUIRE_REAL → 0(SKIP)。"""
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


def resolve_dumpbin() -> Path | None:
    p = MSVC_BIN / "dumpbin.exe"
    if p.is_file():
        return p
    from shutil import which

    w = which("dumpbin")
    return Path(w) if w else None


def run_gate(frames: int, cap: int, seed: int) -> int:
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:220]}")

    if not GATE_SCHEMA_PATH.is_file():
        fail(f"gate schema 缺失: {GATE_SCHEMA_PATH}")
        return 1

    # ── 构建(host probe bin + SDK crate;本门纯 host 不占 GPU 锁)──
    ok = build_or_fail(
        ["cargo", "build", "-p", "rurix-render", "--bin", "g35_authoring_probe", "--quiet"],
        "probe bin(host-only 无 features)",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurix-renderer-sdk", "--features", "sdk-device", "--quiet"],
        "SDK crate(sdk-device 特征面)",
    )
    if not ok:
        return 1

    WORK.mkdir(parents=True, exist_ok=True)
    degrade: list[str] = []  # 三态形骸:本门纯 host 恒可跑,常空(评注见 docstring)。

    # ── 内嵌契约样例写 .tmp(不进 milestones)──
    assets_dir = WORK / "assets"
    r = run([str(BIN), "--write-samples", str(assets_dir)])
    m = re.search(r"AUTHORING_SAMPLES a=(\S+) b=(\S+)", r.stdout or "")
    if r.returncode != 0 or m is None:
        fail(f"--write-samples 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}")
        return 1
    asset_a, asset_b = m.group(1), m.group(2)

    # ── 绿臂(双跑 + 基线 + 热重载判据全内置;--evidence-out 留 .tmp)──
    ev_green = WORK / "probe_green.json"
    rg = run([str(BIN), "--asset", asset_a, "--asset2", asset_b, "--reload-at", "32",
              "--frames", str(frames), "--cap", str(cap), "--seed", str(seed),
              "--evidence-out", str(ev_green)])
    doc_green: dict = {}
    if ev_green.is_file():
        try:
            doc_green = json.loads(ev_green.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            fail("绿臂 evidence JSON 解析失败")
    if rg.returncode != 0:
        fail(f"绿臂真跑失败 rc={rg.returncode}: {(rg.stdout + rg.stderr)[-300:]}")

    # ── 红臂(field-tamper 双面)──
    ev_red = WORK / "probe_red.json"
    rr = run([str(BIN), "--red-arm", "field-tamper", "--asset", asset_a,
              "--frames", str(frames), "--cap", str(cap), "--seed", str(seed),
              "--evidence-out", str(ev_red)])
    doc_red: dict = {}
    if ev_red.is_file():
        try:
            doc_red = json.loads(ev_red.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            fail("红臂 evidence JSON 解析失败")
    if rr.returncode != 0:
        fail(f"红臂真跑失败 rc={rr.returncode}: {(rr.stdout + rr.stderr)[-300:]}")

    # ── ① 十种非法资产 typed 退出码(fail-closed 真跑)──
    case_rows: list[dict] = []
    for case, asset, expected in illegal_cases():
        p = WORK / f"bad_{case}.json"
        p.write_text(json.dumps(asset, ensure_ascii=False), encoding="utf-8")
        rc = run([str(BIN), "--asset", str(p)], timeout=600)
        seen = asset_err_kind(rc.stderr) or "NONE"
        case_rows.append({
            "case": case,
            "kind_expected": expected,
            "kind_seen": seen,
            "exit_code": rc.returncode,
            "ok": bool(rc.returncode == 3 and seen == expected),
        })
    cases_ok = sum(1 for c in case_rows if c["ok"])
    set_fact(
        "asset_schema_fail_closed",
        fail_closed_ok(case_rows),
        f"十种非法(缺字段/多字段/类型错×2/闭集外枚举×3/嵌套缺字段/域违约×2)"
        f"逐一 typed 退出码 3 + kind 精确匹配:{cases_ok}/10"
        f"(违例: {[c['case'] for c in case_rows if not c['ok']] or '无'};十字段闭集 RFC-0049 §3 冻结,禁默认值兜底)",
    )

    # ── ②③④⑦(绿臂 evidence 判读)──
    set_fact(
        "curve_eval_deterministic",
        curve_ok(doc_green),
        f"emit_curve 求值互核(库实现 vs probe 独立参考实现逐帧全等 + 双求值确定)"
        f"= {doc_green.get('curve_crosscheck_ok')!r};A(const)头样本 {doc_green.get('curve_samples_a')!r} "
        f"B(step)头样本 {doc_green.get('curve_samples_b')!r}",
    )
    set_fact(
        "hot_reload_semantics",
        hot_reload_ok(doc_green),
        f"重载生效(digest 异基线)={doc_green.get('reload_effective_digest_diff')!r} "
        f"下一帧生效(重载帧 emit=={doc_green.get('accepted_at_reload')!r}==asset2 曲线求值)"
        f"={doc_green.get('reload_next_frame_effective')!r} 旧粒子不瞬移(冻结运算序单步重放 bitwise)"
        f"={doc_green.get('old_particles_continuous')!r}(边界幸存核验 {doc_green.get('boundary_survivors_checked')!r})",
    )
    set_fact(
        "pid_continuity_across_reload",
        pid_ok(doc_green),
        f"pid 唯一={doc_green.get('pid_unique')!r} 幸存段 ⊆ 上帧集={doc_green.get('pid_survivor_subset')!r} "
        f"发射段精确区间={doc_green.get('pid_emit_range_exact')!r} 跨重载水位单调 pids_issued="
        f"{doc_green.get('pids_issued')!r}(钳制 rejected={doc_green.get('rejected_total')!r} 如实登记,F7)",
    )
    set_fact(
        "determinism_double_run",
        determinism_ok(doc_green),
        f"同 seed 同资产全链双跑 digest 位级一致={doc_green.get('double_run_bitexact')!r}"
        f"(digest_a={str(doc_green.get('digest_a'))[:23]}…;逐帧 n‖pid‖8 f32 流‖args 链式 sha256)",
    )

    # ── ⑤ SDK ABI 面(源级冻结签名 + 符号面 + stable 快照 0 漂移)──
    lib_text = SDK_LIB_RS.read_text(encoding="utf-8") if SDK_LIB_RS.is_file() else ""
    sig_ok, sig_missing = frozen_sigs_in_source(lib_text)
    dumpbin = resolve_dumpbin()
    symbol_method = "dumpbin" if (dumpbin and SDK_DLL.is_file()) else "cargo-test-source"
    if symbol_method == "dumpbin":
        de = run([str(dumpbin), "/exports", str(SDK_DLL)], timeout=600)
        exports_text = de.stdout or ""
        new_present = all(re.search(rf"\b{s}\b", exports_text) for s in PARTICLES_SYMBOLS)
        old_present = all(re.search(rf"\b{s}\b", exports_text) for s in EXISTING_RXSDK_SYMBOLS)
        symbols_ok = de.returncode == 0 and new_present and old_present
        sym_detail = f"dumpbin /exports:4 新符号全在={new_present} 既有 9 rxsdk_* 全在={old_present}"
    else:
        # 回退路(任务字面:dumpbin 不可用则 cargo test 层核验):源级
        # #[unsafe(no_mangle)] extern "C" 标记 + cdylib 构建绿 + 单测直调
        # 四函数(下 fact ⑥)= 符号存在性链条,如实登记方法。
        new_present = all(
            re.search(rf'#\[unsafe\(no_mangle\)\]\s*pub extern "C" fn {s}\b', lib_text, re.DOTALL)
            for s in PARTICLES_SYMBOLS
        )
        old_present = all(
            re.search(rf'pub extern "C" fn {s}\b', lib_text) for s in EXISTING_RXSDK_SYMBOLS
        )
        symbols_ok = new_present and old_present and SDK_DLL.is_file()
        sym_detail = f"cargo-test-source 路:no_mangle 源级 4 新={new_present} 既有 9={old_present} cdylib 在={SDK_DLL.is_file()}"
    snap = run(["py", "-3", "ci/stable_snapshot.py", "--check"], timeout=600)
    set_fact(
        "sdk_abi_surface_frozen",
        sdk_surface_ok(sig_ok, symbols_ok, snap.returncode),
        f"RFC-0049 §4.11 冻结四签名源级字面在档={sig_ok}(缺={sig_missing or '无'});{sym_detail};"
        f"stable_snapshot --check exit={snap.returncode}(用户面 sdk.rx 导出集/ABI 1.0.0 0-byte 机器证明——"
        f"rxsdk_* 为内部实现面加性,用户面薄转发+MINOR 1.1.0 待收口批,API_VERSIONING.md §6 登记)",
    )

    # ── ⑥ SDK 句柄/状态码 fail-closed(cargo test 真跑)──
    rt = run(["cargo", "test", "-p", "rurix-renderer-sdk", "--features", "sdk-device"], timeout=7200)
    test_out = (rt.stdout or "") + (rt.stderr or "")
    if "skip: vulkan loader" in test_out:
        degrade.append("SDK 粒子生命周期单测 vulkan loader 缺席跳过(仅 loader 探测面)")
    seen_tests = [n for n in PARTICLES_TEST_NAMES
                  if re.search(rf"test sdk::tests::{re.escape(n)} \.\.\. ok", test_out)]
    set_fact(
        "sdk_handle_fail_closed",
        sdk_tests_ok(rt.returncode, test_out) and not degrade,
        f"cargo test -p rurix-renderer-sdk --features sdk-device exit={rt.returncode};"
        f"粒子面单测 ok 在案={seen_tests}(悬空句柄 ST_HANDLE/空指针与资产违例与 key 闭集外 "
        f"ST_INPUT/会话回收连带悬空/tick 精确账目 24×n;degrade={degrade or '无'})",
    )

    # ── ⑧ RED 臂 ──
    set_fact(
        "red_arm_effective",
        red_ok(doc_red),
        f"field-tamper 双面:值篡改 digest 必异 detected={doc_red.get('detected')!r}"
        f"(green={str(doc_red.get('digest_green'))[:23]}… red={str(doc_red.get('digest_red'))[:23]}…)+ "
        f"schema 篡改 typed 检出={doc_red.get('schema_tamper_detected')!r} kind={doc_red.get('schema_tamper_kind')!r}",
    )

    # ── 三态形骸裁决(本门纯 host,degrade 常空;唯一项 = SDK 单测 loader 跳过)──
    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g35.authoring.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    # ── evidence 落盘(门裁决件;jsonschema 自校验硬门)──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_authoring_gate_{ts}.json"
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "本门纯 host(无 GPU 锁;SDK 单测仅 vulkan loader 探测)",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    probe_evidence = [
        str(ev_green.relative_to(ROOT)).replace("\\", "/"),
        str(ev_red.relative_to(ROOT)).replace("\\", "/"),
    ]
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "asset_protocol": {
            "field_closed_set": FIELD_CLOSED_SET,
            "sample_a": doc_green.get("asset_name") or "(缺)",
            "sample_b": doc_green.get("asset2_name") or "(缺)",
            "fail_closed_cases": case_rows,
            "cases_total": 10,
            "cases_ok": cases_ok,
        },
        "curve_eval": {
            "crosscheck_ok": bool(doc_green.get("curve_crosscheck_ok")),
            "samples_a": doc_green.get("curve_samples_a") or [],
            "samples_b": doc_green.get("curve_samples_b") or [],
        },
        "hot_reload": {
            "reload_at": doc_green.get("reload_at", 32),
            "effective_digest_diff": bool(doc_green.get("reload_effective_digest_diff")),
            "next_frame_effective": bool(doc_green.get("reload_next_frame_effective")),
            "old_particles_continuous": bool(doc_green.get("old_particles_continuous")),
            "boundary_survivors_checked": doc_green.get("boundary_survivors_checked", 0),
            "continuity_checked": doc_green.get("continuity_checked", 0),
            "digest_baseline": doc_green.get("digest_baseline") or ("sha256:" + "0" * 64),
            "digest_reload": doc_green.get("digest_a") or ("sha256:" + "0" * 64),
        },
        "pid_continuity": {
            "unique": bool(doc_green.get("pid_unique")),
            "survivor_subset": bool(doc_green.get("pid_survivor_subset")),
            "emit_range_exact": bool(doc_green.get("pid_emit_range_exact")),
            "pids_issued": doc_green.get("pids_issued", 0),
            "rejected_total": doc_green.get("rejected_total", 0),
        },
        "sdk": {
            "impl_crate": "src/rurix-renderer-sdk",
            "frozen_signatures_present": bool(sig_ok),
            "symbols_present": bool(symbols_ok),
            "symbol_check_method": symbol_method,
            "particles_symbols": PARTICLES_SYMBOLS,
            "existing_exports_present": bool(old_present),
            "existing_export_count": 9,
            "cargo_test_exit": rt.returncode,
            "particles_tests_seen": seen_tests or ["(缺)", "(缺)"],
            "stable_snapshot_check_exit": snap.returncode,
            "abi_version_unchanged": "1.0.0",
            "user_face_minor_bump": "deferred_to_closeout",
        },
        "determinism": {
            "double_run_bitexact": bool(doc_green.get("double_run_bitexact")),
            "digest_a": doc_green.get("digest_a") or ("sha256:" + "0" * 64),
            "digest_b": doc_green.get("digest_b") or ("sha256:" + "0" * 64),
        },
        "red_arm": {
            "arm": "field-tamper",
            "detected": bool(doc_red.get("detected")),
            "digest_green": doc_red.get("digest_green") or ("sha256:" + "0" * 64),
            "digest_red": doc_red.get("digest_red") or ("sha256:" + "0" * 64),
            "schema_tamper_detected": bool(doc_red.get("schema_tamper_detected")),
            "schema_tamper_kind": doc_red.get("schema_tamper_kind") or "UnknownField",
        },
        "tri_state": {
            "degrade": degrade,
            "note": (
                "本门纯 host(资产解析/曲线求值/热重载/pid 连续全 host 纯函数;probe 无 device "
                "依赖无 required-features)——host 面永可跑,--gate 不占 GPU 锁;三态形骸保留:唯一"
                "可能降级项 = SDK 单测 vulkan loader 缺席跳过面(仅 loader 探测零 GPU 工作),届时 "
                "SKIP 退 0 不冒充 PASS、RURIX_REQUIRE_REAL=1 翻硬 FAIL(九门三态同律)。"
            ),
        },
        "probe_evidence": probe_evidence,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-8 数据驱动作者面与 SDK:声明式 emitter 资产(JSON v1 十字段闭集 RFC-0049 §3 冻结,"
            "src/rurix-render/src/particles/emitter_asset.rs 库面最小 JSON 子集解析〔g14_3_lane_body "
            "bin-local 先例同型子集,零外部 crate〕+ fail-closed typed 错误七类闭集)+ 参数化映射"
            "(EmitterAsset::to_desc → particles::core::EmitterDesc 六标量域;emit_count_at 曲线求值 = "
            "const 恒值取整/step 阶梯查表,纯函数)+ 热重载(EmitterRuntime::reload = 纯参数面替换,"
            "池/pid/帧钟连续不重置,下一帧生效;判据 = digest 必异 + 旧粒子冻结运算序单步重放 bitwise "
            "不瞬移)+ SDK C ABI 加性(RFC-0049 §4.11 冻结四签名 rxsdk_particles_* 落实现层 "
            "src/rurix-renderer-sdk mod sdk〔feature sdk-device〕:u64 句柄 BTreeMap 表 + i32 状态码"
            "复用既有闭集 0/2/3 + 单线程 apartment;v1 SDK 粒子臂 = host 臂——每 emitter 独立池 "
            "cap=4096 随 rxsdk_renderer_render_frame 每成功帧 tick 一帧,stats 写 alive_total;"
            "device 车道接线归收口批)。既有导出/句柄语义 0-byte(stable_snapshot --check 机器证明:"
            "用户面 sdk.rx 导出集与 ABI_VERSION_PACKED 1.0.0 本批不动);用户面 rurix_renderer_"
            "particles_* 薄转发 + 生成头再生 + MINOR 1.0.0→1.1.0 + 快照重 bless 登记为待 G35 收口批"
            "(g31.waveC 门族五面 1.0.0/9 导出字面须同批更新方保门可复跑绿,apps/g31-renderer-sdk/"
            "API_VERSIONING.md §6 如实登记禁伪造)。本门无 kernels_spv_valid/frame_ms 面(纯 host "
            "无 kernel 无 device 帧时);evidence 恒落 evidence/(G35 族同律,verdict 如实)。"
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
    io.open(gate_path, "w", encoding="utf-8", newline="\n").write(
        json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n"
    )
    note(f"evidence: {gate_path.relative_to(ROOT)}(probe 件 2 份留 .tmp 工作区)")
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿穷举 + schema 互核;零构建零 GPU 零依赖)
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
    # 红绿臂①:十例闭集判 + typed token 解析。
    good_cases = [{"case": f"c{i}", "kind_expected": "Domain", "kind_seen": "Domain",
                   "exit_code": 3, "ok": True} for i in range(10)]
    expect(fail_closed_ok(good_cases), "GREEN:十例全过正例")
    expect(not fail_closed_ok(good_cases[:9]), "RED:九例(缺例)必红")
    bad = [dict(c) for c in good_cases]
    bad[3]["exit_code"] = 1
    bad[3]["ok"] = False
    expect(not fail_closed_ok(bad), "RED:退出码非 3 必红")
    bad2 = [dict(c) for c in good_cases]
    bad2[5]["kind_seen"] = "Json"
    expect(not fail_closed_ok(bad2), "RED:kind 不匹配必红(ok 旗标与内容互核)")
    bad3 = [dict(c) for c in good_cases]
    bad3[0]["ok"] = "true"
    expect(not fail_closed_ok(bad3), "RED:字符串冒充 bool 必红")
    expect(asset_err_kind("x AUTHORING_ASSET_ERR kind=MissingField detail=y") == "MissingField",
           "GREEN:typed token 解析")
    expect(asset_err_kind("no token") is None, "RED:缺 token 拒判")
    expect(len(illegal_cases()) == 10, "十例闭集基数 = 10")
    expect(len({c[0] for c in illegal_cases()}) == 10, "十例 case 名互异")
    kinds = {c[2] for c in illegal_cases()}
    expect(kinds == {"MissingField", "UnknownField", "Type", "EnumOutOfSet", "Domain"},
           "十例覆盖五类 typed 错(缺字段/多字段/类型/枚举/域)")
    # 红绿臂②:曲线求值判。
    good_curve = {"curve_crosscheck_ok": True, "curve_samples_a": [24], "curve_samples_b": [0, 16]}
    expect(curve_ok(good_curve), "GREEN:曲线互核正例")
    expect(not curve_ok({**good_curve, "curve_crosscheck_ok": False}), "RED:互核破必红")
    expect(not curve_ok({**good_curve, "curve_samples_a": []}), "RED:样本空必红")
    expect(not curve_ok({**good_curve, "curve_crosscheck_ok": "true"}), "RED:字符串冒充 bool 必红")
    # 红绿臂③:热重载判。
    good_hr = {"reload_effective_digest_diff": True, "reload_next_frame_effective": True,
               "old_particles_continuous": True, "boundary_survivors_checked": 768,
               "digest_baseline": d0, "digest_a": d1}
    expect(hot_reload_ok(good_hr), "GREEN:热重载正例")
    expect(not hot_reload_ok({**good_hr, "reload_effective_digest_diff": False}), "RED:重载未生效必红")
    expect(not hot_reload_ok({**good_hr, "digest_a": d0}), "RED:旗标真但 digest 同基线(自相矛盾)必红")
    expect(not hot_reload_ok({**good_hr, "old_particles_continuous": False}), "RED:旧粒子瞬移必红")
    expect(not hot_reload_ok({**good_hr, "reload_next_frame_effective": False}), "RED:下一帧未生效必红")
    expect(not hot_reload_ok({**good_hr, "boundary_survivors_checked": 0}), "RED:边界零样本(空转)必红")
    expect(not hot_reload_ok({**good_hr, "digest_baseline": "xx"}), "RED:digest 形态破必红")
    # 红绿臂④:pid 连续判。
    good_pid = {"pid_unique": True, "pid_survivor_subset": True,
                "pid_emit_range_exact": True, "pids_issued": 1344}
    expect(pid_ok(good_pid), "GREEN:pid 正例")
    expect(not pid_ok({**good_pid, "pid_unique": False}), "RED:pid 重复必红")
    expect(not pid_ok({**good_pid, "pid_survivor_subset": False}), "RED:幸存非子集必红")
    expect(not pid_ok({**good_pid, "pid_emit_range_exact": False}), "RED:发射区间不精确必红")
    expect(not pid_ok({**good_pid, "pids_issued": 0}), "RED:零发行(空转)必红")
    # 红绿臂⑤:SDK 面判 + 冻结签名源级机核。
    expect(sdk_surface_ok(True, True, 0), "GREEN:SDK 面正例")
    expect(not sdk_surface_ok(False, True, 0), "RED:冻结签名缺必红")
    expect(not sdk_surface_ok(True, False, 0), "RED:符号面缺必红")
    expect(not sdk_surface_ok(True, True, 1), "RED:stable 快照漂移必红(加性 0-byte 破)")
    sig_sample = (
        '#[unsafe(no_mangle)]\n    pub extern "C" fn rxsdk_particles_emitter_create(\n'
        "        sys: u64,\n        desc_json: *const u8,\n        len: usize,\n"
        "        out: *mut u64,\n    ) -> i32 {\n"
        '    pub extern "C" fn rxsdk_particles_emitter_set_param(\n'
        "        h: u64,\n        key: *const u8,\n        klen: usize,\n        value: f32,\n    ) -> i32 {\n"
        '    pub extern "C" fn rxsdk_particles_emitter_destroy(h: u64) -> i32 {\n'
        '    pub extern "C" fn rxsdk_particles_stats(sys: u64, out: *mut u64) -> i32 {\n'
    )
    ok_all, missing = frozen_sigs_in_source(sig_sample)
    expect(ok_all and not missing, "GREEN:冻结四签名样本全中")
    ok_m, missing_m = frozen_sigs_in_source(sig_sample.replace("value: f32", "value: f64"))
    expect(not ok_m and missing_m == ["rxsdk_particles_emitter_set_param"],
           "RED:签名类型漂移(f32→f64)必检出")
    expect(frozen_sigs_in_source("")[0] is False, "RED:空源必红")
    # 红绿臂⑥:SDK 单测判。
    two_ok = ("test sdk::tests::particles_handle_and_input_fail_closed ... ok\n"
              "test sdk::tests::particles_lifecycle_tick_and_stats ... ok\n")
    expect(sdk_tests_ok(0, two_ok), "GREEN:SDK 单测正例")
    expect(not sdk_tests_ok(1, two_ok), "RED:cargo test 退非 0 必红")
    expect(not sdk_tests_ok(0, two_ok.replace(" ok\n", " FAILED\n", 1)), "RED:粒子单测缺 ok 必红")
    expect(not sdk_tests_ok(0, ""), "RED:输出空必红")
    # 红绿臂⑦:双跑位级判。
    expect(determinism_ok({"double_run_bitexact": True, "digest_a": d0, "digest_b": d0}),
           "GREEN:双跑位级正例")
    expect(not determinism_ok({"double_run_bitexact": True, "digest_a": d0, "digest_b": d1}),
           "RED:旗标真但 digest 异(自相矛盾)必红")
    expect(not determinism_ok({"double_run_bitexact": False, "digest_a": d0, "digest_b": d0}),
           "RED:旗标假必红")
    expect(not determinism_ok({"double_run_bitexact": True, "digest_a": "z", "digest_b": "z"}),
           "RED:digest 形态破必红")
    # 红绿臂⑧:RED 臂判。
    good_red = {"arm": "field-tamper", "detected": True, "digest_green": d0, "digest_red": d1,
                "schema_tamper_detected": True, "schema_tamper_kind": "UnknownField"}
    expect(red_ok(good_red), "GREEN:RED 臂正例")
    expect(not red_ok({**good_red, "detected": False}), "RED:值篡改漏检必红")
    expect(not red_ok({**good_red, "digest_red": d0}), "RED:digest 未变(镂空 digest)必红")
    expect(not red_ok({**good_red, "schema_tamper_detected": False}), "RED:schema 篡改漏检必红")
    expect(not red_ok({**good_red, "schema_tamper_kind": "Json"}), "RED:篡改 kind 非 UnknownField 必红")
    expect(not red_ok({**good_red, "arm": "seed-change"}), "RED:臂名不符必红")
    # 红绿臂⑨:三态形骸。
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    # schema 互核:在树 + Draft7 合法 + facts enum == FACT_IDS + const 互核。
    expect(GATE_SCHEMA_PATH.is_file(), "gate schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["facts"]["minItems"] == 8
               and gs["properties"]["facts"]["maxItems"] == 8, "facts 基数 = 8 互核")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate schema 门键 const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        fe = gs["properties"]["asset_protocol"]["properties"]["field_closed_set"]
        expect(sorted(fe["items"]["enum"]) == sorted(FIELD_CLOSED_SET),
               "十字段闭集 enum 互核(RFC-0049 §3)")
        se = gs["properties"]["sdk"]["properties"]["particles_symbols"]["items"]["enum"]
        expect(sorted(se) == sorted(PARTICLES_SYMBOLS), "SDK 四符号闭集互核(RFC-0049 §4.11)")
        expect(gs["properties"]["sdk"]["properties"]["existing_export_count"]["const"] == 9,
               "既有导出数 const=9 互核(加性 0-byte 面)")
        expect(gs["properties"]["sdk"]["properties"]["user_face_minor_bump"]["const"]
               == "deferred_to_closeout", "MINOR 待收口批登记 const 互核")
        expect(gs["properties"]["red_arm"]["properties"]["arm"]["const"] == "field-tamper",
               "红臂名 const 互核")
        import jsonschema as _js
        _js.Draft7Validator.check_schema(gs)
        print("  ok   — gate schema Draft7 合法(check_schema 绿)")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=8;9 红绿臂组 + 十例闭集 + 冻结签名机核 + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=64)
    ap.add_argument("--cap", type=int, default=2048)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 64:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 64(冻结默认窗:样例 A life ∈ "
                  f"[0.6,1.2)s/dt=1/60 下寿命死亡覆盖 + reload-at 32 需 ≥64)", file=sys.stderr)
            return 1
        if args.cap <= 0 or args.cap % 256 != 0:
            print(f"[{TAG}] FAIL: --cap {args.cap} 须为 SEG=256 正整倍数", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.cap, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
