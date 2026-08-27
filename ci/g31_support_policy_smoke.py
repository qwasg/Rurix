#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C8 支持渠道与版本政策文档化）
"""G31+ 波 C Task C8：支持渠道与版本政策门冒烟（g31.waveC.support；
G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #55「支持渠道与版本政策：issue 流程、
LTS/release 节奏、安全响应（SECURITY.md 已有语言面）」兑现面）。

判据闭集（milestones/g31/g31_support_policy_evidence_schema.json 描述段逐字）：
1. docs_present_with_anchors：docs/renderer/ 两文档（support_policy /
   release_checklist）在树 + 关键节锚（标题逐字）+ 关键在案字面全在场（防
   文档腐化——字面全引自既有机器面，禁新造）。
2. referenced_ci_scripts_exist：两文档引用的全部 ci/*.py 脚本名（机器提取
   `ci/[A-Za-z0-9_]+\\.py` token 闭集）逐一真实在树——引用不存在的脚本即红。
3. referenced_surfaces_exist：文档引用的关键仓内面（API_VERSIONING.md /
   SECURITY.md / 10_GOVERNANCE.md / AGENTS.md / sdk.rx / lib.rs /
   stable_api.snapshot / bless_log.md / channels/stable.json /
   registry/deferred.json / g31_compatibility_matrix.json /
   vendor_upscale_license_clearance.md / CI_GATES.md / 姊妹文档四件 /
   ci/stable_snapshot.py / src/rurix-basis-sys）真实在树。
4. version_policy_matches_stable_snapshot：版本政策与 stable 快照面一致——
   ①ci/stable_snapshot.py 含 renderer_sdk_api 段（collect_renderer_sdk_api）
   ②src/rurix-renderer-sdk/src/lib.rs 的 ABI_VERSION_PACKED 程序读 = 1.0.0
   （镜像 stable_snapshot.py ABI_VERSION_RE 同一正则）
   ③tests/stable/stable_api.snapshot renderer_sdk_api 段 abi_version=1.0.0
   且 export_count=9 ④API_VERSIONING.md 含 1.0.0 / 0x00010000
   ⑤support_policy.md 版本字面同一（1.0.0 + ABI_VERSION_PACKED +
   renderer_sdk_api）——五面同一字面，漂移即红。
5. security_response_mirrored：support_policy.md §3 镜像语言面 SECURITY.md
   结构（渠道 25890346@qq.com / Report a vulnerability / 3 个工作日 /
   协调公开四要素逐字在场）+ 渲染器特有面（驱动交互 / shader 供应链 /
   vendor SDK 逐字）+ SECURITY.md 渲染器增补段锚 + SECURITY.en.md 双件
   增补段锚（中英双件 parity）。
6. pending_items_honestly_marked：待建立/在飞登记在场——C5 分发打包 /
   C6 许可矩阵 / C7 profiler 面 / AMD·Intel 格 G-MB1-6 锚 / 商业 SLA 未建立
   （support_policy.md §5 + release_checklist.md 对应标注逐字）。
7. frozen_docs_untouched：00~14 号根冻结规划文档（15 件）git status 面零改动
   （任务纪律「不触碰 00-14 冻结文档」机器核对）。

全门 = host 恒跑面（文档/政策核验无 GPU/工具链腿）——缺件即硬 FAIL，不设
DEV_ENV_DEGRADE 降级（无真跑面可降级）。

evidence 纪律：PASS 才落 evidence/g31_support_policy_<ts>.json（check_schemas
前缀路由 g31_support_policy_，与既有 g31_* 全族及 gpu fallthrough 互不包含）；
FAIL 诊断件落 .tmp/g31_gates/support_policy/ 工作区不污染 evidence/ 路由面
（fail-closed：evidence/ 无件 = 门未过）。

用法：
  py -3 ci/g31_support_policy_smoke.py --selftest
  py -3 ci/g31_support_policy_smoke.py --gate g31.waveC.support
"""
from __future__ import annotations

import argparse
import datetime as _dt
import io
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

GATE_KEY = "g31.waveC.support"
SUBJECT = "g31_support_policy"
WAVE = "G31+.C"
TAG = "g31_support_policy"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_support_policy_evidence_schema.json"
SCHEMA_ID = "rurix.g31.support_policy_evidence.v1"
POLICY_PATH = ROOT / "docs" / "renderer" / "support_policy.md"
CHECKLIST_PATH = ROOT / "docs" / "renderer" / "release_checklist.md"
SECURITY_MD = ROOT / "SECURITY.md"
SECURITY_EN = ROOT / "SECURITY.en.md"
STABLE_SNAPSHOT_PY = ROOT / "ci" / "stable_snapshot.py"
SDK_LIB_RS = ROOT / "src" / "rurix-renderer-sdk" / "src" / "lib.rs"
SNAPSHOT_PATH = ROOT / "tests" / "stable" / "stable_api.snapshot"
API_VERSIONING_MD = ROOT / "apps" / "g31-renderer-sdk" / "API_VERSIONING.md"
WORK = ROOT / ".tmp" / "g31_gates" / "support_policy"

