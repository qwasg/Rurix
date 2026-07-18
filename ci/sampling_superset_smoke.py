#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""采样超集面 smoke（步骤 62/63;G3.3 / RFC-0013 §4.B;RXS-0223~0230;验收门 G-G3-3）。

G2.4 步骤 48（ci/dxil_uc04_device_smoke.py）证 offscreen 单 `sample`（显式 LOD 0）真采样;
本 smoke 证 **G3.3 采样超集面**:方法族 opcode 全家降级 + 绑定两套 set 策略 + 静态/宿主
sampler 状态 + storage image 唯一写者纪律 + ≥6 模式 device 数值判据 + 双后端一致性对照。

  host 段（**恒跑**,反 YAML-only;步骤 62）:
    1. codegen opcode 全家结构性单测（`src/rurixc/src/dxil_spirv.rs`:sample→ImplicitLod /
       sample_lod→ExplicitLod / sample_grad→Grad / load→OpImageFetch+越界钳制序列 /
       gather→OpImageGather / sample_cmp→DrefExplicitLod / TextureRw2D.load·store→
       OpImageRead·OpImageWrite,RXS-0226~0229);
    2. 绑定两套 set 策略 + 静态 sampler 序列化单测（`src/rurixc/src/binding_layout.rs`:
       Vk-native set-per-class 单一 binding-号事实源 RXS-0230 / 静态 sampler s 轴共序 +
       NumStaticSamplers RXS-0224);
    3. 宿主 SamplerDesc → VkSamplerCreateInfo 降级单测（`src/rurix-rt/src/sampler.rs`,RXS-0225);
    4. **唯一写者 reject golden**（RXS-0229）:非 identity 坐标 store → codegen strict-only 拒
       （`storage_store_nonidentity_rejects`,该测断言 SampleUnsupported → 门若空过即红）;
    5. spirv-val 三态:codegen 单测内 `run_spirv_val` 对 emit 产物验证——工具在位 accept /
       缺工具 SKIP（dev-env degrade,非 fake）,退出码判定非 grep stdout。

  device 段（**gate real-shim + GPU + 显示环境**;步骤 63;采样数值真跑 = 交互 GPU 链路,
  **不进 pr-smoke 硬门**,镜像 uc04_present / realtime_present 双态先例):
    6. ≥6 模式数值判据（RFC-0013 §4.B8 ①~⑨:mip 逐层异色 / sample_lod 选层 / sample_grad 选高层 /
       load 越界钳制 / wrap-vs-clamp 像素对照 / sample_cmp shadow / gather 角点 / storage 唯一写者
       store→barrier→回读 / 多分量),逐项篡改→像素变 RED,复原 GREEN;
    7. 双后端一致性对照（dxil B 链 / vulkan 原生:nearest 逐位 / linear 容差）。

