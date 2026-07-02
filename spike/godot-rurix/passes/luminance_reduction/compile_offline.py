#!/usr/bin/env python3
"""Emit offline compile evidence for GRX-009 luminance reduction."""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import pathlib
import subprocess
import sys


PASS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = PASS_DIR.parents[3]
ARTIFACT_DIR = PASS_DIR / "artifacts"
DEBUG_ARTIFACT_DIR = ARTIFACT_DIR / "debug_artifacts"
PACKAGE_MANIFEST = PASS_DIR / "rurix.toml"
ENTRY_FILE = PASS_DIR / "src" / "lib.rx"
EVIDENCE_PATH = PASS_DIR / "offline_compile_evidence.json"
SCHEMA_PATH = PASS_DIR / "compile_evidence.schema.json"
DXIL_PATH = ARTIFACT_DIR / "luminance_reduction.dxil"
ROOT_SIGNATURE_PATH = ARTIFACT_DIR / "luminance_reduction.rts0.bin"
DESCRIPTOR_LAYOUT_PATH = ARTIFACT_DIR / "luminance_reduction_descriptor_layout.json"
STDOUT_PATH = ARTIFACT_DIR / "compile_stdout.txt"
STDERR_PATH = ARTIFACT_DIR / "compile_stderr.txt"
CURRENT_ARTIFACT_PATHS = [DXIL_PATH, ROOT_SIGNATURE_PATH, DESCRIPTOR_LAYOUT_PATH]
REFERENCE_FILES = [
    "external/godot-master/servers/rendering/renderer_rd/effects/luminance.cpp",
    "external/godot-master/servers/rendering/renderer_rd/shaders/effects/luminance_reduce.glsl",
]


def rel(path: pathlib.Path) -> str:
    return str(path.relative_to(ROOT)).replace("\\", "/")


