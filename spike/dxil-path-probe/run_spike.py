# SPIKE(RD-010) — 编排器:跑 A/B 双路探针,产 evidence/dxil_path_spike_<date>.json。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# measured-first / blocked-honest;A/B 结论留 owner(硬规则 1),本编排器只汇总证据、不代决。
"""DXIL path spike 编排器。

用法:py -3 spike/dxil-path-probe/run_spike.py
产出:evidence/dxil_path_spike_<YYYYMMDD>.json(严格符合
      milestones/g2/dxil_path_spike_evidence_schema.json;经 ci/check_schemas.py 校验)。

顶层 status:两路皆 measured_local → measured_local;否则 blocked。
所有写入用二进制模式 + 显式 LF(\\n),禁文本模式(.gitattributes * -text,LF 字节精确)。
"""
from __future__ import annotations

import datetime
import json
import os
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent  # spike/dxil-path-probe/ → repo root
sys.path.insert(0, str(HERE))

import probe_a_llvm_directx as pa  # noqa: E402
import probe_b_spirv_to_dxil as pb  # noqa: E402


def build_evidence() -> dict:
    a = pa.probe()
    b = pb.probe()
    a_res = a["result"]
    b_res = b["result"]

    top_status = "measured_local" if (a_res["status"] == "measured_local" and b_res["status"] == "measured_local") else "blocked"

    now = datetime.datetime.now().astimezone()
    timestamp = now.strftime("%Y-%m-%dT%H:%M:%S%z")
    # 插入冒号到时区偏移(ISO 8601 / RFC3339,schema format date-time)
    timestamp = timestamp[:-2] + ":" + timestamp[-2:]

    tool_versions = {
        "clang": a["versions_subset"]["clang"],
        "llc": a["versions_subset"]["llc"],
        "dxc": a["versions_subset"]["dxc"],
        "dxv": a["versions_subset"]["dxv"],
        "spirv_to_dxil": b["versions_subset"]["spirv_to_dxil"],
        "spirv_cross": b["versions_subset"]["spirv_cross"],
        "spirv_producer": b["versions_subset"]["spirv_producer"],
    }

    return {
        "schema_version": 1,
        "subject": "dxil_path_spike",
        "status": top_status,
        "rd_ref": "RD-010",
        "decision_ref": "RFC-0003 §9 Q-D131=C / 13 §D-131",
        "timestamp": timestamp,
        "host_env": {
            "os": f"{os.name} / {sys.platform}",
            "clang_source": a["clang_source"],
        },
        "tool_versions": tool_versions,
        "path_a": a_res,
        "path_b": b_res,
        "comparison_criteria": [
            "target_or_translator_available",
            "dxil_emit_ok",
            "validator_pass(dxc/dxv)",
            "shader_model_coverage",
            "supply_chain_cost",
            "determinism_strict_only_fidelity",
            "fit_with_D-205_single_llvm_stack",
        ],
        "run_command": "py -3 spike/dxil-path-probe/run_spike.py",
        "owner_decision": "A/B 最终路径裁决权属 owner(RFC-0003 §9 Q-D131 / 13 §D-131 / AGENTS 硬规则 1);本 spike 仅产证据基底,AI 不代决。owner 裁定后回填 RFC-0003 §9 + 13 §D-131(经勘误 PR)+ close RD-010,再进 PR-C1 spec 脚手架。",
        "notes": "纯取证 spike,非性能基准、非常驻 CI 门。探针隔离于 spike/dxil-path-probe/ 标 // SPIKE(RD-010),不入 src/ 生产路径、不随产品编译、spike 结束可弃。measured-first / blocked-honest:工具/target 探到记实测,探不到如实 blocked + repro,绝不杜撰(AGENTS 硬规则 3/4)。",
    }


def main() -> int:
    ev = build_evidence()
    date_tag = datetime.datetime.now().strftime("%Y%m%d")
    # round 后缀(env RURIX_SPIKE_SUFFIX,默认 _r4):避免覆盖既有证据(round-2 的
    # dxil_path_spike_20260624.json 等须 byte-unchanged,evidence/ 不可篡改门强制);
    # 新名仍 startswith "dxil_path_spike_"(check_schemas dxil_path_spike 校验器匹配)。
    suffix = os.environ.get("RURIX_SPIKE_SUFFIX", "_r4")
    out_path = ROOT / "evidence" / f"dxil_path_spike_{date_tag}{suffix}.json"
    payload = json.dumps(ev, ensure_ascii=False, indent=2) + "\n"
    # 二进制写 + 显式 LF,禁文本模式(防 CRLF;.gitattributes * -text)
    data = payload.encode("utf-8").replace(b"\r\n", b"\n")
    with open(out_path, "wb") as f:
        f.write(data)
    print(f"[run_spike] wrote {out_path.relative_to(ROOT)} (status={ev['status']}, CRLF={data.count(chr(13).encode())})")
    print(f"[run_spike] path_a.status={ev['path_a']['status']} path_b.status={ev['path_b']['status']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