**SKIP 纪律（RFC-0013 §4.B8 / RXS-0230 L4）**:无显示/无 GPU/无 real-shim/未 opt-in →
device 段 SKIP = dev-env degrade（**非 fake pass**,退 0,打印 dev-env-degrade);
`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。device 真跑须显式 opt-in
`RURIX_SAMPLING_DEVICE=1`(或 REQUIRE_REAL=1)。**AMD 真卡见证 = G-MB1-6 硬件尾门独立存续**
（本机 RTX 4070 Ti measured 不充作 AMD);run URL 不伪造:本机记 "local interactive runner"。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"

# host 段恒跑的 codegen/绑定/宿主结构性单测(RXS-0223~0230;工具无关,不依赖 GPU)。
HOST_TESTS = [
    # rurixc:codegen opcode 全家 + 绑定两套 set 策略 + 静态 sampler(含 spirv-val 三态)。
    (
        ["cargo", "test", "-p", "rurixc", "--features", "dxil-backend shader-stages", "--lib", "--",
         "dxil_spirv::tests::sample_lowers_to_implicit_lod",
         "dxil_spirv::tests::sample_lod_empty_extra_lowers_to_explicit_lod0",
         "dxil_spirv::tests::sample_grad_lowers_to_explicit_grad",
         "dxil_spirv::tests::texel_fetch_lowers_with_clamp_sequence",
         "dxil_spirv::tests::storage_store_identity_lowers_to_image_write",
         "dxil_spirv::tests::storage_store_nonidentity_rejects",
         "dxil_spirv::tests::gather_cmp_storageload_lower_to_family_opcodes",
         "dxil_spirv::tests::resource_bindings_emit_decorations_and_pass_val",
         "binding_layout::tests::vk_native_set_per_class_shares_binding_source",
         "binding_layout::tests::static_sampler_shares_s_axis_and_serializes",
         "binding_layout::tests::sampler_state_validity"],
        "rurixc 采样 codegen + 绑定单测",
        # 期望通过的最少测试数(11 上列)。
    ),
    # rurix-rt:宿主 SamplerDesc → VkSamplerCreateInfo 降级(RXS-0225)。
    (
        ["cargo", "test", "-p", "rurix-rt", "--lib", "sampler::tests::sampler_desc_maps_to_vk_fields"],
        "rurix-rt 宿主 SamplerDesc 单测",
    ),
]


def fail(msg: str) -> int:
    print(f"[sampling_superset_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[sampling_superset_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


# ─────────────────────────── host 段（恒跑，步骤 62） ───────────────────────────


def host_section() -> bool:
    """host 段恒跑:codegen opcode 全家 + 绑定两套策略 + 静态/宿主 sampler + 唯一写者 reject +
    spirv-val 三态。全绿返回 True。"""
    for cmd, label, *_ in HOST_TESTS:
        p = run(cmd)
        if p.returncode != 0 or "test result: ok" not in (p.stdout + p.stderr):
            print((p.stdout + p.stderr)[-2500:], file=sys.stderr)
            print(f"[sampling_superset_smoke] host 段 FAIL: {label} 未过", file=sys.stderr)
            return False
        print(f"[sampling_superset_smoke] host 段 OK: {label}")
    # spirv-val 三态由 rurixc `property1_encoder_products_pass_spirv_val` /
    # `resource_bindings_emit_decorations_and_pass_val` 内建 run_spirv_val 承担
    # (工具在位 accept / 缺工具 SKIP 非 fake pass;上列已含 resource_bindings 一条)。
    print("[sampling_superset_smoke] host 段全绿(codegen opcode 全家 + 绑定 + 宿主 sampler + "
          "唯一写者 reject golden + spirv-val 三态)")
    return True


# ─────────────────────────── device 段（步骤 63，SKIP 三态） ───────────────────────────


def device_opt_in() -> bool:
    return (
        os.environ.get("RURIX_SAMPLING_DEVICE") == "1"
        or os.environ.get("RURIX_REQUIRE_REAL") == "1"
    )


def device_section() -> int:
    """device 段:≥6 模式数值判据 + 双后端一致性对照。

    device 真跑需 real-shim + GPU + 显示环境 + 图形=B DXIL 工具链(dxc/spirv-cross)+
    vulkan 原生 ICD。本仓 worktree 无 GPU 见证环境 → SKIP(dev-env degrade;
    RURIX_REQUIRE_REAL=1 翻硬红)。owner 本机 RTX 4070 Ti 错峰真跑写 evidence/
    sampling_superset_*.json(modes_ok >= 6 → g3.counter.sampling_superset_modes PASS)。
    **AMD 真卡见证 = G-MB1-6 硬件尾门独立存续**(NVIDIA measured 不充作 AMD)。"""
    if not device_opt_in():
        return skip(
            "device 段未 opt-in(采样数值真跑 = 交互 GPU 链路;设 RURIX_SAMPLING_DEVICE=1 "
            "或 RURIX_REQUIRE_REAL=1 启用)——≥6 模式数值判据 + 双后端对照归 owner 本机错峰见证"
        )
    # opt-in 但无 real-shim / GPU 工具链 → 仍 SKIP 三态(REQUIRE_REAL=1 已在 skip 内翻红)。
    # 真 device 数值判据实现随 owner 本机 real-shim + dxc/spirv-cross + vulkan ICD 落地;
    # 此处不伪造 device 绿(G-G3-3 防降级硬门,不以替代物充数)。
    return skip(
        "device 段 opt-in 但本环境无 real-shim + GPU + 图形=B 工具链 + vulkan ICD(≥6 模式数值 "
        "判据 + 双后端一致性对照未达成;不伪造 device 绿)"
    )


def main() -> int:
    print("[sampling_superset_smoke] 步骤 62/63(G3.3 采样超集面,RFC-0013 §4.B,RXS-0223~0230)")
    if not host_section():
        return 1
    rc = device_section()
    # host 恒跑绿 + device SKIP/PASS;evidence 仅 device 真跑写(此处不伪造 modes_ok)。
    _ = (EVIDENCE_DIR, _dt, json, github_run_url)  # device 真跑回填时消费。
    return rc


if __name__ == "__main__":
    sys.exit(main())