def sha256_file(path: pathlib.Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_text_prefix(path: pathlib.Path, limit: int = 65536) -> str:
    if not path.is_file():
        return ""
    return path.read_bytes()[:limit].decode("utf-8", errors="ignore")


def classify_dxil_artifact(path: pathlib.Path) -> tuple[str, str]:
    if not path.is_file():
        return "missing", "unknown"
    text = read_text_prefix(path)
    if text.startswith("; ModuleID") or "target triple = \"dxil-unknown-shadermodel" in text:
        if "entry:\n  ret void" in text or "entry:\r\n  ret void" in text:
            return "dxil_ir_text", "entry_shell_only"
        return "dxil_ir_text", "unknown"
    return "dxil_container", "unknown"


def artifact_entry(
    path: pathlib.Path,
    artifact_kind: str,
    semantic_status: str | None = None,
    produced_by_current_run: bool | None = None,
) -> dict[str, object]:
    exists = path.is_file()
    entry: dict[str, object] = {
        "path": rel(path),
        "exists": exists,
        "sha256": sha256_file(path),
        "artifact_kind": artifact_kind,
        "produced_by_current_run": exists if produced_by_current_run is None else produced_by_current_run,
    }
    if semantic_status is not None:
        entry["semantic_status"] = semantic_status
    return entry


def ordinary_artifact_entry(path: pathlib.Path) -> dict[str, object]:
    exists = path.is_file()
    return {
        "path": rel(path),
        "exists": exists,
        "sha256": sha256_file(path),
        "artifact_kind": "binary" if exists else "missing",
        "produced_by_current_run": exists,
    }


def debug_artifact_entry(path: pathlib.Path) -> dict[str, object]:
    kind, semantic_status = classify_dxil_artifact(path)
    return artifact_entry(path, kind, semantic_status, True)


def validate_evidence(evidence: dict[str, object]) -> None:
    required = {
        "pass_id",
        "segment",
        "status",
        "runtime_state",
        "manifest_segment_after_run",
        "attempted_at_utc",
        "inputs",
        "commands",
        "artifacts",
    }
    missing = sorted(required.difference(evidence))
    if missing:
        raise ValueError(f"offline compile evidence missing required keys: {missing}")
    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("offline compile evidence.artifacts must be an object")
    for key in ("dxil", "root_signature", "descriptor_layout"):
        value = artifacts.get(key)
        if not isinstance(value, dict):
            raise ValueError(f"offline compile evidence.artifacts.{key} must be an object")
        for inner in ("path", "exists", "sha256", "artifact_kind", "produced_by_current_run"):
            if inner not in value:
                raise ValueError(
                    f"offline compile evidence.artifacts.{key} missing `{inner}`"
                )
    dxil = artifacts["dxil"]
    status = evidence.get("status")
    if status == "success":
        if evidence.get("manifest_segment_after_run") != 3:
            raise ValueError("success evidence must advance manifest_segment_after_run to 3")
        if dxil.get("artifact_kind") != "dxil_container":
            raise ValueError("success evidence requires dxil artifact_kind=dxil_container")
        if dxil.get("semantic_status") == "entry_shell_only":
            raise ValueError("success evidence cannot use entry_shell_only DXIL artifact")
        for key in ("dxil", "root_signature", "descriptor_layout"):
            if artifacts[key]["exists"] is not True:
                raise ValueError(f"success evidence requires `{key}` artifact to exist")
            if artifacts[key].get("produced_by_current_run") is not True:
                raise ValueError(
                    f"success evidence requires `{key}` produced_by_current_run=true"
                )
    elif status in {"compile_failed", "toolchain_missing"}:
        if evidence.get("manifest_segment_after_run") != 2:
            raise ValueError(
                f"{status} evidence must keep manifest_segment_after_run at 2"
            )
        if not evidence.get("blocker_category") or not evidence.get("blocker_summary"):
            raise ValueError(
                f"{status} evidence requires blocker_category and blocker_summary"
            )
    else:
        raise ValueError(f"unexpected evidence status: {status!r}")


def classify_failure(
    exit_code: int,
    stderr_text: str,
    dxil_kind: str,
    semantic_status: str,
) -> tuple[str, str]:
    lowered = stderr_text.lower()
    if "patched llc not found" in lowered or "dxc validator not found" in lowered:
        return (
            "toolchain_missing",
            "The offline compile attempt did not produce a validated DXIL container because the patched llc or dxc validator path was unavailable.",
        )
    if "threadctx.global_id" in lowered or "threadctx body lowering" in lowered:
        return (
            "threadctx_global_id_lowering_missing",
            "The DXIL compute backend advanced past scalar if/select lowering and now rejects ThreadCtx.global_id body lowering in the luminance kernel.",
        )
    if "不支持尾表达式" in stderr_text and "if" in lowered:
        return (
            "complex_if_lowering_missing",
            "The DXIL compute backend now lowers ThreadCtx.global_id in the luminance kernel, and the first remaining blocker is the tail if statement body lowering.",
        )
    if "取模" in stderr_text or "modulo" in lowered:
        return (
            "modulo_lowering_missing",
            "The DXIL compute backend now lowers the tail if statement body in the luminance kernel, and the first remaining blocker is `%` modulo lowering.",
        )
    if "不支持 while" in stderr_text or "unsupported while" in lowered:
        return (
            "while_lowering_missing",
            "The DXIL compute backend now lowers integer `%` modulo in the luminance kernel, and the first remaining blocker is while-loop body lowering.",
        )
    if "body lowering" in lowered or "entry shell compile success" in lowered:
        return (
            "body_lowering_missing",
            "The DXIL compute backend rejected a non-trivial compute body because real body lowering is not implemented yet.",
        )
    if dxil_kind == "dxil_ir_text":
        if semantic_status == "entry_shell_only":
            return (
                "body_lowering_missing",
                "The DXIL artifact is LLVM IR text containing only an entry shell, not a real lowered luminance compute body.",
            )
        return (
            "dxil_container_missing",
            "The DXIL artifact is LLVM IR text rather than a real DXIL container.",
        )
    if dxil_kind != "dxil_container":
        return (
            "dxil_container_missing",
            "The offline compile attempt did not leave a real DXIL container artifact.",
        )
    if "暂不支持带形参的 compute 入口" in stderr_text or "unsupported: dxil" in lowered:
        return (
            "unsupported_compute_entry_params",
            "The real offline compile attempt reached the DXIL backend, but the "
            "current minimal compute path still rejects parameterized compute kernel "
            "entries such as the luminance reduction draft.",
        )
    if "no compute `kernel fn` found" in lowered or "no `kernel fn` found" in lowered:
        return (
            "kernel_entry_not_found",
            "The offline compile attempt did not find a usable compute kernel entry.",
        )
    if exit_code != 0:
        return (
            "compute_backend_capability_gap",
            "The real offline compile attempt failed inside the current compute DXIL path; "
            "see compile stderr for the blocker details.",
        )
    return (
        "descriptor_artifacts_missing",
        "The offline compile attempt completed without a hard failure, but the required "
        "DXIL/root signature/descriptor layout artifact set is still incomplete.",
    )


def _schema_type_ok(value: object, type_spec: object) -> bool:
    """draft-07 `type` 关键字机械校验(仅覆盖本 schema 用到的类型)。"""
    types = type_spec if isinstance(type_spec, list) else [type_spec]
    for t in types:
        if t == "object" and isinstance(value, dict):
            return True
        if t == "array" and isinstance(value, list):
            return True
        if t == "string" and isinstance(value, str):
            return True
        # bool 是 int 的子类,integer 校验须显式排除 bool。
        if t == "integer" and isinstance(value, int) and not isinstance(value, bool):
            return True
        if t == "boolean" and isinstance(value, bool):
            return True
        if t == "null" and value is None:
            return True
    return False


def _validate_node(value: object, schema: dict, defs: dict, path: str) -> None:
    """按 schema 节点机械校验(覆盖 $ref/const/enum/type/required/properties/
    additionalProperties/minItems/items/allOf-if-then 子集,足够本 evidence schema)。"""
    if "$ref" in schema:
        ref = schema["$ref"]
        if not isinstance(ref, str) or not ref.startswith("#/definitions/"):
            raise ValueError(f"{path}: 不支持的 $ref {ref!r}")
        _validate_node(value, defs[ref[len("#/definitions/"):]], defs, path)
        return
    if "const" in schema and value != schema["const"]:
        raise ValueError(f"{path}: 期望 const {schema['const']!r},实得 {value!r}")
    if "enum" in schema and value not in schema["enum"]:
        raise ValueError(f"{path}: {value!r} 不在 enum {schema['enum']!r} 内")
    if "type" in schema and not _schema_type_ok(value, schema["type"]):
        raise ValueError(f"{path}: 类型不符 {schema['type']!r}(实得 {type(value).__name__})")
    if isinstance(value, dict):
        for req in schema.get("required", []):
            if req not in value:
                raise ValueError(f"{path}: 缺必需键 `{req}`")
        props = schema.get("properties", {})
        for key, sub in props.items():
            if key in value:
                _validate_node(value[key], sub, defs, f"{path}.{key}")
        if schema.get("additionalProperties") is False:
            extra = sorted(set(value) - set(props))
            if extra:
                raise ValueError(f"{path}: 不允许的额外键 {extra}")
    if isinstance(value, list):
        min_items = schema.get("minItems")
        if isinstance(min_items, int) and len(value) < min_items:
            raise ValueError(f"{path}: 数组长度 {len(value)} < minItems {min_items}")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for i, elem in enumerate(value):
                _validate_node(elem, item_schema, defs, f"{path}[{i}]")
    for clause in schema.get("allOf", []):
        if not isinstance(clause, dict):
            continue
        cond = clause.get("if")
        then = clause.get("then")
        if isinstance(cond, dict) and isinstance(then, dict):
            if _matches(value, cond, defs):
                _validate_node(value, then, defs, path)
        else:
            _validate_node(value, clause, defs, path)


def _matches(value: object, schema: dict, defs: dict) -> bool:
    """`if` 子句匹配判定(本 schema 仅用 properties.const 形态)。"""
    try:
        _validate_node(value, schema, defs, "<if>")
    except ValueError:
        return False
    return True


def validate_against_schema(evidence: dict[str, object], schema_path: pathlib.Path) -> None:
    """按 compile_evidence.schema.json 机械校验 evidence(等价 reader,零第三方依赖)。

    覆盖本 schema 实际使用的 draft-07 子集:顶层 required、const/enum、
    artifacts 结构 + additionalProperties:false、以及 allOf 的
    success→segment3/exists、compile_failed→segment2/blocker 条件约束。
    """
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    defs = schema.get("definitions", {})
    _validate_node(evidence, schema, defs, "evidence")


def main() -> int:
    if not PACKAGE_MANIFEST.is_file():
        raise SystemExit(f"missing package manifest: {PACKAGE_MANIFEST}")
    if not ENTRY_FILE.is_file():
        raise SystemExit(f"missing entry file: {ENTRY_FILE}")
    if not SCHEMA_PATH.is_file():
        raise SystemExit(f"missing compile evidence schema: {SCHEMA_PATH}")

    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    DEBUG_ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    for path in CURRENT_ARTIFACT_PATHS:
        if path.is_file():
            path.unlink()

    command = [
        "cargo",
        "run",
        "-p",
        "rurixc",
        "--bin",
        "rurixc",
        "--features",
        "dxil-backend shader-stages",
        "--",
        str(ENTRY_FILE),
        "--target",
        "dxil",
        "-o",
        str(DXIL_PATH),
    ]
    proc = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    # LF 写出(仓库 LF byte-exact;text 模式 write_text 在 Windows 会翻译成 CRLF)。
    with STDOUT_PATH.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(proc.stdout or "")
    with STDERR_PATH.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(proc.stderr or "")

    dxil_kind, semantic_status = classify_dxil_artifact(DXIL_PATH)
    debug_artifacts: dict[str, object] = {}
    if dxil_kind == "dxil_ir_text":
        debug_dxil_path = DEBUG_ARTIFACT_DIR / DXIL_PATH.name
        DXIL_PATH.replace(debug_dxil_path)
        debug_artifacts["dxil_ir_text"] = debug_artifact_entry(debug_dxil_path)
        dxil_kind, semantic_status = classify_dxil_artifact(DXIL_PATH)
    dxil_entry = artifact_entry(DXIL_PATH, dxil_kind, semantic_status)
    root_signature_entry = ordinary_artifact_entry(ROOT_SIGNATURE_PATH)
    descriptor_layout_entry = ordinary_artifact_entry(DESCRIPTOR_LAYOUT_PATH)
    stderr_lower = (proc.stderr or "").lower()
    skipped_toolchain = "patched llc not found" in stderr_lower or "dxc validator not found" in stderr_lower or "skipped" in stderr_lower

    success = (
        proc.returncode == 0
        and dxil_entry["exists"] is True
        and dxil_entry["artifact_kind"] == "dxil_container"
        and dxil_entry.get("semantic_status") != "entry_shell_only"
        and root_signature_entry["exists"] is True
        and descriptor_layout_entry["exists"] is True
        and not skipped_toolchain
    )

    blocker_category = None
    blocker_summary = None
    manifest_segment_after_run = 3 if success else 2
    status = "success" if success else "compile_failed"
    if not success and skipped_toolchain:
        status = "toolchain_missing"
    if not success:
        blocker_category, blocker_summary = classify_failure(
            proc.returncode,
            proc.stderr or "",
            str(dxil_entry["artifact_kind"]),
            str(dxil_entry.get("semantic_status") or "unknown"),
        )

    evidence = {
        "pass_id": "luminance_reduction",
        "segment": "3a",
        "status": status,
        "runtime_state": "fallback_only",
        "manifest_segment_after_run": manifest_segment_after_run,
        "attempted_at_utc": dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "inputs": {
            "package_manifest": rel(PACKAGE_MANIFEST),
            "entry_file": rel(ENTRY_FILE),
            "godot_reference_files": REFERENCE_FILES,
        },
        "commands": [
            {
                "label": "cargo_run_rurixc_target_dxil",
                "argv": command,
                "exit_code": proc.returncode,
                "stdout_path": rel(STDOUT_PATH),
                "stderr_path": rel(STDERR_PATH),
            }
        ],
        "artifacts": {
            "dxil": dxil_entry,
            "root_signature": root_signature_entry,
            "descriptor_layout": descriptor_layout_entry,
        },
        "blocker_category": blocker_category,
        "blocker_summary": blocker_summary,
        "notes": [
            "Runtime remains fallback_only for GRX-009 segment 3a.",
            "Current artifacts describe only files produced by this compile attempt after stale outputs were cleared.",
            "Do not advance pass_manifest to segment 3 unless the DXIL artifact is a real container produced by the current run and not dxil_ir_text.",
            "debug_artifacts and entry_shell_only IR are debugging evidence, not real luminance pass compile success.",
        ],
    }
    if debug_artifacts:
        evidence["debug_artifacts"] = debug_artifacts

    validate_evidence(evidence)
    validate_against_schema(evidence, SCHEMA_PATH)
    # LF 写出(仓库 LF byte-exact;text 模式 write_text 在 Windows 会翻译成 CRLF)。
    with EVIDENCE_PATH.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(evidence, indent=2, ensure_ascii=True) + "\n")

    print(f"[grx009] wrote offline compile evidence: {EVIDENCE_PATH}")
    print(f"[grx009] status: {status}")
    if blocker_summary:
        print(f"[grx009] blocker: {blocker_category}: {blocker_summary}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