# 镜像 ci/stable_snapshot.py ABI_VERSION_RE 同一正则（程序读 ABI_VERSION_PACKED）。
ABI_VERSION_RE = re.compile(
    r"ABI_VERSION_PACKED:\s*u32\s*=\s*\(\s*(\d+)\s*<<\s*16\s*\)\s*\|\s*"
    r"\(\s*(\d+)\s*<<\s*8\s*\)\s*\|\s*(\d+)"
)
CI_SCRIPT_RE = re.compile(r"ci/[A-Za-z0-9_]+\.py\b")

EXPECTED_ABI_VERSION = "1.0.0"
EXPECTED_EXPORT_COUNT = 9

# 文档判据面：关键节锚（标题逐字）+ 关键在案字面（全引自既有机器面，禁新造）。
DOC_SPECS = {
    "support_policy": {
        "path": POLICY_PATH,
        "headings": [
            "# Rurix 渲染器支持政策（issue / 版本 / 安全响应 / 兼容承诺）",
            "## 1. 缺陷报告流程",
            "### 1.1 报告要素（缺陷模板）",
            "### 1.2 分类（四面闭集）",
            "### 1.3 响应口径（诚实面）",
            "## 2. 版本政策",
            "### 2.1 SDK 语义化版本（事实源 = API_VERSIONING.md）",
            "### 2.2 release 节奏（里程碑期联动）",
            "### 2.3 LTS / 修复线政策",
            "## 3. 安全响应",
            "### 3.1 报告渠道（镜像 SECURITY.md）",
            "### 3.2 渲染器特有面（驱动交互 / shader 供应链 / vendor SDK）",
            "### 3.3 处理时间线与披露",
            "## 4. 兼容承诺",
            "### 4.1 stable ABI 守卫（stable 快照 renderer_sdk_api 段）",
            "### 4.2 破坏性变更走 RFC 纪律",
            "## 5. 待建立项（诚实登记，不冒充）",
            "## 修订记录",
        ],
        "numbers": [
            "1.0.0", "0x00010000", "ABI_VERSION_PACKED", "renderer_sdk_api",
            "rurix.g31.capability_report.v1", "last_frame_digest", "stats_post_warmup",
            "RURIX_REQUIRE_REAL", "RURIX_VK_VALIDATION", "25890346@qq.com",
            "3 个工作日", "bistro-interior_t100_tsr_device", "RURIX_BLESS=1",
            "G-MB1-6", "RXS-0255", "v1.0.1-dist", "Streamline 2.10.3",
            "FidelityFX SDK 2.0.0", "g31.waveC.support",
        ],
    },
    "release_checklist": {
        "path": CHECKLIST_PATH,
        "headings": [
            "# Rurix 渲染器发布核对清单（机器门操作单）",
            "## 1. stable ABI 守卫",
            "## 2. 渲染器面 gate 套件（波 A/B/C 全绿）",
            "## 3. 签名 / SBOM / 分发链",
            "## 4. 许可 / 再分发面",
            "## 5. 兼容矩阵",
            "## 6. soak / 健壮性",
            "## 7. 文档与政策面",
            "## 8. 环境纪律（三态）",
            "## 修订记录",
        ],
        "numbers": [
            "g31.waveC.support", "g31.waveC.sdk", "g31.waveC.docs",
            "g31.waveC.capability", "g31.waveC.robustness", "g31.waveC.ngx_decomp",
            "RURIX_REQUIRE_REAL=1", "RURIX_VK_VALIDATION=1", "RURIX_BLESS=1",
            "v1.0.1-dist", "nvidia-ada-rtx4070ti", "dev_env_degrade", "G-MB1-6",
            "RXS-0218", "1.0.0", "capability_matrix",
        ],
    },
}

# fact 3：文档引用的关键仓内面闭集（repo 相对路径，逐一在树核验）。
REFERENCED_SURFACES = [
    "10_GOVERNANCE.md",
    "agents/AGENTS.md",
    "SECURITY.md",
    "apps/g31-renderer-sdk/API_VERSIONING.md",
    "apps/g31-renderer-sdk/src/sdk.rx",
    "src/rurix-renderer-sdk/src/lib.rs",
    "src/rurix-basis-sys",
    "channels/stable.json",
    "registry/deferred.json",
    "tests/stable/stable_api.snapshot",
    "tests/stable/bless_log.md",
    "milestones/g31/CI_GATES.md",
    "milestones/g31/g31_compatibility_matrix.json",
    "milestones/g13/design/vendor_upscale_license_clearance.md",
    "docs/renderer/integration_guide.md",
    "docs/renderer/feature_matrix.md",
    "docs/renderer/performance_tuning.md",
    "docs/renderer/compatibility_matrix.md",
    "ci/stable_snapshot.py",
]

