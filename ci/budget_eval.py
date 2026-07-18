"""PR Smoke 步骤 6:预算 evaluator(14 §3 / CI_GATES.md §3.6)。

- 多预算合并加载 + 命名空间前缀与冲突检测;
- estimated 条目自动 skip 并输出 skip_reason 留痕;
- measured_local 条目:读取 evidence_file,断言 results.trimmed_mean 对 threshold;
- --strict:estimated 即 FAIL(close-out / Release 模式,M0 关闭用,契约 G-M0-1);
- --allow-pending <id>:strict 模式下对尚未到期的计数器保留 SKIP(Release 分阶段落地用,
  例如 M8.4 发布链路先于 M8.5 双语覆盖)。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []
SKIPS: list[str] = []
PASSES: list[str] = []
ALLOW_PENDING_IDS: set[str] = set()


def err(msg: str) -> None:
    ERRORS.append(msg)


def load_budgets() -> dict[str, dict]:
    """合并加载全部预算,命名空间冲突即 FAIL。"""
    merged: dict[str, dict] = {}
    for path in sorted(ROOT.glob("milestones/*/*_budget.json")):
        doc = json.loads(path.read_text(encoding="utf-8"))
        ns = doc.get("namespace", "")
        for group in ("entries", "ratio_assertions", "counter_assertions"):
            for entry in doc.get(group, []):
                eid = entry["id"]
                if not eid.startswith(ns + "."):
                    err(f"{path.name}: {eid} 未带前缀 {ns}.")
                if eid in merged:
                    err(f"命名空间冲突: {eid}(多预算合并检测,14 §3)")
                merged[eid] = {**entry, "_group": group, "_file": path.name}
    return merged


def measured_value(entry: dict) -> float | None:
    ef = entry.get("evidence_file")
    if not ef or not (ROOT / ef).is_file():
        err(f"{entry['id']}: evidence_file 缺失或不存在: {ef!r}")
        return None
    doc = json.loads((ROOT / ef).read_text(encoding="utf-8"))
    return doc["results"]["trimmed_mean"]


def eval_lsp_latency(entry: dict) -> None:
    """M6.5 特例(契约 G-M6-2):单 entry 表达 LSP 三类交互延迟,逐交互对子阈值判定。

    证据 results.per_interaction.{completion,definition,publishDiagnostics}.trimmed_mean
    逐一对 entry.thresholds[name] 校验(direction=max,任一超阈 → FAIL,normal/strict 同)。
    """
    eid = entry["id"]
    ef = entry.get("evidence_file")
    if not ef or not (ROOT / ef).is_file():
        err(f"{eid}: evidence_file 缺失或不存在: {ef!r}")
        return
    doc = json.loads((ROOT / ef).read_text(encoding="utf-8"))
    per = doc.get("results", {}).get("per_interaction", {})
    thresholds = entry.get("thresholds")
    if not thresholds:
        err(f"{eid}: measured_local 缺逐交互 thresholds(M6.5 回填,契约 G-M6-2)")
        return
    direction = entry.get("direction", "max")
    unit = entry.get("unit", "ms")
    for name, thr in thresholds.items():
        sub = per.get(name)
        if not sub or "trimmed_mean" not in sub:
            err(f"{eid}.{name}: evidence per_interaction 缺该交互 trimmed_mean")
            continue
        value = sub["trimmed_mean"]
        ok = value <= thr if direction == "max" else value >= thr
        if ok:
            PASSES.append(f"{eid}.{name}: PASS — {value:.3f} {unit} vs {direction} {thr}")
        else:
            err(f"{eid}.{name}: FAIL — {value:.3f} 违反 {direction} {thr}")


def eval_cold_start(entry: dict) -> None:
    """EA1 冷启动两段式(契约 G-EA1-6/G-EA1-8;RXS-0219,裁决 C):evidence 为
    install_e2e 档(segment/pass/duration_s 字段面),非 BENCH_PROTOCOL results 形
    ——专属分支(对齐 M6.5 eval_lsp_latency 特例先例)。判据:evidence pass 为真 +
    duration_s 对 threshold(direction=max,秒);evidence 段位须与 entry id 后缀
    一致(防拿 A 段档充 B 段条目);失败 attempt 档(pass=false)不得作达标依据。
    """
    eid = entry["id"]
    ef = entry.get("evidence_file")
    if not ef or not (ROOT / ef).is_file():
        err(f"{eid}: evidence_file 缺失或不存在: {ef!r}")
        return
    doc = json.loads((ROOT / ef).read_text(encoding="utf-8"))
    seg = doc.get("segment")
    if not eid.endswith(f"cold_start_{seg}_s"):
        err(f"{eid}: evidence segment {seg!r} 与 entry id 不一致")
        return
    if doc.get("pass") is not True:
        err(f"{eid}: evidence pass 非 true(该 attempt 不可作达标依据)")
        return
    value = doc["duration_s"]
    thr = entry["threshold"]
    if value <= thr:
        PASSES.append(
            f"{eid}: PASS — {value:.2f} s vs max {thr}(attempt {doc.get('attempt')},"
            f" {doc.get('toolchain_version')})"
        )
    else:
        err(f"{eid}: FAIL — {value:.2f} s 违反 max {thr}")


def eval_entry(entry: dict, strict: bool) -> None:
    eid = entry["id"]
    ev = entry.get("evidence")
    if ev == "estimated":
        if strict:
            err(f"{eid}: estimated 占位在严格模式下 FAIL(占位存活规则,14 §3)")
        else:
            SKIPS.append(f"{eid}: SKIP — {entry.get('skip_reason', '(无 skip_reason)')}")
        return
    if ev == "unlocked":
        err(f"{eid}: unlocked 证据不得作为预算断言依据(BENCH_PROTOCOL §2.1)")
        return
    if eid == "m6.bench.lsp_interaction_latency_ms":
        eval_lsp_latency(entry)
        return
    if eid.startswith("ea1.bench.cold_start_"):
        eval_cold_start(entry)
        return
    value = measured_value(entry)
    if value is None:
        return
    threshold = entry.get("threshold")
    direction = entry.get("direction", "min")
    ok = value >= threshold if direction == "min" else value <= threshold
    if ok:
        PASSES.append(f"{eid}: PASS — {value:.3f} {entry.get('unit', '')} vs {direction} {threshold}")
    else:
        err(f"{eid}: FAIL — {value:.3f} 违反 {direction} {threshold}")


def eval_ratio(entry: dict, merged: dict[str, dict], strict: bool) -> None:
    eid = entry["id"]
    if entry.get("evidence") == "estimated":
        if strict:
            err(f"{eid}: estimated 占位在严格模式下 FAIL")
        else:
            SKIPS.append(f"{eid}: SKIP — {entry.get('skip_reason', '(无 skip_reason)')}")
        return
    num = merged.get(entry["numerator"])
    den = merged.get(entry["denominator"])
    if not num or not den:
        err(f"{eid}: numerator/denominator 条目不存在")
        return
    nv, dv = measured_value(num), measured_value(den)
    if nv is None or dv is None or dv == 0:
        return
    ratio = nv / dv
    threshold = entry.get("threshold")
    direction = entry.get("direction", "min")
    ok = ratio >= threshold if direction == "min" else ratio <= threshold
    (PASSES if ok else ERRORS).append(
        f"{eid}: {'PASS' if ok else 'FAIL'} — ratio {ratio:.4f} vs {direction} {threshold}"
    )


def count_or_gate(eid: str, n: int, required: int, what: str, pending_hint: str, strict: bool) -> None:
    """M1 计数器通用判定:达标 PASS;未达标 → normal skip(建设期)/ strict FAIL(close-out)。"""
    if n >= required:
        PASSES.append(f"{eid}: PASS — {n} {what}(要求 ≥{required})")
    elif strict and eid in ALLOW_PENDING_IDS:
        SKIPS.append(f"{eid}: SKIP(strict allow-pending) — 当前 {n} {what}({pending_hint})")
    elif strict:
        err(f"{eid}: FAIL — 仅 {n} {what}(要求 ≥{required})")
    else:
        SKIPS.append(f"{eid}: SKIP — 当前 {n} {what}({pending_hint})")


def parse_args(argv: list[str]) -> tuple[bool, set[str]]:
    strict = False
    allow_pending: set[str] = set()
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--strict":
            strict = True
        elif arg == "--allow-pending":
            if i + 1 >= len(argv):
                err("--allow-pending 缺少计数器 id")
            else:
                allow_pending.add(argv[i + 1])
                i += 1
        else:
            err(f"未知参数: {arg}")
        i += 1
    return strict, allow_pending


def eval_counter(entry: dict, strict: bool) -> None:
    """计数器断言:已知 id 逐条实现,未知 id 强制 FAIL(逼迫维护,防僵尸计数器,14 §5)。"""
    eid = entry["id"]
    if eid == "m0.counter.env_profile_required_fields":
        # 字段完整性由 check_schemas.py 对证据文件做 JSON Schema 校验兜底
        PASSES.append(f"{eid}: PASS(delegated to check_schemas.py)")
    elif eid == "m0.counter.evidence_files_saxpy_runs":
        n = 0
        for f in (ROOT / "evidence").glob("saxpy_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("evidence_level") == "measured_local":
                n += 1
        if n >= 3:
            PASSES.append(f"{eid}: PASS — {n} 份 measured_local 证据")
        elif strict:
            err(f"{eid}: FAIL — 仅 {n} 份 measured_local 证据(要求 ≥3,契约 G-M0-1)")
        else:
            SKIPS.append(f"{eid}: SKIP — 当前 {n} 份(M0.3 回填前为正常状态)")
    elif eid == "m1.counter.syntax_corpus_size":
        n = len(list((ROOT / "conformance" / "syntax").glob("**/*.rx")))
        count_or_gate(eid, n, 100, "个语法样例", "M1.2/M1.3 建设期为正常状态,契约 G-M1-1", strict)
    elif eid == "m1.counter.ui_golden_path1_snapshots":
        n = len(list((ROOT / "tests" / "ui").glob("**/*.stderr")))
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M1.4 建设期为正常状态,契约 G-M1-2", strict)
    elif eid == "m2.counter.ui_golden_path2_snapshots":
        typeck_dir = ROOT / "tests" / "ui" / "typeck"
        n = len(list(typeck_dir.glob("**/*.stderr"))) if typeck_dir.is_dir() else 0
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M2.2 建设期为正常状态,契约 G-M2-3", strict)
    elif eid == "m3.counter.ui_golden_path3_snapshots":
        borrowck_dir = ROOT / "tests" / "ui" / "borrowck"
        n = len(list(borrowck_dir.glob("**/*.stderr"))) if borrowck_dir.is_dir() else 0
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M3.3 建设期为正常状态,契约 G-M3-2", strict)
    elif eid == "m3.counter.borrowck_conformance_categories":
        reject_dir = ROOT / "conformance" / "borrowck" / "reject"
        n = len([p for p in reject_dir.iterdir() if p.is_dir()]) if reject_dir.is_dir() else 0
        count_or_gate(eid, n, 7, "个预设错误类别目录", "M3.3 建设期为正常状态,契约 G-M3-1", strict)
    elif eid == "m4.counter.launch_conformance_categories":
        reject_dir = ROOT / "conformance" / "launch" / "reject"
        n = len([p for p in reject_dir.iterdir() if p.is_dir()]) if reject_dir.is_dir() else 0
        count_or_gate(eid, n, 4, "个预设错误类别目录", "M4.3 建设期为正常状态,契约 G-M4-2", strict)
    elif eid == "m4.counter.ui_golden_path4_snapshots":
        # 黄金路径 4 = 目标后端错误:3xxx 着色/地址空间(M4.1)+ 6xxx codegen/ptxas
        # (M4.2)+ launch 类型契约(M4.3,3xxx 续接 RX3004~3006 + RX2001 复用),
        # 契约 G-M4-3 覆盖各段;计数聚合四目录。
        path4_dirs = ["coloring", "addrspace", "codegen", "launch"]
        n = sum(
            len(list((ROOT / "tests" / "ui" / d).glob("**/*.stderr")))
            for d in path4_dirs
            if (ROOT / "tests" / "ui" / d).is_dir()
        )
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M4.1 3xxx 子集已入,6xxx 随 M4.3,契约 G-M4-3", strict)
    elif eid == "m5.counter.views_conformance_categories":
        reject_dir = ROOT / "conformance" / "views" / "reject"
        n = len([p for p in reject_dir.iterdir() if p.is_dir()]) if reject_dir.is_dir() else 0
        count_or_gate(eid, n, 4, "个预设错误类别目录", "M5.1 建设期为正常状态,契约 G-M5-2", strict)
    elif eid == "m5.counter.ui_golden_path5_snapshots":
        # 黄金路径 5 = 并行安全错误:views 重叠/别名(M5.1,3xxx 续接)
        # + shared+barrier 一致性违例(M5.2)+ scoped atomics scope 误用(M5.2);
        # 契约 G-M5-3 覆盖各类;计数聚合三目录。
        path5_dirs = ["views", "shared", "atomics"]
        n = sum(
            len(list((ROOT / "tests" / "ui" / d).glob("**/*.stderr")))
            for d in path5_dirs
            if (ROOT / "tests" / "ui" / d).is_dir()
        )
        count_or_gate(eid, n, 10, "条 .stderr snapshot", "M5.1/M5.2 建设期为正常状态,契约 G-M5-3", strict)
    elif eid == "m5.counter.compute_sanitizer_clean":
        # Compute Sanitizer racecheck+memcheck nightly 全绿(契约 G-M5-4,08 §5);
        # 计数 = evidence/compute_sanitizer_*.json 中 clean=true 的报告数。
        # M5.4 nightly 接入前为 0 → 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("compute_sanitizer_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("clean") is True:
                n += 1
        count_or_gate(eid, n, 1, "份 clean Sanitizer 报告", "M5.4 nightly 接入前为正常状态,契约 G-M5-4", strict)
    elif eid == "m5.counter.redistribution_audit_clean":
        # NVIDIA 再分发白名单审计 formal 激活(CI_GATES §4 第 2 项,M5.4 第 5 步);
        # 计数 = evidence/redistribution_audit_*.json 中 redistribution_surface_empty=true
        # 的报告数。机器事实(嵌入 PTX 无 __nv_*、libdevice.10.bc 不入产物);键于机器事实,
        # 不键于 EULA 法律签署(裁决保持 pending-human-review)。
        n = 0
        for f in (ROOT / "evidence").glob("redistribution_audit_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("redistribution_surface_empty") is True:
                n += 1
        count_or_gate(eid, n, 1, "份再分发面为空的审计报告", "审计证据回填前为正常状态,CI_GATES §4 第 2 项", strict)
    elif eid == "m6.counter.rx_cli_core_subcommands":
        # rx CLI 核心子命令端到端覆盖数(契约 G-M6-3);计数源 = evidence/
        # rx_cli_smoke_*.json 中 subcommands_passed 去重集的最大基数(机器事实:
        # ci/rx_cli_smoke.py 在样例工程逐子命令端到端真跑,退出码符合 RXS-0083)。
        # M6.1 落地 build/run/check/fmt/bench(5)< 6 → 建设期 normal SKIP;
        # rx test(M6.3)端到端纳入后达 6 转 PASS。证据缺失 → 0,对齐 M4/M5 先例。
        n = 0
        for f in (ROOT / "evidence").glob("rx_cli_smoke_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            n = max(n, len(set(doc.get("subcommands_passed", []))))
        count_or_gate(eid, n, 6, "个 rx CLI 核心子命令端到端", "M6.1 落地 5/6(rx test 待 M6.3),建设期为正常状态,契约 G-M6-3", strict)
    elif eid == "m6.counter.offline_rebuild_reproducible":
        # 三包 workspace 离线重建逐字节可复现(契约 G-M6-1,09 §7.1);
        # 计数 = evidence/offline_rebuild_*.json 中 reproducible=true 的报告数。
        # M6.3 复现门接入前为 0 → 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("offline_rebuild_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("reproducible") is True:
                n += 1
        count_or_gate(eid, n, 1, "份逐字节可复现的离线重建证据", "M6.3 离线重建复现门接入前为正常状态,契约 G-M6-1", strict)
    elif eid == "m6.counter.lsp_capabilities":
        # LSP MVP 能力面覆盖数(契约 G-M6-2/G-M6-5,07 §9);计数源 =
        # evidence/lsp_smoke_*.json 中 capabilities_passed 去重集的最大基数。
        n = 0
        for f in (ROOT / "evidence").glob("lsp_smoke_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            n = max(n, len(set(doc.get("capabilities_passed", []))))
        count_or_gate(eid, n, 5, "项 LSP MVP 能力面", "M6.4 LSP server 落地前为正常状态,契约 G-M6-2", strict)
    elif eid == "m7.counter.math_primitives":
        # core 数学库原语端到端覆盖数(契约 G-M7-4;Vec/Mat/swizzle/几何原语,
        # host+device 双路径,11 §3 M7);计数源 = evidence/stdlib_math_*.json 中
        # primitives_passed 去重集的最大基数。M7.1 数学库落地前为 0 → 建设期
        # normal SKIP / close-out strict FAIL,对齐 M4/M5/M6 计数器先例。
        n = 0
        for f in (ROOT / "evidence").glob("stdlib_math_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            n = max(n, len(set(doc.get("primitives_passed", []))))
        count_or_gate(eid, n, 8, "个 core 数学库原语端到端", "M7.1 数学库落地前为正常状态,契约 G-M7-4", strict)
    elif eid == "m7.counter.soft_raster_kernels_safe":
        # G0 软光栅 kernel safe 覆盖数(契约 G-M7-3;binning/tile 光栅/深度/tonemap
        # 全 safe 代码目标,11 §3 M7);计数源 = evidence/soft_raster_*.json 中
        # safe_kernels 去重集的最大基数(凡落 unsafe 的 kernel 不计入 safe 覆盖,
        # 须 // SAFETY: + safe 覆盖率报告留痕原因)。M7.3 软光栅落地前为 0 →
        # 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("soft_raster_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            n = max(n, len(set(doc.get("safe_kernels", []))))
        count_or_gate(eid, n, 4, "个 safe 软光栅 kernel", "M7.3 软光栅 kernel 落地前为正常状态,契约 G-M7-3", strict)
    elif eid == "m7.counter.uc03_demo_image_sequence":
        # UC-03 demo 单 EXE 输出确定性图像序列证据数 ≥1(契约 G-M7-1,01 §6 UC-03);
        # 计数 = evidence/uc03_demo_*.json 中 image_sequence_ok=true 的报告数
        # (机器事实:rx build 产单 EXE,运行输出确定性图像序列,逐帧 content
        # SHA-256 两次运行逐字节一致)。M7.4 demo 落地前为 0 → 建设期 normal SKIP /
        # close-out strict FAIL,对齐 m6.counter.offline_rebuild_reproducible 先例。
        n = 0
        for f in (ROOT / "evidence").glob("uc03_demo_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("image_sequence_ok") is True:
                n += 1
        count_or_gate(eid, n, 1, "份 UC-03 demo 图像序列证据", "M7.4 UC-03 demo 落地前为正常状态,契约 G-M7-1", strict)
    elif eid == "m8.counter.uc01_pytorch_operators":
        # UC-01 PyTorch 算子替换端到端覆盖数(契约 G-M8-1;rx build --emit=pyd 产
        # PYD,经 __cuda_array_interface__/DLPack 双协议零拷贝接入 PyTorch,02 §U1 / 09);
        # 计数源 = evidence/uc01_*.json 中 operators_passed 去重集的最大基数。M8.1
        # 互操作落地前为 0 → 建设期 normal SKIP / close-out strict FAIL,对齐 M5/M6/M7。
        n = 0
        for f in (ROOT / "evidence").glob("uc01_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            n = max(n, len(set(doc.get("operators_passed", []))))
        count_or_gate(eid, n, 3, "个 UC-01 PyTorch 算子替换端到端", "M8.1 互操作落地前为正常状态,契约 G-M8-1", strict)
    elif eid == "m8.counter.uc02_stream_pipeline":
        # UC-02 三 stream 重叠流水线端到端证据数 ≥1(契约 G-M8-3;affine Context/
        # Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化,02 §U2);计数 =
        # evidence/uc02_*.json 中 stream_pipeline_ok=true 的报告数。UC-02 demo 落地前
        # 为 0 → 建设期 normal SKIP / close-out strict FAIL,对齐 m6.counter.offline_rebuild。
        n = 0
        for f in (ROOT / "evidence").glob("uc02_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("stream_pipeline_ok") is True:
                n += 1
        count_or_gate(eid, n, 1, "份 UC-02 三 stream 流水线端到端证据", "M8 UC-02 落地前为正常状态,契约 G-M8-3", strict)
    elif eid == "m8.counter.cublas_bindings":
        # cublas 绑定包覆盖数(契约 G-M8-2;GEMM/GEMV 三层绑定 raw FFI / safe wrapper /
        # 高层 API,09);计数源 = evidence/cublas_*.json 中 bindings_passed 去重集的最大
        # 基数。cublas 包落地前为 0 → 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("cublas_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            n = max(n, len(set(doc.get("bindings_passed", []))))
        count_or_gate(eid, n, 2, "个 cublas GEMM/GEMV 绑定", "M8 cublas 包落地前为正常状态,契约 G-M8-2", strict)
    elif eid == "m8.counter.release_artifacts_signed":
        # 发布链路签名产物数 ≥1(契约 G-M8-4,RD-001;EXE/DLL/MSI 经 Azure Artifact
        # Signing Authenticode + 时间戳,08 §9 / 14 §8 Release 层);计数源 =
        # evidence/release_*.json 中 signed_artifacts 去重集的最大基数(机器事实:验签
        # 通过 + SBOM 齐备 + 许可白名单审计通过)。发布链路建成前为 0 → 建设期 normal
        # SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("release_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            n = max(n, len(set(doc.get("signed_artifacts", []))))
        count_or_gate(eid, n, 1, "个发布链路签名产物", "M8 发布链路建成前为正常状态,契约 G-M8-4", strict)
    elif eid == "m8.counter.bilingual_diagnostic_coverage":
        # 诊断消息中英双语全量覆盖完整报告数 ≥1(契约 G-M8-5,RD-006;message-key zh/en
        # key 集对齐,覆盖率核对入发布门,10 §6);计数 = evidence/bilingual_*.json 中
        # coverage_complete=true 的报告数(机器事实:zh 与 en 消息表 key 集合一致)。
        # 双语全量回填前为 0 → 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("bilingual_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("coverage_complete") is True:
                n += 1
        count_or_gate(eid, n, 1, "份诊断双语全量覆盖证据", "M8 双语全量回填前为正常状态,契约 G-M8-5", strict)
    elif eid == "m1.counter.spec_clause_test_anchoring":
        # 条款 ↔ 测试锚定由 traceability 矩阵工具核对(M1.4 交付物,契约 G-M1-4);
        # 矩阵产物落地前 normal skip / strict FAIL,落地后委托其自身校验结果。
        matrix = ROOT / "conformance" / "traceability_matrix.json"
        if matrix.is_file():
            doc = json.loads(matrix.read_text(encoding="utf-8"))
            unanchored = [c for c, tests in doc.get("clauses", {}).items() if not tests]
            if unanchored:
                err(f"{eid}: FAIL — 未锚定条款: {', '.join(sorted(unanchored))}(10 §4)")
            else:
                PASSES.append(f"{eid}: PASS — {len(doc.get('clauses', {}))} 条款全部 ≥1 测试锚定")
        elif strict:
            err(f"{eid}: FAIL — traceability 矩阵不存在(契约 G-M1-4)")
        else:
            SKIPS.append(f"{eid}: SKIP — traceability 矩阵未生成(M1.4 交付物,建设期为正常状态)")
    elif eid == "g1.counter.d3d12_interop":
        # CUDA–D3D12 interop 端到端证据数 ≥1(契约 G-G1-1;ExternalBuffer/
        # ExternalSemaphore import D3D12 共享堆/信号量 → Rurix kernel 写 backbuffer
        # 等价纹理数值对照 + 句柄生命周期/跨 context/信号时序违例编译期拦截,06 §8.1 /
        # D-130);计数 = evidence/d3d12_interop_*.json 中 interop_ok=true 的报告数。
        # G1.1 落地前为 0 → 建设期 normal SKIP / close-out strict FAIL,对齐 M8 先例。
        n = 0
        for f in (ROOT / "evidence").glob("d3d12_interop_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("interop_ok") is True:
                n += 1
        count_or_gate(eid, n, 1, "份 CUDA–D3D12 interop 端到端证据", "G1.1 interop 落地前为正常状态,契约 G-G1-1", strict)
    elif eid == "g1.counter.realtime_present":
        # 软光栅 demo 实时窗口呈现端到端证据数 ≥1(契约 G-G1-1;G0 kernel
        # RXS-0118~0121 语义 0-byte,写 backbuffer → 信号量同步 present,11 §4 /
        # spec/softraster.md:153);计数 = evidence/realtime_present_*.json 中
        # present_ok=true 的报告数。无窗口/显示环境冒烟降级 SKIP 不写证据。
        # G1.1 落地前为 0 → 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("realtime_present_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("present_ok") is True:
                n += 1
        count_or_gate(eid, n, 1, "份软光栅实时窗口呈现证据", "G1.1 实时呈现落地前为正常状态,契约 G-G1-1", strict)
    elif eid == "g1.counter.async_buffer_pipeline":
        # 流序分配 AsyncBuffer<'stream,T> 端到端证据数 ≥1(契约 G-G1-2;三 stream
        # 流序分配 + 分配未完成/释放后/跨 stream 未同步三类生命周期错误编译期拦截,
        # 06 §5.4 / 08 §2.2 / D-122);计数 = evidence/async_buffer_*.json 中
        # pipeline_ok=true 的报告数。device 路径并入 Compute Sanitizer nightly
        # (CUDA.jl #780 事故类回归)。G1.2 落地前为 0 → 建设期 normal SKIP / strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("async_buffer_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("pipeline_ok") is True:
                n += 1
        count_or_gate(eid, n, 1, "份流序分配 AsyncBuffer 端到端证据", "G1.2 AsyncBuffer 落地前为正常状态,契约 G-G1-2", strict)
    elif eid == "g1.counter.engine_integration":
        # 首个引擎集成端到端证据数 ≥1(契约 G-G1-3;Rurix DLL #[export(c)] C ABI
        # 嵌入现存 C++/D3D12 渲染框架承担 compute pass,UC-05 前奏,06 §8.3 / 02 §U5);
        # 计数 = evidence/engine_integration_*.json 中 integration_ok=true 的报告数。
        # G1.3 落地前为 0 → 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("engine_integration_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("integration_ok") is True:
                n += 1
        count_or_gate(eid, n, 1, "份首个引擎集成端到端证据", "G1.3 引擎集成落地前为正常状态,契约 G-G1-3", strict)
    elif eid == "ms1.counter.host_orch_single_source":
        # single-source 宿主编排端到端证据数 ≥1(契约 G-MS1-2;host .rx 经 std::gpu
        # 首期收敛子集编排 GPU + 同源 kernel PTX 嵌入单 EXE,rx build 一步出可执行,
        # device 真跑数值自校验 + 篡改嵌入 PTX 装载协商拒/桩化 kernel 写回数值红双红绿,
        # RFC-0009 / RXS-0189~0196,CI 步骤 52 ci/host_orch_smoke.py);计数 =
        # evidence/host_orch_smoke*.json 中 single_source=true 且 device_run=true 的
        # 报告数。无 CUDA 环境冒烟降级 SKIP 不写证据 → 0 → 建设期 normal SKIP /
        # close-out strict FAIL,对齐 g1.counter.d3d12_interop 先例。
        n = 0
        for f in (ROOT / "evidence").glob("host_orch_smoke*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("single_source") is True and doc.get("device_run") is True:
                n += 1
        count_or_gate(eid, n, 1, "份 single-source 宿主编排端到端证据", "MS1.2 device 真跑回填前为正常状态,契约 G-MS1-2", strict)
    elif eid == "ms1.counter.uc07_offline_golden_frames":
        # UC-07 离线 golden 端到端证据数 ≥1(契约 G-MS1-3/G-MS1-4;apps/ruridrop 零 .rs
        # 主语言判据审计 + 三层 golden(同机两跑逐帧 SHA-256 一致 / GPU vs refcpu 量化域
        # 容差 / blessed 哈希 == tests/uc07/golden_manifest)+ 篡改 sim_forces 重力常数经
        # 同一 rx build 链重编 digest 变红复原绿,RFC-0010 §4.1/§4.4,CI 步骤 53
        # ci/uc07_offline_golden_smoke.py);计数 = evidence/uc07_offline_golden*.json 中
        # digest_match=true 的报告数。无 CUDA 环境冒烟降级 SKIP 不写证据 → 0 → 建设期
        # normal SKIP / close-out strict FAIL,对齐 ms1.counter.host_orch_single_source 先例。
        n = 0
        for f in (ROOT / "evidence").glob("uc07_offline_golden*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("digest_match") is True:
                n += 1
        count_or_gate(eid, n, 1, "份 UC-07 离线 golden 端到端证据", "MS1.3 device 真跑回填前为正常状态,契约 G-MS1-4", strict)
    elif eid == "g3.counter.uc04_present_frames":
        # UC-04 可见窗口 flip-model swapchain present 端到端证据数 ≥1(契约 G-G3-2;RFC-0013
        # §4.A / RXS-0220~0222;可见窗口 WS_OVERLAPPEDWINDOW + CreateSwapChainForHwnd FLIP_DISCARD
        # 逐帧 present + RENDER_TARGET→COPY_SOURCE→PRESENT 迁移锚点 + SetWindowPos 合成 WM_SIZE→
        # ResizeBuffers 重建 + 三点 backbuffer readback 数值断言〔首/重建后/末帧〕,着色器 Rurix 源
        # 经图形=B DXIL 非手写);计数 = evidence/uc04_present_*.json 中 present_ok=true 的报告数
        # (ci/uc04_present_smoke.py device 段真跑写)。present 真跑 = 交互桌面人工链路不进 pr-smoke
        # 硬门(镜像 g1.counter.realtime_present / uc07_present 双态);无显示/无 GPU/未 opt-in →
        # device SKIP=dev-env degrade → 0 → 建设期 normal SKIP / close-out strict FAIL。
        n = 0
        for f in (ROOT / "evidence").glob("uc04_present_*.json"):
            doc = json.loads(f.read_text(encoding="utf-8"))
            if doc.get("present_ok") is True:
                n += 1
        count_or_gate(eid, n, 1, "份 UC-04 可见窗口 present 端到端证据",
                      "G3.2 present device 见证回填前为正常状态,契约 G-G3-2", strict)
    else:
        err(f"{eid}: 未知计数器断言,无对应 evaluator 实现")


def main() -> int:
    global ALLOW_PENDING_IDS
    strict, ALLOW_PENDING_IDS = parse_args(sys.argv[1:])
    merged = load_budgets()
    for entry in merged.values():
        group = entry["_group"]
        if group == "entries":
            eval_entry(entry, strict)
        elif group == "ratio_assertions":
            eval_ratio(entry, merged, strict)
        else:
            eval_counter(entry, strict)
    for line in PASSES:
        print(f"  PASS {line}")
    for line in SKIPS:
        print(f"  SKIP {line}")
    if ERRORS:
        print(f"[budget_eval] FAIL ({'strict' if strict else 'normal'} mode)")
        for e in ERRORS:
            print(f"  - {e}")
        return 1
    print(f"[budget_eval] PASS ({len(PASSES)} pass, {len(SKIPS)} skip, {'strict' if strict else 'normal'} mode)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
