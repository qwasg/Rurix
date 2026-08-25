#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G30.2 P0 M-a smoke）
"""G30.2 P0 smoke — g30.p0.m_a.tail_anchor_rejudgment_closure（步骤 512）。

六件尾锚 + RD-042/043/044 三条的 G30 收官窗重判聚合（法定判据 = RFC-0047 §1 全节
+ G30_CONTRACT §4.2 M-a 行；全机器取证零冒充）：

* 九组 pattern 常量表脚本字面承载（F6 三件：禁运行时构造 + 逐 pattern 为锚字面
  派生关键词 + evidence 逐件载 {pattern 表, 检索根, 逐 pattern 命中数}）。锚字面
  = g25 registry `g26_anchor` 逐行原文（F8 锚源钉死；RD-044 展开 =
  g23_rd044_subitem_registry 三分项 reeval_anchor 字面），selftest 与 --gate 均
  与锚源文件机核比对。检索面沿上游单件重判 smoke 只追加禁缩面（g23_jolt_56 /
  g23_neural_deform / g24_hair_strand_oit / g24_hdr_probe / g24_bistro_exterior /
  g24_safe_gpu 源常量逐字复用）。
* 外部盘（K:/rurix-ext/assets）与 vulkaninfo 不可达走 SKIP 如实登记 + 在案态
  兜底（F15/RFC-0046 §4.2 同律），不 FAIL、不冒充命中；维持 / 命中重判启动均
  合法门绿（F10 分支捕获非透传），门 FAIL 只保留给程序未诚实执行（含 sys56
  cargo check 评估臂可编译性硬前提失败如实红）。
* --gate 真跑：evidence 落档 + RD 三条零命中时 registry/deferred.json 对应条目
  history 数组尾部只追加 G30 行（g29 RD-041 先例位置同律；四不可变字段 0-byte
  由 check_deferred_append_only vs G30.0 不可变 ref 机核）；同 event 字面已在案
  则幂等跳过不再追加（F9），在案 G23.3 重复行如实登记为既往数据质量事件不回写
  清理。--selftest 结构自检零副作用（不触 deferred.json）；--verify-latest 读
  最新 evidence 判绿。argparse 形制镜像 g29 P0 smoke（--gate 兼容裸开关与
  ACCEPTANCE_MAP 字面 `--gate g30.p0.m_a.tail_anchor_rejudgment_closure`）。
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from g30_interlock_check import G30_0_IMMUTABLE_REF, check_deferred_append_only, _git_show_file  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g30.p0.m_a.tail_anchor_rejudgment_closure"
NUMERIC_STEP = 512  # post-interlock actual-next-free 顺位领取（治理三门 509~511 后）
SUBJECT = "g30_m_a_tail_anchor_rejudgment_closure"
WAVE = "G30.2"
SCHEMA_PATH = ROOT / "milestones/g30/g30_m_a_tail_anchor_rejudgment_closure_evidence_schema.json"
SOURCE_REF = ("G30_CONTRACT §4.2 M-a;RFC-0047 §1;"
              "milestones/g25/g25_campaign_handover_registry.json（g26_anchor 锚源）;"
              "milestones/g23/g23_rd044_subitem_registry.json（RD-044 三分项锚源）")

DEFERRED = ROOT / "registry/deferred.json"
G25_REGISTRY = ROOT / "milestones/g25/g25_campaign_handover_registry.json"
G23_RD044_REGISTRY = ROOT / "milestones/g23/g23_rd044_subitem_registry.json"
G30_CONTRACT = ROOT / "milestones/g30/G30_CONTRACT.md"
LOG_DIR = ROOT / ".tmp/g30_ma"

# ---------------------------------------------------------------------------
# 锚字面闭集（F8 锚源钉死 = g25 registry g26_anchor / g23_rd044 reeval_anchor
# 逐行原文；selftest 与 --gate 均与锚源文件机核比对，漂移即红）。
# ---------------------------------------------------------------------------
ANCHORS = {
    "M125-adopt3": "需求证据三类任一命中（5.6 独有 API 引用/5.3 缺陷命中/A/B 超带）",
    "M127": "corpus + PhysicsAsset residual 消费方出现（搜索面闭集只追加扩面）",
    "M114-strand": "毛发资产入压测闭集",
    "M118-hdr-cal": "显示链变化 + HDR 资产需求成立",
    "G10-N6": "FBX2glTF 上游修复在树或替代臂+源资产同窗齐备",
    "SAFE-GPU": "独立期资源窗 + 平台需求方（外部采纳生态）出现时立项评估",
    "RD-042": "可微仿真需求场景出现（G23 四轨 maintain-observe 落档）",
    "RD-043": "GPU 刚体 out_of_scope 翻转程序 + wgrapier 成熟度证据",
    "RD-044": "三分项 reeval_anchor（G23 闭集落档）",
}
RD044_SUBANCHORS = {
    "JOLT-SOFT": "软体/布料需求场景入 workload（角色布料/破坏效果需求证据）",
    "TAICHI-MPM": "体积模拟资产需求成立（MPM 烘焙序列消费方出现）",
    "RAPIER-FAST": "Rapier 上游快路径重构发布 + 真实 workload 采用证据",
}

# 上游单件重判 smoke 源常量逐字复用（检索面只追加禁缩面）。
CORPUS_CANDIDATES = ("corpus", "assets/corpus", "assets/neural", "conformance/neural")  # g23_neural_deform 逐字
HDR_TOKENS = (  # milestones/g24/harness/g24_hdr_probe.py HDR_TOKENS 逐字
    "VK_COLOR_SPACE_HDR10_ST2084_EXT",
    "VK_COLOR_SPACE_BT2020_LINEAR_EXT",
    "VK_COLOR_SPACE_HDR10_HLG_EXT",
)
BISTRO_ROOTS = ("K:/rurix-ext", "assets", "external")  # g24_bistro_exterior_recheck.json search_roots 字面
HAIR_EXT_ROOT = "K:/rurix-ext/assets"  # RFC-0047 §1.3 hair 资产面（M114 追加检索根）
M125_CRATE = "rurix-physics-sys56"  # g23_jolt_56 源 SYS56 crate 路径 src/rurix-physics-sys56
# 类① 在案基线排除清单：G9.6 M125 A/B 评估臂接线四件（types.rs:18「评估用途，
# 不升格生产默认」字面在案）——基线外净命中才构成「5.6 独有 API 生产引用」证据。
M125_API56_BASELINE = (
    "src/rurix-physics/src/world.rs",
    "src/rurix-physics/src/types.rs",
    "src/rurix-physics/src/ab_eval.rs",
    "src/rurix-physics/src/bin/g9_m125_jolt56_ab.rs",
)

# ---------------------------------------------------------------------------
# 九组 pattern 常量表（F6：脚本字面承载禁运行时构造；行 = (kind, pattern,
# 检索根闭集, 锚派生关键词旁注)；kind ∈ grep=git grep -l -i -E 内容检索 /
# glob=树内 ROOT 相对 glob / extglob=外部绝对根 glob（根不可达 → SKIP——F15）/
# multiglob=多根 glob（K: 根不可达如实登记）/ dir=目录存在性 / which=PATH 实测 /
# vk_token=vulkaninfo 新鲜探针 token / contract_window=合同字面核验）。
# ---------------------------------------------------------------------------
PATTERNS: dict[str, tuple[tuple[str, str, tuple[str, ...], str], ...]] = {
    "M125-adopt3": (
        ("grep", "rurix_physics_sys56|JPC56_|JPH56", ("src/rurix-physics/src", "apps"),
         "API56/类① 5.6 独有 API 生产引用（锚「5.6 独有 API 引用」派生；G9.6 评估臂四件基线排除）"),
        ("glob", "apps/**/*jolt56*", ("apps",),
         "API56/类① 5.6 面 workload 文件名（锚「5.6 独有 API 引用」派生）"),
        ("grep", "jolt_53_defect|jolt53_workaround", ("src/rurix-physics/src", "apps"),
         "DEFECT53/类② 5.3 缺陷 workaround 代码登记（锚「5.3 缺陷命中」派生）"),
        ("glob", "evidence/*jolt_53_defect*", ("evidence",),
         "DEFECT53/类② 5.3 缺陷取证档（锚「5.3 缺陷命中」派生）"),
        ("glob", "evidence/*jolt_56_ab_overband*", ("evidence",),
         "OVERBAND/类③ A/B 超带 measured 档（锚「A/B 超带」派生）"),
        ("grep", "ab_overband", ("src/rurix-physics/src", "apps"),
         "OVERBAND/类③ 超带登记面（锚「A/B 超带」派生）"),
    ),
    "M127": (
        ("dir", "corpus", ("corpus",), "corpus 半/离线语料目录（锚「corpus」派生；g23 搜索面闭集第 1 项）"),
        ("dir", "assets/corpus", ("assets/corpus",), "corpus 半（g23 搜索面闭集第 2 项）"),
        ("dir", "assets/neural", ("assets/neural",), "corpus 半（g23 搜索面闭集第 3 项）"),
        ("dir", "conformance/neural", ("conformance/neural",), "corpus 半（g23 搜索面闭集第 4 项）"),
        ("grep", "neural_deform", ("src", "apps"),
         "消费方半/PhysicsAsset residual 消费方 token（锚「PhysicsAsset residual 消费方出现」派生；g23 逐字）"),
    ),
    "M114-strand": (
        ("grep", "strand", ("milestones/g13/g13_ue_upscale_parity_contract.json",
                            "milestones/g18/g18_presentation_contract.json"),
         "压测闭集契约毛发资产 token（锚「毛发资产入压测闭集」派生；g24 检索面沿用禁缩面）"),
        ("extglob", "**/*hair*", (HAIR_EXT_ROOT,),
         "毛发资产外部盘面（锚「毛发资产入压测闭集」派生；RFC-0047 §1.3 追加面；根不可达 SKIP——F15）"),
    ),
    "M118-hdr-cal": (
        ("vk_token", HDR_TOKENS[0], ("vulkaninfo",), "HDR10 ST2084 表面色彩空间（锚「显示链变化」派生；g24 probe 逐字）"),
        ("vk_token", HDR_TOKENS[1], ("vulkaninfo",), "BT2020 LINEAR 表面色彩空间（锚「显示链变化」派生；g24 probe 逐字）"),
        ("vk_token", HDR_TOKENS[2], ("vulkaninfo",), "HDR10 HLG 表面色彩空间（锚「显示链变化」派生；g24 probe 逐字）"),
    ),
    "G10-N6": (
        ("which", "fbx2gltf|FBX2glTF", (), "FBX2glTF 工具 PATH 实测（锚「FBX2glTF 上游修复在树」派生；g24 逐字含大写变体）"),
        ("which", "assimp", (), "assimp 替代臂 PATH 实测（锚「替代臂」派生；g24 逐字）"),
        ("which", "blender", (), "blender 替代臂 PATH 实测（锚「替代臂」派生；g24 逐字）"),
        ("multiglob", "**/*BistroExterior*", BISTRO_ROOTS,
         "BistroExterior 独立源资产（锚「源资产同窗齐备」派生；g24 recheck search_roots 字面三根）"),
    ),
    "SAFE-GPU": (
        ("contract_window", "商用终审收官期", ("milestones/g30/G30_CONTRACT.md",),
         "独立期资源窗核验（锚「独立期资源窗」判据面：收官期字面在案 = 战役收官期无专属资源，判据字面直接不成立如实登记——RFC-0047 §1.6）"),
        ("glob", "docs/**/*platform*demand*", ("docs",), "平台需求方文档面（锚「平台需求方」派生）"),
        ("glob", "docs/**/*safe*gpu*", ("docs",), "SAFE-GPU 需求文档面（锚「平台需求方（外部采纳生态）」派生）"),
        ("grep", "平台需求方|外部采纳生态", ("docs",),
         "平台需求方内容 token（锚字面派生；检索根限 docs/ 排除 registry/milestones/rfcs 登记面自命中）"),
    ),
    "RD-042": (
        ("grep", "differentiable", ("src", "apps"), "可微仿真消费面 token（锚「可微仿真需求场景出现」派生）"),
        ("grep", "autodiff", ("src", "apps"), "自动微分消费面 token（锚「可微仿真需求场景出现」派生）"),
    ),
    "RD-043": (
        ("grep", "wgrapier", ("src", "apps"), "wgrapier 代码消费面（锚「wgrapier 成熟度证据」派生；登记面 registry/rfcs 不在根内）"),
        ("glob", "rfcs/*wgrapier*", ("rfcs",), "wgrapier 立项 RFC 面（锚「翻转程序」派生）"),
        ("glob", "rfcs/*gpu*rigid*", ("rfcs",), "GPU 刚体翻转 RFC 面（锚「GPU 刚体 out_of_scope 翻转程序」派生）"),
    ),
    "RD-044": (
        ("grep", "character_cloth|destructible", ("src/rurix-physics/src", "apps"),
         "JOLT-SOFT/角色布料+破坏效果需求 token（分项锚「角色布料/破坏效果需求证据」派生；vendor 与在案 cloth 基建面外）"),
        ("glob", "apps/**/*cloth*", ("apps",),
         "JOLT-SOFT/布料 workload 文件面（分项锚「软体/布料需求场景入 workload」派生）"),
        ("grep", "mpm_bake", ("src", "apps"),
         "TAICHI-MPM/MPM 烘焙序列消费方 token（分项锚「MPM 烘焙序列消费方出现」派生）"),
        ("glob", "assets/**/*mpm*", ("assets",),
         "TAICHI-MPM/体积模拟资产面（分项锚「体积模拟资产需求成立」派生）"),
        ("grep", "rapier_fast_path|rapier_fastpath", ("src", "apps"),
         "RAPIER-FAST/快路径重构消费面 token（分项锚「上游快路径重构发布」派生）"),
        ("glob", "evidence/*rapier_fastpath_adoption*", ("evidence",),
         "RAPIER-FAST/workload 采用证据档（分项锚「真实 workload 采用证据」派生）"),
    ),
}
M125_CLASS_SLICES = {"api56_unique_api_ref": (0, 2), "defect53_hit": (2, 4), "ab_overband": (4, 6)}

# RD 三条维持 open 时的 deferred history 追加 event 字面常量（幂等键 = event 全文
# 相等——F9；date/evidence 槽外的字面确定，不含运行时间戳）。
G30_HISTORY_EVENTS = {
    "RD-042": ("G30.2 M-a 尾锚窗同批重判登记（agent 完全自主 D-406 v3.0；RFC-0047 §1.7；锚源钉死 = g25 handover "
               "rd_eight g26_anchor 字面）：「可微仿真需求场景出现（G23 四轨 maintain-observe 落档）」锚 2 pattern "
               "树内检索（differentiable/autodiff @ src/+apps/，常量 pattern 表 + 锚派生映射——F6）零命中 ⇒ 维持 "
               "open，backfill_condition 0-byte 不回写；在案 G23.3 重复行如实登记为既往数据质量事件不回写清理"),
    "RD-043": ("G30.2 M-a 尾锚窗同批重判登记（agent 完全自主 D-406 v3.0；RFC-0047 §1.7；锚源钉死 = g25 handover "
               "rd_eight g26_anchor 字面）：「GPU 刚体 out_of_scope 翻转程序 + wgrapier 成熟度证据」锚 3 pattern "
               "检索（wgrapier @ src/+apps/ + rfcs/*wgrapier*/*gpu*rigid* 立项面，常量 pattern 表 + 锚派生映射——F6）"
               "零命中 ⇒ 维持 open，backfill_condition 0-byte 不回写；在案 G23.3 重复行如实登记为既往数据质量事件"
               "不回写清理"),
    "RD-044": ("G30.2 M-a 尾锚窗同批重判登记（agent 完全自主 D-406 v3.0；RFC-0047 §1.7；检索面显式展开 = "
               "milestones/g23/g23_rd044_subitem_registry.json 三分项 reeval_anchor 字面——F8）：JOLT-SOFT"
               "（character_cloth|destructible + apps/*cloth* 面）/ TAICHI-MPM（mpm_bake + assets/*mpm* 面）/ "
               "RAPIER-FAST（rapier_fastpath + adoption 证据档）六 pattern（常量 pattern 表 + 锚派生映射——F6）"
               "零命中 ⇒ 维持 open，backfill_condition 0-byte 不回写；在案 G23.3 重复行如实登记为既往数据质量事件"
               "不回写清理"),
}
G30_HISTORY_EVIDENCE = ("evidence/g30_m_a_tail_anchor_rejudgment_closure_*.json + "
                        "milestones/g25/g25_campaign_handover_registry.json + "
                        "milestones/g23/g23_rd044_subitem_registry.json")


# ---------------------------------------------------------------------------
# 检索原语（manifest 行 = {kind, pattern, search_roots, anchor_keyword, hits,
# files, state}；state ∈ ok / skipped_root_unreachable——F15 三态承载）。
# ---------------------------------------------------------------------------

def _git_grep(pattern: str, roots: tuple[str, ...]) -> list[str]:
    live = [r for r in roots if (ROOT / r).exists()]
    if not live:
        return []
    r = subprocess.run(["git", "grep", "-l", "-i", "-E", pattern, "--", *live],
                       cwd=ROOT, capture_output=True, text=True)
    return [ln.strip().replace("\\", "/") for ln in (r.stdout or "").splitlines() if ln.strip()]


def _glob_hits(pattern: str) -> list[str]:
    try:
        return [str(p.relative_to(ROOT)).replace("\\", "/") for p in sorted(ROOT.glob(pattern))]
    except (OSError, ValueError):
        return []


def _ext_glob(root_str: str, pattern: str) -> tuple[list[str], str]:
    base = Path(root_str)
    if not base.is_dir():
        return [], "skipped_root_unreachable"
    try:
        return [str(p) for p in sorted(base.glob(pattern))], "ok"
    except OSError:
        return [], "skipped_root_unreachable"


def _run_vulkaninfo() -> tuple[str, bool]:
    """g24_hdr_probe 逐字判式：返回 (全量输出, tool_available)。"""
    try:
        r = subprocess.run(["vulkaninfo"], capture_output=True, text=True, timeout=300)
        out = (r.stdout or "") + (r.stderr or "")
        tool_available = r.returncode == 0 or bool(out.strip())
    except (OSError, subprocess.TimeoutExpired) as e:
        out = f"<vulkaninfo 不可得: {e}>"
        tool_available = False
    return out, tool_available


def run_group_manifest(group: str, vk_out: str | None, vk_tool_available: bool) -> list[dict]:
    """执行一组 pattern，产出 manifest（F6：逐 pattern {检索根, 命中数} 入档）。"""
    manifest: list[dict] = []
    for kind, pat, roots, anchor_kw in PATTERNS[group]:
        row: dict = {"kind": kind, "pattern": pat, "search_roots": list(roots),
                     "anchor_keyword": anchor_kw, "state": "ok", "hits": 0, "files": []}
        if kind == "grep":
            files = _git_grep(pat, roots)
            if group == "M125-adopt3" and "API56" in anchor_kw:
                raw = list(files)
                files = [f for f in files if f not in M125_API56_BASELINE]
                row["raw_hits"] = len(raw)
                row["baseline_excluded"] = [f for f in raw if f in M125_API56_BASELINE]
            row["hits"], row["files"] = len(files), files
        elif kind == "glob":
            files = _glob_hits(pat)
            row["hits"], row["files"] = len(files), files
        elif kind == "extglob":
            files, state = _ext_glob(roots[0], pat)
            row["hits"], row["files"], row["state"] = len(files), files[:20], state
        elif kind == "multiglob":
            files: list[str] = []
            root_states: dict[str, str] = {}
            for r_ in roots:
                base = Path(r_) if ":" in r_ else ROOT / r_
                if not base.is_dir():
                    root_states[r_] = "root_missing_or_unreachable"
                    continue
                try:
                    files += [str(p) for p in sorted(base.glob(pat))]
                    root_states[r_] = "ok"
                except OSError:
                    root_states[r_] = "root_missing_or_unreachable"
            row["hits"], row["files"], row["root_states"] = len(files), files[:20], root_states
        elif kind == "dir":
            row["hits"] = 1 if (ROOT / pat).is_dir() else 0
        elif kind == "which":
            row["hits"] = 1 if any(shutil.which(t) for t in pat.split("|")) else 0
        elif kind == "vk_token":
            if not vk_tool_available:
                row["state"] = "skipped_root_unreachable"
            else:
                found = bool([ln for ln in (vk_out or "").splitlines() if pat in ln])  # g24 probe 逐字判式
                row["hits"] = 1 if found else 0
        elif kind == "contract_window":
            text = G30_CONTRACT.read_text(encoding="utf-8") if G30_CONTRACT.is_file() else ""
            row["hits"] = text.count(pat)
        manifest.append(row)
    return manifest


def _in_scope_has_safe_gpu() -> bool:
    text = G30_CONTRACT.read_text(encoding="utf-8") if G30_CONTRACT.is_file() else ""
    items: list[str] = []
    in_block = False
    for line in text.splitlines():
        if line.strip() == "in_scope:":
            in_block = True
            continue
        if in_block:
            if line.startswith("  - "):
                items.append(line.strip()[2:].strip())
            else:
                break
    return any("safe_gpu" in it.lower() for it in items)


def _anchor_source_findings() -> list[str]:
    """F8 锚源钉死机核：ANCHORS/RD044_SUBANCHORS 与锚源文件字面全等。"""
    findings: list[str] = []
    g25 = wel.load_json(G25_REGISTRY) if G25_REGISTRY.is_file() else {}
    by_id = {r.get("id"): r.get("g26_anchor") for r in g25.get("campaign_period_rows", [])}
    by_id.update({r.get("id"): r.get("g26_anchor") for r in g25.get("rd_eight", [])})
    for aid, lit in ANCHORS.items():
        if by_id.get(aid) != lit:
            findings.append(f"{aid} 锚字面漂移（g25 registry={by_id.get(aid)!r}）")
    g23 = wel.load_json(G23_RD044_REGISTRY) if G23_RD044_REGISTRY.is_file() else {}
    sub = {s.get("id"): s.get("reeval_anchor") for s in g23.get("subitems", [])}
    for sid, lit in RD044_SUBANCHORS.items():
        if sub.get(sid) != lit:
            findings.append(f"RD-044/{sid} 分项锚字面漂移（g23 registry={sub.get(sid)!r}）")
    return findings


# ---------------------------------------------------------------------------
# deferred history 尾部只追加（g29 RD-041 先例位置同律）+ 幂等（F9）。
# ---------------------------------------------------------------------------

def append_deferred_g30_rows(rd_maintained: list[str]) -> tuple[list[dict], list[str]]:
    facts: list[dict] = []
    doc = json.loads(DEFERRED.read_text(encoding="utf-8"))
    by_id = {e.get("id"): e for e in doc.get("entries", [])}
    today = _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%d")
    changed = False
    dup_notes: list[str] = []
    for rid in ("RD-042", "RD-043", "RD-044"):
        entry = by_id.get(rid)
        if entry is None:
            facts.append({"id": f"deferred_{rid.lower().replace('-', '_')}_g30_row", "status": "FAIL",
                          "detail": f"{rid} 条目缺失，无法追加"})
            continue
        hist = entry.setdefault("history", [])
        events = [h.get("event") for h in hist]
        dups = len(events) - len(set(events))
        if dups:
            dup_notes.append(f"{rid} 在案重复行 {dups} 对")
        event = G30_HISTORY_EVENTS[rid]
        if rid not in rd_maintained:
            facts.append({"id": f"deferred_{rid.lower().replace('-', '_')}_g30_row", "status": "PASS",
                          "detail": f"{rid} 本窗命中重判启动分支——维持行不追加（重判程序另行；F10 门绿）"})
            continue
        if event in events:
            facts.append({"id": f"deferred_{rid.lower().replace('-', '_')}_g30_row", "status": "PASS",
                          "detail": f"{rid} 同 event 字面已在案——幂等跳过不再追加（F9）"})
            continue
        hist.append({"date": today, "event": event, "evidence": G30_HISTORY_EVIDENCE})
        changed = True
        facts.append({"id": f"deferred_{rid.lower().replace('-', '_')}_g30_row", "status": "PASS",
                      "detail": f"{rid} history 尾部追加 G30.2 维持 open 行（g29 RD-041 先例位置同律）"})
    if changed:
        DEFERRED.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
                            encoding="utf-8", newline="\n")
    facts.append({"id": "rd_preexisting_duplicate_history_rows", "status": "PASS",
                  "detail": ("在案历史重复行如实登记为既往数据质量事件，不回写清理（F9）：" + "；".join(dup_notes))
                  if dup_notes else "在案历史无重复行"})
    return facts, dup_notes


# ---------------------------------------------------------------------------
# 主评估（--gate 专用：含 cargo check / vulkaninfo / 外部盘遍历真跑面）。
# ---------------------------------------------------------------------------

def evaluate() -> tuple[list[dict], list[str]]:
    facts: list[dict] = []
    rd_maintained: list[str] = []

    anchor_findings = _anchor_source_findings()
    facts.append({"id": "anchor_source_pinned", "status": "PASS" if not anchor_findings else "FAIL",
                  "detail": "九组锚字面 = g25 registry g26_anchor / g23_rd044 reeval_anchor 逐行原文机核比对（F8）"
                  if not anchor_findings else str(anchor_findings)})

    LOG_DIR.mkdir(parents=True, exist_ok=True)
    vk_out, vk_tool = _run_vulkaninfo()
    (LOG_DIR / "vulkaninfo.log").write_text(vk_out, encoding="utf-8", newline="\n")

    manifests = {g: run_group_manifest(g, vk_out, vk_tool) for g in PATTERNS}

    # --- 1. M125-adopt3 ---
    m = manifests["M125-adopt3"]
    cls_empty = {c: all(r["hits"] == 0 for r in m[a:b]) for c, (a, b) in M125_CLASS_SLICES.items()}
    facts.append({"id": "m125_demand_three_classes_manifest", "status": "PASS",
                  "detail": ("三类需求证据逐类独立 manifest（api56_unique_api_ref/defect53_hit/ab_overband 空判 = "
                             f"{cls_empty}；类① G9.6 评估臂基线四件排除后净命中）"),
                  "anchor_literal": ANCHORS["M125-adopt3"], "manifest": m})
    try:
        r = subprocess.run(["cargo", "check", "-p", M125_CRATE], cwd=ROOT,
                           capture_output=True, text=True, timeout=600)
        cargo_ok, cargo_detail = r.returncode == 0, f"rc={r.returncode}"
    except subprocess.TimeoutExpired:
        cargo_ok, cargo_detail = False, "timeout=600s"
    except OSError as e:
        cargo_ok, cargo_detail = False, f"cargo 不可得: {e}"
    facts.append({"id": "m125_sys56_cargo_check_fresh", "status": "PASS" if cargo_ok else "FAIL",
                  "detail": f"cargo check -p {M125_CRATE} {cargo_detail}（评估臂可编译性硬前提，失败如实红；g23 源 crate 路径）"})
    p9 = wel.load_latest_evidence("g9_m125_jolt_56_ab_evaluation")
    doc9 = wel.load_json(p9) if p9 else {}
    ab_ok = str(doc9.get("status", "")).upper() == "PASS" or doc9.get("host_section_pass") is True
    facts.append({"id": "m125_g9_ab_evidence_readonly_green", "status": "PASS" if ab_ok else "FAIL",
                  "detail": f"g9_m125 A/B latest 只读盘点存在且绿（{p9.name if p9 else 'missing'}；禁 --gate 重跑——F17）"})
    m125_all_empty = all(cls_empty.values())
    facts.append({"id": "m125_branch_verdict", "status": "PASS",
                  "detail": ("三类全空 ⇒ 维持 maintain-5.3（在案三件条件 1/3 不变）" if m125_all_empty
                             else f"需求证据命中 {[c for c, e in cls_empty.items() if not e]} ⇒ 命中重判启动（合法门绿分支，F10）")})

    # --- 2. M127 ---
    m = manifests["M127"]
    corpus_hits = [r["pattern"] for r in m if r["kind"] == "dir" and r["hits"]]
    consumers = next(r for r in m if r["kind"] == "grep")["files"]
    facts.append({"id": "m127_two_halves_manifest", "status": "PASS",
                  "detail": (f"corpus 四目录存在性 = {corpus_hits or 'NONE'}（搜索面闭集 {list(CORPUS_CANDIDATES)}）+ "
                             f"neural_deform 消费方（src/+apps/）= {consumers or 'NONE'}——g23 检索面逐字沿用禁缩面"),
                  "anchor_literal": ANCHORS["M127"], "manifest": m})
    m127_miss = not corpus_hits and not consumers
    facts.append({"id": "m127_branch_verdict", "status": "PASS",
                  "detail": ("两半未命中 ⇒ 维持研究子轨（无主线门口径 0-byte）" if m127_miss
                             else "任一半命中 ⇒ 命中重判启动（合法门绿分支，F10）")})

    # --- 3. M114-strand（三态闭集——F15） ---
    m = manifests["M114-strand"]
    strand_files = m[0]["files"]
    hair_row = m[1]
    facts.append({"id": "m114_manifest_three_state", "status": "PASS",
                  "detail": (f"契约 strand token = {strand_files or 'NONE'}（g24 面沿用）+ 外部盘 hair 面 "
                             f"state={hair_row['state']} hits={hair_row['hits']}（根 {HAIR_EXT_ROOT}；"
                             "不可达 SKIP 如实登记 + 在案态兜底，不 FAIL 不冒充命中——F15）"),
                  "anchor_literal": ANCHORS["M114-strand"], "manifest": m})
    m114_hit = bool(strand_files) or hair_row["hits"] > 0
    facts.append({"id": "m114_branch_verdict", "status": "PASS",
                  "detail": ("命中 ⇒ 命中重判启动（合法门绿分支，F10）" if m114_hit
                             else ("检索根不可达 SKIP ⇒ 维持 card/mesh（在案态兜底）"
                                   if hair_row["state"] != "ok" else "未命中 ⇒ 维持 card/mesh"))})

    # --- 4. M118-hdr-cal（三态 absent/present/SKIP） ---
    m = manifests["M118-hdr-cal"]
    if not vk_tool:
        m118_state = "SKIP"
    else:
        m118_state = "present" if any(r["hits"] for r in m) else "absent"
    facts.append({"id": "m118_fresh_probe_manifest", "status": "PASS",
                  "detail": (f"vulkaninfo 新鲜探针三 token（g24 probe 常量逐字）state={m118_state}"
                             f"（tool_available={vk_tool}；全量 log 存档 .tmp/g30_ma/vulkaninfo.log；"
                             "工具缺 SKIP 如实登记 + 在案态兜底——RFC-0046 §4.2 同律）"),
                  "anchor_literal": ANCHORS["M118-hdr-cal"], "manifest": m})
    facts.append({"id": "m118_branch_verdict", "status": "PASS",
                  "detail": ("present ⇒ 命中重判启动（合法门绿分支，F10）" if m118_state == "present"
                             else f"{m118_state} ⇒ 维持 maintain-SDR（{'工具缺在案态兜底' if m118_state == 'SKIP' else '三 token 全 absent'}）")})

    # --- 5. G10-N6 ---
    m = manifests["G10-N6"]
    tools_now = {r["pattern"]: bool(r["hits"]) for r in m if r["kind"] == "which"}
    asset_row = next(r for r in m if r["kind"] == "multiglob")
    facts.append({"id": "g10n6_manifest", "status": "PASS",
                  "detail": (f"三工具 PATH 实测 = {tools_now}（g24 逐字含 FBX2glTF 变体）+ BistroExterior 源资产 "
                             f"hits={asset_row['hits']}（roots {list(BISTRO_ROOTS)} 逐根态 {asset_row['root_states']}；"
                             "K: 根不可达如实登记不冒充命中）"),
                  "anchor_literal": ANCHORS["G10-N6"], "manifest": m})
    g10_ready = all(tools_now.values()) and asset_row["hits"] > 0
    facts.append({"id": "g10n6_branch_verdict", "status": "PASS",
                  "detail": ("工具+源资产同窗齐备 ⇒ 命中重判启动（合法门绿分支，F10）" if g10_ready
                             else "任一缺 ⇒ 维持双场景闭集（BistroInterior + CornellBox 兜底字面 0-byte）")})

    # --- 6. SAFE-GPU ---
    m = manifests["SAFE-GPU"]
    closeout_literal_on_file = m[0]["hits"] > 0
    window_absent = closeout_literal_on_file and not _in_scope_has_safe_gpu()
    docs_hits = sum(r["hits"] for r in m[1:])
    facts.append({"id": "safegpu_manifest", "status": "PASS",
                  "detail": (f"独立期资源窗核验：G30_CONTRACT「商用终审收官期」字面在案={closeout_literal_on_file} 且 "
                             f"in_scope 无 safe_gpu 立项 ⇒ 资源窗不成立={window_absent}（收官期无专属资源 = 判据字面"
                             f"直接不成立，如实登记——RFC-0047 §1.6）；平台需求方文档检索（docs/ 面）hits={docs_hits}"),
                  "anchor_literal": ANCHORS["SAFE-GPU"], "manifest": m})
    safegpu_hit = (not window_absent) and docs_hits > 0
    facts.append({"id": "safegpu_branch_verdict", "status": "PASS",
                  "detail": ("资源窗+需求方同现 ⇒ 命中立项评估启动（合法门绿分支，F10）" if safegpu_hit
                             else "未出现 ⇒ 维持 defer，归档行改锚 defer-to-G31+（RFC-0047 §1.6 字面登记）")})

    # --- 7. RD-042/043/044 同批逐锚（F8 锚源钉死 + ≥2 pattern） ---
    for rid in ("RD-042", "RD-043", "RD-044"):
        m = manifests[rid]
        hit_rows = [r["pattern"] for r in m if r["hits"]]
        anchor_detail = ANCHORS[rid] if rid != "RD-044" else f"{ANCHORS[rid]}；三分项展开 = {RD044_SUBANCHORS}"
        facts.append({"id": f"{rid.lower().replace('-', '_')}_manifest", "status": "PASS",
                      "detail": (f"{rid} 锚 {len(m)} pattern 检索（常量表 + 锚派生映射）命中 = {hit_rows or '零命中'}"),
                      "anchor_literal": anchor_detail, "manifest": m})
        if hit_rows:
            facts.append({"id": f"{rid.lower().replace('-', '_')}_branch_verdict", "status": "PASS",
                          "detail": f"{rid} 命中 ⇒ 命中重判启动（合法门绿分支，F10）；维持行不追加"})
        else:
            rd_maintained.append(rid)
            facts.append({"id": f"{rid.lower().replace('-', '_')}_branch_verdict", "status": "PASS",
                          "detail": f"{rid} 零命中 ⇒ 维持 open + deferred history 只追加 G30 行（四不可变字段 0-byte）"})

    # --- 8. 纪律 facts（F6/F10/manifest 必填） ---
    facts.append({"id": "f6_pattern_faithfulness_three_pieces", "status": "PASS",
                  "detail": ("F6 三件全承接：PATTERNS 脚本字面常量表（禁运行时构造）+ 逐 pattern 锚字面派生关键词旁注"
                             "（pattern↔锚映射表 = 各 manifest anchor_keyword/anchor_literal 字段落档）+ evidence 逐件载 "
                             "{pattern 表, 检索根, 逐 pattern 命中数}")})
    facts.append({"id": "f10_gate_state_mapping", "status": "PASS",
                  "detail": ("F10 门态映射：维持 / 命中重判启动均合法门绿（分支捕获非透传，各分支终态字面在 detail "
                             "如实登记）；门 FAIL 只保留给程序未诚实执行")})
    all_manifest_filled = all(len(manifests[g]) >= 1 for g in PATTERNS)
    facts.append({"id": "searched_paths_manifest_mandatory", "status": "PASS" if all_manifest_filled else "FAIL",
                  "detail": f"九组 searched-paths manifest 必填（{ {g: len(manifests[g]) for g in PATTERNS} }）"})
    return facts, rd_maintained


def run_gate() -> int:
    facts, rd_maintained = evaluate()
    append_facts, _ = append_deferred_g30_rows(rd_maintained)
    facts.extend(append_facts)
    base_text = _git_show_file(ROOT, G30_0_IMMUTABLE_REF, "registry/deferred.json")
    base_doc = json.loads(base_text) if base_text else None
    cur_doc = json.loads(DEFERRED.read_text(encoding="utf-8")) if DEFERRED.is_file() else None
    findings = check_deferred_append_only(base_doc, cur_doc)
    facts.append({"id": "deferred_append_only_mechanized", "status": "PASS" if findings == [] else "FAIL",
                  "detail": "append-only 机核（vs G30.0 ref；四不可变字段 0-byte + history 前缀不变）"
                  + ("" if not findings else f"；违例 {findings[:2]}")})
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes=("G30.2 M-a：六件尾锚 + RD-042/043/044 同批 G30 收官窗重判（九组常量 pattern 表 + 锚派生映射；"
               "维持/SKIP 兜底/命中启动分支字面逐件在案；deferred G30 行尾部只追加幂等——F9）"),
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def run_selftest() -> int:
    """结构自检（零副作用：不触 deferred.json、不跑探针/cargo/外部盘）。"""
    assert set(PATTERNS) == set(ANCHORS) == {
        "M125-adopt3", "M127", "M114-strand", "M118-hdr-cal", "G10-N6", "SAFE-GPU",
        "RD-042", "RD-043", "RD-044"}, "九组闭集漂移"
    assert len(PATTERNS["RD-042"]) >= 2 and len(PATTERNS["RD-043"]) >= 2, "RD 锚 pattern 数 <2"
    for tag in ("JOLT-SOFT", "TAICHI-MPM", "RAPIER-FAST"):
        n = sum(1 for _, _, _, kw in PATTERNS["RD-044"] if kw.startswith(f"{tag}/"))
        assert n >= 2, f"RD-044/{tag} 分项 pattern 数 {n} <2"
    for cls, (a, b) in M125_CLASS_SLICES.items():
        assert b - a >= 2, f"M125/{cls} 类 manifest pattern 数 <2"
    assert HDR_TOKENS == ("VK_COLOR_SPACE_HDR10_ST2084_EXT", "VK_COLOR_SPACE_BT2020_LINEAR_EXT",
                          "VK_COLOR_SPACE_HDR10_HLG_EXT"), "HDR token 与 g24 probe 源漂移"
    assert CORPUS_CANDIDATES == ("corpus", "assets/corpus", "assets/neural", "conformance/neural"), \
        "M127 搜索面闭集与 g23 源漂移"
    assert SCHEMA_PATH.is_file(), f"schema 缺失 {SCHEMA_PATH}"
    findings = _anchor_source_findings()
    assert findings == [], f"锚源钉死机核红（F8）：{findings}"
    assert set(G30_HISTORY_EVENTS) == {"RD-042", "RD-043", "RD-044"}
    for rid, ev in G30_HISTORY_EVENTS.items():
        assert "维持 open" in ev and "G30.2" in ev, f"{rid} event 字面缺维持/窗位标识"
    print(f"[{SUBJECT}] SELFTEST PASS")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", nargs="?", const=GATE_KEY, choices=[GATE_KEY], default=None)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    if args.gate:
        return run_gate()
    ap.print_usage()
    return 2


if __name__ == "__main__":
    sys.exit(main())
