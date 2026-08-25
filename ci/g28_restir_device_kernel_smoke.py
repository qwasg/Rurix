#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G28.2 实现波)
"""G28.2 M-a ReSTIR device kernel 兑现门冒烟
(g28.p0.m_a.restir_device_kernel;G28_CONTRACT §4.2 M-a 行判据逐字;
rfcs/0045-lighting-device-realization.md §1 判据事实源;
G28_ACCEPTANCE_MAP §1 M-a 行)。

硬判据:kernels/g28_restir.rx(rurixc --target vulkan 产 SPV + spirv-val
通过)经 vk::run_compute 派发 + device vs host 金标准(gi/restir_reservoir.rs
estimate_ris 直调,本期 vs g27-closed 0-byte 冻结)同输入逐 trial 对拍——
随机带单源纪律(PCG32 u64 状态面留 host,已对齐消费序双带 + offset 三元组表
+ 录制自检锚位级前置,RFC-0045 §1.2)——20000 trial:①前置整数锚(保留样本 y
全等 = 真实承重锚 + 判定带消费计数全等,钉死夹具下恒 16 平凡化事实照登恒跑)
②逐 trial estimate 绝对差 p100 ≤ 标定容差(threshold = measured × 2.0 冻结 k,
标定腿两跑位级一致程序产,禁手写;实测 p100=0 → 零容差零条目 measured 事实;
量化兜底 F4:断言 tol < RED_BIAS 0.05 × 0.5 = 0.025,封死「标定与判定同实现」
循环论证)③无偏 3σ 维持(device 均值 vs exact_direct 解析参考,纵深防御并列)
④device 双跑位级一致(输出缓冲 digest)⑤kernel-bias RED 臂必检出
+ gi/ 两文件(restir_reservoir.rs + multi_light.rs)vs g27-closed 0-byte 机核。

三态:无 Vulkan loader/设备 → harness skipped_dev_env(退 0 非 fake pass);
本脚本默认 RURIX_REQUIRE_REAL=1(setdefault),该态下 SKIP → 硬红如实登记
FAIL,不假绿。

用法:
  py -3 ci/g28_restir_device_kernel_smoke.py --gate g28.p0.m_a.restir_device_kernel
  py -3 ci/g28_restir_device_kernel_smoke.py --verify-latest
  py -3 ci/g28_restir_device_kernel_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g28.p0.m_a.restir_device_kernel"
NUMERIC_STEP = 480
SUBJECT = "g28_m_a_restir_device_kernel"
WAVE = "G28.2"
SCHEMA_PATH = ROOT / "milestones/g28/g28_m_a_restir_device_kernel_evidence_schema.json"
SOURCE_REF = (
    "G28_CONTRACT §4.2 M-a;rfcs/0045-lighting-device-realization.md §1;"
    "G28_ACCEPTANCE_MAP §1 M-a 行"
)
TAG = "g28_m_a"

KERNEL = ROOT / "src/rurix-render/kernels/g28_restir.rx"
WORK_DIR = ROOT / ".tmp/g28_gates"
SPV_PATH = WORK_DIR / "g28_restir.spv"
BUDGET_PATH = ROOT / "milestones/g28/g28_budget.json"
CALIB_EVIDENCE_REL = "evidence/g28_restir_device_calibration.json"
TOL_ENTRY_ID = "g28.restir_device.host_device_estimate_tol"
HARNESS_BIN = "g28_restir_device"
FROZEN_BASE = "g27-closed"
# RFC-0045 §1.8:host 参考臂 + M100 低档生产默认面双冻结。
FROZEN_FILES = [
    "src/rurix-render/src/gi/restir_reservoir.rs",
    "src/rurix-render/src/gi/multi_light.rs",
]
# RFC-0045 §1.5 ④ 量化兜底:RED_BIAS=0.05(g13/g26 同值),标定容差绝对上界 = ×0.5。
RED_BIAS = 0.05
TOL_BOUND = RED_BIAS * 0.5

# facts 闭集(≥8;schema extra_facts minItems 6)。
FACT_IDS = [
    "spv_compile_spirv_val_pass",
    "calibration_two_run_bitexact",
    "calibration_budget_entry_measured",
    "tol_under_red_bias_bound",
    "y_integer_anchor_all_equal",
    "device_host_parity_p100",
    "unbiased_3sigma_maintained",
    "device_double_run_bitexact",
    "red_arm_kernel_bias_detected",
    "host_frozen_0byte",
]


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def build_rurixc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    return exe if exe.is_file() else None


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin {HARNESS_BIN}")
    r = run(["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", HARNESS_BIN])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / (f"{HARNESS_BIN}.exe" if sys.platform == "win32" else HARNESS_BIN)
    return exe if exe.is_file() else None


def compile_spv(rurixc: Path) -> tuple[bool, str]:
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    print(f"[{TAG}] rurixc {KERNEL.name} --target vulkan -o {SPV_PATH.relative_to(ROOT)}")
    r = run([str(rurixc), str(KERNEL), "--target", "vulkan", "-o", str(SPV_PATH)])
    if r.returncode != 0 or not SPV_PATH.is_file():
        return False, f"SPV 编译失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}"
    # spirv-val 独立校验(rurixc 内建校验之外的第二判读面;缺工具即 RED——
    # M-a 行「spirv-val 通过」为硬判据,不 SKIP)。
    val = run(["spirv-val", str(SPV_PATH)])
    if val.returncode != 0:
        return False, f"spirv-val 未过: {(val.stdout + val.stderr)[-300:]}"
    return True, "rurixc --target vulkan 产 SPV + spirv-val 独立校验通过"


def json_line(stdout: str, schema_token: str) -> str | None:
    for line in stdout.splitlines():
        if schema_token in line:
            return line.strip()
    return None


# ---------------------------------------------------------------------------
# g28_budget(标定条目程序产追加;确定性面复跑位级期望,漂移即 RED)
# ---------------------------------------------------------------------------


def load_budget() -> dict | None:
    if not BUDGET_PATH.is_file():
        return None
    return json.loads(BUDGET_PATH.read_text(encoding="utf-8"))


def budget_entry(budget: dict, eid: str) -> dict | None:
    for e in budget.get("entries", []):
        if e.get("id") == eid:
            return e
    return None


def _entry_is_measured(entry: dict) -> bool:
    """budget 条目 measured_local 判读器(estimated 冒充 measured 检出面)。"""
    return entry.get("evidence") == "measured_local" and not entry.get("skip_reason")


def append_budget_entries(new_entries: list[dict]) -> list[str]:
    """字节级纯追加(既有字节 0-byte;g13/g26 append_budget_entries 同模)。"""
    problems: list[str] = []
    if not new_entries:
        return problems
    budget_text = BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    to_add: list[dict] = []
    for entry in new_entries:
        if budget_entry(budget, entry["id"]) is not None:
            problems.append(f"{entry['id']} 已在树(追加面不改写)")
            continue
        to_add.append(entry)
    if problems or not to_add:
        return problems
    nl = "\r\n" if "\r\n" in budget_text else "\n"
    frag = ""
    for entry in to_add:
        body = json.dumps(entry, ensure_ascii=False, indent=2)
        body = body.replace("\n", nl)
        body = "    " + body.replace(nl, nl + "    ")
        frag += "," + nl + body
    anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
    if anchor not in budget_text:
        return ["g28_budget.json 结构锚缺失(拒改写)"]
    head, sep, tail = budget_text.partition(anchor)
    budget_text = head + frag + sep + tail
    json.loads(budget_text)
    BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems


def _tol_under_bound(tol: float) -> bool:
    """F4 量化兜底:标定容差(threshold = measured×2.0)必须 < RED_BIAS×0.5。"""
    return tol < TOL_BOUND


def _calibration_lines_bitexact(lines: list[str]) -> bool:
    return len(lines) == 2 and lines[0] == lines[1]


def _harness_state(line: str) -> str:
    try:
        return json.loads(line).get("state", "")
    except json.JSONDecodeError:
        return ""


# ---------------------------------------------------------------------------
# 标定腿(两跑位级一致 + threshold = measured × 2.0 程序产入 budget;
# 实测 p100=0 → 零容差零条目 measured 事实,RFC-0045 §1.3)
# ---------------------------------------------------------------------------


def run_calibration(harness: Path) -> tuple[dict | None, bool, str]:
    lines: list[str] = []
    for run_idx in (1, 2):
        print(f"[{TAG}] 标定跑 {run_idx}: --calibrate")
        r = run(
            [str(harness), "--calibrate", "--spv", str(SPV_PATH)],
            env=device_env(), timeout=3600,
        )
        line = json_line(r.stdout, "rurix.g28restir.calibration_entry.v1")
        if r.returncode != 0 or line is None:
            skip = json_line(r.stdout, "rurix.g28restir.calibration_skip.v1")
            if skip is not None:
                return None, False, f"标定腿 SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP 充绿): {skip[:200]}"
            return None, False, f"标定腿跑 {run_idx} 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}"
        lines.append(line)
    if not _calibration_lines_bitexact(lines):
        return None, False, "标定腿两跑非位级一致(确定性协议漂移即 RED)"
    doc = json.loads(lines[0])
    return doc, True, f"两跑位级一致;p100={doc['results']['trimmed_mean']:.15e}(20000 trial 全集)"


def register_tol_entry(doc: dict) -> tuple[float | None, bool, str]:
    """标定条目登记:实测 p100=0 → 零容差零条目(budget 零追加,measured 事实
    如实登记);p100>0 → 缺则程序产追加(threshold = measured × 2.0),在档则
    复跑位级期望(measured 漂移即 RED,确定性面)。返回 (tol, ok, detail)。"""
    measured = float(doc["results"]["trimmed_mean"])
    budget = load_budget()
    if budget is None:
        return None, False, "g28_budget.json 缺失"
    existing = budget_entry(budget, TOL_ENTRY_ID)
    if measured == 0.0:
        # 零容差零条目(RFC-0045 §1.3:零容差只能是 measured 事实;禁手写)。
        if existing is not None:
            return None, False, (
                f"实测 p100=0 但 {TOL_ENTRY_ID} 在档(在档值与零容差态矛盾,确定性面漂移即 RED)"
            )
        (ROOT / CALIB_EVIDENCE_REL).write_text(
            json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        return 0.0, True, "实测 p100=0 → 零容差零条目(measured 事实;budget 零追加,标定件落 evidence)"
    threshold = measured * 2.0
    if existing is not None:
        if not _entry_is_measured(existing):
            return None, False, f"{TOL_ENTRY_ID} 非 measured_local(estimated 冒充 measured 即 RED)"
        if float(existing.get("measured_value", "nan")) != measured:
            return None, False, (
                f"{TOL_ENTRY_ID} 在档值 {existing.get('measured_value')} ≠ 复跑 {measured}"
                "(确定性面漂移即 RED,只追加禁改写)"
            )
        if not (ROOT / existing.get("evidence_file", "")).is_file():
            return None, False, f"{TOL_ENTRY_ID} evidence_file 缺失: {existing.get('evidence_file')}"
        return float(existing["threshold"]), True, (
            f"在档条目复跑位级一致 measured={measured:.15e} threshold={float(existing['threshold']):.15e}"
        )
    # 首次登记:标定 JSON 落 evidence(budget 通用路读 results.trimmed_mean vs threshold)。
    (ROOT / CALIB_EVIDENCE_REL).write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    entry = {
        "id": TOL_ENTRY_ID,
        "description": (
            "ReSTIR device vs host 金标准(estimate_ris 直调)同输入逐 trial estimate 对拍"
            "容差冻结带(20000 trial × M=16 候选 × 64 灯环形夹具,随机带 host 单源预生成"
            "已对齐消费序 + 录制自检锚位级前置,逐 trial 绝对差 p100;threshold = measured "
            "× 2.0 协议冻结 k,方向 max;M-a 标定腿产,两跑位级一致,禁手写 P-09;量化兜底"
            f"断言 tol < RED_BIAS 0.05 × 0.5(RFC-0045 §1.5 ④);样本集 digest "
            f"{doc['sample_manifest']['digest']}(count={doc['sample_manifest']['count']});"
            "标定程序 ci/g28_restir_device_kernel_smoke.py 标定腿可复跑"
        ),
        "direction": "max",
        "evidence": "measured_local",
        "skip_reason": None,
        "unit": "f32_absdiff",
        "threshold": threshold,
        "evidence_file": CALIB_EVIDENCE_REL,
        "measured_value": measured,
    }
    problems = append_budget_entries([entry])
    if problems:
        return None, False, f"budget 追加失败: {problems[:2]}"
    return threshold, True, f"程序产追加 measured={measured:.15e} threshold={threshold:.15e}"


# ---------------------------------------------------------------------------
# gi/ 冻结 0-byte 机核(RFC-0045 §1.8:vs g27-closed + 工作树双面)
# ---------------------------------------------------------------------------


def host_frozen_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--quiet", FROZEN_BASE, "--", *FROZEN_FILES])
    if r.returncode != 0:
        d = run(["git", "diff", "--name-only", FROZEN_BASE, "--", *FROZEN_FILES])
        changed = [x.strip() for x in d.stdout.splitlines() if x.strip()]
        return False, f"gi/ 冻结面有差分 vs {FROZEN_BASE}(触碰即 RED): {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", *FROZEN_FILES])
    if u.stdout.strip():
        dirty = [x for x in u.stdout.splitlines() if x.strip()]
        return False, f"gi/ 冻结面工作树未提交面: {dirty[:3]}"
    return True, (
        f"git diff --quiet {FROZEN_BASE} -- restir_reservoir.rs+multi_light.rs 0-byte"
        "(提交面 + 工作树双面)"
    )


# ---------------------------------------------------------------------------
# gate
# ---------------------------------------------------------------------------


def run_gate() -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}

    with gpu_device_lock(purpose=f"{TAG} 构建+SPV+标定+全档+RED 臂"):
        rurixc = build_rurixc()
        if rurixc is None:
            set_fact("spv_compile_spirv_val_pass", False, "rurixc 构建失败")
        else:
            ok, detail = compile_spv(rurixc)
            set_fact("spv_compile_spirv_val_pass", ok, detail)
        harness = build_harness()
        if harness is None:
            set_fact("calibration_two_run_bitexact", False, "harness 构建失败")
        elif facts["spv_compile_spirv_val_pass"]["status"] == "PASS":
            # ── 标定腿(两跑位级)+ budget 程序产(零容差零条目分支)+ F4 量化兜底 ──
            calib_doc, bitexact, detail = run_calibration(harness)
            set_fact("calibration_two_run_bitexact", bitexact, detail)
            tol: float | None = None
            if calib_doc is not None and bitexact:
                tol, reg_ok, reg_detail = register_tol_entry(calib_doc)
                set_fact("calibration_budget_entry_measured", reg_ok, reg_detail)
                if tol is not None:
                    set_fact(
                        "tol_under_red_bias_bound",
                        _tol_under_bound(tol),
                        f"tol={tol:.15e} vs 上界 {TOL_BOUND}(RED_BIAS {RED_BIAS} × 0.5,F4)",
                    )
            # ── 全档验证(--spv --tol;整数锚前置 + p100 + 3σ + 双跑位级)──
            if tol is not None and facts["tol_under_red_bias_bound"]["status"] == "PASS":
                print(f"[{TAG}] 全档验证: --spv --tol {tol!r}(REQUIRE_REAL+VK_VALIDATION)")
                r = run(
                    [str(harness), "--spv", str(SPV_PATH), "--tol", repr(tol)],
                    env=device_env(), timeout=3600,
                )
                line = json_line(r.stdout, "rurix.g28restir.harness.v1")
                doc = json.loads(line) if line else {}
                state = doc.get("state", "")
                verify_facts = (
                    "y_integer_anchor_all_equal",
                    "device_host_parity_p100",
                    "unbiased_3sigma_maintained",
                    "device_double_run_bitexact",
                )
                if state == "skipped_dev_env":
                    for fid in verify_facts:
                        set_fact(fid, False, "device SKIP(skipped_dev_env;RURIX_REQUIRE_REAL=1 下如实 FAIL 不假绿)")
                elif not doc:
                    for fid in verify_facts:
                        set_fact(fid, False, f"harness 无 evidence 行 rc={r.returncode}: {(r.stdout + r.stderr)[-200:]}")
                else:
                    anchor_ok = (
                        doc.get("y_anchor_all_equal") is True
                        and doc.get("dec_consumed_all_equal") is True
                    )
                    set_fact(
                        "y_integer_anchor_all_equal", anchor_ok,
                        f"保留样本 y 逐 trial 全等(真实承重锚)={doc.get('y_anchor_all_equal')};"
                        f"判定带消费计数全等={doc.get('dec_consumed_all_equal')}"
                        f"(钉死夹具下恒 16 平凡化事实照登:constant_16={doc.get('dec_consumed_constant_16')},"
                        "恒跑防协议漂移,RFC-0045 §1.2);录制自检锚位级前置="
                        f"{doc.get('recorder_selfcheck_bitexact')}",
                    )
                    parity = state == "pass" and r.returncode == 0 and doc.get("in_tol") is True
                    set_fact(
                        "device_host_parity_p100", parity,
                        f"全档 state={state};p100={doc.get('p100_vs_host')} ≤ tol={tol:.6e}"
                        f"(20000 trial 逐 trial estimate 绝对差)",
                    )
                    unb = doc.get("unbiased") or {}
                    set_fact(
                        "unbiased_3sigma_maintained", unb.get("pass") is True,
                        f"device 20000 estimate 均值 {unb.get('mean')} vs exact_direct {unb.get('reference')};"
                        f"dev={unb.get('dev')} < 3σ_mean+1e-9={unb.get('bound_3sigma')}(独立解析参考复核)",
                    )
                    set_fact(
                        "device_double_run_bitexact", doc.get("bitexact") is True,
                        "固定输入双跑输出缓冲 digest 位级相等" if doc.get("bitexact") is True
                        else "双跑 digest 不等",
                    )
                # ── kernel-bias RED 臂独立复跑 ──
                print(f"[{TAG}] RED 臂: --red-arm kernel-bias")
                ra = run(
                    [str(harness), "--red-arm", "kernel-bias", "--spv", str(SPV_PATH),
                     "--tol", repr(tol)],
                    env=device_env(), timeout=3600,
                )
                rl = json_line(ra.stdout, "rurix.g28restir.red_arm.v1")
                try:
                    rdoc = json.loads(rl) if rl else {}
                except json.JSONDecodeError:
                    rdoc = {}
                arm_ok = ra.returncode == 0 and rdoc.get("detected") is True
                set_fact(
                    "red_arm_kernel_bias_detected", arm_ok,
                    f"kernel-bias(RED_BIAS={RED_BIAS} 输出面加性偏置)"
                    f"{'检出' if arm_ok else 'FAIL'}({rdoc.get('detail', '')[:120]})",
                )

    ok, detail = host_frozen_0byte()
    set_fact("host_frozen_0byte", ok, detail)

    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=fact_rows,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            "G28.2 M-a:ReSTIR device kernel 兑现——g28_restir.rx 经 vk::run_compute 真跑"
            "(逐 trial 单 invocation WRS/RIS 链,dispatch [20000,1,1]);随机带单源纪律"
            "(PCG32 状态面留 host,已对齐消费序双带 + offset 三元组表 + 录制自检锚位级"
            "前置);y 整数锚前置(真实承重)+ 消费计数恒 16 平凡化照登 + estimate p100 ≤ "
            "程序产标定容差(×2.0 冻结 k,F4 量化兜底 tol<0.025)+ 无偏 3σ 独立复核 + "
            "device 双跑位级 + kernel-bias RED 臂 + gi/ 两文件 0-byte(vs g27-closed);"
            "RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1"
        ),
        host_section_pass=all_pass,
    )
    return 0 if (all_pass and code == 0) else 1


# ---------------------------------------------------------------------------
# selftest(反 YAML-only:判读器红绿两臂,无 GPU 依赖)
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

    # facts 闭集 ≥8 且 schema 在树(extra_facts minItems 6 被满足)。
    expect(len(FACT_IDS) >= 8, f"facts 闭集 {len(FACT_IDS)} ≥ 8")
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    min_items = schema.get("properties", {}).get("extra_facts", {}).get("minItems", 99)
    expect(len(FACT_IDS) >= min_items, f"facts {len(FACT_IDS)} ≥ schema minItems {min_items}")
    # 红臂①:F4 量化兜底判读器——超上界必拒,带内正例过。
    expect(not _tol_under_bound(0.03), "RED:tol=0.03 超上界必拒")
    expect(not _tol_under_bound(TOL_BOUND), "RED:tol=上界 0.025 必拒(严格 <)")
    expect(_tol_under_bound(5.7e-6), "GREEN:tol=5.7e-6 带内过")
    expect(_tol_under_bound(0.0), "GREEN:零容差态带内过(measured 事实)")
    # 红臂②:标定两跑位级判读器——漂移必拒。
    expect(not _calibration_lines_bitexact(['{"a":1}', '{"a":2}']), "RED:标定两跑漂移必拒")
    expect(_calibration_lines_bitexact(['{"a":1}', '{"a":1}']), "GREEN:标定两跑位级过")
    # 红臂③:harness 态判读——skipped_dev_env/fail 不得判 pass。
    expect(_harness_state('{"state":"skipped_dev_env"}') != "pass", "RED:SKIP 态非 pass")
    expect(_harness_state('{"state":"fail"}') != "pass", "RED:fail 态非 pass")
    expect(_harness_state('{"state":"pass"}') == "pass", "GREEN:pass 态正例")
    # 红臂④:estimated 冒充 measured 必拒。
    expect(not _entry_is_measured({"evidence": "estimated"}), "RED:estimated 注入必拒")
    expect(not _entry_is_measured({"evidence": "measured_local", "skip_reason": "no gpu"}), "RED:skip_reason 携带必拒")
    expect(_entry_is_measured({"evidence": "measured_local", "skip_reason": None}), "GREEN:measured_local 正例")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts={len(FACT_IDS)};4 红臂组 + 正例组)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=None)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    if args.gate is not None and args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
