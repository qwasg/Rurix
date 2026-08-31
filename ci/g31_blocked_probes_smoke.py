#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C17 阻塞项新鲜探针门）
"""G31+ 波 C Task C17 TODO 阻塞项全量新鲜探针门（g31.waveC.blockedprobes）。

承接:G31_PLUS_COMMERCIAL_RENDERER_TODO 阻塞项 12 探针(#11/#15/#17/#18/#19/
#30/#40/#41/#42/#45/#46/#47)——探针登记件 = milestones/g31/
g31_blocked_probes_2026.json(探针 agent 真跑/真检索只追加落盘)。本门:

1. **登记件机器核验(host 恒跑)**:12 探针齐备 + 逐项 verdict ∈
   {open-maintained, blocked-dev-env} 闭集 + **零冒充机核**(无一项被标
   closed/resolved)+ 逐项 anchor_unchanged=true + summary 计数重算一致。
2. **活体复核(host 恒跑腿)**:轻快探针面新鲜重跑/重检索并与登记不变量比对
   ——三工具 PATH/BistroExterior 检索、vulkaninfo 五 token(HDR×3 + WG + DGC
   ×3)与设备枚举、VM 面、OMM 材质计数、物理观察轨 22 pattern、SAFE-GPU 三面、
   legacy 清册零 close、本地工具链版本、RD-034 blocked 探针、RD-045 三件面。
   **翻转纪律(F10 门态映射)**:阻塞→解锁翻转 = 锚命中重判信号(合法门绿,
   如实登记 signals);门 FAIL 只留程序未诚实执行(探针未真跑/登记件畸形/
   verdict 越闭集/冒充 closed)。
3. **device 腿(三态)**:RD-045 新鲜 digest 抽查 1 轮(orbit 64+10 对波 B 锚)
   ——GPU/harness/SPV/bistro 缺 → DEV_ENV_DEGRADE 输出 SKIP(退 0,禁冒充
   PASS);RURIX_REQUIRE_REAL=1 翻硬 FAIL;digest ≠ 锚 = 漂移事件诚实红。
4. **--selftest**:verdict 闭集校验/零冒充扫描/digest 比较/翻转分类器纯函数
   红绿臂 + schema 互核,不依赖树上文件与 GPU。

产物:evidence/g31_blocked_probes_<utc>.json(schema
milestones/g31/g31_blocked_probes_evidence_schema.json;PASS-only 闭集,
FAIL 诊断件留 .tmp 工作区)。

用法:
  py -3 ci/g31_blocked_probes_smoke.py --gate g31.waveC.blockedprobes
  py -3 ci/g31_blocked_probes_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

TAG = "g31_blocked_probes"
GATE_KEY = "g31.waveC.blockedprobes"
SCHEMA_ID = "rurix.g31.blocked_probes_smoke_evidence.v1"
SCHEMA_PATH = ROOT / "milestones/g31/g31_blocked_probes_evidence_schema.json"
PROBES_PATH = ROOT / "milestones/g31/g31_blocked_probes_2026.json"
LEGACY_REGISTRY = ROOT / "milestones/g24/g24_legacy_rd_registry.json"
G31_CONTRACT = ROOT / "milestones/g31/G31_CONTRACT.md"
WORK = ROOT / ".tmp" / "g31_blocked_probes"

VERDICT_CLOSED_SET = ("open-maintained", "blocked-dev-env")
EXPECTED_PROBE_IDS = [f"P{i:02d}" for i in range(1, 13)]
# G37 W4 重收割(默认翻转获批执行,2026-08-30):波 B 锚 060e69a81e26…(旧十臂
# 展开前默认臂,且其消费二进制已被 day_0828 Phase F 构建事故覆盖 ⇒ 锚本已漂)
# 作废;W4 锚 = 翻转后默认臂(--quality 缺省 full 十九臂)orbit 64+10,release 与
# target-night 双二进制收割同值 + 各自双跑位级(w4_flip/W4_ANCHORS.json /
# ev/rd045_release_r{1,2}.json 在案)。
# G38 Wave3 重收割(法线 v2 消费切换,2026-08-30):ef2b5b19…(v1 法线面)作废;
# 新锚 = 同臂形 @ baked_normals_bin_v2 消费面(slot14 桌布坏件替平坦,语义变更即
# 重锚),仍为 release 与 target-night 双二进制收割同值 + 各自双跑位级
# (artifacts/day_0830_g38/reanchor/G38_ANCHORS.json 在案)。
RD045_ANCHOR_DIGEST = "sha256:066395b0b6d877f546b7082560c093b4c325f39dae446abf3d59a8ad1023d56d"
HDR_TOKENS = (
    "VK_COLOR_SPACE_HDR10_ST2084_EXT",
    "VK_COLOR_SPACE_BT2020_LINEAR_EXT",
    "VK_COLOR_SPACE_HDR10_HLG_EXT",
)
WG_TOKEN = "VK_AMDX_shader_enqueue"
DGC_TOKENS = (
    "VK_EXT_device_generated_commands",
    "VK_NV_device_generated_commands",
    "VK_NV_device_generated_commands_compute",
)
# 物理观察轨 22 pattern(g30 尾锚窗常量表逐字沿用禁缩面;M125 三件基线排除)
PATTERN_GREPS = (
    ("rd042", "differentiable", ("src", "apps")),
    ("rd042", "autodiff", ("src", "apps")),
    ("rd043", "wgrapier", ("src", "apps")),
    ("rd044", "character_cloth|destructible", ("src/rurix-physics/src", "apps")),
    ("rd044", "mpm_bake", ("src", "apps")),
    ("rd044", "rapier_fast_path|rapier_fastpath", ("src", "apps")),
    ("m125", "rurix_physics_sys56|JPC56_|JPH56", ("src/rurix-physics/src", "apps")),
    ("m125", "jolt_53_defect|jolt53_workaround", ("src/rurix-physics/src", "apps")),
    ("m125", "ab_overband", ("src/rurix-physics/src", "apps")),
    ("m127", "neural_deform", ("src", "apps")),
)
M125_BASELINE = {
    "src/rurix-physics/src/ab_eval.rs",
    "src/rurix-physics/src/bin/g9_m125_jolt56_ab.rs",
    "src/rurix-physics/src/world.rs",
}
PATTERN_GLOBS = (
    ("rd043", "rfcs/*wgrapier*"),
    ("rd043", "rfcs/*gpu*rigid*"),
    ("rd044", "apps/**/*cloth*"),
    ("rd044", "assets/**/*mpm*"),
    ("rd044", "evidence/*rapier_fastpath_adoption*"),
    ("m125", "apps/**/*jolt56*"),
    ("m125", "evidence/*jolt_53_defect*"),
    ("m125", "evidence/*jolt_56_ab_overband*"),
)
M127_DIRS = ("corpus", "assets/corpus", "assets/neural", "conformance/neural")
GREP_FILE_EXTS = {".rs", ".rx", ".toml", ".md", ".py", ".json", ".c", ".cpp", ".h", ".hpp", ".cu", ".txt"}
WALK_SKIP_DIRS = {".git", "target", ".tmp", "node_modules", "__pycache__"}
SPV_DIR = ROOT / ".tmp" / "g14_gates" / "m_c"
SPV_FILES = (
    "g14_3_direct_gi.spv",
    "g14_mv.spv",
    "g14_8_tsr_resample.spv",
    "g14_8_tsr_resolve.spv",
    "g31_display_encode.spv",
)
BISTRO_GLTF = Path("K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf")

FAILURES: list[str] = []
NOTES: list[str] = []
SIGNALS: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)
    print(f"[{TAG}] {msg}", flush=True)


def signal(msg: str) -> None:
    """锚命中/翻转信号(合法门绿分支,F10 门态映射——分支捕获非透传)。"""
    SIGNALS.append(msg)
    print(f"[{TAG}] SIGNAL {msg}", flush=True)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def run_cmd(argv: list[str], timeout: int = 300, cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True, timeout=timeout,
                          cwd=str(cwd) if cwd else None)


# ---------------------------------------------------------------------------
# 纯函数面(--selftest 同消费)
# ---------------------------------------------------------------------------

def verdict_in_closed_set(verdict) -> bool:
    return isinstance(verdict, str) and verdict in VERDICT_CLOSED_SET


def zero_masquerade_scan(probes: list[dict]) -> list[str]:
    """零冒充机核:任一探针 verdict/状态面标 closed/resolved → 违例列表。"""
    bad: list[str] = []
    forbidden = ("closed", "resolved", "close", "done", "fixed")
    for p in probes:
        pid = p.get("probe_id", "?")
        v = p.get("verdict")
        if isinstance(v, str) and v.strip().lower() in forbidden:
            bad.append(f"{pid} verdict 冒充 closed/resolved: {v!r}")
        sm = p.get("status_maintained_literal", "")
        if isinstance(sm, str) and re.search(r"\bclosed\b|\bresolved\b", sm):
            # 维持字面只允许「未/不/零 close」语境;裸标 closed 即冒充
            if not re.search(r"(未|不|零|不冒充|不得|禁).{0,6}(close|closed|resolved)", sm):
                bad.append(f"{pid} status 字面疑似冒充 closed: {sm[:60]!r}")
    return bad


def digest_equal(a: str, b: str) -> bool:
    return isinstance(a, str) and isinstance(b, str) and a.strip().lower() == b.strip().lower()


def flip_classify(registered_blocked: bool, live_present: bool) -> str:
    """翻转分类器:登记阻塞态 × 活体实测在机性 → 四分支出。"""
    if registered_blocked and not live_present:
        return "maintain"           # 阻塞维持(登记与活体一致)
    if registered_blocked and live_present:
        return "signal_flip"        # 阻塞→解锁翻转 = 锚命中重判信号(合法绿)
    if live_present:
        return "maintain_present"   # 在机面维持
    return "dev_env_change"         # 登记在机面消失 = 环境变化(三态如实)


def validate_probes_doc(doc: dict) -> list[str]:
    """登记件结构核验:12 探针 + 必填字段 + verdict 闭集 + 零冒充 + 锚不变 + 计数重算。"""
    fails: list[str] = []
    probes = doc.get("probes")
    if not isinstance(probes, list):
        return ["probes 字段缺失或非数组"]
    ids = [p.get("probe_id") for p in probes]
    if ids != EXPECTED_PROBE_IDS:
        fails.append(f"探针 id 序列不等 {EXPECTED_PROBE_IDS}: {ids}")
    for p in probes:
        pid = p.get("probe_id", "?")
        for field in ("todo_ref", "subject", "anchor_literal", "method", "commands",
                      "results", "verdict", "status_maintained_literal", "anchor_unchanged"):
            if field not in p:
                fails.append(f"{pid} 缺字段 {field}")
        if not verdict_in_closed_set(p.get("verdict")):
            fails.append(f"{pid} verdict 越闭集: {p.get('verdict')!r}")
        if p.get("anchor_unchanged") is not True:
            fails.append(f"{pid} anchor_unchanged ≠ true")
        if not p.get("commands"):
            fails.append(f"{pid} commands 空(探针命令必须真实执行留痕)")
    fails.extend(zero_masquerade_scan(probes))
    summary = doc.get("summary", {})
    if summary.get("probes_total") != 12:
        fails.append(f"summary.probes_total ≠ 12: {summary.get('probes_total')}")
    if summary.get("closed_or_resolved") != 0:
        fails.append(f"summary.closed_or_resolved ≠ 0: {summary.get('closed_or_resolved')}")
    if summary.get("zero_masquerade") is not True:
        fails.append("summary.zero_masquerade ≠ true")
    if summary.get("anchors_all_unchanged") is not True:
        fails.append("summary.anchors_all_unchanged ≠ true")
    om = sum(1 for p in probes if p.get("verdict") == "open-maintained")
    bde = sum(1 for p in probes if p.get("verdict") == "blocked-dev-env")
    if summary.get("open_maintained") != om or summary.get("blocked_dev_env") != bde:
        fails.append(f"summary 计数重算不一致: 登记 {summary.get('open_maintained')}/{summary.get('blocked_dev_env')} vs 重算 {om}/{bde}")
    return fails


def walk_grep(roots: tuple[str, ...], pattern: str) -> list[str]:
    """常量 pattern 表 grep(g30 检索面逐字沿用;扩展名过滤 + 噪声目录剪枝)。"""
    rx = re.compile(pattern)
    hits: list[str] = []
    for root in roots:
        rp = ROOT / root
        if not rp.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(rp):
            dirnames[:] = [d for d in dirnames if d not in WALK_SKIP_DIRS]
            for fn in filenames:
                if Path(fn).suffix.lower() not in GREP_FILE_EXTS:
                    continue
                fp = Path(dirpath) / fn
                try:
                    text = fp.read_text(encoding="utf-8", errors="ignore")
                except OSError:
                    continue
                if rx.search(text):
                    hits.append(str(fp.relative_to(ROOT)).replace("\\", "/"))
    return hits


# ---------------------------------------------------------------------------
# 活体复核腿(host 恒跑)
# ---------------------------------------------------------------------------

def leg_tools_bistro() -> dict:
    """P01:三工具 PATH + BistroExterior 三检索根。"""
    tools = {t: shutil.which(t) is not None for t in ("fbx2gltf", "FBX2glTF", "assimp", "blender")}
    leg = {"leg": "tools_bistro", "state": "ok", "tools": tools, "flips": []}
    for t, present in tools.items():
        cls = flip_classify(True, present)
        if cls == "signal_flip":
            leg["flips"].append(f"工具 {t} 出现于 PATH——#11 替代臂命中信号")
    hits: list[str] = []
    root_states: dict[str, str] = {}
    for root in ("K:/rurix-ext", "external", "assets"):
        rp = Path(root) if root.startswith("K:") else ROOT / root
        if not rp.is_dir():
            root_states[root] = "root_missing_or_unreachable"
            continue
        root_states[root] = "ok"
        r = run_cmd(["where", "/r", str(rp), "*BistroExterior*"], timeout=180)
        if r.returncode == 0 and r.stdout.strip():
            hits.extend(x for x in r.stdout.splitlines() if x.strip())
    leg["root_states"] = root_states
    leg["bistro_exterior_hits"] = len(hits)
    if hits:
        leg["flips"].append(f"BistroExterior 源资产命中 {len(hits)} 件——#11 源资产半命中信号")
    return leg


def leg_vulkan() -> dict:
    """P03/P04/P07:vulkaninfo 五 token + 设备枚举。"""
    exe = shutil.which("vulkaninfo")
    if exe is None:
        return {"leg": "vulkan", "state": "skipped_dev_env", "reason": "vulkaninfo 不在 PATH"}
    WORK.mkdir(parents=True, exist_ok=True)
    log_path = WORK / "vulkaninfo_gate.log"
    with log_path.open("w", encoding="utf-8", errors="ignore") as fh:
        r = subprocess.run([exe], stdout=fh, stderr=subprocess.STDOUT, timeout=180)
    if r.returncode != 0:
        return {"leg": "vulkan", "state": "skipped_dev_env", "reason": f"vulkaninfo 退出 {r.returncode}"}
    text = log_path.read_text(encoding="utf-8", errors="ignore")
    leg: dict = {"leg": "vulkan", "state": "ok", "flips": []}
    for tok in HDR_TOKENS:
        present = tok in text
        leg[tok] = present
        if flip_classify(True, present) == "signal_flip":
            leg["flips"].append(f"HDR token {tok} 出现——#17 显示链变化信号")
    wg = WG_TOKEN in text
    leg[WG_TOKEN] = wg
    if flip_classify(True, wg) == "signal_flip":
        leg["flips"].append("VK_AMDX_shader_enqueue 出现——#40 WG present 翻转信号(复评启动)")
    for tok in DGC_TOKENS:
        present = tok in text
        leg[tok] = present
        if not present:
            leg["flips"].append(f"DGC token {tok} 消失——#40 DGC 互核面变化信号")
    amd = "0x1002" in text
    intel = "0x8086" in text.lower()
    leg["amd_device_present"] = amd
    leg["intel_device_present"] = intel
    if amd:
        leg["flips"].append("AMD 设备出现——#18 G-MB1-6 硬件获得信号")
    if intel:
        leg["flips"].append("Intel 设备出现——C3 兼容矩阵新厂商信号")
    m = re.search(r"deviceName\s*=\s*(.+)", text)
    leg["device_name"] = m.group(1).strip() if m else "unknown"
    return leg


def leg_vm() -> dict:
    """P05:Hyper-V/VMware 面。"""
    leg: dict = {"leg": "vm", "state": "ok", "flips": []}
    r_vmms = run_cmd(["sc", "query", "vmms"], timeout=30)
    leg["hyperv_vmms_present"] = r_vmms.returncode == 0
    store = Path("C:/ProgramData/Microsoft/Windows/Hyper-V/Virtual Machines")
    leg["hyperv_vm_store_items"] = len(list(store.iterdir())) if store.is_dir() else 0
    ws = Path("C:/Program Files (x86)/VMware/VMware Workstation")
    leg["vmware_workstation_installed"] = ws.is_dir()
    vmx = Path("F:/Windows 11 x64.vmx")
    leg["registered_win11_vmx_on_disk"] = vmx.is_file()
    if leg["hyperv_vmms_present"]:
        leg["flips"].append("Hyper-V vmms 出现——#19 Hyper-V 面变化信号")
    if not leg["registered_win11_vmx_on_disk"]:
        leg["flips"].append("登记在案 Win11 VM 候选 F:/Windows 11 x64.vmx 不在盘——候选失效如实登记")
    return leg


def leg_omm() -> dict:
    """P06:bistro 派生 gltf 材质计数。"""
    if not BISTRO_GLTF.is_file():
        return {"leg": "omm", "state": "skipped_dev_env", "reason": f"bistro gltf 缺失 {BISTRO_GLTF}"}
    text = BISTRO_GLTF.read_text(encoding="utf-8", errors="ignore")
    leg = {
        "leg": "omm", "state": "ok",
        "alphaMode_OPAQUE": len(re.findall(r'"alphaMode"\s*:\s*"OPAQUE"', text)),
        "alphaMode_BLEND": len(re.findall(r'"alphaMode"\s*:\s*"BLEND"', text)),
        "alphaMode_MASK": len(re.findall(r'"alphaMode"\s*:\s*"MASK"', text)),
        "KHR_materials_transmission": len(re.findall(r"KHR_materials_transmission", text)),
        "flips": [],
    }
    if leg["alphaMode_BLEND"] or leg["alphaMode_MASK"] or leg["KHR_materials_transmission"]:
        leg["flips"].append("压测闭集出现半透明/透射面——#30 OMM 锚命中信号")
    if leg["alphaMode_OPAQUE"] != 70:
        leg["flips"].append(f"OPAQUE 计数 {leg['alphaMode_OPAQUE']} ≠ 在案 70——闭集资产面变化信号")
    return leg


def leg_patterns() -> dict:
    """P10:物理观察轨 22 pattern。"""
    leg: dict = {"leg": "patterns", "state": "ok", "grep_hits": {}, "glob_hits": {}, "flips": []}
    for who, pattern, roots in PATTERN_GREPS:
        hits = walk_grep(roots, pattern)
        key = f"{who}:{pattern}"
        if who == "m125" and pattern.startswith("rurix_physics_sys56"):
            net = [h for h in hits if h not in M125_BASELINE]
            leg["grep_hits"][key] = {"raw": len(hits), "net": len(net)}
            if net:
                leg["flips"].append(f"M125 类① 净命中 {len(net)} 件——5.6 独有 API 引用信号: {net[:3]}")
        else:
            leg["grep_hits"][key] = len(hits)
            if hits:
                leg["flips"].append(f"{who} pattern {pattern!r} 命中 {len(hits)}——reeval_anchor 命中信号: {hits[:3]}")
    for who, glob_pat in PATTERN_GLOBS:
        root_name = glob_pat.split("/")[0]
        rp = ROOT / root_name
        found = sorted(str(p.relative_to(ROOT)).replace("\\", "/") for p in rp.glob(glob_pat[len(root_name) + 1:])) if rp.is_dir() else []
        leg["glob_hits"][f"{who}:{glob_pat}"] = len(found)
        if found:
            leg["flips"].append(f"{who} glob {glob_pat} 命中 {len(found)}——reeval_anchor 命中信号: {found[:3]}")
    for d in M127_DIRS:
        if (ROOT / d).is_dir():
            leg["flips"].append(f"M127 corpus 目录 {d} 出现——corpus 半命中信号")
    return leg


def leg_safegpu() -> dict:
    """P11:SAFE-GPU 期面 + docs/ 三面。"""
    leg: dict = {"leg": "safegpu", "state": "ok", "flips": []}
    text = G31_CONTRACT.read_text(encoding="utf-8", errors="ignore") if G31_CONTRACT.is_file() else ""
    leg["g31_contract_safe_gpu_hits"] = len(re.findall(r"safe_gpu|SAFE-GPU|safe-gpu", text))
    if leg["g31_contract_safe_gpu_hits"]:
        leg["flips"].append("G31_CONTRACT 现 safe_gpu 字面——独立期立项面变化信号")
    docs = ROOT / "docs"
    leg["docs_root"] = docs.is_dir()
    for pat in ("docs/**/*platform*demand*", "docs/**/*safe*gpu*"):
        found = list(docs.glob(pat[len("docs/"):])) if docs.is_dir() else []
        leg[f"glob:{pat}"] = len(found)
        if found:
            leg["flips"].append(f"平台需求方文档面 {pat} 命中——SAFE-GPU 需求信号")
    if docs.is_dir():
        hits = walk_grep(("docs",), "平台需求方|外部采纳生态")
        leg["docs_token_hits"] = len(hits)
        if hits:
            leg["flips"].append(f"docs/ 平台需求方 token 命中 {len(hits)}——SAFE-GPU 需求信号")
    return leg


def leg_legacy() -> dict:
    """P12:legacy 十一条清册零 close 机核(引用不复制)。

    在案实态:12 行 = 11 RD 条(「十一条」字面)+ SAFE-GPU 承接池行(非 RD 条目);
    summary「maintain-open 9」与实际重算 10 的在案计数出入 = 既往数据质量事件
    (0-byte 不回写,G30 重复行注记同律)——机核不变量 = 行数/RD 数/disposition
    闭集/「零 close」字面;计数出入注记不作翻转信号。"""
    if not LEGACY_REGISTRY.is_file():
        return {"leg": "legacy", "state": "skipped_dev_env", "reason": f"清册缺失 {LEGACY_REGISTRY}"}
    doc = json.loads(LEGACY_REGISTRY.read_text(encoding="utf-8"))
    entries = doc.get("entries", [])
    dispositions = [e.get("disposition") for e in entries]
    allowed = {"maintain-open", "maintain-inherited", "defer-to-G25+"}
    rd_rows = [e for e in entries if str(e.get("id", "")).startswith("RD-")]
    leg: dict = {
        "leg": "legacy", "state": "ok",
        "entries": len(entries),
        "rd_entries": len(rd_rows),
        "dispositions_in_closed_set": all(d in allowed for d in dispositions),
        "zero_close_literal": "零 close" in doc.get("summary", ""),
        "preexisting_summary_count_note": True,
        "flips": [],
    }
    if (len(entries) != 12 or len(rd_rows) != 11
            or not leg["dispositions_in_closed_set"] or not leg["zero_close_literal"]):
        leg["flips"].append("legacy 清册行数/disposition 闭集/零 close 字面变化——清册面变化信号")
    return leg


def leg_toolchain() -> dict:
    """P09 本地面:spirv-cross/dxc/glslang 版本。"""
    sdk = os.environ.get("VULKAN_SDK", "")
    search = []
    if sdk:
        search.append(str(Path(sdk) / "Bin"))
    leg: dict = {"leg": "toolchain", "state": "ok", "versions": {}, "flips": []}
    expected = {
        "spirv-cross": "vulkan-sdk-1.3.290.0-44-g65d73934",
        "dxc": "1.8.0.4739",
        "glslangValidator": "11:15.0.0",
    }
    missing = []
    for tool, want in expected.items():
        exe = None
        for base in search:
            cand = Path(base) / (tool + ".exe")
            if cand.is_file():
                exe = str(cand)
                break
        if exe is None:
            exe = shutil.which(tool)
        if exe is None:
            missing.append(tool)
            continue
        r = run_cmd([exe, "--version"], timeout=60)
        out = (r.stdout or "") + (r.stderr or "")
        leg["versions"][tool] = {"found": True, "contains_registered": want in out}
        if want not in out:
            leg["flips"].append(f"{tool} 版本面漂移(登记 {want} 不在输出)——RD-014 供应链面变化信号")
    if missing:
        leg["state"] = "skipped_dev_env"
        leg["reason"] = f"工具缺失 {missing}"
    return leg


def leg_meshrt() -> dict:
    """P08:RD-034 blocked 探针真跑(退出码判定)。"""
    r = run_cmd([sys.executable, str(ROOT / "ci" / "meshrt_probe_smoke.py")], timeout=600, cwd=ROOT)
    out = (r.stdout or "") + (r.stderr or "")
    leg: dict = {"leg": "meshrt_probe", "state": "ok", "exit": r.returncode, "flips": []}
    if "SKIP" in out:
        leg["state"] = "skipped_dev_env"
        leg["reason"] = "meshrt_probe 三态 SKIP(glslang/spirv-cross/dxc 缺)"
        return leg
    leg["step68_green"] = "步骤 68 PASS" in out
    leg["step69_blocked_fresh"] = "步骤 69 PASS" in out
    if r.returncode != 0 or not leg["step68_green"] or not leg["step69_blocked_fresh"]:
        # 步骤 69 意外成功 = 探针翻红(上游解锁信号);其余非零 = 探针执行异常诚实红
        if "意外" in out or ("步骤 68 PASS" in out and "步骤 69" in out and r.returncode != 0):
            leg["flips"].append(f"RD-034 探针 exit 语义反转(rc={r.returncode})——上游解锁复评信号")
        else:
            leg["state"] = "error"
            leg["reason"] = f"meshrt_probe 非零退出 {r.returncode}: {out.strip()[-300:]}"
    return leg


def leg_rd045_faces() -> dict:
    """P02 host 面:三件快检(①件落点 + ③件 rfcs 文件名闭集)。"""
    leg: dict = {"leg": "rd045_faces", "state": "ok", "flips": []}
    leg["piece1_root_cause_file_absent"] = not (ROOT / "evidence" / "g26_rd045_root_cause_confirmation.json").is_file()
    if not leg["piece1_root_cause_file_absent"]:
        leg["flips"].append("①件根因确证记录出现——RD-045 三件面变化信号(重判窗)")
    rfcs = ROOT / "rfcs"
    names = [p.name.lower() for p in rfcs.iterdir()] if rfcs.is_dir() else []
    topic = [n for n in names if ("rd045" in n or "digest-drift" in n or "determinism-defect" in n)]
    leg["piece3_rfc_filename_hits"] = len(topic)
    if topic:
        leg["flips"].append(f"③件主题 RFC 文件名命中 {topic}——RD-045 三件面变化信号")
    return leg


def leg_device_rd045_digest() -> dict:
    """P02 device 腿(三态):orbit 64+10 新鲜 digest 对波 B 锚。"""
    exe = ROOT / "target" / "release" / ("g31_window_present.exe" if os.name == "nt" else "g31_window_present")
    reasons: list[str] = []
    if not exe.is_file():
        reasons.append("g31_window_present 产物缺失(target/release)")
    missing_spv = [f for f in SPV_FILES if not (SPV_DIR / f).is_file()]
    if missing_spv:
        reasons.append(f"SPV 缺失 {missing_spv}")
    if not BISTRO_GLTF.is_file():
        reasons.append("bistro gltf 缺失")
    if reasons:
        return {"leg": "rd045_digest", "state": "skipped_dev_env", "reason": "; ".join(reasons)}
    WORK.mkdir(parents=True, exist_ok=True)
    ev = WORK / "rd045_gate_spot_orbit_64p10.json"
    env = dict(os.environ)
    env["RURIX_VK_VALIDATION"] = "1"
    with gpu_device_lock(purpose="g31 blocked probes RD-045 digest 腿"):
        r = subprocess.run(
            [str(exe), "--frames", "64", "--warmup", "10", "--hidden",
             "--auto-move", "orbit", "--evidence", str(ev)],
            capture_output=True, text=True, timeout=1800, env=env, cwd=str(ROOT))
    out = (r.stdout or "") + (r.stderr or "")
    if '"state":"skipped_dev_env"' in out:
        return {"leg": "rd045_digest", "state": "skipped_dev_env", "reason": "harness skipped_dev_env"}
    leg: dict = {"leg": "rd045_digest", "state": "ok", "exit": r.returncode, "flips": []}
    if r.returncode != 0 or not ev.is_file():
        leg["state"] = "error"
        leg["reason"] = f"harness 非零退出 {r.returncode}: {out.strip()[-300:]}"
        return leg
    doc = json.loads(ev.read_text(encoding="utf-8"))
    digest = doc.get("digest", "")
    leg["digest"] = digest
    leg["digest_match_anchor"] = digest_equal(digest, RD045_ANCHOR_DIGEST)
    leg["frames_completed"] = doc.get("frames_completed")
    leg["validation_silent"] = "Validation Error" not in out and "VUID-" not in out
    if not leg["digest_match_anchor"]:
        leg["state"] = "error"
        leg["reason"] = f"digest 漂移事件: {digest} ≠ 波 B 锚 {RD045_ANCHOR_DIGEST}(诚实红——漂移复现促重判)"
    return leg


# ---------------------------------------------------------------------------
# selftest
# ---------------------------------------------------------------------------

REQUIRED_KEYS = [
    "schema", "subject", "symbolic_gate_key", "wave", "registration",
    "live_rechecks", "device_leg", "signals", "environment", "timestamp", "notes",
]


def run_selftest() -> int:
    fails: list[str] = []
    # ① verdict 闭集校验器红绿臂。
    for v in VERDICT_CLOSED_SET:
        if not verdict_in_closed_set(v):
            fails.append(f"verdict 闭集绿臂误判红: {v}")
    for v in ("closed", "resolved", "open", "maintain", "close", "", None, 0):
        if verdict_in_closed_set(v):
            fails.append(f"verdict 闭集红臂漏检: {v!r}")
    # ② 零冒充扫描红绿臂。
    green_probes = [
        {"probe_id": "P01", "verdict": "open-maintained", "status_maintained_literal": "maintain-open 不冒充 close"},
        {"probe_id": "P02", "verdict": "blocked-dev-env", "status_maintained_literal": "维持双场景闭集"},
    ]
    if zero_masquerade_scan(green_probes):
        fails.append("零冒充扫描绿臂误判红")
    red_probes = [
        {"probe_id": "P99", "verdict": "closed", "status_maintained_literal": "x"},
        {"probe_id": "P98", "verdict": "resolved", "status_maintained_literal": "x"},
        {"probe_id": "P97", "verdict": "open-maintained", "status_maintained_literal": "已 closed 收口"},
    ]
    bad = zero_masquerade_scan(red_probes)
    if len(bad) != 3:
        fails.append(f"零冒充扫描红臂漏检: 期望 3 违例实得 {len(bad)} {bad}")
    # ③ digest 比较器两臂。
    if not digest_equal(RD045_ANCHOR_DIGEST, RD045_ANCHOR_DIGEST.upper().replace("SHA256:", "sha256:")):
        fails.append("digest 比较器绿臂误判红")
    if digest_equal(RD045_ANCHOR_DIGEST, "sha256:" + "0" * 64):
        fails.append("digest 比较器红臂漏检")
    # ④ 翻转分类器四象限。
    quad = {(True, False): "maintain", (True, True): "signal_flip",
            (False, True): "maintain_present", (False, False): "dev_env_change"}
    for (rb, lp), want in quad.items():
        got = flip_classify(rb, lp)
        if got != want:
            fails.append(f"翻转分类器象限 ({rb},{lp}) 期望 {want} 实得 {got}")
    # ⑤ 登记件校验器红绿臂(合成夹具,不依赖树上件)。
    good_probe = {f: True for f in ()}
    good_probe = {
        "probe_id": "P01", "todo_ref": "#11", "subject": "s", "anchor_literal": "a",
        "method": "m", "commands": ["c"], "results": {}, "verdict": "open-maintained",
        "status_maintained_literal": "maintain", "anchor_unchanged": True,
    }
    good_doc = {
        "probes": [dict(good_probe, probe_id=f"P{i:02d}") for i in range(1, 13)],
        "summary": {"probes_total": 12, "closed_or_resolved": 0, "zero_masquerade": True,
                    "anchors_all_unchanged": True, "open_maintained": 12, "blocked_dev_env": 0},
    }
    if validate_probes_doc(good_doc):
        fails.append(f"登记件校验器绿臂误判红: {validate_probes_doc(good_doc)}")
    bad_doc = json.loads(json.dumps(good_doc))
    bad_doc["probes"][3]["verdict"] = "closed"
    if not validate_probes_doc(bad_doc):
        fails.append("登记件校验器红臂(verdict=closed)漏检")
    bad_doc2 = json.loads(json.dumps(good_doc))
    bad_doc2["probes"] = bad_doc2["probes"][:11]
    if not validate_probes_doc(bad_doc2):
        fails.append("登记件校验器红臂(11 探针)漏检")
    bad_doc3 = json.loads(json.dumps(good_doc))
    bad_doc3["probes"][0]["anchor_unchanged"] = False
    if not validate_probes_doc(bad_doc3):
        fails.append("登记件校验器红臂(anchor_unchanged=false)漏检")
    bad_doc4 = json.loads(json.dumps(good_doc))
    bad_doc4["summary"]["open_maintained"] = 9
    if not validate_probes_doc(bad_doc4):
        fails.append("登记件校验器红臂(summary 计数不符)漏检")
    # ⑥ schema 文件互核:required 闭集 == REQUIRED_KEYS。
    if not SCHEMA_PATH.is_file():
        fails.append(f"schema 文件缺失 {SCHEMA_PATH}")
    else:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        if set(schema.get("required", [])) != set(REQUIRED_KEYS):
            fails.append(f"schema required 与校验键集不等 {set(schema.get('required', [])) ^ set(REQUIRED_KEYS)}")
    if fails:
        for m in fails:
            print(f"[{TAG}] selftest FAIL: {m}", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS (verdict 闭集 10 臂 + 零冒充 5 臂 + digest 2 臂 + 翻转 4 象限 + 登记件校验 5 臂 + schema 互核)")
    return 0


# ---------------------------------------------------------------------------
# gate
# ---------------------------------------------------------------------------

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--skip-device", action="store_true", help="跳过 device 腿(登记 skipped_dev_env)")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    # ① schema 在树 + required 闭集互核。
    check(SCHEMA_PATH.is_file(), f"schema 文件缺失: {SCHEMA_PATH}")
    if SCHEMA_PATH.is_file():
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        check(set(schema.get("required", [])) == set(REQUIRED_KEYS),
              f"schema required 与校验键集不等: {set(schema.get('required', [])) ^ set(REQUIRED_KEYS)}")

    # ② 登记件机器核验(12 探针/verdict 闭集/零冒充/锚不变/计数重算)。
    check(PROBES_PATH.is_file(), f"探针登记件缺失: {PROBES_PATH}")
    probes_doc: dict = {}
    if PROBES_PATH.is_file():
        try:
            probes_doc = json.loads(PROBES_PATH.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            check(False, f"探针登记件不可解析: {e}")
        else:
            for m in validate_probes_doc(probes_doc):
                check(False, f"登记件核验: {m}")

    # ③ 活体复核(host 恒跑腿)。
    legs: list[dict] = []
    if not FAILURES:
        for leg_fn in (leg_tools_bistro, leg_vulkan, leg_vm, leg_omm, leg_patterns,
                       leg_safegpu, leg_legacy, leg_toolchain, leg_meshrt, leg_rd045_faces):
            try:
                leg = leg_fn()
            except Exception as e:  # 探针执行异常 = 程序未诚实执行面,诚实红
                leg = {"leg": leg_fn.__name__, "state": "error", "reason": f"探针执行异常: {e}"}
            legs.append(leg)
            for f in leg.get("flips", []):
                signal(f)
            if leg.get("state") == "error":
                check(False, f"活体腿 {leg.get('leg')} 执行异常: {leg.get('reason')}")
            note(f"腿 {leg.get('leg')}: state={leg.get('state')} flips={len(leg.get('flips', []))}")

    # ④ device 腿(三态):RD-045 digest 抽查。
    device_leg: dict
    if args.skip_device:
        device_leg = {"leg": "rd045_digest", "state": "skipped_dev_env", "reason": "--skip-device 显式跳过"}
    elif FAILURES:
        device_leg = {"leg": "rd045_digest", "state": "skipped_dev_env", "reason": "前序失败未执行"}
    else:
        try:
            device_leg = leg_device_rd045_digest()
        except Exception as e:
            device_leg = {"leg": "rd045_digest", "state": "error", "reason": f"device 腿执行异常: {e}"}
    for f in device_leg.get("flips", []):
        signal(f)
    note(f"腿 rd045_digest: state={device_leg.get('state')}")

    # 三态归并:dev-env SKIP 如实登记退 0;RURIX_REQUIRE_REAL=1 翻硬 FAIL。
    skipped = [l for l in legs + [device_leg] if l.get("state") == "skipped_dev_env"]
    if device_leg.get("state") == "error":
        check(False, f"device 腿: {device_leg.get('reason')}")
    if skipped and require_real():
        for l in skipped:
            check(False, f"REQUIRE_REAL: 腿 {l.get('leg')} dev-env SKIP 翻硬 FAIL: {l.get('reason')}")

    # ⑤ 汇总与落盘(PASS-only;FAIL 诊断件留 .tmp 不污染路由面)。
    legs_ok = [l for l in legs if l.get("state") == "ok"]
    verdict_doc = {
        "schema": SCHEMA_ID,
        "subject": TAG,
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31+.C",
        "registration": {
            "probes_path": "milestones/g31/g31_blocked_probes_2026.json",
            "probes_total": len(probes_doc.get("probes", [])),
            "verdicts_in_closed_set": all(verdict_in_closed_set(p.get("verdict")) for p in probes_doc.get("probes", [])),
            "zero_masquerade": not zero_masquerade_scan(probes_doc.get("probes", [])),
            "anchors_all_unchanged": all(p.get("anchor_unchanged") is True for p in probes_doc.get("probes", [])),
            "closed_or_resolved": 0,
        },
        "live_rechecks": {
            "legs_run": len(legs_ok),
            "legs_skipped_dev_env": len([l for l in legs if l.get("state") == "skipped_dev_env"]),
            "flips_registered": len(SIGNALS),
            "legs": [{k: v for k, v in l.items() if k != "flips"} for l in legs],
        },
        "device_leg": {k: v for k, v in device_leg.items() if k != "flips"},
        "signals": SIGNALS,
        "environment": {
            "os": f"{os.name}-{sys.platform}",
            "python_version": sys.version.split()[0],
            "host": "RTX 4070 Ti + Vulkan",
        },
        "timestamp": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "notes": "G31+ 波 C Task C17:12 探针 verdict ∈ {open-maintained, blocked-dev-env} 闭集机核 + 零冒充(无一项标 closed/resolved)+ 活体复核翻转 = 锚命中信号合法绿(F10 门态映射);门 FAIL 只留程序未诚实执行",
    }
    if FAILURES:
        WORK.mkdir(parents=True, exist_ok=True)
        diag = WORK / f"g31_blocked_probes_FAIL_{datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}.json"
        diag.write_text(json.dumps({"failures": FAILURES, "signals": SIGNALS,
                                    "verdict_doc": verdict_doc}, ensure_ascii=False, indent=2), encoding="utf-8")
        print(f"[{TAG}] FAIL gate={GATE_KEY}({len(FAILURES)} 违例;诊断件 {diag})", file=sys.stderr)
        for m in FAILURES:
            print(f"  - {m}", file=sys.stderr)
        return 1
    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    ev_path = ROOT / "evidence" / f"g31_blocked_probes_{ts}.json"
    ev_path.write_text(json.dumps(verdict_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] PASS gate={GATE_KEY}(登记 12 探针闭集零冒充 + 活体 {len(legs_ok)} 腿 + device {device_leg.get('state')} + signals {len(SIGNALS)};evidence {ev_path})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