# fact 4：API_VERSIONING.md 版本字面（五面同一字面互核之一环）。
API_VERSIONING_TOKENS = ["1.0.0", "0x00010000", "ABI_VERSION_PACKED"]
# fact 4：support_policy.md 版本字面。
POLICY_VERSION_TOKENS = ["1.0.0", "ABI_VERSION_PACKED", "renderer_sdk_api"]

# fact 5：SECURITY.md 镜像四要素 + 渲染器特有面 + 双件增补段锚。
SECURITY_MIRROR_TOKENS = [
    "25890346@qq.com", "Report a vulnerability", "3 个工作日", "协调公开",
]
SECURITY_RENDERER_TOKENS = ["驱动交互", "shader 供应链", "vendor SDK"]
SECURITY_MD_ANCHOR = "## 渲染器面(Rurix 渲染器 SDK)"
SECURITY_EN_ANCHOR = "## Renderer surface (Rurix renderer SDK)"

# fact 6：待建立/在飞诚实登记字面。
PENDING_POLICY_TOKENS = [
    "待建立", "在飞未落地", "C5", "C6", "C7", "G-MB1-6", "未建立",
]
PENDING_CHECKLIST_TOKENS = [
    "待建立（C5 在飞", "待建立（C6 在飞",
]

FROZEN_DOCS = [
    "00_MASTER_INDEX.md", "01_VISION_AND_MISSION.md", "02_USERS_AND_USE_CASES.md",
    "03_POSITIONING_AND_LANDSCAPE.md", "04_DESIGN_PRINCIPLES.md",
    "05_LANGUAGE_ARCHITECTURE.md", "06_GPU_GRAPHICS_PROGRAMMING_MODEL.md",
    "07_COMPILER_ARCHITECTURE.md", "08_RUNTIME_AND_TOOLING.md",
    "09_STDLIB_AND_ECOSYSTEM.md", "10_GOVERNANCE.md", "11_ROADMAP.md",
    "12_RISKS.md", "13_DECISION_LOG.md", "14_ENGINEERING_DISCIPLINE.md",
]

FACT_IDS = [
    "docs_present_with_anchors",
    "referenced_ci_scripts_exist",
    "referenced_surfaces_exist",
    "version_policy_matches_stable_snapshot",
    "security_response_mirrored",
    "pending_items_honestly_marked",
    "frozen_docs_untouched",
]

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 120) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout)


# ---------------------------------------------------------------------------
# 判读器（selftest 红绿两臂消费面；全纯函数无 IO 依赖）
# ---------------------------------------------------------------------------


def missing_tokens(text: str, tokens: list[str]) -> list[str]:
    """逐字在场核验（标题/字面同一判据面）——缺项清单（空 = 全在场）。"""
    return [t for t in tokens if t not in text]


def extract_ci_scripts(text: str) -> list[str]:
    """文档引用的 ci 脚本名闭集（排序去重）——存在性核验输入面。"""
    return sorted(set(CI_SCRIPT_RE.findall(text)))


def parse_abi_version(lib_text: str) -> str | None:
    """ABI_VERSION_PACKED 程序读（镜像 stable_snapshot.py 同一正则）→ 'M.m.p'。"""
    m = ABI_VERSION_RE.search(lib_text)
    if not m:
        return None
    return f"{m.group(1)}.{m.group(2)}.{m.group(3)}"


def version_consistency_problems(
    snapshot_py_text: str,
    lib_text: str,
    snapshot_doc: dict,
    versioning_text: str,
    policy_text: str,
) -> list[str]:
    """版本政策五面同一字面互核（空 = 全一致）。"""
    problems: list[str] = []
    if "renderer_sdk_api" not in snapshot_py_text or "collect_renderer_sdk_api" not in snapshot_py_text:
        problems.append("ci/stable_snapshot.py 缺 renderer_sdk_api 段")
    lib_ver = parse_abi_version(lib_text)
    if lib_ver != EXPECTED_ABI_VERSION:
        problems.append(f"lib.rs ABI_VERSION_PACKED 程序读 = {lib_ver!r}（期望 {EXPECTED_ABI_VERSION}）")
    rs = snapshot_doc.get("renderer_sdk_api") if isinstance(snapshot_doc, dict) else None
    if not isinstance(rs, dict):
        problems.append("stable_api.snapshot 缺 renderer_sdk_api 段")
    else:
        if rs.get("abi_version") != EXPECTED_ABI_VERSION:
            problems.append(f"snapshot abi_version = {rs.get('abi_version')!r}（期望 {EXPECTED_ABI_VERSION}）")
        if rs.get("export_count") != EXPECTED_EXPORT_COUNT:
            problems.append(f"snapshot export_count = {rs.get('export_count')!r}（期望 {EXPECTED_EXPORT_COUNT}）")
    mv = missing_tokens(versioning_text, API_VERSIONING_TOKENS)
    if mv:
        problems.append(f"API_VERSIONING.md 缺版本字面 {mv}")
    mp = missing_tokens(policy_text, POLICY_VERSION_TOKENS)
    if mp:
        problems.append(f"support_policy.md 缺版本字面 {mp}")
    return problems


