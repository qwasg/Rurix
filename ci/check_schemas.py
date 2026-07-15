"""PR Smoke 步骤 2:注册表/预算/证据 JSON 的 schema 校验(CI_GATES.md §3.2)。

- registry/deferred.json / spike_gating.json:结构字段与编号格式;
- milestones/*/m*_budget.json:结构 + 命名空间强制前缀(14 §3);
- evidence/*.json:对 milestones/m0/evidence_schema.json 做 JSON Schema 校验。
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []


def err(msg: str) -> None:
    ERRORS.append(msg)


def load(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001 - 报告而非崩溃
        err(f"{path.relative_to(ROOT)}: 无法解析 JSON: {e}")
        return None


def check_deferred(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"RD-\d{3}", eid):
            err(f"deferred: 编号格式非法: {eid!r}")
        if eid in seen:
            err(f"deferred: 编号重复: {eid}")
        seen.add(eid)
        for field in ("title", "reason", "backfill_condition", "owner_milestone", "status", "history"):
            if field not in entry:
                err(f"deferred {eid}: 缺字段 {field}")
        if entry.get("status") not in ("open", "inherited", "closed"):
            err(f"deferred {eid}: status 非法: {entry.get('status')!r}")
        if not entry.get("history"):
            err(f"deferred {eid}: history 不得为空(留痕要求,14 §4)")


def check_gating(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"SG-\d{3}", eid):
            err(f"spike_gating: 编号格式非法: {eid!r}")
        if eid in seen:
            err(f"spike_gating: 编号重复: {eid}")
        seen.add(eid)
        for field in ("direction", "trigger_condition", "permanence", "current_verdict", "decisions"):
            if field not in entry:
                err(f"spike_gating {eid}: 缺字段 {field}")
        if entry.get("permanence") not in ("permanent", "conditional"):
            err(f"spike_gating {eid}: permanence 非法")
        if not entry.get("decisions"):
            err(f"spike_gating {eid}: decisions 不得为空(留痕要求,14 §7)")


def parse_message_keys(path: Path) -> set[str] | None:
    """解析 rurixc 消息表行格式(key = 模板;# 注释),返回 key 集。"""
    if not path.is_file():
        return None
    keys: set[str] = set()
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            err(f"messages: 第 {lineno} 行缺 '=': {line!r}")
            continue
        key = line.split("=", 1)[0].strip()
        if not key or any(c.isspace() for c in key):
            err(f"messages: 第 {lineno} 行 key 非法: {key!r}")
            continue
        if key in keys:
            err(f"messages: key 重复: {key}")
        keys.add(key)
    return keys


def check_error_codes(path: Path) -> None:
    """错误码注册表校验(07 §5 分配制;M1 CI_GATES §2 步骤 11)。"""
    if not path.is_file():
        return  # M1.1 落地前不存在,放行
    data = load(path)
    if data is None:
        return
    message_keys = parse_message_keys(ROOT / "src/rurixc/src/messages/en.messages")
    seen: set[str] = set()
    for entry in data.get("entries", []):
        eid = entry.get("id", "")
        if not re.fullmatch(r"RX\d{4}", eid):
            err(f"error_codes: 编号格式非法: {eid!r}")
        elif eid[2] not in "01234567":
            err(f"error_codes {eid}: 段位非法(0-7,07 §5)")
        if eid in seen:
            err(f"error_codes: 编号重复: {eid}(编号永不复用,10 §9.5)")
        seen.add(eid)
        for field in ("title", "message_key", "status", "introduced_in"):
            if not entry.get(field):
                err(f"error_codes {eid}: 缺字段 {field}")
        if entry.get("status") not in ("active", "deprecated"):
            err(f"error_codes {eid}: status 非法: {entry.get('status')!r}")
        mk = entry.get("message_key")
        if mk and message_keys is not None and mk not in message_keys:
            err(f"error_codes {eid}: message_key 未在 en.messages 注册: {mk!r}")


def check_budget(path: Path) -> None:
    data = load(path)
    if data is None:
        return
    ns = data.get("namespace")
    if not ns:
        err(f"{path.name}: 缺 namespace 字段")
        return
    prefix = ns + "."
    ids: set[str] = set()
    groups = ("entries", "ratio_assertions", "counter_assertions")
    for group in groups:
        for entry in data.get(group, []):
            eid = entry.get("id", "")
            if not eid.startswith(prefix):
                err(f"{path.name}: id {eid!r} 未带强制前缀 {prefix!r}(14 §3)")
            if eid in ids:
                err(f"{path.name}: id 重复(命名空间冲突): {eid}")
            ids.add(eid)
    for entry in data.get("entries", []):
        ev = entry.get("evidence")
        if ev not in ("measured_local", "unlocked", "estimated"):
            err(f"{path.name} {entry.get('id')}: evidence 非法: {ev!r}")
        if ev == "estimated" and not entry.get("skip_reason"):
            err(f"{path.name} {entry.get('id')}: estimated 占位必须输出 skip_reason(14 §3)")
        if ev == "measured_local":
            if entry.get("threshold") is None:
                err(f"{path.name} {entry.get('id')}: measured_local 必须有 threshold")
            if not entry.get("evidence_file"):
                err(f"{path.name} {entry.get('id')}: measured_local 必须登记 evidence_file")
            elif not (ROOT / entry["evidence_file"]).is_file():
                err(f"{path.name} {entry.get('id')}: evidence_file 不存在: {entry['evidence_file']}")


def check_evidence_files() -> None:
    gpu_schema = load(ROOT / "milestones/m0/evidence_schema.json")
    frontend_schema = load(ROOT / "milestones/m1/frontend_evidence_schema.json")
    compile_schema = load(ROOT / "milestones/m3/compile_evidence_schema.json")
    sanitizer_schema = load(ROOT / "milestones/m5/compute_sanitizer_evidence_schema.json")
    redistribution_schema = load(ROOT / "milestones/m5/redistribution_audit_evidence_schema.json")
    rx_cli_smoke_schema = load(ROOT / "milestones/m6/rx_cli_smoke_evidence_schema.json")
    offline_rebuild_schema = load(ROOT / "milestones/m6/offline_rebuild_evidence_schema.json")
    lsp_smoke_schema = load(ROOT / "milestones/m6/lsp_smoke_evidence_schema.json")
    lsp_latency_schema = load(ROOT / "milestones/m6/lsp_latency_evidence_schema.json")
    stdlib_math_schema = load(ROOT / "milestones/m7/stdlib_math_evidence_schema.json")
    soft_raster_schema = load(ROOT / "milestones/m7/soft_raster_evidence_schema.json")
    uc03_demo_schema = load(ROOT / "milestones/m7/uc03_demo_evidence_schema.json")
    uc01_interop_schema = load(ROOT / "milestones/m8/uc01_interop_evidence_schema.json")
    cublas_binding_schema = load(ROOT / "milestones/m8/cublas_binding_evidence_schema.json")
    uc02_stream_pipeline_schema = load(
        ROOT / "milestones/m8/uc02_stream_pipeline_evidence_schema.json"
    )
    release_schema = load(ROOT / "milestones/m8/release_evidence_schema.json")
    bilingual_schema = load(
        ROOT / "milestones/m8/bilingual_diagnostic_coverage_evidence_schema.json"
    )
    doc_site_schema = load(ROOT / "milestones/m8/doc_site_smoke_evidence_schema.json")
    d3d12_interop_schema = load(ROOT / "milestones/g1/d3d12_interop_evidence_schema.json")
    realtime_present_schema = load(ROOT / "milestones/g1/realtime_present_evidence_schema.json")
    async_buffer_schema = load(ROOT / "milestones/g1/async_buffer_evidence_schema.json")
    engine_integration_schema = load(ROOT / "milestones/g1/engine_integration_evidence_schema.json")
    fatbin_dist_schema = load(ROOT / "milestones/g1/fatbin_dist_evidence_schema.json")
    dxil_path_spike_schema = load(ROOT / "milestones/g2/dxil_path_spike_evidence_schema.json")
    dxil_b_graphics_sig_schema = load(ROOT / "milestones/g2/dxil_b_graphics_sig_evidence_schema.json")
    dxil_b_strict_only_schema = load(ROOT / "milestones/g2/dxil_b_strict_only_evidence_schema.json")
    dxil_a_graphics_sig_effort_schema = load(
        ROOT / "milestones/g2/dxil_a_graphics_sig_effort_evidence_schema.json"
    )
    rd017_varying_semantic_spike_schema = load(
        ROOT / "milestones/g2/rd017_varying_semantic_spike_evidence_schema.json"
    )
    host_orch_smoke_schema = load(
        ROOT / "milestones/ms1/host_orch_smoke_evidence_schema.json"
    )
    if (gpu_schema is None or frontend_schema is None or compile_schema is None
            or sanitizer_schema is None or redistribution_schema is None
            or rx_cli_smoke_schema is None or offline_rebuild_schema is None
            or lsp_smoke_schema is None or lsp_latency_schema is None
            or stdlib_math_schema is None or soft_raster_schema is None
            or uc03_demo_schema is None or uc01_interop_schema is None
            or cublas_binding_schema is None or uc02_stream_pipeline_schema is None
            or release_schema is None or bilingual_schema is None
            or doc_site_schema is None):
        return
    evidence_files = sorted((ROOT / "evidence").glob("*.json"))
    if not evidence_files:
        print("[check_schemas] evidence/ 暂无证据文件(M0.3 前为正常状态)")
        return
    try:
        import jsonschema
    except ImportError:
        err("缺 jsonschema 依赖(pip install -r requirements.txt)")
        return
    gpu_validator = jsonschema.Draft7Validator(gpu_schema)
    frontend_validator = jsonschema.Draft7Validator(frontend_schema)
    compile_validator = jsonschema.Draft7Validator(compile_schema)
    sanitizer_validator = jsonschema.Draft7Validator(sanitizer_schema)
    redistribution_validator = jsonschema.Draft7Validator(redistribution_schema)
    rx_cli_smoke_validator = jsonschema.Draft7Validator(rx_cli_smoke_schema)
    offline_rebuild_validator = jsonschema.Draft7Validator(offline_rebuild_schema)
    lsp_smoke_validator = jsonschema.Draft7Validator(lsp_smoke_schema)
    lsp_latency_validator = jsonschema.Draft7Validator(lsp_latency_schema)
    stdlib_math_validator = jsonschema.Draft7Validator(stdlib_math_schema)
    soft_raster_validator = jsonschema.Draft7Validator(soft_raster_schema)
    uc03_demo_validator = jsonschema.Draft7Validator(uc03_demo_schema)
    uc01_interop_validator = jsonschema.Draft7Validator(uc01_interop_schema)
    cublas_binding_validator = jsonschema.Draft7Validator(cublas_binding_schema)
    uc02_stream_pipeline_validator = jsonschema.Draft7Validator(uc02_stream_pipeline_schema)
    release_validator = jsonschema.Draft7Validator(release_schema)
    bilingual_validator = jsonschema.Draft7Validator(bilingual_schema)
    doc_site_validator = jsonschema.Draft7Validator(doc_site_schema)
    d3d12_interop_validator = jsonschema.Draft7Validator(d3d12_interop_schema)
    realtime_present_validator = jsonschema.Draft7Validator(realtime_present_schema)
    async_buffer_validator = (
        jsonschema.Draft7Validator(async_buffer_schema) if async_buffer_schema else None
    )
    engine_integration_validator = (
        jsonschema.Draft7Validator(engine_integration_schema)
        if engine_integration_schema
        else None
    )
    fatbin_dist_validator = (
        jsonschema.Draft7Validator(fatbin_dist_schema) if fatbin_dist_schema else None
    )
    dxil_path_spike_validator = (
        jsonschema.Draft7Validator(dxil_path_spike_schema) if dxil_path_spike_schema else None
    )
    dxil_b_graphics_sig_validator = (
        jsonschema.Draft7Validator(dxil_b_graphics_sig_schema)
        if dxil_b_graphics_sig_schema
        else None
    )
    dxil_b_strict_only_validator = (
        jsonschema.Draft7Validator(dxil_b_strict_only_schema)
        if dxil_b_strict_only_schema
        else None
    )
    dxil_a_graphics_sig_effort_validator = (
        jsonschema.Draft7Validator(dxil_a_graphics_sig_effort_schema)
        if dxil_a_graphics_sig_effort_schema
        else None
    )
    rd017_varying_semantic_spike_validator = (
        jsonschema.Draft7Validator(rd017_varying_semantic_spike_schema)
        if rd017_varying_semantic_spike_schema
        else None
    )
    host_orch_smoke_validator = (
        jsonschema.Draft7Validator(host_orch_smoke_schema)
        if host_orch_smoke_schema
        else None
    )
    for f in evidence_files:
        doc = load(f)
        if doc is None:
            continue
        # 路由(按文件名前缀):frontend_ → m1 前端 schema;compile_ → m3 编译
        # schema(G-M3-3 配套);compute_sanitizer_ → m5 Sanitizer schema
        # (G-M5-4 配套);redistribution_audit_ → m5 再分发审计 schema
        # (CI_GATES §4 第 2 项配套);rx_cli_smoke_ → m6 rx CLI 子命令冒烟 schema
        # (G-M6-3 配套);offline_rebuild_ → m6 离线重建复现 schema
        # (G-M6-1 配套);lsp_smoke_ → m6 LSP 能力面冒烟 schema
        # (G-M6-2/G-M6-5 配套);lsp_latency_ → m6 LSP 10k 行交互延迟 schema
        # (G-M6-2 measured_local 配套);stdlib_math_ → m7 core 数学库原语冒烟
        # schema(G-M7-4 配套,m7.counter.math_primitives);soft_raster_ → m7
        # 软光栅 kernel safe 覆盖 + 确定性帧像素冒烟 schema(G-M7-3 配套,
        # m7.counter.soft_raster_kernels_safe);uc03_demo_ → m7 UC-03 demo 单 EXE +
        # 确定性图像序列冒烟 schema(G-M7-1 配套,m7.counter.uc03_demo_image_sequence);
        # uc01_/cublas_/uc02_ → m8 互操作/cublas/UC-02 流水线 schema;release_ → m8
        # 发布链路签名/SBOM/许可审计冒烟 schema(G-M8-4 配套,m8.counter.release_artifacts_signed);
        # bilingual_ → m8 诊断双语全量覆盖 schema(G-M8-5/RD-006 配套,
        # m8.counter.bilingual_diagnostic_coverage);其余 → m0 GPU schema
        if f.name.startswith("frontend_"):
            validator = frontend_validator
        elif f.name.startswith("compile_"):
            validator = compile_validator
        elif f.name.startswith("compute_sanitizer_"):
            validator = sanitizer_validator
        elif f.name.startswith("redistribution_audit_"):
            validator = redistribution_validator
        elif f.name.startswith("rx_cli_smoke_"):
            validator = rx_cli_smoke_validator
        elif f.name.startswith("offline_rebuild_"):
            validator = offline_rebuild_validator
        elif f.name.startswith("lsp_smoke_"):
            validator = lsp_smoke_validator
        elif f.name.startswith("lsp_latency_"):
            validator = lsp_latency_validator
        elif f.name.startswith("stdlib_math_"):
            validator = stdlib_math_validator
        elif f.name.startswith("soft_raster_"):
            validator = soft_raster_validator
        elif f.name.startswith("uc03_demo_"):
            validator = uc03_demo_validator
        elif f.name.startswith("uc01_"):
            validator = uc01_interop_validator
        elif f.name.startswith("cublas_"):
            validator = cublas_binding_validator
        elif f.name.startswith("uc02_"):
            validator = uc02_stream_pipeline_validator
        elif f.name.startswith("release_"):
            validator = release_validator
        elif f.name.startswith("bilingual_"):
            validator = bilingual_validator
        elif f.name.startswith("doc_"):
            validator = doc_site_validator
        elif f.name.startswith("d3d12_interop_"):
            validator = d3d12_interop_validator
        elif f.name.startswith("realtime_present_"):
            validator = realtime_present_validator
        elif f.name.startswith("async_buffer_") and async_buffer_validator is not None:
            validator = async_buffer_validator
        elif (
            f.name.startswith("engine_integration_")
            and engine_integration_validator is not None
        ):
            validator = engine_integration_validator
        elif (
            f.name.startswith("fatbin_dist_")
            and fatbin_dist_validator is not None
        ):
            validator = fatbin_dist_validator
        elif (
            f.name.startswith("dxil_a_graphics_sig_effort_")
            and dxil_a_graphics_sig_effort_validator is not None
        ):
            # G2.2 A 路图形签名工作量评估 spike 证据(RD-010;RFC-0003 §9 Q-D131=A /
            # issue #90504 / #57928)→ milestones/g2/dxil_a_graphics_sig_effort_evidence_schema.json
            # (measured-first / blocked-honest,纯评估 spike 非性能基准;源码勘察 + 上游状态 +
            # 禁区vs conformance 裁断 + 分档工作量 estimated + carry-patch + PoC 锚定;
            # 不入 budget counter,A/B/混合架构结论留 owner)
            validator = dxil_a_graphics_sig_effort_validator
        elif (
            f.name.startswith("dxil_b_strict_only_")
            and dxil_b_strict_only_validator is not None
        ):
            # G2.2 B 路 strict-only 达标取证证据(RD-014;RFC-0004 §4.4 / 04 P-01 / P-13)→
            # milestones/g2/dxil_b_strict_only_evidence_schema.json(measured-first /
            # blocked-honest,纯取证非性能基准;语义名保持配置 b_keep vs 默认 b_default vs direct
            # 三链签名 part dump 对照,证语言层零静默降级能否不靠 P-01 例外达标;不入 budget
            # counter,P-01 规范线 / A/B / ②③契约线归属裁断留 owner)
            validator = dxil_b_strict_only_validator
        elif (
            f.name.startswith("dxil_b_graphics_sig_")
            and dxil_b_graphics_sig_validator is not None
        ):
            # G2.2 B 路图形签名能力取证证据(RD-010;RFC-0003 §9 Q-D131 / §7 B 路)→
            # milestones/g2/dxil_b_graphics_sig_evidence_schema.json(measured-first /
            # blocked-honest,纯取证非性能基准;ISG1/OSG1 签名 part dump 对照 A elemcount=0,
            # 不入 budget counter,A/B/混合架构结论留 owner)
            validator = dxil_b_graphics_sig_validator
        elif (
            f.name.startswith("dxil_path_spike_")
            and dxil_path_spike_validator is not None
        ):
            # G2.2 Q-D131=C 双路 DXIL spike 取证证据(RD-010;RFC-0003 §9 Q-D131)→
            # milestones/g2/dxil_path_spike_evidence_schema.json(measured-first /
            # blocked-honest,纯取证非性能基准;不入 budget counter,A/B 结论留 owner)
            validator = dxil_path_spike_validator
        elif (
            f.name.startswith("host_orch_smoke")
            and host_orch_smoke_validator is not None
        ):
            # MS1.2 single-source 宿主编排冒烟证据(G-MS1-2;RFC-0009 / RXS-0189~0196)→
            # milestones/ms1/host_orch_smoke_evidence_schema.json(CI 步骤 52
            # ci/host_orch_smoke.py 仅 device 段真跑时写;host .rx 经 std::gpu 编排 +
            # 同源 kernel PTX 嵌入单 EXE,device 真跑数值自校验 + 篡改 PTX/桩化写回
            # 双红绿;single_source=true 且 device_run=true 计入
            # ms1.counter.host_orch_single_source,ci/budget_eval.py)
            validator = host_orch_smoke_validator
        elif (
            f.name.startswith("rd017_varying_semantic_spike_")
            and rd017_varying_semantic_spike_validator is not None
        ):
            # G2.4 RD-017 varying 语义名保名机制 spike 证据(owner ruling 选项① HLSL 边界
            # 改写 / 否决③)→ milestones/g2/rd017_varying_semantic_spike_evidence_schema.json
            # (measured-first / blocked-honest,纯取证非性能基准;输出/片元输入 varying 用户名
            # 经 HLSL 边界改写后 dxc 接受 + signature_gate 不放宽也过 + 物理 ABI 不变 + 确定性,
            # 不入 budget counter;golden bless / device 真跑 / RD-017 状态翻转留 owner,G-G2-4)
            validator = rd017_varying_semantic_spike_validator
        else:
            validator = gpu_validator
        for v in validator.iter_errors(doc):
            err(f"evidence/{f.name}: {'/'.join(str(p) for p in v.path)}: {v.message}")


def main() -> int:
    check_deferred(ROOT / "registry/deferred.json")
    check_gating(ROOT / "registry/spike_gating.json")
    check_error_codes(ROOT / "registry/error_codes.json")
    for budget in sorted(ROOT.glob("milestones/*/*_budget.json")):
        check_budget(budget)
    check_evidence_files()
    if ERRORS:
        print("[check_schemas] FAIL")
        for e in ERRORS:
            print(f"  - {e}")
        return 1
    print("[check_schemas] PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
