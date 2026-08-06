#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.2 M32 capability_profile 硬门冒烟(g8.p0.m32.capability_profile;
RFC-0019 §4.5;spec/shader_stages.md RXS-0311 + spec/rendering_platform.md
RXS-0312~0313)。

host/compile 纯 host 门(host 恒跑,check_* 风格;device 段 not_applicable)。
验收判据(G8_ACCEPTANCE_MAP §2 M32 行逐字):

  支持 profile 的 fixture 类型检查 0 诊断;同一 fixture 移除一项必需
  capability 后以 RFC-0019 冻结的 symbolic diagnostic key 确定性拒录;声明
  fallback 的 fixture 在低 profile 只生成允许的 specialization,禁止能力对应
  指令/扩展计数为 0。三腿(accept、reject、fallback)缺一即 FAIL。

RFC-0019 冻结 symbolic key:capability.missing_required(消息须列缺失 ID +
首个引入它的可达 callee)/ capability.forbidden_used /
capability.fallback_incompatible(消息给出不兼容字段)/
capability.runtime_snapshot_mismatch(库层 typed Err 不占 RX 码,M32 只落
verify_profile_snapshot host 原语)。第五 key(RXS-0311 加性冻结):
capability.unknown_id。

checks.* 14 项布尔(缺一 FAIL;三腿各自独立字段):
  accept_leg_zero_diagnostics / reject_leg_exact_symbolic_key /
  reject_leg_deterministic / forbidden_used_red / fallback_incompatible_red /
  unknown_id_red / fallback_leg_selects_fallback_only /
  fallback_leg_zero_forbidden_instructions / fallback_leg_no_diagnostic_keys /
  profile_digest_deterministic / no_profile_zero_drift /
  runtime_snapshot_verify_fail_closed / accept_corpus_green /
  reject_corpus_red_with_codes。

「移除一项必需 capability 后确定性拒录」落地:同一 fixture
(requires_supported.rx)对 high/low 两 profile 跑——high 绿、low(缺
rt.pipeline 且无 fallback)红且 key/缺失 ID/引入 callee 精确匹配。
退出码判定(非 grep stdout)。任一判据红 → 逐项打印定位后 exit 1
(evidence 仍如实落盘,红不充绿)。cargo test capability 15 单测全绿为前置。

用法:
  py -3 ci/g8_capability_profile_smoke.py --gate g8.p0.m32.capability_profile
  py -3 ci/g8_capability_profile_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import platform
import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
ACCEPT_DIR = ROOT / "conformance" / "capability" / "accept"
REJECT_DIR = ROOT / "conformance" / "capability" / "reject"
PROFILES_DIR = ROOT / "conformance" / "capability" / "profiles"
EXPECT_ERROR_RE = re.compile(r"//@\s*expect-error:\s*(RX\d{4})")
PROFILE_RE = re.compile(r"//@\s*profile:\s*([^\n]+)")

GATE_KEY = "g8.p0.m32.capability_profile"
NUMERIC_STEP = 99

# M31 基线常量(RXS-0304 空编码;与 ci/g8_reflection_hash_smoke.py 腿⑤同一字面)。
PROFILE_NONE_DIGEST = "2997fd21a324a39e63cd1da6970db88c511e8d025d24fbce0bbb94c5ea8c28b6"
EMPTY_DOMAIN_DIGEST = "160d241dc1681a927e8edbdd07a15e508f9f5aeb68da8bc92274332cb8541f31"

HIGH = PROFILES_DIR / "high.json"
LOW = PROFILES_DIR / "low.json"
LOW_FB = PROFILES_DIR / "low_with_fallback.json"

REQUIRES_RX = ACCEPT_DIR / "requires_supported.rx"
IMPLICIT_RX = ACCEPT_DIR / "implicit_propagation.rx"
FALLBACK_RX = ACCEPT_DIR / "fallback_low_profile.rx"

