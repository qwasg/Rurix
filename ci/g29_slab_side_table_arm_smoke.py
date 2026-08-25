#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G29.2 实现波)
"""G29.2 M-b 侧表供参加性臂兑现门冒烟
(g29.p0.m_b.slab_side_table_arm;G29_CONTRACT §4.2 M-b 行判据逐字;
rfcs/0046-material-device-integration.md §2 判据事实源;
G29_ACCEPTANCE_MAP §1 M-b 行)。

硬判据:g29_slab_device --side-table(bin-local;device 腿)——16 材质槽
slab 参数侧表(bin 内合成独立 SSBO:逐槽 [rc, ab],rc_k = k/15·0.95、
ab_k = (15−k)/15;**0.95 上限系有意规避 denom→0 角点区**,角点语义覆盖由
M-a 主网格独担 F5;侧表 SSBO host 单源生成一次原字节上传,device 不重算槽
参数——k/15 非 2 幂分母求值序位级敏感)——kernel 复用 §1(输入换侧表
SSBO + 槽索引寻址):⓪输出有限性一等断言(全槽 is_finite 硬 FAIL 先行,
F3 同律)①逐槽对拍 p100 ≤ M-a 同源标定容差(budget 条目程序产;零容差态
= calibration evidence measured=0)②逐槽白炉互核 dev 登记(每槽 ab=1 变体
host/device 双端重算,16 槽逐槽登记)③双跑位级 ④graph/types.rs +
material/ 整目录 vs g28-closed 0-byte 机核(MaterialClosure 32B 与 reserved
拓扑位零触碰)⑤防混淆声明机核(F8):bin-local SSBO 与 material/ 生产资产
侧表设施零挂接——检索 bin 源码零 import(pattern 常量表)。

三态:无 Vulkan loader/设备 → harness skipped_dev_env(退 0 非 fake pass);
本脚本默认 RURIX_REQUIRE_REAL=1(setdefault),该态下 SKIP → 硬红如实登记
FAIL,不假绿。

用法:
  py -3 ci/g29_slab_side_table_arm_smoke.py --gate g29.p0.m_b.slab_side_table_arm
  py -3 ci/g29_slab_side_table_arm_smoke.py --verify-latest
  py -3 ci/g29_slab_side_table_arm_smoke.py --selftest
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

GATE_KEY = "g29.p0.m_b.slab_side_table_arm"
NUMERIC_STEP = 498
SUBJECT = "g29_m_b_slab_side_table_arm"
WAVE = "G29.2"
SCHEMA_PATH = ROOT / "milestones/g29/g29_m_b_slab_side_table_arm_evidence_schema.json"
SOURCE_REF = (
    "G29_CONTRACT §4.2 M-b;rfcs/0046-material-device-integration.md §2;"
    "G29_ACCEPTANCE_MAP §1 M-b 行"
)
TAG = "g29_m_b"

KERNEL = ROOT / "src/rurix-render/kernels/g29_slab.rx"
WORK_DIR = ROOT / ".tmp/g29_gates"
SPV_PATH = WORK_DIR / "g29_slab.spv"
BUDGET_PATH = ROOT / "milestones/g29/g29_budget.json"
CALIB_EVIDENCE_REL = "evidence/g29_slab_device_calibration.json"
TOL_ENTRY_ID = "g29.slab_device.host_device_reflectance_tol"
HARNESS_BIN = "g29_slab_device"
HARNESS_SRC_REL = "src/rurix-render/src/bin/g29_slab_device.rs"
SIDE_TABLE_EVIDENCE_REL = "evidence/g29_slab_side_table_arm.json"
FROZEN_BASE = "g28-closed"
# RFC-0046 §2.3 ④:MaterialClosure 32B 冻结边界(graph/types.rs)+ material/
# 整目录(§1.7 F8 同面;生产资产侧表设施本体同受机核保护)。
FROZEN_PATHS = [
    "src/rurix-render/src/graph/types.rs",
    "src/rurix-render/src/material",
]
N_SLOTS = 16
# 防混淆声明机核(F8):bin 源码禁挂接生产资产侧表设施的检索 pattern 常量表
# (rust path 段 + 类型名;声明性机核,零命中 = 零挂接)。
CONFLATION_PATTERNS = ("material::side_table", "side_table::", "MaterialSideTable")

# facts 闭集(≥6;schema extra_facts minItems 6)。
FACT_IDS = [
    "finiteness_first_class",
    "per_slot_parity_p100",
    "per_slot_furnace_dev_registered",
    "double_run_bitexact",
    "material_closure_frozen",
    "no_production_side_table_conflation",
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
# M-a 同源容差解析(RFC-0046 §2.2:p100 同 §1.3 容差协议——budget 条目在档
# 读 threshold;零容差态 = 条目零追加 + calibration evidence measured=0)
# ---------------------------------------------------------------------------


def resolve_tol() -> tuple[float | None, str]:
    if not BUDGET_PATH.is_file():
        return None, "g29_budget.json 缺失"
    budget = json.loads(BUDGET_PATH.read_text(encoding="utf-8"))
    entry = next((e for e in budget.get("entries", []) if e.get("id") == TOL_ENTRY_ID), None)
    if entry is not None:
        if entry.get("evidence") != "measured_local" or entry.get("skip_reason"):
            return None, f"{TOL_ENTRY_ID} 非 measured_local(estimated 冒充 measured 即 RED)"
        return float(entry["threshold"]), (
            f"M-a 同源容差(budget 在档条目 threshold={float(entry['threshold']):.15e},"
            f"measured={entry.get('measured_value')},×2.0 冻结 k 程序产)"
        )
    calib = ROOT / CALIB_EVIDENCE_REL
    if calib.is_file():
        doc = json.loads(calib.read_text(encoding="utf-8"))
        measured = float(doc.get("results", {}).get("trimmed_mean", "nan"))
        if measured == 0.0:
            return 0.0, "M-a 零容差零条目态(实测 p100=0 measured 事实;calibration evidence 在档)"
        return None, f"budget 无条目但标定件 measured={measured!r} ≠ 0(状态矛盾即 RED)"
    return None, (
        "M-a 容差未定(budget 条目与 calibration evidence 均缺;"
        "先跑 ci/g29_slab_device_kernel_smoke.py --gate 标定腿)"
    )


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面)
# ---------------------------------------------------------------------------


def _finiteness_pass(doc: dict) -> bool:
    """判据⓪判读器(F3 同律):全槽 is_finite 且断言先于聚合协议字面在档。"""
    return (
        doc.get("finite_all") is True
        and doc.get("finiteness_checked_before_aggregation") is True
    )


def _parity_pass(doc: dict, rc: int) -> bool:
    return doc.get("state") == "pass" and rc == 0 and doc.get("in_tol") is True


def _furnace_registered(doc: dict) -> tuple[bool, str]:
    """逐槽白炉互核登记核验:16 行全量 + 每行 host/device 双端 dev 字段。"""
    rows = doc.get("per_slot_rows") or []
    if len(rows) != N_SLOTS:
        return False, f"逐槽登记行数 {len(rows)} ≠ {N_SLOTS}"
    keys_ok = all(
        all(k in row for k in ("k", "rc", "ab", "host_r", "device_r", "absdiff",
                               "furnace_host_dev", "furnace_device_dev"))
        for row in rows
    )
    if not keys_ok:
        return False, "逐槽行字段不全(白炉互核须 host/device 双端 dev)"
    return True, (
        f"16 槽逐槽白炉互核(每槽 ab=1 变体 host/device 双端重算)dev 全量登记:"
        f"host dev max={doc.get('furnace_host_dev_max')} device dev max="
        f"{doc.get('furnace_device_dev_max')}(如实登记面;侧表槽值非 2 幂网格值,"
        "host 位级 0 论证不适用,照实登记)"
    )


def _bitexact_pass(doc: dict) -> bool:
    return doc.get("double_run_bitexact") is True


def _conflation_scan(src_text: str) -> tuple[bool, list[str]]:
    """防混淆声明机核(F8):bin 源码对 pattern 常量表零命中 = 零挂接。"""
    hits = [p for p in CONFLATION_PATTERNS if p in src_text]
    return not hits, hits


# ---------------------------------------------------------------------------
# 冻结 0-byte 机核(RFC-0046 §2.3 ④:graph/types.rs + material/ 整目录 vs
# g28-closed + 工作树双面)
# ---------------------------------------------------------------------------


def frozen_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--quiet", FROZEN_BASE, "--", *FROZEN_PATHS])
    if r.returncode != 0:
        d = run(["git", "diff", "--name-only", FROZEN_BASE, "--", *FROZEN_PATHS])
        changed = [x.strip() for x in d.stdout.splitlines() if x.strip()]
        return False, f"冻结面有差分 vs {FROZEN_BASE}(触碰即 RED): {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", *FROZEN_PATHS])
    if u.stdout.strip():
        dirty = [x for x in u.stdout.splitlines() if x.strip()]
        return False, f"冻结面工作树未提交面: {dirty[:3]}"
    return True, (
        f"git diff --quiet {FROZEN_BASE} -- graph/types.rs + src/material 0-byte"
        "(提交面 + 工作树双面;MaterialClosure 32B 布局与 reserved 拓扑位零触碰,"
        "reserved RED 守卫本体同受机核保护)"
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

    with gpu_device_lock(purpose=f"{TAG} 构建+SPV+侧表臂(device 腿)"):
        rurixc = build_rurixc()
        spv_ok = False
        if rurixc is None:
            set_fact("finiteness_first_class", False, "rurixc 构建失败")
        else:
            spv_ok, spv_detail = compile_spv(rurixc)
            if not spv_ok:
                set_fact("finiteness_first_class", False, spv_detail)
        harness = build_harness() if spv_ok else None
        if spv_ok and harness is None:
            set_fact("finiteness_first_class", False, "harness 构建失败")
        if harness is not None:
            tol, tol_detail = resolve_tol()
            if tol is None:
                set_fact("per_slot_parity_p100", False, tol_detail)
            else:
                print(f"[{TAG}] 侧表臂: --side-table --tol {tol!r}(REQUIRE_REAL+VK_VALIDATION)")
                r = run(
                    [str(harness), "--side-table", "--spv", str(SPV_PATH),
                     "--tol", repr(tol), "--out", str(ROOT / SIDE_TABLE_EVIDENCE_REL)],
                    env=device_env(), timeout=3600,
                )
                line = json_line(r.stdout, "rurix.g29slab.side_table.v1")
                doc = json.loads(line) if line else {}
                state = doc.get("state", "")
                run_facts = (
                    "finiteness_first_class",
                    "per_slot_parity_p100",
                    "per_slot_furnace_dev_registered",
                    "double_run_bitexact",
                )
                if state == "skipped_dev_env":
                    for fid in run_facts:
                        set_fact(fid, False, "device SKIP(skipped_dev_env;RURIX_REQUIRE_REAL=1 下如实 FAIL 不假绿)")
                elif not doc:
                    for fid in run_facts:
                        set_fact(fid, False, f"--side-table 无 evidence 行 rc={r.returncode}: {(r.stdout + r.stderr)[-200:]}")
                else:
                    set_fact(
                        "finiteness_first_class",
                        _finiteness_pass(doc),
                        f"判据⓪:全槽(主表 16 + 白炉变体 16)is_finite={doc.get('finite_all')},"
                        f"断言先于聚合执行={doc.get('finiteness_checked_before_aggregation')}"
                        "(硬 FAIL 先行,RFC-0046 §2.3 F3 同律)",
                    )
                    set_fact(
                        "per_slot_parity_p100",
                        _parity_pass(doc, r.returncode),
                        f"state={state};逐槽对拍 p100={doc.get('parity_p100')} ≤ tol={tol:.6e}"
                        f"({tol_detail});16 槽 device 逐槽求值 vs host total_reflectance 直调",
                    )
                    fur_ok, fur_detail = _furnace_registered(doc)
                    set_fact("per_slot_furnace_dev_registered", fur_ok, fur_detail)
                    set_fact(
                        "double_run_bitexact",
                        _bitexact_pass(doc),
                        "固定输入双跑(主表 + 白炉变体两组 dispatch)输出缓冲 digest 位级相等"
                        if _bitexact_pass(doc) else "双跑 digest 不等",
                    )

    ok, detail = frozen_0byte()
    set_fact("material_closure_frozen", ok, detail)

    src_path = ROOT / HARNESS_SRC_REL
    if src_path.is_file():
        scan_ok, hits = _conflation_scan(src_path.read_text(encoding="utf-8"))
        set_fact(
            "no_production_side_table_conflation",
            scan_ok,
            (
                f"声明性机核(F8):检索 {HARNESS_SRC_REL} 对 pattern 常量表 "
                f"{CONFLATION_PATTERNS} 零命中——bin-local SSBO(bin 内合成不落资产)与 "
                "material/ 生产资产侧表设施(RFC-0025 通道)零挂接,禁其编解码/digest 设施"
            ) if scan_ok else f"bin 源码命中禁挂接 pattern: {hits}(防混淆声明破面即 RED)",
        )
    else:
        set_fact("no_production_side_table_conflation", False, f"{HARNESS_SRC_REL} 缺失")

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
            "G29.2 M-b:侧表供参加性臂兑现(bin-local device 腿)——16 材质槽 slab 参数"
            "侧表(bin 内合成独立 SSBO,rc_k=k/15·0.95、ab_k=(15−k)/15,0.95 有意规避 "
            "denom→0 角点区 F5;host 单源生成原字节上传,device 不重算槽参数);kernel 复用"
            " g29_slab.rx 逐槽求值 + host total_reflectance 直调对拍(p100 ≤ M-a 同源容差)"
            "+ 判据⓪全槽有限性一等断言先行(F3 同律)+ 逐槽白炉互核(每槽 ab=1 变体 host/"
            "device 双端重算 dev 逐槽登记)+ 双跑位级 + graph/types.rs 与 material/ 整目录 "
            "0-byte(vs g28-closed;MaterialClosure 32B 零触碰)+ 防混淆声明机核(bin 源码"
            f"零挂接生产侧表设施);全量臂产物 {SIDE_TABLE_EVIDENCE_REL};"
            "RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1"
        ),
        host_section_pass=all_pass,
    )
    return 0 if (all_pass and code == 0) else 1


# ---------------------------------------------------------------------------
# selftest(反 YAML-only:判读器红绿两臂,无 GPU/无构建依赖)
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

    # facts 闭集 ≥6 且 schema 在树(extra_facts minItems 6 被满足)。
    expect(len(FACT_IDS) >= 6, f"facts 闭集 {len(FACT_IDS)} ≥ 6")
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    min_items = schema.get("properties", {}).get("extra_facts", {}).get("minItems", 99)
    expect(len(FACT_IDS) >= min_items, f"facts {len(FACT_IDS)} ≥ schema minItems {min_items}")

    good_row = {
        "k": 0, "rc": 0.0, "ab": 1.0, "host_r": 1.0, "device_r": 1.0, "absdiff": 0.0,
        "furnace_host_dev": 0.0, "furnace_device_dev": 0.0,
    }
    good = {
        "state": "pass",
        "finite_all": True,
        "finiteness_checked_before_aggregation": True,
        "in_tol": True,
        "parity_p100": 3.7e-8,
        "furnace_host_dev_max": 0.0,
        "furnace_device_dev_max": 6.0e-8,
        "per_slot_rows": [dict(good_row, k=i) for i in range(N_SLOTS)],
        "double_run_bitexact": True,
    }
    # 红臂①:判据⓪——finite_all 假/协议字面缺失必拒(F3 同律)。
    expect(not _finiteness_pass({**good, "finite_all": False}), "RED:finite_all=false 必拒")
    expect(
        not _finiteness_pass({k: v for k, v in good.items() if k != "finiteness_checked_before_aggregation"}),
        "RED:缺先于聚合协议字面必拒",
    )
    expect(_finiteness_pass(good), "GREEN:判据⓪正例")
    # 红臂②:逐槽对拍——state/rc/in_tol 任一破必拒。
    expect(not _parity_pass({**good, "state": "fail"}, 1), "RED:state=fail 必拒")
    expect(not _parity_pass({**good, "in_tol": False}, 0), "RED:in_tol=false 必拒")
    expect(not _parity_pass(good, 1), "RED:rc=1 必拒")
    expect(_parity_pass(good, 0), "GREEN:逐槽对拍正例")
    # 红臂③:逐槽白炉登记——行缺/字段缺必拒;dev 非零如实登记过(登记面)。
    expect(not _furnace_registered({**good, "per_slot_rows": good["per_slot_rows"][:15]})[0], "RED:15 行必拒")
    bad_row_doc = {**good, "per_slot_rows": [dict(good_row, k=i) for i in range(15)] + [{"k": 15}]}
    expect(not _furnace_registered(bad_row_doc)[0], "RED:行缺白炉双端 dev 字段必拒")
    nonzero = {**good, "per_slot_rows": [dict(good_row, k=i, furnace_device_dev=6e-8) for i in range(N_SLOTS)]}
    expect(_furnace_registered(nonzero)[0], "GREEN:device dev 非零如实登记过(登记面)")
    # 红臂④:双跑位级漂移必拒。
    expect(not _bitexact_pass({**good, "double_run_bitexact": False}), "RED:双跑漂移必拒")
    expect(_bitexact_pass(good), "GREEN:双跑位级正例")
    # 红臂⑤:防混淆声明机核——生产侧表设施 import 注入必拒。
    ok, hits = _conflation_scan("use rurix_render::material::slab::SlabStack;\n")
    expect(ok and not hits, "GREEN:仅挂接 material::slab 金标准过")
    ok, hits = _conflation_scan("use rurix_render::material::side_table::MaterialSideTable;\n")
    expect(not ok and len(hits) >= 1, "RED:import 生产侧表设施必拒")
    ok, _hits = _conflation_scan("let d = side_table::encode(&t);\n")
    expect(not ok, "RED:path 段挂接编解码设施必拒")
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
