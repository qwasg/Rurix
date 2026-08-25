#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G29.2 实现波)
"""G29.2 M-a slab device kernel 兑现门冒烟
(g29.p0.m_a.slab_device_kernel;G29_CONTRACT §4.2 M-a 行判据逐字;
rfcs/0046-material-device-integration.md §1 判据事实源;
G29_ACCEPTANCE_MAP §1 M-a 行)。

硬判据:kernels/g29_slab.rx(rurixc --target vulkan 产 SPV + spirv-val
通过)经 vk::run_compute 派发 + device vs host 金标准(material/slab.rs
total_reflectance f64 直调,material/ 整目录 vs g28-closed 0-byte 冻结)
同输入逐样本对拍——16641 样本 = 129×129 参数网格(rc=i/128、ab=j/128,
g22_slab_probe GRID=128 经 furnace_audit (grid+1)² 格点口径,F4 血缘钉死;
host 单源生成原字节上传,device 不重算格点)——公式面修法 A(RFC-0046 §1.1
blocker F2 disposition):R = rc + tc·tc·ab / max(denom, 1e-30) 直线代码
零分支零门,角点 rc=ab=1 → 分子 0 → 0/1e-30=0 → R=rc=1.0 位级同 host
分支值:⓪输出有限性一等断言(16641 样本全量 is_finite,任一非有限硬 FAIL
**先于对拍聚合执行**——封死 NaN 经 f64::max(NaN,x)=x 聚合静默吞掉的假绿
路径,F3)①逐样本 |device−host| 绝对差 p100 ≤ 标定容差(threshold =
measured × 2.0 冻结 k,标定腿两跑位级一致程序产,禁手写;实测 p100=0 →
零容差零条目 measured 事实;量化兜底:断言 tol < RED_BIAS 0.05 × 0.5 =
0.025,封死「标定与判定同实现」循环论证)②白炉行(ab=1 列 129 样本)
device dev 最大值如实登记(host 白炉 R 位级 ≡ 1.0 可断言;device dev 来源
= Vulkan FP32 OpFDiv ≤2.5 ULP + FMA 收缩可能性,不冒充解析 0;覆盖论证
F1:白炉行 ⊂ 网格 ⇒ 已被判据①容差线传递覆盖)③能量上界 device 复核
(全样本 device R ≤ 1 + 容差)④device 双跑位级一致(输出缓冲 digest)
⑤kernel-bias RED 臂必检出(仅评判据①,判据⓪②③④不跨臂,F11)
+ material/ 整目录 vs g28-closed 0-byte 机核(F8 扩面)。

三态:无 Vulkan loader/设备 → harness skipped_dev_env(退 0 非 fake pass);
本脚本默认 RURIX_REQUIRE_REAL=1(setdefault),该态下 SKIP → 硬红如实登记
FAIL,不假绿。

用法:
  py -3 ci/g29_slab_device_kernel_smoke.py --gate g29.p0.m_a.slab_device_kernel
  py -3 ci/g29_slab_device_kernel_smoke.py --verify-latest
  py -3 ci/g29_slab_device_kernel_smoke.py --selftest
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

GATE_KEY = "g29.p0.m_a.slab_device_kernel"
NUMERIC_STEP = 496
SUBJECT = "g29_m_a_slab_device_kernel"
WAVE = "G29.2"
SCHEMA_PATH = ROOT / "milestones/g29/g29_m_a_slab_device_kernel_evidence_schema.json"
SOURCE_REF = (
    "G29_CONTRACT §4.2 M-a;rfcs/0046-material-device-integration.md §1;"
    "G29_ACCEPTANCE_MAP §1 M-a 行"
)
TAG = "g29_m_a"

KERNEL = ROOT / "src/rurix-render/kernels/g29_slab.rx"
WORK_DIR = ROOT / ".tmp/g29_gates"
SPV_PATH = WORK_DIR / "g29_slab.spv"
BUDGET_PATH = ROOT / "milestones/g29/g29_budget.json"
CALIB_EVIDENCE_REL = "evidence/g29_slab_device_calibration.json"
TOL_ENTRY_ID = "g29.slab_device.host_device_reflectance_tol"
HARNESS_BIN = "g29_slab_device"
FROZEN_BASE = "g28-closed"
# RFC-0046 §1.7(F8 扩面):material/ 整目录冻结——slab.rs/closure/side_table.rs/
# table.rs 等生产面全部圈入;reserved RED 守卫本体因此同受机核保护。
FROZEN_PATHS = [
    "src/rurix-render/src/material",
]
# RFC-0046 §1.4 ⑤ 量化兜底:RED_BIAS=0.05(g13/g26/g28 同值),标定容差绝对上界 = ×0.5。
RED_BIAS = 0.05
TOL_BOUND = RED_BIAS * 0.5

# facts 闭集(≥8;schema extra_facts minItems 6;finiteness 独立 fact,F3)。
FACT_IDS = [
    "spv_compile_spirv_val_pass",
    "calibration_two_run_bitexact",
    "calibration_budget_entry_measured",
    "tol_under_red_bias_bound",
    "finiteness_first_class_assertion",
    "device_host_parity_p100",
    "white_furnace_row_dev_registered",
    "energy_upper_bound_device",
    "device_double_run_bitexact",
    "red_arm_kernel_bias_detected",
    "material_dir_frozen_0byte",
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
    # M-a 行「spirv-val 通过」为硬判据,不 SKIP,RFC-0046 §1.4 ⑥)。
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
# g29_budget(标定条目程序产追加;确定性面复跑位级期望,漂移即 RED)
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
    """字节级纯追加(既有字节 0-byte;g13/g26/g28 append_budget_entries 同模)。"""
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
        return ["g29_budget.json 结构锚缺失(拒改写)"]
    head, sep, tail = budget_text.partition(anchor)
    budget_text = head + frag + sep + tail
    json.loads(budget_text)
    BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems


def _tol_under_bound(tol: float) -> bool:
    """量化兜底:标定容差(threshold = measured×2.0)必须 < RED_BIAS×0.5。"""
    return tol < TOL_BOUND


def _calibration_lines_bitexact(lines: list[str]) -> bool:
    return len(lines) == 2 and lines[0] == lines[1]


def _harness_state(line: str) -> str:
    try:
        return json.loads(line).get("state", "")
    except json.JSONDecodeError:
        return ""


def _finiteness_first_class(doc: dict) -> bool:
    """判据⓪判读器(F3):finite_all 且断言先于聚合执行的协议字面在档。"""
    return (
        doc.get("finite_all") is True
        and doc.get("finiteness_checked_before_aggregation") is True
    )


# ---------------------------------------------------------------------------
# 标定腿(两跑位级一致 + threshold = measured × 2.0 程序产入 budget;
# 实测 p100=0 → 零容差零条目 measured 事实,RFC-0046 §1.3)
# ---------------------------------------------------------------------------


def run_calibration(harness: Path) -> tuple[dict | None, bool, str]:
    lines: list[str] = []
    for run_idx in (1, 2):
        print(f"[{TAG}] 标定跑 {run_idx}: --calibrate")
        r = run(
            [str(harness), "--calibrate", "--spv", str(SPV_PATH)],
            env=device_env(), timeout=3600,
        )
        line = json_line(r.stdout, "rurix.g29slab.calibration_entry.v1")
        if r.returncode != 0 or line is None:
            skip = json_line(r.stdout, "rurix.g29slab.calibration_skip.v1")
            if skip is not None:
                return None, False, f"标定腿 SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP 充绿): {skip[:200]}"
            return None, False, f"标定腿跑 {run_idx} 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}"
        lines.append(line)
    if not _calibration_lines_bitexact(lines):
        return None, False, "标定腿两跑非位级一致(确定性协议漂移即 RED)"
    doc = json.loads(lines[0])
    return doc, True, f"两跑位级一致;p100={doc['results']['trimmed_mean']:.15e}(16641 样本全集)"


def register_tol_entry(doc: dict) -> tuple[float | None, bool, str]:
    """标定条目登记:实测 p100=0 → 零容差零条目(budget 零追加,measured 事实
    如实登记);p100>0 → 缺则程序产追加(threshold = measured × 2.0),在档则
    复跑位级期望(measured 漂移即 RED,确定性面)。返回 (tol, ok, detail)。"""
    measured = float(doc["results"]["trimmed_mean"])
    budget = load_budget()
    if budget is None:
        return None, False, "g29_budget.json 缺失"
    existing = budget_entry(budget, TOL_ENTRY_ID)
    if measured == 0.0:
        # 零容差零条目(RFC-0046 §1.3:零容差只能是 measured 事实;禁手写)。
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
            "slab device vs host 金标准(material/slab.rs::total_reflectance f64 直调)"
            "同输入逐样本反照率对拍容差冻结带(16641 样本 = 129×129 参数网格 rc=i/128、"
            "ab=j/128〔g22_slab_probe GRID=128 (grid+1)² 格点口径,F4 血缘钉死〕,host "
            "单源生成原字节上传,kernel 全 f32 修法 A 分母安全化,判据⓪有限性一等断言"
            "先于聚合;逐样本绝对差 p100;threshold = measured × 2.0 协议冻结 k,方向 "
            "max;M-a 标定腿产,两跑位级一致,禁手写 P-09;量化兜底断言 tol < RED_BIAS "
            f"0.05 × 0.5(RFC-0046 §1.4 ⑤);样本集 digest {doc['sample_manifest']['digest']}"
            f"(count={doc['sample_manifest']['count']});标定程序 "
            "ci/g29_slab_device_kernel_smoke.py 标定腿可复跑"
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
# material/ 整目录冻结 0-byte 机核(RFC-0046 §1.7 F8 扩面:vs g28-closed +
# 工作树双面)
# ---------------------------------------------------------------------------


def host_frozen_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--quiet", FROZEN_BASE, "--", *FROZEN_PATHS])
    if r.returncode != 0:
        d = run(["git", "diff", "--name-only", FROZEN_BASE, "--", *FROZEN_PATHS])
        changed = [x.strip() for x in d.stdout.splitlines() if x.strip()]
        return False, f"material/ 冻结面有差分 vs {FROZEN_BASE}(触碰即 RED): {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", *FROZEN_PATHS])
    if u.stdout.strip():
        dirty = [x for x in u.stdout.splitlines() if x.strip()]
        return False, f"material/ 冻结面工作树未提交面: {dirty[:3]}"
    return True, (
        f"git diff --quiet {FROZEN_BASE} -- src/rurix-render/src/material 0-byte"
        "(整目录;提交面 + 工作树双面;reserved RED 守卫本体同受保护)"
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
            # ── 标定腿(两跑位级)+ budget 程序产(零容差零条目分支)+ 量化兜底 ──
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
                        f"tol={tol:.15e} vs 上界 {TOL_BOUND}(RED_BIAS {RED_BIAS} × 0.5,"
                        "RFC-0046 §1.4 ⑤)",
                    )
            # ── 全档验证(--spv --tol;⓪有限性一等 → ①p100 → ②白炉行 →
            #    ③能量上界 → ④双跑位级)──
            if tol is not None and facts["tol_under_red_bias_bound"]["status"] == "PASS":
                print(f"[{TAG}] 全档验证: --spv --tol {tol!r}(REQUIRE_REAL+VK_VALIDATION)")
                r = run(
                    [str(harness), "--spv", str(SPV_PATH), "--tol", repr(tol)],
                    env=device_env(), timeout=3600,
                )
                line = json_line(r.stdout, "rurix.g29slab.harness.v1")
                doc = json.loads(line) if line else {}
                state = doc.get("state", "")
                verify_facts = (
                    "finiteness_first_class_assertion",
                    "device_host_parity_p100",
                    "white_furnace_row_dev_registered",
                    "energy_upper_bound_device",
                    "device_double_run_bitexact",
                )
                if state == "skipped_dev_env":
                    for fid in verify_facts:
                        set_fact(fid, False, "device SKIP(skipped_dev_env;RURIX_REQUIRE_REAL=1 下如实 FAIL 不假绿)")
                elif not doc:
                    for fid in verify_facts:
                        set_fact(fid, False, f"harness 无 evidence 行 rc={r.returncode}: {(r.stdout + r.stderr)[-200:]}")
                else:
                    set_fact(
                        "finiteness_first_class_assertion",
                        _finiteness_first_class(doc),
                        f"判据⓪:16641 样本全量 is_finite={doc.get('finite_all')},断言先于对拍"
                        f"聚合执行={doc.get('finiteness_checked_before_aggregation')}(封死 NaN "
                        "经 f64::max(NaN,x)=x 聚合静默吞掉的假绿路径,RFC-0046 §1.4 F3)",
                    )
                    corner = doc.get("corner") or {}
                    parity = state == "pass" and r.returncode == 0 and doc.get("in_tol") is True
                    set_fact(
                        "device_host_parity_p100", parity,
                        f"全档 state={state};p100={doc.get('p100_vs_host')} ≤ tol={tol:.6e}"
                        f"(16641 样本逐样本反照率绝对差);角点 rc=ab=1 device R="
                        f"{corner.get('device_r')}(位级 1.0={corner.get('device_bitexact_one')},"
                        "修法 A 角点论证兑现)",
                    )
                    fur = doc.get("furnace_row") or {}
                    set_fact(
                        "white_furnace_row_dev_registered",
                        fur.get("host_bitexact_one") is True and "device_dev_max" in fur,
                        f"白炉行(ab=1 列 {fur.get('samples')} 样本):host R 位级 ≡ 1.0="
                        f"{fur.get('host_bitexact_one')};device dev 最大值 "
                        f"{fur.get('device_dev_max')} 如实登记(来源 = OpFDiv ≤2.5 ULP + FMA "
                        "收缩可能性;登记面被判据①容差线传递覆盖,F1 不另设线)",
                    )
                    eb = doc.get("energy_bound") or {}
                    set_fact(
                        "energy_upper_bound_device", eb.get("pass") is True,
                        f"全样本 device max R={eb.get('device_max_r')} ≤ 1+tol={eb.get('bound')}"
                        "(host energy_never_exceeds_unity 同律的 device 面)",
                    )
                    set_fact(
                        "device_double_run_bitexact", doc.get("bitexact") is True,
                        "固定输入双跑输出缓冲 digest 位级相等" if doc.get("bitexact") is True
                        else "双跑 digest 不等",
                    )
                # ── kernel-bias RED 臂独立复跑(仅评判据①,F11)──
                print(f"[{TAG}] RED 臂: --red-arm kernel-bias")
                ra = run(
                    [str(harness), "--red-arm", "kernel-bias", "--spv", str(SPV_PATH),
                     "--tol", repr(tol)],
                    env=device_env(), timeout=3600,
                )
                rl = json_line(ra.stdout, "rurix.g29slab.red_arm.v1")
                try:
                    rdoc = json.loads(rl) if rl else {}
                except json.JSONDecodeError:
                    rdoc = {}
                arm_ok = ra.returncode == 0 and rdoc.get("detected") is True
                set_fact(
                    "red_arm_kernel_bias_detected", arm_ok,
                    f"kernel-bias(RED_BIAS={RED_BIAS} 输出面加性偏置)"
                    f"{'检出' if arm_ok else 'FAIL'}({rdoc.get('detail', '')[:160]});"
                    "臂间判据归属 F11:仅评判据①,判据⓪②③④不跨臂",
                )

    ok, detail = host_frozen_0byte()
    set_fact("material_dir_frozen_0byte", ok, detail)

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
            "G29.2 M-a:slab device kernel 兑现——g29_slab.rx 经 vk::run_compute 真跑"
            "(逐样本单 invocation,dispatch [16641,1,1];129×129 参数网格 host 单源生成"
            "原字节上传);公式面修法 A(max(denom,1e-30) 分母安全化直线代码零分支,角点"
            "rc=ab=1 → R=1.0 位级同 host 分支值,F2);判据⓪输出有限性一等断言先于聚合"
            "(F3)+ 逐样本 p100 ≤ 程序产标定容差(×2.0 冻结 k,量化兜底 tol<0.025)+ "
            "白炉行 dev 如实登记(F1 覆盖论证)+ 能量上界 device 复核 + device 双跑位级 + "
            "kernel-bias RED 臂(仅评判据①,F11)+ material/ 整目录 0-byte(vs g28-closed,"
            "F8);RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1"
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
    expect("finiteness_first_class_assertion" in FACT_IDS, "finiteness 独立 fact 在闭集(F3)")
    # 红臂①:量化兜底判读器——超上界必拒,带内正例过。
    expect(not _tol_under_bound(0.03), "RED:tol=0.03 超上界必拒")
    expect(not _tol_under_bound(TOL_BOUND), "RED:tol=上界 0.025 必拒(严格 <)")
    expect(_tol_under_bound(2.4e-7), "GREEN:tol=2.4e-7 带内过")
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
    # 红臂⑤:判据⓪判读器——finite_all 假/协议字面缺失必拒(F3)。
    expect(
        not _finiteness_first_class({"finite_all": False, "finiteness_checked_before_aggregation": True}),
        "RED:finite_all=false 必拒",
    )
    expect(
        not _finiteness_first_class({"finite_all": True}),
        "RED:缺先于聚合协议字面必拒",
    )
    expect(
        _finiteness_first_class({"finite_all": True, "finiteness_checked_before_aggregation": True}),
        "GREEN:判据⓪正例",
    )
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts={len(FACT_IDS)};5 红臂组 + 正例组)")
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