def security_mirror_problems(policy_text: str, security_text: str, security_en_text: str) -> list[str]:
    """安全响应镜像核验：四要素 + 渲染器特有面 + 双件增补段锚（空 = 全在场）。"""
    problems: list[str] = []
    m1 = missing_tokens(policy_text, SECURITY_MIRROR_TOKENS)
    if m1:
        problems.append(f"support_policy.md 缺 SECURITY.md 镜像要素 {m1}")
    m2 = missing_tokens(policy_text, SECURITY_RENDERER_TOKENS)
    if m2:
        problems.append(f"support_policy.md 缺渲染器特有面 {m2}")
    if SECURITY_MD_ANCHOR not in security_text:
        problems.append(f"SECURITY.md 缺渲染器增补段锚 {SECURITY_MD_ANCHOR!r}")
    if "docs/renderer/support_policy.md" not in security_text:
        problems.append("SECURITY.md 增补段缺 support_policy.md 指针")
    if SECURITY_EN_ANCHOR not in security_en_text:
        problems.append(f"SECURITY.en.md 缺渲染器增补段锚 {SECURITY_EN_ANCHOR!r}")
    return problems


def pending_marker_problems(policy_text: str, checklist_text: str) -> list[str]:
    """待建立/在飞诚实登记核验（空 = 全在场）。"""
    problems: list[str] = []
    m1 = missing_tokens(policy_text, PENDING_POLICY_TOKENS)
    if m1:
        problems.append(f"support_policy.md 缺待建立登记字面 {m1}")
    m2 = missing_tokens(checklist_text, PENDING_CHECKLIST_TOKENS)
    if m2:
        problems.append(f"release_checklist.md 缺在飞标注 {m2}")
    return problems


def frozen_violations(porcelain_text: str) -> list[str]:
    """git status --porcelain 文本 → 冻结根规划文档改动清单（空 = 零触碰）。
    非 ?? 状态（M/A/D/R 等）命中冻结清单即违例（00~14 十五件）。"""
    out = []
    for line in porcelain_text.splitlines():
        if len(line) < 4:
            continue
        status, path = line[:2], line[3:].strip().strip('"')
        name = path.rsplit("/", 1)[-1]
        if "/" not in path and name in FROZEN_DOCS and status != "??":
            out.append(f"{status} {path}")
    return out


# ---------------------------------------------------------------------------
# gate 腿（host 恒跑面，无 GPU/工具链依赖，缺件即硬 FAIL）
# ---------------------------------------------------------------------------