# 五 symbolic key 字面(RFC-0019 四键 + RXS-0311 加性第五键)。
FIVE_KEYS = [
    "capability.missing_required",
    "capability.forbidden_used",
    "capability.fallback_incompatible",
    "capability.runtime_snapshot_mismatch",
    "capability.unknown_id",
]
EXPECTED_KEY_OF_CODE = {
    "RX3020": "capability.missing_required",
    "RX3021": "capability.forbidden_used",
    "RX3022": "capability.fallback_incompatible",
    "RX3023": "capability.unknown_id",
}
# SPIR-V RayQuery 指令 opcode 区间(OpTypeRayQueryKHR 4472 起族;SPV_KHR_ray_query)。
RAYQUERY_OPCODE_LO, RAYQUERY_OPCODE_HI = 4472, 4489

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def build_rurixc() -> Path:
    """构建 default + vulkan-backend feature 的 rurixc(vulkan 腿共用同一产物;
    feature 为纯加性,其余 emit 路径行为与 default 逐字节一致)。"""
    print("[g8_m32] cargo build -p rurixc --features vulkan-backend")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--quiet"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(f"[g8_m32] FAIL cargo build:\n{r.stdout}\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        print(f"[g8_m32] FAIL rurixc 产物缺失: {exe}", file=sys.stderr)
        sys.exit(1)
    return exe


def run_rx(exe: Path, rx_path: Path, emit: str | None,
           profile: Path | None = None, extra: list[str] | None = None,
           out_path: Path | None = None) -> tuple[int, str, str]:
    """rurixc <rx> [--emit=<emit>] [--profile <p>] [extra] [-o <out>];
    返回 (returncode, stdout, stderr)。"""
    cmd = [str(exe), str(rx_path)]
    if emit is not None:
        cmd.append(f"--emit={emit}")
    if profile is not None:
        cmd += ["--profile", str(profile)]
    if extra:
        cmd += extra
    if out_path is not None:
        cmd += ["-o", str(out_path)]
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


def cargo_capability_tests() -> tuple[bool, str]:
    """cargo test -p rurixc --lib capability(15 单测全绿为前置;非 quiet 模式,
    输出含逐测试名供 verify_profile_snapshot 负样本锚定)。"""
    r = subprocess.run(
        ["cargo", "test", "-p", "rurixc", "--lib", "capability"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    blob = r.stdout + r.stderr
    if r.returncode != 0:
        check(False, f"cargo test capability 单测失败:\n{blob}")
        return False, blob
    if "15 passed" not in blob and "test result: ok" not in blob:
        check(False, f"cargo test capability 单测结果异常:\n{blob}")
        return False, blob
    return True, blob


def extract_json(stdout: str) -> dict | None:
    try:
        return json.loads(stdout)
    except Exception:
        return None


def find_entry(doc: dict, name: str) -> dict | None:
    for e in doc.get("entries", []):
        if e["name"] == name:
            return e
    return None


def declared_profiles(rx: Path) -> list[Path]:
    """语料头 `//@ profile: profiles/a.json, profiles/b.json` 声明的配套 profile
    表(fixture×profile 矩阵事实源)。"""
    m = PROFILE_RE.search(rx.read_text(encoding="utf-8"))
    if m is None:
        return []
    return [
        ROOT / "conformance" / "capability" / p.strip()
        for p in m.group(1).split(",")
        if p.strip()
    ]


def spv_rayquery_counts(spv: Path) -> tuple[int, int]:
    """`.spv` 字节扫描:(RayQuery 指令数〔u32 字低 16 位 ∈ 4472..=4489〕,
    SPV_KHR_ray_query 扩展声明数〔OpExtension 字面量子串〕)。"""
    data = spv.read_bytes()
    words = struct.unpack(f"<{len(data) // 4}I", data[: len(data) // 4 * 4])
    ops = sum(1 for w in words if RAYQUERY_OPCODE_LO <= (w & 0xFFFF) <= RAYQUERY_OPCODE_HI)
    exts = data.count(b"SPV_KHR_ray_query")
    return ops, exts


# ═══════════════════════ accept 腿 ═══════════════════════


def leg_accept_zero_diagnostics(exe: Path) -> None:
    """accept_leg_zero_diagnostics:三件 accept fixture × high profile
    (--emit=check)全部 0 诊断退出 0;fallback fixture 在低 profile+fallback
    映射下亦 0 诊断。"""
    for rx in (REQUIRES_RX, IMPLICIT_RX, FALLBACK_RX):
        code, _, stderr = run_rx(exe, rx, "check", HIGH)
        check(code == 0, f"accept_leg: {rx.name} × high 应绿(0)却退出 {code}\n{stderr}")
        check(stderr.strip() == "", f"accept_leg: {rx.name} × high 应 0 诊断,stderr 有内容:\n{stderr}")
    code, _, stderr = run_rx(exe, FALLBACK_RX, "check", LOW_FB)
    check(code == 0, f"accept_leg: fallback fixture × low_with_fallback 应绿(0)却退出 {code}\n{stderr}")


# ═══════════════════════ reject 腿 ═══════════════════════


def leg_reject_exact_symbolic_key(exe: Path) -> None:
    """reject_leg_exact_symbolic_key:同一 fixture 移除一项必需 capability 后
    确定性拒录——requires_supported.rx × low(缺 rt.pipeline)stderr 精确含
    `capability.missing_required` 字面 + 缺失 ID + 首个引入 callee(entry 自身);
    implicit_propagation.rx × low 同构(引入 callee = device fn record_weight)。"""
    code, _, stderr = run_rx(exe, REQUIRES_RX, "check", LOW)
    check(code != 0, "reject_key: requires_supported × low 应红却退出 0")
    check(
        "capability.missing_required" in stderr,
        f"reject_key: stderr 须精确含 capability.missing_required 字面:\n{stderr}",
    )
    check("rt.pipeline" in stderr, f"reject_key: stderr 须含缺失 ID rt.pipeline:\n{stderr}")
    check("kmain" in stderr, f"reject_key: stderr 须含首个引入 callee `kmain`(entry 自身):\n{stderr}")
    check("RX3020" in stderr, f"reject_key: 应发 RX3020:\n{stderr}")

    code, _, stderr = run_rx(exe, IMPLICIT_RX, "check", LOW)
    check(code != 0, "reject_key: implicit_propagation × low 应红却退出 0")
    check(
        "capability.missing_required" in stderr,
        f"reject_key: implicit stderr 须精确含 capability.missing_required 字面:\n{stderr}",
    )
    check(
        "rt.sbt_user_data" in stderr,
        f"reject_key: implicit stderr 须含缺失 ID rt.sbt_user_data:\n{stderr}",
    )
    check(
        "record_weight" in stderr,
        f"reject_key: implicit stderr 须含首个引入 callee `record_weight`(上浮自 device callee):\n{stderr}",
    )


def leg_reject_deterministic(exe: Path) -> None:
    """reject_leg_deterministic:同一 reject 场景双跑同码同消息(逐字节)。"""
    c1, o1, e1 = run_rx(exe, REQUIRES_RX, "check", LOW)
    c2, o2, e2 = run_rx(exe, REQUIRES_RX, "check", LOW)
    check(c1 == c2 and c1 != 0, f"reject_deterministic: 双跑退出码不一致({c1} vs {c2})")
    check(
        (o1, e1) == (o2, e2),
        "reject_deterministic: 双跑诊断输出不逐字节相等(非确定性)",
    )


def leg_forbidden_used_red(exe: Path) -> None:
    """forbidden_used_red:有效集 ∩ forbidden ≠ ∅ → RX3021 +
    `capability.forbidden_used` 字面 + 违禁 ID。"""
    rx = REJECT_DIR / "forbidden_used.rx"
    code, _, stderr = run_rx(exe, rx, "check", LOW)
    check(code != 0, "forbidden_used: 应红却退出 0")
    check("RX3021" in stderr, f"forbidden_used: 应发 RX3021:\n{stderr}")
    check(
        "capability.forbidden_used" in stderr,
        f"forbidden_used: stderr 须含 capability.forbidden_used 字面:\n{stderr}",
    )
    check("rt.ray_query" in stderr, f"forbidden_used: 须含违禁 ID rt.ray_query:\n{stderr}")


def leg_fallback_incompatible_red(exe: Path) -> None:
    """fallback_incompatible_red:有映射但接口契约不兼容 → RX3022 +
    `capability.fallback_incompatible` 字面 + 不兼容字段。"""
    rx = REJECT_DIR / "fallback_incompatible.rx"
    code, _, stderr = run_rx(exe, rx, "check", LOW_FB)
    check(code != 0, "fallback_incompatible: 应红却退出 0")
    check("RX3022" in stderr, f"fallback_incompatible: 应发 RX3022:\n{stderr}")
    check(
        "capability.fallback_incompatible" in stderr,
        f"fallback_incompatible: stderr 须含 capability.fallback_incompatible 字面:\n{stderr}",
    )
    check(
        "push_constants" in stderr,
        f"fallback_incompatible: 须给出不兼容字段 push_constants:\n{stderr}",
    )


def leg_unknown_id_red(exe: Path) -> None:
    """unknown_id_red:闭集外 capability ID → RX3023 + `capability.unknown_id`
    字面 + 违例 ID。"""
    rx = REJECT_DIR / "unknown_capability_id.rx"
    code, _, stderr = run_rx(exe, rx, "check", HIGH)
    check(code != 0, "unknown_id: 应红却退出 0")
    check("RX3023" in stderr, f"unknown_id: 应发 RX3023:\n{stderr}")
    check(
        "capability.unknown_id" in stderr,
        f"unknown_id: stderr 须含 capability.unknown_id 字面:\n{stderr}",
    )
    check("rt.magic_boost" in stderr, f"unknown_id: 须含违例 ID rt.magic_boost:\n{stderr}")


# ═══════════════════════ fallback 腿 ═══════════════════════


def leg_fallback_selects_fallback_only(exe: Path) -> None:
    """fallback_leg_selects_fallback_only:--emit=capabilities 报告——低
    profile 下逻辑 entry → fallback,主 variant 不在发射集(status ≠ emitted);
    高 profile 对照:主 variant emitted。"""
    code, stdout, stderr = run_rx(exe, FALLBACK_RX, "capabilities", LOW_FB)
    check(code == 0, f"fallback_select: capabilities 报告应绿却退出 {code}\n{stderr}")
    doc = extract_json(stdout)
    if not doc:
        check(False, "fallback_select: manifest JSON 解析失败")
        return
    check(
        doc.get("schema") == "rurix.capability-selection.v1",
        "fallback_select: manifest schema 常量不符",
    )
    kmain = find_entry(doc, "kmain")
    fb = find_entry(doc, "kmain_fallback")
    if not kmain or not fb:
        check(False, "fallback_select: kmain/kmain_fallback 记录缺失")
        return
    check(
        kmain.get("status") == "fallback" and kmain.get("selected_entry") == "kmain_fallback",
        f"fallback_select: 逻辑 entry kmain 须 → fallback 实体(got {kmain})",
    )
    check(
        kmain.get("missing") == ["rt.ray_query"],
        f"fallback_select: kmain missing 须为 [rt.ray_query](got {kmain.get('missing')})",
    )
    check(fb.get("status") == "emitted", "fallback_select: fallback entry 自身须 emitted")
    # 主 variant 不在发射集:无任何名为 kmain 的 emitted 记录。
    emitted_names = [e["name"] for e in doc.get("entries", []) if e.get("status") == "emitted"]
    check("kmain" not in emitted_names, f"fallback_select: 主 variant kmain 不得在发射集(got {emitted_names})")
    # 高 profile 对照:主 variant emitted、无 fallback 记录。
    code_h, stdout_h, _ = run_rx(exe, FALLBACK_RX, "capabilities", HIGH)
    doc_h = extract_json(stdout_h) if code_h == 0 else None
    if doc_h and (kh := find_entry(doc_h, "kmain")):
        check(
            kh.get("status") == "emitted" and kh.get("selected_entry") == "kmain",
            "fallback_select: 高 profile 下主 variant 须 emitted 且选自身",
        )
    else:
        check(False, "fallback_select: 高 profile manifest 异常")


def leg_fallback_zero_forbidden_instructions(exe: Path) -> None:
    """fallback_leg_zero_forbidden_instructions:低 profile 选中产物 SPIR-V 的
    OpRayQuery* 指令数与 SPV_KHR_ray_query 扩展声明 == 0;高 profile 正对照
    (同 fixture 产物该两计数 > 0,证明扫描非空过)。"""
    with tempfile.TemporaryDirectory() as d:
        spv_low = Path(d) / "low.spv"
        code, _, stderr = run_rx(
            exe, FALLBACK_RX, None, LOW_FB, extra=["--target", "vulkan"], out_path=spv_low
        )
        check(code == 0, f"fallback_spv: 低 profile --target vulkan 应绿却退出 {code}\n{stderr}")
        if code == 0 and spv_low.is_file():
            ops, exts = spv_rayquery_counts(spv_low)
            check(ops == 0, f"fallback_spv: 选中产物 OpRayQuery* 指令数须为 0(实测 {ops})")
            check(
                exts == 0,
                f"fallback_spv: 选中产物 SPV_KHR_ray_query 扩展声明须为 0(实测 {exts})",
            )
        else:
            check(False, "fallback_spv: 低 profile 产物缺失")
        spv_high = Path(d) / "high.spv"
        code_h, _, stderr_h = run_rx(
            exe, FALLBACK_RX, None, HIGH, extra=["--target", "vulkan"], out_path=spv_high
        )
        check(code_h == 0, f"fallback_spv: 高 profile --target vulkan 应绿却退出 {code_h}\n{stderr_h}")
        if code_h == 0 and spv_high.is_file():
            ops_h, exts_h = spv_rayquery_counts(spv_high)
            check(
                ops_h > 0 and exts_h > 0,
                f"fallback_spv: 高 profile 正对照须含 RayQuery 指令/扩展(ops={ops_h}, exts={exts_h};扫描非空过)",
            )
        else:
            check(False, "fallback_spv: 高 profile 产物缺失")


def leg_fallback_no_diagnostic_keys(exe: Path) -> None:
    """fallback_leg_no_diagnostic_keys:低 profile+fallback 下五 symbolic key
    零出现(missing/forbidden/incompatible/runtime_snapshot/unknown 全不触发)。"""
    code, stdout, stderr = run_rx(exe, FALLBACK_RX, "check", LOW_FB)
    check(code == 0, f"fallback_no_keys: 应绿却退出 {code}\n{stderr}")
    blob = stdout + stderr
    for key in FIVE_KEYS:
        check(key not in blob, f"fallback_no_keys: 五 key 之一 `{key}` 出现于输出:\n{blob}")


# ═══════════════════════ digest / 0 漂移腿 ═══════════════════════


def leg_profile_digest_deterministic(exe: Path) -> None:
    """profile_digest_deterministic:同 profile 双跑 digest 相等且真值化进
    reflection(非空编码常量);reflection JSON 双次逐字节相等。"""
    j1 = run_rx(exe, FALLBACK_RX, "reflection", HIGH)[1]
    j2 = run_rx(exe, FALLBACK_RX, "reflection", HIGH)[1]
    check(j1 == j2 and len(j1) > 0, "digest_deterministic: 双次 reflection 不逐字节相等")
    d1, d2 = extract_json(j1), extract_json(j2)
    if not d1 or not d2:
        check(False, "digest_deterministic: reflection JSON 解析失败")
        return
    k1, k2 = find_entry(d1, "kmain"), find_entry(d2, "kmain")
    if not k1 or not k2:
        check(False, "digest_deterministic: kmain 记录缺失")
        return
    check(
        k1["selected_profile_digest"] == k2["selected_profile_digest"],
        "digest_deterministic: 同 profile 双跑 digest 不等",
    )
    check(
        k1["selected_profile_digest"] != PROFILE_NONE_DIGEST,
        "digest_deterministic: --profile 给定时 digest 未真值化(仍空编码常量)",
    )
    check(
        k1["required_capabilities"] == ["rt.ray_query"],
        f"digest_deterministic: required_capabilities 未真值化(got {k1['required_capabilities']})",
    )
    # 无 profile 对照:同一 fixture digest 恒空编码常量。
    j_none = run_rx(exe, FALLBACK_RX, "reflection")[1]
    d_none = extract_json(j_none)
    if d_none and (kn := find_entry(d_none, "kmain")):
        check(
            kn["selected_profile_digest"] == PROFILE_NONE_DIGEST,
            "digest_deterministic: 无 --profile 时 digest 须恒空编码常量",
        )
    else:
        check(False, "digest_deterministic: 无 profile reflection 异常")


def leg_no_profile_zero_drift(exe: Path) -> None:
    """no_profile_zero_drift:无 --profile 时 reflection 与 M31/M29 基线逐字节
    一致——requirement-free entry 的 required_capabilities 恒空表、profile
    digest 恒 M31 常量、其余 digest/key 字段与基线单元逐字段一致(M29 腿⑤口径)。"""
    with tempfile.TemporaryDirectory() as d:
        td = Path(d)
        # 基线单元:单 requirement-free kernel(M31 时代形态)。
        base_rx = td / "baseline.rx"
        base_rx.write_text(
            "kernel fn plain(out: ViewMut<global, f32>) { out[0] = 1.0; }\nfn main() {}\n",
            encoding="utf-8",
        )
        _, out_base, _ = run_rx(exe, base_rx, "reflection")
        # 混合单元:同一 plain entry + 携 #[requires] 的无关 entry 共存。
        mix_rx = td / "mixed.rx"
        mix_rx.write_text(
            "kernel fn plain(out: ViewMut<global, f32>) { out[0] = 1.0; }\n"
            "#[requires(\"rt.pipeline\")]\nkernel fn tagged() {}\nfn main() {}\n",
            encoding="utf-8",
        )
        _, out_mix, _ = run_rx(exe, mix_rx, "reflection")
    doc_base, doc_mix = extract_json(out_base), extract_json(out_mix)
    if not doc_base or not doc_mix:
        check(False, "zero_drift: reflection JSON 解析失败")
        return
    plain_b, plain_m = find_entry(doc_base, "plain"), find_entry(doc_mix, "plain")
    tagged = find_entry(doc_mix, "tagged")
    if not plain_b or not plain_m or not tagged:
        check(False, "zero_drift: plain/tagged entry 缺失")
        return
    check(
        plain_m["required_capabilities"] == [],
        f"zero_drift: requirement-free entry required_capabilities 须恒空表(got {plain_m['required_capabilities']})",
    )
    check(
        plain_m["selected_profile_digest"] == PROFILE_NONE_DIGEST,
        "zero_drift: 无 --profile 时 selected_profile_digest 须恒 M31 常量",
    )
    check(
        tagged["required_capabilities"] == ["rt.pipeline"],
        "zero_drift: 携 #[requires] entry 的 required_capabilities 未真值化",
    )
    # 同一 entry 在基线/混合两单元逐字段一致(M32 特征面 0 扰动)。
    for field in (
        "required_capabilities",
        "selected_profile_digest",
        "permutation_domain_digest",
        "variant_key",
        "interface_hash",
        "source_digest",
        "pipeline_key",
        "canonical_hex",
    ):
        check(
            plain_b[field] == plain_m[field],
            f"zero_drift: plain entry `{field}` 跨单元不一致(0 漂移破)",
        )
    check(
        plain_m["permutation_domain_digest"] == EMPTY_DOMAIN_DIGEST,
        "zero_drift: permutation 空域常量回归(M29 面)",
    )


# ═══════════════════════ runtime snapshot 腿(host 原语) ═══════════════════════


def leg_runtime_snapshot_verify(tests_ok: bool, tests_blob: str) -> None:
    """runtime_snapshot_verify_fail_closed:verify_profile_snapshot 负样本单测
    在场且过(cargo test capability 输出锚定测试名,证明非空跑)。"""
    check(tests_ok, "runtime_snapshot: cargo test capability 前置失败")
    check(
        "verify_profile_snapshot_fail_closed" in tests_blob,
        "runtime_snapshot: verify_profile_snapshot_fail_closed 单测未在 cargo 输出锚定(空跑嫌疑)",
    )


# ═══════════════════════ 语料批跑 ═══════════════════════


def accept_corpus(exe: Path, tests_ok: bool) -> int:
    """conformance/capability/accept/*.rx 逐件按语料头声明 profile 矩阵跑
    --emit=check 与 --emit=capabilities 双通道退出 0、JSON 可解析。"""
    cases = sorted(ACCEPT_DIR.glob("*.rx"))
    check(len(cases) == 3, f"accept 语料应为 3 件,实测 {len(cases)}")
    for rx in cases:
        profiles = declared_profiles(rx)
        check(len(profiles) >= 1, f"accept: {rx.name} 缺 `//@ profile:` 配套声明")
        for prof in profiles:
            code, _, stderr = run_rx(exe, rx, "check", prof)
            check(
                code == 0,
                f"accept: {rx.name} × {prof.name} --emit=check 应绿(0)却退出 {code}\n{stderr}",
            )
            code2, stdout2, stderr2 = run_rx(exe, rx, "capabilities", prof)
            check(
                code2 == 0,
                f"accept: {rx.name} × {prof.name} --emit=capabilities 应绿(0)却退出 {code2}\n{stderr2}",
            )
            if code2 == 0:
                check(
                    extract_json(stdout2) is not None,
                    f"accept: {rx.name} × {prof.name} manifest JSON 解析失败",
                )
    check(tests_ok, "accept: cargo test capability 单测前置失败")
    return len(cases)


def reject_corpus(exe: Path) -> int:
    """conformance/capability/reject/*.rx 逐件按声明 profile 必红且落头部
    声明的错误码 + 对应 symbolic key 字面。"""
    cases = sorted(REJECT_DIR.glob("*.rx"))
    check(len(cases) == 4, f"reject 语料应为 4 件,实测 {len(cases)}")
    for rx in cases:
        m = EXPECT_ERROR_RE.search(rx.read_text(encoding="utf-8"))
        if m is None:
            check(False, f"reject: {rx.name} 缺 `//@ expect-error: RX####` 头声明")
            continue
        want = m.group(1)
        profiles = declared_profiles(rx)
        if not profiles:
            check(False, f"reject: {rx.name} 缺 `//@ profile:` 配套声明")
            continue
        for prof in profiles:
            code, stdout, stderr = run_rx(exe, rx, "check", prof)
            check(code != 0, f"reject: {rx.name} × {prof.name} 应红({want})却退出 0")
            combined = stdout + stderr
            check(
                want in combined,
                f"reject: {rx.name} × {prof.name} 应发 {want},未见于输出:\n{combined}",
            )
            key = EXPECTED_KEY_OF_CODE.get(want)
            if key:
                check(
                    key in combined,
                    f"reject: {rx.name} × {prof.name} 应含 symbolic key `{key}` 字面:\n{combined}",
                )
    return len(cases)


# ═══════════════════════ evidence 落盘 ═══════════════════════


def write_evidence(results: dict, host_ok: bool) -> None:
    EVIDENCE_DIR.mkdir(exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev = {
        "schema_version": 1,
        "subject": "g8_m32_capability_profile",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M32",
        "wave": "G8.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": "RFC-0019 §4.5;spec/shader_stages.md RXS-0311;spec/rendering_platform.md RXS-0312~0313",
        "host_section_pass": host_ok,
        "device_section_state": "not_applicable",
        "checks": results,
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "notes": (
            "host/compile 纯 host 门;device 段 not_applicable(CI_GATES §6 host-only 行)。"
            "14 项判据经 rurixc --emit=check/--emit=capabilities/--emit=reflection "
            "--profile <profile.json> fixture×profile 矩阵端到端 + --target vulkan "
            "产物字节扫描(OpRayQuery* 指令/SPV_KHR_ray_query 扩展计数)+ cargo test "
            "capability 15 单测前置(verify_profile_snapshot 负样本锚定)。"
            "「移除一项必需 capability 后确定性拒录」= requires_supported.rx × "
            "high(绿)/low(红 RX3020,key+缺失 ID+引入 callee 精确匹配)。"
        ),
    }
    path = EVIDENCE_DIR / f"g8_m32_capability_profile_{ts}.json"
    path.write_text(json.dumps(ev, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[g8_m32] evidence 落盘: {path.relative_to(ROOT)}")


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip()
    except Exception:
        return "unknown"


# ═══════════════════════ selftest(反 YAML-only) ═══════════════════════


def selftest() -> None:
    """反 YAML-only:合成数据喂纯判定层,证明每组断言都能红(不跑 cargo、不写 evidence)。"""
    # check() 能正确记录失败
    check(False, "selftest: 合成失败(证明 check() 能红)")
    if len(FAILURES) != 1:
        print("[g8_m32] selftest FAIL: check() 未正确记录合成失败", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    # extract_json 对非法 JSON 返回 None
    assert extract_json("not json") is None
    # find_entry 对缺失 entry 返回 None
    assert find_entry({"entries": []}, "x") is None
    # 五 key 扫描能对合成 key 出现判红
    blob = "error[RX3020]: capability.missing_required: entry `k` ..."
    for key in FIVE_KEYS:
        check(key not in blob, f"selftest: 合成 key 出现(证明五 key 扫描能红: {key})")
        break
    if len(FAILURES) != 1 or "missing_required" not in FAILURES[0]:
        print("[g8_m32] selftest FAIL: 五 key 合成出现未被判红", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    # RayQuery 指令扫描能对合成 SPIR-V 字节判红
    with tempfile.TemporaryDirectory() as d:
        spv = Path(d) / "synth.spv"
        # OpTypeRayQueryKHR = 4472;OpExtension "SPV_KHR_ray_query" 字面。
        words = [0x07230203, 0x00010000, 0, 0, 0, (2 << 16) | 4472]
        spv.write_bytes(struct.pack(f"<{len(words)}I", *words) + b"SPV_KHR_ray_query\x00")
        ops, exts = spv_rayquery_counts(spv)
        check(ops == 0 and exts == 0, "selftest: 合成 RayQuery 指令/扩展(证明字节扫描能红)")
    if len(FAILURES) != 1:
        print(f"[g8_m32] selftest FAIL: 合成 RayQuery 字节未被判红(ops={ops}, exts={exts})", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    # 发射集断言能对合成 emitted 记录判红
    emitted_names = ["kmain", "kmain_fallback"]
    check("kmain" not in emitted_names, "selftest: 合成主 variant 在发射集(证明发射集断言能红)")
    if len(FAILURES) != 1:
        print("[g8_m32] selftest FAIL: 发射集合成违例未被判红", file=sys.stderr)
        sys.exit(1)
    FAILURES.clear()
    print("[g8_m32] selftest PASS(红绿判别有效;未跑 cargo、未写 evidence)")


# ═══════════════════════ main ═══════════════════════


def main() -> int:
    parser = argparse.ArgumentParser(description="G8.2 M32 capability_profile 硬门冒烟")
    parser.add_argument("--gate", default=GATE_KEY, help="symbolic gate key")
    parser.add_argument("--selftest", action="store_true", help="反 YAML-only 红绿自检")
    args = parser.parse_args()

    if args.selftest:
        selftest()
        return 0

    if args.gate != GATE_KEY:
        check(False, f"--gate `{args.gate}` ≠ canonical key `{GATE_KEY}`")

    exe = build_rurixc()

    # 前置:cargo test capability 单测全绿
    tests_ok, tests_blob = cargo_capability_tests()

    # 三腿判据(accept / reject / fallback 各自独立字段)
    leg_accept_zero_diagnostics(exe)
    leg_reject_exact_symbolic_key(exe)
    leg_reject_deterministic(exe)
    leg_forbidden_used_red(exe)
    leg_fallback_incompatible_red(exe)
    leg_unknown_id_red(exe)
    leg_fallback_selects_fallback_only(exe)
    leg_fallback_zero_forbidden_instructions(exe)
    leg_fallback_no_diagnostic_keys(exe)
    leg_profile_digest_deterministic(exe)
    leg_no_profile_zero_drift(exe)
    leg_runtime_snapshot_verify(tests_ok, tests_blob)

    # 语料批跑
    n_accept = accept_corpus(exe, tests_ok)
    n_reject = reject_corpus(exe)

    # 汇总 checks(14 项,缺一 FAIL;三腿各自独立字段)
    results = {
        "accept_leg_zero_diagnostics": not any("accept_leg" in f for f in FAILURES),
        "reject_leg_exact_symbolic_key": not any("reject_key" in f for f in FAILURES),
        "reject_leg_deterministic": not any("reject_deterministic" in f for f in FAILURES),
        "forbidden_used_red": not any("forbidden_used" in f for f in FAILURES),
        "fallback_incompatible_red": not any("fallback_incompatible" in f for f in FAILURES),
        "unknown_id_red": not any("unknown_id" in f for f in FAILURES),
        "fallback_leg_selects_fallback_only": not any("fallback_select" in f for f in FAILURES),
        "fallback_leg_zero_forbidden_instructions": not any("fallback_spv" in f for f in FAILURES),
        "fallback_leg_no_diagnostic_keys": not any("fallback_no_keys" in f for f in FAILURES),
        "profile_digest_deterministic": not any("digest_deterministic" in f for f in FAILURES),
        "no_profile_zero_drift": not any("zero_drift" in f for f in FAILURES),
        "runtime_snapshot_verify_fail_closed": not any("runtime_snapshot" in f for f in FAILURES),
        "accept_corpus_green": tests_ok and not any("accept:" in f or "accept 语料" in f or "cargo test" in f for f in FAILURES),
        "reject_corpus_red_with_codes": not any("reject:" in f or "reject 语料" in f for f in FAILURES),
    }

    host_ok = tests_ok and len(FAILURES) == 0

    write_evidence(results, host_ok)

    for m in NOTES:
        print(f"[g8_m32] NOTE {m}")
    if FAILURES:
        print(f"[g8_m32] FAIL ({len(FAILURES)}):", file=sys.stderr)
        for f in FAILURES:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(
        f"[g8_m32] PASS (host/compile 纯 host 门;"
        f"{n_accept} accept 语料绿 + {n_reject} reject 语料确定性拒;"
        f"cargo test capability 15 单测全绿;14 checks 全真)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