def run_gate() -> int:
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1

    # ── fact 1：两文档在树 + 节锚 + 在案字面 ──
    docs_info: dict[str, dict] = {}
    doc_texts: dict[str, str] = {}
    for key, spec in DOC_SPECS.items():
        p: Path = spec["path"]
        if not p.is_file():
            docs_info[key] = {
                "path": str(p.relative_to(ROOT)).replace("\\", "/"), "bytes": 0,
                "headings_present": 0, "headings_required": len(spec["headings"]),
                "numbers_present": 0, "numbers_required": len(spec["numbers"]),
                "_missing_headings": spec["headings"], "_missing_numbers": spec["numbers"],
            }
            doc_texts[key] = ""
            continue
        text = p.read_text(encoding="utf-8")
        doc_texts[key] = text
        mh = missing_tokens(text, spec["headings"])
        mn = missing_tokens(text, spec["numbers"])
        docs_info[key] = {
            "path": str(p.relative_to(ROOT)).replace("\\", "/"),
            "bytes": p.stat().st_size,
            "headings_present": len(spec["headings"]) - len(mh),
            "headings_required": len(spec["headings"]),
            "numbers_present": len(spec["numbers"]) - len(mn),
            "numbers_required": len(spec["numbers"]),
            "_missing_headings": mh, "_missing_numbers": mn,
        }
    h_ok = all(not d["_missing_headings"] for d in docs_info.values())
    n_ok = all(not d["_missing_numbers"] for d in docs_info.values())
    set_fact(
        "docs_present_with_anchors", h_ok and n_ok,
        "; ".join(
            f"{k}=节 {d['headings_present']}/{d['headings_required']} 字面 {d['numbers_present']}/{d['numbers_required']}"
            + (f" 缺 {(d['_missing_headings'] + d['_missing_numbers'])[:2]}" if (d["_missing_headings"] or d["_missing_numbers"]) else "")
            for k, d in docs_info.items()
        ),
    )

    # ── fact 2：文档引用的 ci 脚本全部真实在树（防文档腐化）──
    all_text = doc_texts["support_policy"] + "\n" + doc_texts["release_checklist"]
    ci_scripts = extract_ci_scripts(all_text)
    missing_scripts = [s for s in ci_scripts if not (ROOT / s).is_file()]
    set_fact(
        "referenced_ci_scripts_exist", not missing_scripts,
        f"文档引用 ci 脚本 {len(ci_scripts)} 件全部在树"
        if not missing_scripts else f"引用脚本缺失: {missing_scripts[:5]}",
    )

    # ── fact 3：文档引用的关键仓内面在树 ──
    missing_surfaces = [s for s in REFERENCED_SURFACES if not (ROOT / s).exists()]
    set_fact(
        "referenced_surfaces_exist", not missing_surfaces,
        f"引用面 {len(REFERENCED_SURFACES)} 件全部在树"
        if not missing_surfaces else f"引用面缺失: {missing_surfaces[:5]}",
    )

    # ── fact 4：版本政策与 stable 快照面一致（五面同一字面）──
    vp_problems: list[str]
    pre_missing = [
        str(p.relative_to(ROOT)) for p in
        (STABLE_SNAPSHOT_PY, SDK_LIB_RS, SNAPSHOT_PATH, API_VERSIONING_MD) if not p.is_file()
    ]
    if pre_missing:
        vp_problems = [f"版本互核前置件缺失: {pre_missing}"]
    else:
        try:
            snapshot_doc = json.loads(SNAPSHOT_PATH.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            snapshot_doc = {}
            vp_problems = [f"stable_api.snapshot JSON 解析失败: {e}"]
        else:
            vp_problems = version_consistency_problems(
                STABLE_SNAPSHOT_PY.read_text(encoding="utf-8"),
                SDK_LIB_RS.read_text(encoding="utf-8"),
                snapshot_doc,
                API_VERSIONING_MD.read_text(encoding="utf-8"),
                doc_texts["support_policy"],
            )
    set_fact(
        "version_policy_matches_stable_snapshot", not vp_problems,
        f"五面同一字面（1.0.0 / 9 导出 / renderer_sdk_api 段）"
        if not vp_problems else "; ".join(vp_problems[:3]),
    )

    # ── fact 5：安全响应镜像（四要素 + 特有面 + 双件增补段锚）──
    if not SECURITY_MD.is_file() or not SECURITY_EN.is_file():
        sec_problems = ["SECURITY.md / SECURITY.en.md 缺失"]
    else:
        sec_problems = security_mirror_problems(
            doc_texts["support_policy"],
            SECURITY_MD.read_text(encoding="utf-8"),
            SECURITY_EN.read_text(encoding="utf-8"),
        )
    set_fact(
        "security_response_mirrored", not sec_problems,
        "镜像四要素 + 渲染器三特有面 + SECURITY 双件增补段锚全在场"
        if not sec_problems else "; ".join(sec_problems[:3]),
    )

    # ── fact 6：待建立/在飞诚实登记 ──
    pend_problems = pending_marker_problems(doc_texts["support_policy"], doc_texts["release_checklist"])
    set_fact(
        "pending_items_honestly_marked", not pend_problems,
        "C5/C6/C7/G-MB1-6/SLA 待建立在飞登记全在场"
        if not pend_problems else "; ".join(pend_problems[:3]),
    )

    # ── fact 7：00~14 冻结根规划文档零触碰（git 面）──
    rp = run(["git", "status", "--porcelain"])
    viol = frozen_violations(rp.stdout or "") if rp.returncode == 0 else ["git status 失败"]
    set_fact(
        "frozen_docs_untouched", not viol,
        f"00~14 号根冻结规划文档 {len(FROZEN_DOCS)} 件零改动（git status --porcelain 机核）"
        if not viol else "违例: " + "; ".join(viol[:3]),
    )

    return finalize(facts, docs_info, ci_scripts, missing_scripts, missing_surfaces, vp_problems, sec_problems)


def finalize(
    facts: dict,
    docs_info: dict,
    ci_scripts: list[str],
    missing_scripts: list[str],
    missing_surfaces: list[str],
    vp_problems: list[str],
    sec_problems: list[str],
) -> int:
    """门裁决 + evidence 落盘（PASS → evidence/；FAIL → .tmp 工作区）。"""
    all_pass = all(f["status"] == "PASS" for f in facts.values()) and not FAILURES
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    docs_clean = {
        k: {kk: vv for kk, vv in d.items() if not kk.startswith("_")}
        for k, d in docs_info.items()
    }
    snapshot_ver, snapshot_count = "", 0
    if SNAPSHOT_PATH.is_file():
        try:
            rs = json.loads(SNAPSHOT_PATH.read_text(encoding="utf-8")).get("renderer_sdk_api") or {}
            snapshot_ver = str(rs.get("abi_version") or "")
            snapshot_count = int(rs.get("export_count") or 0)
        except (json.JSONDecodeError, ValueError):
            pass
    lib_ver = parse_abi_version(SDK_LIB_RS.read_text(encoding="utf-8")) if SDK_LIB_RS.is_file() else None
    env_info = {
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
        "facts": [facts[fid] for fid in FACT_IDS],
        "docs": docs_clean,
        "references": {
            "ci_scripts_found": len(ci_scripts) - len(missing_scripts),
            "ci_scripts_missing": len(missing_scripts),
            "surfaces_found": len(REFERENCED_SURFACES) - len(missing_surfaces),
            "surfaces_missing": len(missing_surfaces),
        },
        "version_policy": {
            "stable_snapshot_section": "renderer_sdk_api",
            "lib_abi_version": lib_ver or "",
            "snapshot_abi_version": snapshot_ver,
            "snapshot_export_count": snapshot_count,
            "policy_doc_version_literal": EXPECTED_ABI_VERSION if not vp_problems else "",
        },
        "security": {
            "security_md_anchor_present": not any("SECURITY.md 缺" in p for p in sec_problems),
            "security_en_anchor_present": not any("SECURITY.en.md 缺" in p for p in sec_problems),
            "policy_mirror_tokens_present": len(SECURITY_MIRROR_TOKENS)
            - len(missing_tokens((POLICY_PATH.read_text(encoding="utf-8") if POLICY_PATH.is_file() else ""), SECURITY_MIRROR_TOKENS)),
            "policy_mirror_tokens_required": len(SECURITY_MIRROR_TOKENS),
        },
        "frozen_docs": {
            "checked": len(FROZEN_DOCS),
            "violations": 0 if facts["frozen_docs_untouched"]["status"] == "PASS" else 1,
            "method": "git status --porcelain 非 ?? 状态命中冻结清单即违例（00~14 号根规划文档 15 件）",
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C8 支持渠道与版本政策门（G31_PLUS §5 #55 兑现面）：两文档节锚 + 在案 "
            "字面防腐化 + 引用 ci 脚本/仓内面存在性机器核验 + 版本政策五面同一字面（lib.rs 程序读 "
            "1.0.0 ≡ snapshot abi_version/export_count=9 ≡ API_VERSIONING.md ≡ 政策文档）+ 安全响应 "
            "镜像 SECURITY.md 四要素与渲染器三特有面 + SECURITY 双件增补段锚 + 待建立/在飞诚实登记 "
            "+ 00~14 冻结文档零触碰。全门 host 恒跑面，无 DEV_ENV_DEGRADE 降级。facts: "
            + "; ".join(f["id"] + "=" + f["status"] for f in (facts[fid] for fid in FACT_IDS))
        ),
    }
    if all_pass:
        import jsonschema  # 自校验硬门（schema 漂移即 RED）

        errs = list(jsonschema.Draft7Validator(
            json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        ).iter_errors(gate_doc))
        if errs:
            for e in errs[:5]:
                fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
            all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_support_policy_{ts}.json"
    else:
        WORK.mkdir(parents=True, exist_ok=True)
        gate_path = WORK / f"gate_fail_{ts}.json"
    io.open(gate_path, "w", encoding="utf-8", newline="\n").write(
        json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n"
    )
    note(f"evidence: {gate_path.relative_to(ROOT)}")
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂 + 事实源互核，无 GPU/工具链依赖）
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

    # 红绿臂①：逐字在场核验。
    expect(missing_tokens("abc 1.0.0 renderer_sdk_api", ["1.0.0", "renderer_sdk_api"]) == [],
           "GREEN:字面全在场")
    expect(missing_tokens("abc", ["1.0.0"]) == ["1.0.0"], "RED:缺字面必检出")
    expect(missing_tokens("## 3. 安全响应\nx", ["## 3. 安全响应"]) == [], "GREEN:节锚在场")
    expect(missing_tokens("# t\n", ["## 5. 待建立项（诚实登记，不冒充）"]) == ["## 5. 待建立项（诚实登记，不冒充）"],
           "RED:缺节锚必检出")
    expect(missing_tokens("", []) == [], "GREEN:空需求闭集恒绿")

    # 红绿臂②：ci 脚本名提取。
    expect(extract_ci_scripts("跑 `ci/g31_support_policy_smoke.py` 与 ci/check_schemas.py。\n"
                              "重复 ci/check_schemas.py 一次") ==
           ["ci/check_schemas.py", "ci/g31_support_policy_smoke.py"],
           "GREEN:脚本名提取 + 去重排序")
    expect(extract_ci_scripts("无脚本引用") == [], "GREEN:零引用空集")
    expect(extract_ci_scripts("ci/g31_foo.py2 与 ci\\bar.py 不误提") == [],
           "RED:非 .py 后缀/反斜杠路径不误提")

    # 红绿臂③：ABI 版本程序读（镜像 stable_snapshot 同一正则）。
    good_lib = "pub const ABI_VERSION_PACKED: u32 = (1 << 16) | (0 << 8) | 0;"
    expect(parse_abi_version(good_lib) == "1.0.0", "GREEN:1.0.0 程序读")
    expect(parse_abi_version(good_lib.replace("(1 << 16)", "(2 << 16)")) == "2.0.0",
           "GREEN:MAJOR 漂移可读出")
    expect(parse_abi_version("pub const OTHER: u32 = 0;") is None, "RED:缺常量必检出")

    # 红绿臂④：版本五面互核。
    good_snapshot = {"renderer_sdk_api": {"abi_version": "1.0.0", "export_count": 9}}
    good_versioning = "v1 = **1.0.0** = `0x00010000`；ABI_VERSION_PACKED 单一事实源"
    good_policy = "1.0.0 + ABI_VERSION_PACKED + renderer_sdk_api 段"
    good_snapshot_py = "def collect_renderer_sdk_api():\n    # renderer_sdk_api 段"
    expect(version_consistency_problems(good_snapshot_py, good_lib, good_snapshot,
                                        good_versioning, good_policy) == [],
           "GREEN:五面同一字面")
    bad_lib = version_consistency_problems(good_snapshot_py, good_lib.replace("(1 << 16)", "(2 << 16)"),
                                           good_snapshot, good_versioning, good_policy)
    expect(any("lib.rs" in p for p in bad_lib), "RED:lib.rs 版本漂移必检出")
    bad_snap = version_consistency_problems(good_snapshot_py, good_lib,
                                            {"renderer_sdk_api": {"abi_version": "1.0.0", "export_count": 10}},
                                            good_versioning, good_policy)
    expect(any("export_count" in p for p in bad_snap), "RED:快照导出数漂移必检出")
    bad_py = version_consistency_problems("def other():\n    pass", good_lib, good_snapshot,
                                          good_versioning, good_policy)
    expect(any("renderer_sdk_api 段" in p for p in bad_py), "RED:stable_snapshot 缺段必检出")
    bad_ver = version_consistency_problems(good_snapshot_py, good_lib, good_snapshot,
                                           "0.9.0", good_policy)
    expect(any("API_VERSIONING.md" in p for p in bad_ver), "RED:版本政策文档字面缺失必检出")

    # 红绿臂⑤：安全镜像 + 待建立登记。
    expect(security_mirror_problems(
        "25890346@qq.com Report a vulnerability 3 个工作日 协调公开 驱动交互 shader 供应链 vendor SDK",
        "## 渲染器面(Rurix 渲染器 SDK)\ndocs/renderer/support_policy.md",
        "## Renderer surface (Rurix renderer SDK)",
    ) == [], "GREEN:安全镜像全在场")
    expect(any("驱动交互" in p for p in security_mirror_problems(
        "25890346@qq.com Report a vulnerability 3 个工作日 协调公开",
        "## 渲染器面(Rurix 渲染器 SDK)\ndocs/renderer/support_policy.md",
        "## Renderer surface (Rurix renderer SDK)",
    )), "RED:缺渲染器特有面必检出")
    expect(any("SECURITY.md 缺" in p for p in security_mirror_problems(
        "25890346@qq.com Report a vulnerability 3 个工作日 协调公开 驱动交互 shader 供应链 vendor SDK",
        "# 安全政策\n", "## Renderer surface (Rurix renderer SDK)",
    )), "RED:SECURITY.md 缺增补段必检出")
    expect(any("SECURITY.en.md 缺" in p for p in security_mirror_problems(
        "25890346@qq.com Report a vulnerability 3 个工作日 协调公开 驱动交互 shader 供应链 vendor SDK",
        "## 渲染器面(Rurix 渲染器 SDK)\ndocs/renderer/support_policy.md", "# Security Policy\n",
    )), "RED:SECURITY.en.md 缺增补段必检出")
    expect(pending_marker_problems("待建立 在飞未落地 C5 C6 C7 G-MB1-6 未建立",
                                   "待建立（C5 在飞 待建立（C6 在飞") == [],
           "GREEN:待建立登记全在场")
    expect(pending_marker_problems("待建立 C5", "x") != [], "RED:缺登记必检出")

    # 红绿臂⑥：冻结面核验（00~14 十五件）。
    expect(frozen_violations(" M src/foo.rs\n?? docs/\n") == [], "GREEN:非冻结面改动不违例")
    expect(frozen_violations(" M 14_ENGINEERING_DISCIPLINE.md\n") == [" M 14_ENGINEERING_DISCIPLINE.md"],
           "RED:14 号冻结文档改动必检出")
    expect(frozen_violations(" M 03_POSITIONING_AND_LANDSCAPE.md\n") == [" M 03_POSITIONING_AND_LANDSCAPE.md"],
           "RED:03 号冻结文档改动必检出")
    expect(frozen_violations("M  13_DECISION_LOG.md") == ["M  13_DECISION_LOG.md"],
           "RED:staged 改动必检出")
    expect(frozen_violations("?? 11_ROADMAP.md") == [], "GREEN:?? 不计改动（冻结件均 tracked）")
    expect(frozen_violations(" M docs/00_MASTER_INDEX.md/x") == [], "GREEN:子路径同名不误判")
    expect(len(FROZEN_DOCS) == 15, "冻结清单 = 15 件（00~14）")

    # 红绿臂⑦：事实源互核（真文档/面/schema 在树 + 判据面自洽）。
    for key, spec in DOC_SPECS.items():
        p: Path = spec["path"]
        expect(p.is_file(), f"文档在树 {key}")
        if p.is_file():
            text = p.read_text(encoding="utf-8")
            expect(missing_tokens(text, spec["headings"]) == [], f"节锚全在场 {key}")
            expect(missing_tokens(text, spec["numbers"]) == [], f"在案字面全在场 {key}")
    if all(spec["path"].is_file() for spec in DOC_SPECS.values()):
        real_all = "\n".join(spec["path"].read_text(encoding="utf-8") for spec in DOC_SPECS.values())
        real_scripts = extract_ci_scripts(real_all)
        expect(len(real_scripts) >= 20, f"文档引用脚本闭集 ≥20（实测 {len(real_scripts)}）")
        expect(all((ROOT / s).is_file() for s in real_scripts), "引用脚本全部真实在树（互核）")
        expect("ci/g31_support_policy_smoke.py" in real_scripts, "自引用脚本在闭集内")
    expect(all((ROOT / s).exists() for s in REFERENCED_SURFACES), "引用面闭集全部在树（互核）")
    expect(version_consistency_problems(
        STABLE_SNAPSHOT_PY.read_text(encoding="utf-8") if STABLE_SNAPSHOT_PY.is_file() else "",
        SDK_LIB_RS.read_text(encoding="utf-8") if SDK_LIB_RS.is_file() else "",
        json.loads(SNAPSHOT_PATH.read_text(encoding="utf-8")) if SNAPSHOT_PATH.is_file() else {},
        API_VERSIONING_MD.read_text(encoding="utf-8") if API_VERSIONING_MD.is_file() else "",
        POLICY_PATH.read_text(encoding="utf-8") if POLICY_PATH.is_file() else "",
    ) == [], "版本五面互核全绿（真实面）")
    expect(security_mirror_problems(
        POLICY_PATH.read_text(encoding="utf-8") if POLICY_PATH.is_file() else "",
        SECURITY_MD.read_text(encoding="utf-8") if SECURITY_MD.is_file() else "",
        SECURITY_EN.read_text(encoding="utf-8") if SECURITY_EN.is_file() else "",
    ) == [], "安全镜像互核全绿（真实面）")
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "wave const 互核")
        expect(
            sorted(gs.get("required", [])) == sorted([
                "schema", "subject", "symbolic_gate_key", "wave", "facts", "docs",
                "references", "version_policy", "security", "frozen_docs",
                "environment", "timestamp", "notes",
            ]),
            "schema required 闭集互核（13 字段）",
        )
        fact_enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(fact_enum) == sorted(FACT_IDS), "facts id 枚举闭集互核（7 facts）")
        expect(gs["properties"]["frozen_docs"]["properties"]["checked"]["const"] == 15,
               "frozen checked const=15 互核")
        expect(gs["properties"]["version_policy"]["properties"]["snapshot_export_count"]["const"] == 9,
               "snapshot export_count const=9 互核")
    expect(len(FACT_IDS) == 7, "facts 闭集 = 7")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=7；7 红臂组 + 事实源互核 + schema 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        return run_gate()
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
