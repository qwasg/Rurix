#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""renderer 光栅 diff 与 RD-038 余项冒烟(步骤 95;G7.5;验收门 G-G7-7)。

host 段(**恒跑**,需 Vulkan SDK 的 `spirv-val`/`spirv-dis`;缺工具 → SKIP):
  1. 冻结面完整性:`RD038_LITERAL_MATRIX.md` 八行字面分项与 `G7_SCENE_FREEZE.md`
     的场景/相机冻结锚(764 三角形 / 3 实例 / 固定相机常量)在位 —— 防「矩阵与
     冻结场景漂移后判据自动放水」。
  2. host oracle 单测:`shadow::`(VSM clipmap/page_table/pool/vsm)+ `temporal::`
     (taa/tsr/common/image/upscale/ssim)+ `geometry::visbuffer`(SW 基准侧金标准)
     全量恒绿 —— **oracle 数值语义 0-byte** 的回归网。
  3. VisBuffer 位格式冻结面审计:`depth30 | cluster27 | tri7` 的位宽与 clear 值在
     host 冻结契约与 W2 SW kernel 源码内**同一套常量**(SW/HW 同 ABI 的前提;
     G5 冻结面不得为迁就 HW 路径而漂移)。
  4. 三余项 kernel 真实 `.rx` → `.spv`(`vsm_depth_raster` / `vsm_sample` /
     `tsr_resample`):`spirv-val --target-env vulkan1.0` accept、SPIR-V 维持 **1.0**
     (不误升 1.4)、同源 ×2 字节全等(确定性)、不得误声明 ray query 能力面。
  5. **HW 光栅 capability 机验**(G7.5b 翻转,设计 §5.2;RXS-0301~0303):
     六枚隔离探针由「必红 RX6026」翻转为「**必绿** + spirv-val」(探针文本不动,
     断言方向翻转)= 六项 capability 的正向机器锁;目标形态 FS/VS 语料(已迁
     `conformance/vulkan/accept/`)必绿 + 版本 1.0 + caps 集合断言;§2.4 四枚
     reject 语料必红 `RX6026`(负面清单双锁);FS dxil-target 必拒(**RXS-0171
     L4 冻结锁**,经 rurixc 单测子进程)。uc06 HW device 装配腿未在树 →
     `hw_raster_diff.status` 维持 blocked,缺项机器产出(不伪造 device 条目)。
  6. W1/W2 零漂移门:五 kernel 逐件对 `tests/vulkan/w1w2_spv_manifest.json` 的
     sha256 + SPIR-V 版本 + capability 集合比对(**不重 bless**)。
  7. RED 反证:篡改 `.spv` 单字节 → `spirv-val` 必拒。

device 段(**gate real**,`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`;门 G-G7-7):
  8. `uc06-renderer --g75-residuals`:RD-038 余项两轴 device 真跑对拍 ——
       · VSM 页内深度光栅(1 048 576 纹素 × 764 三角形 gather)、
       · VSM 阴影采样(0/1 二值,**零容差**)、
       · TSR 空间超分核(16 tap Catmull-Rom + 抗振铃钳制);
     measured 与**冻结**容差成对机验(measured ≤ tol),并带非退化统计
     (覆盖纹素 / 遮蔽比 / 钳制通道数)防判据空转。
     同段复跑 **SW/HW diff 的 SW 基准侧**(`device_w2_visbuffer_u64_bitexact_host`,
     9216 词 u64 逐位相等)—— 记录「diff 的一侧已在位,缺的只有 HW 侧」。
  9. RED 两轴:`--g75-red-vsm`(篡改 device 侧灯空间三角形 → 深度对拍必红的
     **数据流反证**)+ `--g75-red-tsr`(篡改 device jitter → 重采样相位错位必红)。
  无 Vulkan 设备 → SKIP=dev-env degrade;`RURIX_REQUIRE_REAL=1` 翻硬红。

**零容差纪律**:SW/HW VisBuffer 的整数域 `diff = 0` 是 G-G7-7 字面判据,本步骤
**不以任何容差型替代物冒充**;HW 侧未在树时 `hw_raster_diff.status` 记
`blocked-frozen-graphics-body-slice`,由 schema 强制附机器可核的缺失能力清单。
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
MANIFEST = ROOT / "tests" / "vulkan" / "w1w2_spv_manifest.json"
KERNEL_DIR = ROOT / "apps" / "uc06-renderer" / "kernels"
LITERAL_MATRIX = ROOT / "milestones" / "g7" / "RD038_LITERAL_MATRIX.md"
SCENE_FREEZE = ROOT / "milestones" / "g7" / "G7_SCENE_FREEZE.md"
VISBUFFER_HOST = ROOT / "src" / "rurix-render" / "src" / "geometry" / "visbuffer.rs"
GRAPH_TYPES = ROOT / "src" / "rurix-render" / "src" / "graph" / "types.rs"
SW_KERNEL = KERNEL_DIR / "visbuffer_sw_u64.rx"
HW_ACCEPT_FS = ROOT / "conformance" / "vulkan" / "accept" / "vk_hw_raster_visbuffer_fs.rx"
HW_ACCEPT_VS = ROOT / "conformance" / "vulkan" / "accept" / "vk_hw_raster_visbuffer_vs.rx"
# §2.4 四枚负面清单 reject 语料(RXS-0301 L3;与 vulkan_codegen_smoke reject 段双锁)。
HW_REJECTS = tuple(
    ROOT / "conformance" / "vulkan" / "reject" / f"vk_hw_raster_{n}_reject.rx"
    for n in ("loop", "devfn_call", "cta_atomic", "f64")
)
UC06_SRC = ROOT / "apps" / "uc06-renderer" / "src"
# PR-4 装配腿的机器探测字面(设计 §4:uc06 CLI `--g75-hw-raster`)。
HW_DEVICE_FLAG = "--g75-hw-raster"

TAG = "renderer_raster_diff_smoke"

# G7.5 余项三核(RD-038「VSM 深度」+「TAA-TSR」的 TSR 腿)。
G75_KERNELS = ("vsm_depth_raster", "vsm_sample", "tsr_resample")

# RD-038 title 的**八行**字面分项(§0 拆行口径);逐行须在矩阵 §1 在位。
LITERAL_ROWS = (
    "两级剔除",
    "VisBuffer SW(u64 atomicMax)",
    "HW 光栅",
    "classify-resolve",
    "VSM 深度",
    "屏幕探针 GI",
    "RTAO 硬阴影",
    "TAA-TSR",
)

# 冻结场景锚(G7_SCENE_FREEZE.md §1/§2;漂移即判据失据)。
SCENE_ANCHORS = ("764", "实例数 = 3", "[0.0, 2.2, 3.4]", "[0.0, 0.35, 0.0]")

# HW 光栅逐轴隔离探针:源码 → (capability 名, 旧「必红」期望诊断子串〔仅存档〕)。
# 每条只含**一个**RXS-0171 L4 白名单外构造。G7.5b 语义翻转(设计 §5.2):断言方向
# 由「必红且落该诊断」改为「必绿 + spirv-val」——探针文本一字不动,成为
# RXS-0301 六项 capability 的正向机器锁。
HW_PROBES: dict[str, tuple[str, str, str]] = {
    "vector_component": (
        "graphics_vector_component_projection",
        "RXS-0171 最小切片仅支持单层 Field 投影",
        """struct P { #[builtin(frag_coord)] frag: vec4<f32> }
struct O { #[interpolate(perspective)] v: f32 }
fragment fn probe(inp: P) -> O { O { v: inp.frag.0 } }
fn main() {}
""",
    ),
    "comparison_op": (
        "graphics_comparison_ops",
        "RXS-0171 最小切片仅支持 f32/i32/u32 加减乘除",
        """struct P { #[interpolate(flat)] s: f32 }
struct O { #[interpolate(perspective)] color: vec4<f32> }
fragment fn probe(inp: P) -> O {
    let mut v = inp.s;
    if v < 0.0 { v = 0.0; }
    O { color: (v, v, v, v) }
}
fn main() {}
""",
    ),
    "control_flow_and_call": (
        "graphics_control_flow_and_calls",
        "RXS-0171 最小切片仅支持 straight-line Goto/Return",
        """struct P { #[interpolate(flat)] s: f32 }
struct O { #[interpolate(perspective)] color: vec4<f32> }
fragment fn probe(inp: P) -> O {
    let v = inp.s.round();
    O { color: (v, v, v, v) }
}
fn main() {}
""",
    ),
    "buffer_indexing": (
        "graphics_buffer_indexing",
        "RXS-0171 最小切片仅支持 f32/i32/u32 常量",
        """struct P { #[interpolate(perspective)] color: vec4<f32> }
struct O { #[interpolate(perspective)] color: vec4<f32> }
fragment fn probe(inp: P, data: View<global, f32>) -> O {
    O { color: (data[0], inp.color.1, inp.color.2, 1.0) }
}
fn main() {}
""",
    ),
    "output_assembly": (
        "graphics_output_assembly",
        "仅允许声明的输出 I/O 聚合返回值机械分解",
        """struct P { #[interpolate(flat)] s: f32 }
struct O { #[interpolate(perspective)] color: vec4<f32> }
fragment fn probe(inp: P) -> O {
    let v = inp.s * 2.0 - 1.0;
    O { color: (v, v, v, v) }
}
fn main() {}
""",
    ),
}

# 探针方法自证的**绿对照臂**:与 vector_component 探针同形态、只把两层投影换成
# 单层直通 —— 必须编译**绿**。没有它,「所有探针都红」无法排除「探针写法本身有病」。
HW_PROBE_CONTROL = """struct P { #[interpolate(flat)] s: f32 }
struct O { #[interpolate(perspective)] v: f32 }
fragment fn probe(inp: P) -> O { O { v: inp.s } }
fn main() {}
"""

# 图形阶段 SSBO/u64 原子资源面(graphics_ssbo_atomic_u64):翻转后不再静态审计
# 拒绝分支字面 —— 由目标 FS 语料编译绿 + caps 含 Int64Atomics 正向机证(步骤 5 ①)。


def fail(msg: str) -> int:
    print(f"[{TAG}] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[{TAG}] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, env=None, timeout: int = 3600):
    r = subprocess.run(cmd, capture_output=True, cwd=str(ROOT), timeout=timeout, env=env)
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


def tool(name: str) -> str | None:
    env_key = {"spirv-val": "RURIX_SPIRV_VAL", "spirv-dis": "RURIX_SPIRV_DIS"}.get(name)
    if env_key:
        p = os.environ.get(env_key)
        if p and Path(p).is_file():
            return p
    return shutil.which(name)


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def spv_version(path: Path) -> str:
    raw = path.read_bytes()
    if len(raw) < 8:
        return "invalid"
    v = int.from_bytes(raw[4:8], "little")
    return f"{(v >> 16) & 0xFF}.{(v >> 8) & 0xFF}"


def compile_rx(src: Path, out: Path) -> tuple[int, str]:
    code, o, e = run(
        [
            "cargo", "run", "-q", "-p", "rurixc",
            "--features", "vulkan-backend", "--bin", "rurixc", "--",
            str(src), "--target", "vulkan", "-o", str(out),
        ]
    )
    return code, (o + e)


def spirv_val(spv: Path, target_env: str | None) -> tuple[int, str]:
    exe = tool("spirv-val")
    if not exe:
        return (-1, "spirv-val 不可用")
    cmd = [exe]
    if target_env:
        cmd += ["--target-env", target_env]
    cmd.append(str(spv))
    code, o, e = run(cmd)
    return code, (o + e)


def disasm(spv: Path) -> tuple[int, str]:
    exe = tool("spirv-dis")
    if not exe:
        return (-1, "spirv-dis 不可用")
    code, o, e = run([exe, str(spv)])
    return code, (o + e)


# ───────────────────────── host 段(恒跑) ─────────────────────────


def literal_matrix_section(results: dict) -> bool:
    """步骤 1:RD-038 八行字面分项 + 冻结场景锚在位。"""
    ok = True
    if not LITERAL_MATRIX.is_file():
        print(f"[{TAG}] 缺 {LITERAL_MATRIX.relative_to(ROOT)}", file=sys.stderr)
        return False
    text = LITERAL_MATRIX.read_text(encoding="utf-8")
    missing_rows = [r for r in LITERAL_ROWS if f"| {r}" not in text]
    if missing_rows:
        print(f"[{TAG}] RD-038 字面矩阵 §1 缺分项行: {missing_rows}", file=sys.stderr)
        ok = False
    if not SCENE_FREEZE.is_file():
        print(f"[{TAG}] 缺 {SCENE_FREEZE.relative_to(ROOT)}", file=sys.stderr)
        return False
    freeze = SCENE_FREEZE.read_text(encoding="utf-8")
    missing_anchors = [a for a in SCENE_ANCHORS if a not in freeze]
    if missing_anchors:
        print(f"[{TAG}] 场景冻结锚缺失/漂移: {missing_anchors}", file=sys.stderr)
        ok = False
    results["literal_matrix_pass"] = ok
    results["literal_rows"] = list(LITERAL_ROWS)
    if ok:
        print(f"[{TAG}] 步骤 1 PASS: RD-038 八行字面分项 + 冻结场景/相机锚在位")
    return ok


def oracle_section(results: dict) -> bool:
    """步骤 2:VSM / TAA-TSR / VisBuffer host oracle 单测(数值语义 0-byte 回归网)。"""
    ok = True
    for filt, label in (
        ("shadow::", "shadow(clipmap/page_table/pool/vsm)"),
        ("temporal::", "temporal(taa/tsr/common/image/upscale/ssim)"),
        ("geometry::visbuffer", "geometry::visbuffer(SW 基准侧金标准)"),
    ):
        code, o, e = run(["cargo", "test", "-q", "-p", "rurix-render", "--lib", "--", filt])
        if code != 0:
            print((o + e)[-2400:], file=sys.stderr)
            print(f"[{TAG}] host oracle 单测未过: {label}", file=sys.stderr)
            ok = False
    results["oracle_tests_pass"] = ok
    if ok:
        print(f"[{TAG}] 步骤 2 PASS: shadow:: + temporal:: + geometry::visbuffer 全量恒绿")
    return ok


def visbuffer_abi_section(results: dict) -> bool:
    """步骤 3:VisBuffer 位格式冻结面(SW/HW 同 ABI 的前提)。"""
    if not (VISBUFFER_HOST.is_file() and GRAPH_TYPES.is_file() and SW_KERNEL.is_file()):
        results["visbuffer_abi_freeze_pass"] = False
        print(f"[{TAG}] VisBuffer 位格式来源文件缺失", file=sys.stderr)
        return False
    types = GRAPH_TYPES.read_text(encoding="utf-8")
    kernel = SW_KERNEL.read_text(encoding="utf-8")
    host = VISBUFFER_HOST.read_text(encoding="utf-8")

    def const_of(name: str) -> int | None:
        m = re.search(rf"{name}\s*:\s*u32\s*=\s*(\d+)", types)
        return int(m.group(1)) if m else None

    cluster_bits = const_of("VISBUFFER_CLUSTER_BITS")
    tri_bits = const_of("VISBUFFER_TRI_BITS")
    ok = cluster_bits == 27 and tri_bits == 7
    if not ok:
        print(
            f"[{TAG}] VisBuffer 位宽漂移: cluster={cluster_bits} tri={tri_bits}"
            f"(冻结 27/7,G5 冻结面 0-byte)",
            file=sys.stderr,
        )
    # SW kernel 内的位移常量必须与冻结位宽一致(depth 起点 = 27+7 = 34,tri 段 = 7)。
    depth_shift = tri_bits + cluster_bits if (tri_bits and cluster_bits) else None
    if depth_shift is None or f"<< {depth_shift}u64" not in kernel:
        print(
            f"[{TAG}] W2 SW kernel 的 depth 位移与冻结位宽不一致(期望 << {depth_shift}u64)",
            file=sys.stderr,
        )
        ok = False
    if tri_bits is None or f"<< {tri_bits}u64" not in kernel:
        print(f"[{TAG}] W2 SW kernel 的 cluster 位移与冻结 tri 位宽不一致", file=sys.stderr)
        ok = False
    # clear 值语义:pack(0, CLUSTER_INVALID, TRI_INVALID) = 2^34 − 1。
    if "VISBUFFER_CLEAR" not in host:
        print(f"[{TAG}] host 侧缺 VISBUFFER_CLEAR 冻结常量", file=sys.stderr)
        ok = False
    results["visbuffer_abi_freeze_pass"] = ok
    results["visbuffer_abi"] = {
        "depth_bits": 64 - (cluster_bits or 0) - (tri_bits or 0)
        if cluster_bits and tri_bits
        else 0,
        "cluster_bits": cluster_bits,
        "tri_bits": tri_bits,
        "depth_shift": depth_shift,
        "clear_value": (1 << ((cluster_bits or 0) + (tri_bits or 0))) - 1
        if cluster_bits and tri_bits
        else None,
        "sw_kernel": str(SW_KERNEL.relative_to(ROOT)).replace("\\", "/"),
    }
    if ok:
        print(
            f"[{TAG}] 步骤 3 PASS: VisBuffer 位格式冻结面一致"
            f"(depth30 | cluster{cluster_bits} | tri{tri_bits},depth 位移 {depth_shift})"
        )
    return ok


def kernel_emit_section(results: dict, work: Path) -> bool:
    """步骤 4:三余项 kernel → SPIR-V 1.0 + spirv-val + 确定性 + 无 ray query 误声明。"""
    per_kernel: dict = {}
    for name in G75_KERNELS:
        src = KERNEL_DIR / f"{name}.rx"
        if not src.is_file():
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] 缺余项 kernel 源 {src.relative_to(ROOT)}", file=sys.stderr)
            return False
        spv = work / f"{name}.spv"
        spv2 = work / f"{name}_2.spv"
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            print(blob[-2400:], file=sys.stderr)
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name}.rx 编译未产 .spv", file=sys.stderr)
            return False
        ver = spv_version(spv)
        if ver != "1.0":
            results["kernel_emit_pass"] = False
            print(
                f"[{TAG}] {name} SPIR-V 版本 {ver} != 1.0(余项核不用 RayQuery,"
                f"不得误升 1.4;per-entry 分叉纪律 RXS-0300)",
                file=sys.stderr,
            )
            return False
        code2, _ = compile_rx(src, spv2)
        if code2 != 0 or spv.read_bytes() != spv2.read_bytes():
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} 同源 ×2 编译字节不等(非确定性)", file=sys.stderr)
            return False
        vc, vblob = spirv_val(spv, "vulkan1.0")
        if vc == -1:
            results["toolchain_skip"] = "no-spirv-val"
            return True
        if vc != 0:
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} spirv-val --target-env vulkan1.0 拒: {vblob[-800:]}",
                  file=sys.stderr)
            return False
        dc, dis = disasm(spv)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        if dc != 0:
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} spirv-dis 失败: {dis[-800:]}", file=sys.stderr)
            return False
        if "RayQueryKHR" in dis or "SPV_KHR_ray_query" in dis:
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} 误声明 ray query 面(余项核不应触及 W3 能力)",
                  file=sys.stderr)
            return False
        caps = sorted(
            line.split("OpCapability", 1)[1].strip()
            for line in dis.splitlines()
            if "OpCapability" in line
        )
        per_kernel[name] = {
            "spirv_version": ver,
            "sha256": sha256_of(spv),
            "spirv_val": {"vulkan1.0": "accepted"},
            "deterministic": True,
            "capabilities": caps,
        }
    results["kernel_emit_pass"] = True
    results["kernels"] = per_kernel
    print(f"[{TAG}] 步骤 4 PASS: 余项三核 SPIR-V 1.0 + spirv-val + 确定性 + 零 ray query 声明")
    return True


def hw_raster_capability_section(results: dict, work: Path) -> bool:
    """步骤 5(G7.5b 翻转,设计 §5.2;RXS-0301~0303):HW 光栅 capability 机验。

    由 `hw_raster_blocked_section` 演进:① 目标形态 FS/VS 语料(accept/)必绿 +
    版本 1.0 + caps 集合断言;② 六枚隔离探针「必红」→「必绿 + spirv-val」翻转
    (文本不动)= 六项 capability 正向机器锁;③ §2.4 四枚 reject 语料必红 RX6026;
    ④ FS dxil-target 必拒(RXS-0171 冻结锁)。uc06 device 装配腿未在树 → status
    维持 blocked-frozen-graphics-body-slice,缺项机器产出(不伪造 device 条目)。
    """
    probes: dict = {}
    green_caps: list[str] = []
    ok = True

    # ① 目标形态语料必绿:FS caps=={Shader,Int64,Int64Atomics} / VS caps=={Shader},
    #    版本字均恒 1.0,spirv-val vulkan1.0 accept(RXS-0302 L3/L4)。
    for src, want_caps in (
        (HW_ACCEPT_FS, ["Int64", "Int64Atomics", "Shader"]),
        (HW_ACCEPT_VS, ["Shader"]),
    ):
        if not src.is_file():
            results["hw_raster_blocked_honest_pass"] = False
            print(f"[{TAG}] 缺 HW 光栅目标形态语料 {src}", file=sys.stderr)
            return False
        spv = work / f"hw_{src.stem}.spv"
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            results["hw_raster_blocked_honest_pass"] = False
            print(
                f"[{TAG}] HW 光栅目标形态语料 {src.stem} 应绿,实测 rc={code}\n{blob[-1200:]}",
                file=sys.stderr,
            )
            return False
        if spv_version(spv) != "1.0":
            results["hw_raster_blocked_honest_pass"] = False
            print(
                f"[{TAG}] {src.stem} SPIR-V 版本 {spv_version(spv)} != 1.0(RXS-0302 L4)",
                file=sys.stderr,
            )
            return False
        vc, vblob = spirv_val(spv, "vulkan1.0")
        if vc == -1:
            results["toolchain_skip"] = "no-spirv-val"
            return True
        if vc != 0:
            results["hw_raster_blocked_honest_pass"] = False
            print(f"[{TAG}] {src.stem} spirv-val 拒: {vblob[-800:]}", file=sys.stderr)
            return False
        dc, dis = disasm(spv)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        caps = sorted(
            line.split("OpCapability", 1)[1].strip()
            for line in dis.splitlines()
            if "OpCapability" in line
        )
        if caps != want_caps:
            results["hw_raster_blocked_honest_pass"] = False
            print(
                f"[{TAG}] {src.stem} capability 集合 {caps} != {want_caps}(按需声明,不用不发)",
                file=sys.stderr,
            )
            return False
        probes[f"target_{src.stem}"] = {
            "missing_capability": "n/a(目标形态语料:必绿 + spirv-val + caps 集合)",
            "rc": code,
            "expected_diagnostic": "",
            "matched": True,
        }
    with tempfile.TemporaryDirectory() as d:
        # 绿对照臂(保留):同形态单层直通必绿 —— 探针方法自证基线不变。
        ctl = Path(d) / "control_green.rx"
        ctl.write_text(HW_PROBE_CONTROL, encoding="utf-8", newline="\n")
        ctl_rc, ctl_out = compile_rx(ctl, Path(d) / "control_green.spv")
        probes["control_green_arm"] = {
            "missing_capability": "n/a(绿对照臂:图形直通形态可编译)",
            "rc": ctl_rc,
            "expected_diagnostic": "",
            "matched": ctl_rc == 0,
        }
        if ctl_rc != 0:
            ok = False
            print(
                f"[{TAG}] 绿对照臂未编译通过(rc={ctl_rc}):\n{ctl_out[-900:]}",
                file=sys.stderr,
            )
        # ② 六枚隔离探针必绿 + spirv-val(语义翻转;探针文本一字不动)。
        for pname, (cap, _old_want, src_text) in HW_PROBES.items():
            rx = Path(d) / f"{pname}.rx"
            rx.write_text(src_text, encoding="utf-8", newline="\n")
            spv = Path(d) / f"{pname}.spv"
            rc, out = compile_rx(rx, spv)
            green = rc == 0 and spv.is_file()
            if green:
                vc, vblob = spirv_val(spv, "vulkan1.0")
                if vc == -1:
                    results["toolchain_skip"] = "no-spirv-val"
                    return True
                green = vc == 0
                if not green:
                    out = vblob
            probes[pname] = {
                "missing_capability": cap,
                "rc": rc,
                "expected_diagnostic": "",
                "matched": green,
            }
            if green:
                green_caps.append(cap)
            else:
                ok = False
                print(
                    f"[{TAG}] capability 探针 {pname} 应绿 + spirv-val(rc={rc}):\n"
                    f"{out[-900:]}",
                    file=sys.stderr,
                )
        # ③ §2.4 四枚 reject 语料必红 RX6026 且零 .spv(负面清单双锁,RXS-0301 L3)。
        for rej in HW_REJECTS:
            if not rej.is_file():
                ok = False
                print(f"[{TAG}] 缺负面清单 reject 语料 {rej}", file=sys.stderr)
                continue
            spv = Path(d) / f"{rej.stem}.spv"
            rc, out = compile_rx(rej, spv)
            red = rc != 0 and "RX6026" in out and not spv.is_file()
            probes[rej.stem] = {
                "missing_capability": "n/a(负面清单 RED:恒拒 RX6026)",
                "rc": rc,
                "expected_diagnostic": "RX6026",
                "matched": red,
            }
            if not red:
                ok = False
                print(
                    f"[{TAG}] 负面清单语料 {rej.stem} 应红 RX6026 且零 .spv,"
                    f"实测 rc={rc} spv={spv.is_file()}\n{out[-900:]}",
                    file=sys.stderr,
                )
    # ④ DXIL 冻结锁:FS 语料 dxil-target 必拒(RXS-0171 L4 一字不动;经 rurixc 单测)。
    rc, o, e = run(
        [
            "cargo", "test", "-q", "-p", "rurixc", "--features", "vulkan-backend",
            "--test", "hw_raster_vulkan_spirv_val", "hw_raster_fs_dxil_target_still_rejected",
        ]
    )
    probes["dxil_target_freeze_lock"] = {
        "missing_capability": "n/a(RXS-0171 L4 冻结锁:FS dxil-target 必拒)",
        "rc": rc,
        "expected_diagnostic": "",
        "matched": rc == 0,
    }
    if rc != 0:
        ok = False
        print(f"[{TAG}] DXIL 冻结锁单测失败(rc={rc}):\n{(o + e)[-900:]}", file=sys.stderr)
    # ⑤ blocked-honest 余项:uc06 device 装配腿(PR-4)在树与否的机器探测。
    device_leg = any(
        HW_DEVICE_FLAG in p.read_text(encoding="utf-8", errors="replace")
        for p in sorted(UC06_SRC.rglob("*.rs"))
    )
    missing = [] if device_leg else [
        f"uc06_g75_hw_raster_device_assembly({HW_DEVICE_FLAG} CLI 装配腿未在树,设计 §4/PR-4)"
    ]
    results["hw_raster_blocked_honest_pass"] = ok
    results["hw_raster_diff"] = {
        "status": "blocked-frozen-graphics-body-slice",
        "hw_side": None,
        "diff_pixels": None,
        "missing_toolchain_caps": missing,
        "blocking_probes": probes,
        "target_corpus": str(HW_ACCEPT_FS.relative_to(ROOT)).replace("\\", "/"),
        "spec_anchor": "spec/vulkan_backend.md RXS-0301/0302/0303(两遍编译扩展白名单 + "
                       "资源绑定/原子语义 + 保守光栅执行语义);spec/dxil_backend.md "
                       "RXS-0171 L4 冻结不动(DXIL 路必拒)",
        "escalation": "capability 面已翻绿(RFC-0018 §E 裁定兑现);余项 = uc06 device "
                      "装配腿真跑 diff=0(设计 §4/PR-4);仍禁止以容差型替代物冒充 diff=0",
    }
    if ok:
        # 六项 capability = 五枚隔离探针 + graphics_ssbo_atomic_u64(由目标 FS 语料
        # 编译绿 + caps 含 Int64Atomics 正向机证,见步骤 ①)。
        green_caps.append("graphics_ssbo_atomic_u64(经目标 FS 语料)")
        print(
            f"[{TAG}] 步骤 5 PASS(capability 翻转): 目标 FS/VS 必绿 + 版本 1.0 + caps 集合;"
            f"{len(green_caps)}/6 capability 翻绿 = {sorted(green_caps)};"
            f"4 reject 恒红 RX6026;dxil 冻结锁绿;device 装配腿余项 = {missing or '在树'}"
        )
    return ok


def w1w2_zero_drift_section(results: dict, work: Path) -> bool:
    """步骤 6:W1/W2 五 kernel 逐字节 + 版本 + capability 零漂移(不重 bless)。"""
    if not MANIFEST.is_file():
        results["w1w2_zero_drift_pass"] = False
        print(f"[{TAG}] 缺 golden manifest {MANIFEST.relative_to(ROOT)}", file=sys.stderr)
        return False
    expected = json.loads(MANIFEST.read_text(encoding="utf-8"))["kernels"]
    for name, want in sorted(expected.items()):
        src = KERNEL_DIR / f"{name}.rx"
        spv = work / f"w1w2_{name}.spv"
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            print(blob[-2000:], file=sys.stderr)
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} 编译失败", file=sys.stderr)
            return False
        ver, digest = spv_version(spv), sha256_of(spv)
        dc, dis = disasm(spv)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        caps = sorted(
            line.split("OpCapability", 1)[1].strip()
            for line in dis.splitlines()
            if "OpCapability" in line
        )
        if ver != want["spirv_version"] or digest != want["sha256"] or caps != want["capabilities"]:
            results["w1w2_zero_drift_pass"] = False
            print(
                f"[{TAG}] W1/W2 {name} **漂移**: ver={ver} sha={digest} caps={caps} "
                f"vs manifest {want}(零漂移门:既有 golden 不重 bless)",
                file=sys.stderr,
            )
            return False
    results["w1w2_zero_drift_pass"] = True
    print(f"[{TAG}] 步骤 6 PASS: W1/W2 {len(expected)} kernel 字节 + 版本 + capability 零漂移")
    return True


def red_tamper_spv_section(results: dict, work: Path) -> bool:
    """步骤 7:篡改 .spv 单字节 → spirv-val 必拒(编译校验轴生效反证)。"""
    spv = work / "vsm_depth_raster.spv"
    if not spv.is_file():
        results["red_tamper_spv_pass"] = False
        print(f"[{TAG}] 缺 {spv.name},无法做 RED 反证", file=sys.stderr)
        return False
    raw = bytearray(spv.read_bytes())
    raw[0] ^= 0xFF
    bad = work / "vsm_depth_raster_tampered.spv"
    bad.write_bytes(bytes(raw))
    code, _ = spirv_val(bad, None)
    if code == -1:
        results["toolchain_skip"] = "no-spirv-val"
        return True
    if code == 0:
        results["red_tamper_spv_pass"] = False
        print(f"[{TAG}] 篡改后的 .spv 仍被 spirv-val 接受(校验轴失效)", file=sys.stderr)
        return False
    results["red_tamper_spv_pass"] = True
    print(f"[{TAG}] 步骤 7 PASS: 篡改 .spv → spirv-val 拒(退出码 {code}),校验轴生效")
    return True


# ───────────────────── device 段(gate real,门 G-G7-7) ─────────────────────


def sw_baseline(results: dict, env: dict) -> bool:
    """SW/HW diff 的 **SW 基准侧**:W2 u64 VisBuffer 对 host 逐位相等(9216 词)。"""
    code, o, e = run(
        [
            "cargo", "test", "-q", "-p", "uc06-renderer", "--features", "vulkan",
            "--", "device_w2_visbuffer_u64_bitexact_host", "--nocapture",
        ],
        env=env,
    )
    blob = o + e
    m = re.search(r"visbuffer pixels=(\d+) covered=(\d+) triangles=(\d+)", blob)
    if code != 0 or m is None:
        print(blob[-2400:], file=sys.stderr)
        print(f"[{TAG}] SW 基准侧(W2 u64 VisBuffer 逐位)未取到真跑观测", file=sys.stderr)
        return False
    results["sw_baseline"] = {
        "kernel": "visbuffer_sw_u64",
        "pixels": int(m.group(1)),
        "covered_pixels": int(m.group(2)),
        "triangles": int(m.group(3)),
        "bitexact_vs_host": True,
        "tolerance": 0,
        "note": "SW/HW 整数域 diff 的 SW 侧基准已在树且逐位相等;缺的只有 HW 侧"
                "(见 hw_raster_diff.missing_toolchain_caps)",
    }
    print(
        f"[{TAG}] SW 基准侧 PASS: visbuffer_sw_u64 {m.group(1)} 词 u64 对 host 逐位相等"
        f"(覆盖 {m.group(2)} 像素 / {m.group(3)} 三角形,容差 0)"
    )
    return True


def device_section(results: dict) -> int:
    """步骤 8+9:RD-038 余项 device 真跑对拍 + SW 基准侧 + RED 两轴。"""
    code, o, e = run(
        ["cargo", "build", "-p", "uc06-renderer", "--features", "vulkan",
         "--bin", "uc06-renderer", "--quiet"]
    )
    if code != 0:
        print((o + e)[-2400:], file=sys.stderr)
        results["device_pass"] = False
        return fail("[device] cargo build uc06-renderer --features vulkan 失败(host 编译红)")

    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    results["validation_enabled"] = True
    code, out, err = run(
        ["cargo", "run", "-q", "-p", "uc06-renderer", "--features", "vulkan",
         "--bin", "uc06-renderer", "--", "--g75-residuals"],
        env=env,
    )
    blob = out + err
    red = {"tamper_vsm_depth": None, "tamper_tsr_jitter": None}
    results["device_red"] = red
    if "G75: SKIP" in blob:
        reason = next(
            (ln.split("G75: SKIP", 1)[1].strip() for ln in blob.splitlines() if "G75: SKIP" in ln),
            "unknown",
        )
        results["device_pass"] = None
        results["device_skip_reason"] = reason
        return skip(f"[device] uc06-renderer --g75-residuals SKIP({reason})")
    if code != 0 or "G75: PASS" not in blob:
        print(blob[-3000:], file=sys.stderr)
        results["device_pass"] = False
        return fail(f"[device] --g75-residuals 未 PASS(rc={code})")
    doc = None
    for line in blob.splitlines():
        line = line.strip()
        if line.startswith("{") and "uc06_g75_residuals" in line:
            try:
                doc = json.loads(line)
            except json.JSONDecodeError:
                doc = None
    if doc is None:
        results["device_pass"] = False
        return fail("[device] --g75-residuals 未产可解析的单行 JSON")

    # ── 机验:零容差量 + measured ≤ 冻结容差 + 非退化 ──
    if doc.get("vsm_sample_mismatches") != 0:
        results["device_pass"] = False
        return fail(
            f"[device] VSM 采样(0/1 二值,零容差)不一致数 = {doc.get('vsm_sample_mismatches')}"
        )
    pairs = (
        ("measured_vsm_depth_max_abs", "tol_vsm_depth"),
        ("measured_vsm_sample_max_abs", "tol_vsm_sample"),
        ("measured_tsr_max_abs", "tol_tsr"),
    )
    for m, t in pairs:
        if doc.get(m) is None or doc.get(t) is None:
            results["device_pass"] = False
            return fail(f"[device] evidence 缺 measured/tol 成对字段: {m}/{t}")
        if float(doc[m]) > float(doc[t]):
            results["device_pass"] = False
            return fail(f"[device] {m}={doc[m]} > {t}={doc[t]}(冻结容差外)")
    degenerate = {
        "vsm_depth_covered_texels": doc.get("vsm_depth_covered_texels"),
        "tsr_clamped_channels": doc.get("tsr_clamped_channels"),
        "vsm_pages": doc.get("vsm_pages"),
    }
    bad_deg = {k: v for k, v in degenerate.items() if not v}
    if bad_deg:
        results["device_pass"] = False
        return fail(f"[device] 判据面退化(对拍在常量上空转): {bad_deg}")
    if not (doc.get("vsm_depth_pass") and doc.get("vsm_sample_pass") and doc.get("tsr_pass")):
        results["device_pass"] = False
        return fail(f"[device] 逐项判定未全过: {doc}")

    results["device_name"] = doc.get("device_name")
    results["residual_parity"] = {
        "measured_vsm_depth_max_abs": doc["measured_vsm_depth_max_abs"],
        "tol_vsm_depth": doc["tol_vsm_depth"],
        "measured_vsm_sample_max_abs": doc["measured_vsm_sample_max_abs"],
        "tol_vsm_sample": doc["tol_vsm_sample"],
        "vsm_sample_mismatches": doc["vsm_sample_mismatches"],
        "measured_tsr_max_abs": doc["measured_tsr_max_abs"],
        "tol_tsr": doc["tol_tsr"],
        "vsm_depth_pass": doc["vsm_depth_pass"],
        "vsm_sample_pass": doc["vsm_sample_pass"],
        "tsr_pass": doc["tsr_pass"],
    }
    results["residual_stats"] = {
        "vsm_pages": doc["vsm_pages"],
        "vsm_depth_texels": doc["vsm_depth_texels"],
        "vsm_triangles": doc["vsm_triangles"],
        "vsm_depth_bitexact_texels": doc["vsm_depth_bitexact_texels"],
        "vsm_depth_covered_texels": doc["vsm_depth_covered_texels"],
        "vsm_samples": doc["vsm_samples"],
        "vsm_shadowed_ratio_device": doc["vsm_shadowed_ratio_device"],
        "tsr_in_w": doc["tsr_in_w"],
        "tsr_in_h": doc["tsr_in_h"],
        "tsr_out_w": doc["tsr_out_w"],
        "tsr_out_h": doc["tsr_out_h"],
        "tsr_channels": doc["tsr_channels"],
        "tsr_bitexact_channels": doc["tsr_bitexact_channels"],
        "tsr_clamped_channels": doc["tsr_clamped_channels"],
    }
    results["input_provenance"] = {
        "summary": doc["input_provenance"],
        "vsm_light_space": "灯空间三角形由 host LightBasis::to_light 预变换(场景装配面);"
                           "逐页 origin/page_world/z_range 为 host 页表窗口状态快照(配置面)",
        "vsm_device_work": "device 真做逐纹素 × 逐三角形边函数覆盖、重心深度插值与 min 归约;"
                           "采样侧真做距离/选级/回退环/页表寻址解包/纹素定位/深度比较",
        "tsr_color_source": "冻结场景 GBuffer(世界法线 × 前景深度权重,含真实轮廓硬边)",
        "tsr_jitter": "冻结 Halton 序列首项(temporal::common::jitter_sequence)",
        "residual_note": "device 与 host 表达式及求值序逐字一致;残差唯一来源 = SPIR-V 侧未加 "
                         "NoContraction 装饰时驱动可做 FMA 收缩(实测量级为 f32 ULP 数倍),"
                         "故按 measured→冻结口径设容差,未改 host oracle 数值语义",
        "temporal_arm_boundary": "TSR 时域臂(history 双缓冲 / 闪烁 EMA / reproject+validity)"
                                 "为跨帧状态机,归 G7.6 步骤 96;本波只兑现空间超分核",
    }
    results["device_capability_snapshot"] = (
        f"device_name={doc.get('device_name')} wave=W1(require_wave 通过;余项三核仅用 "
        f"f32/u32 SSBO,不触 W2/W3 能力面)"
    )

    if not sw_baseline(results, env):
        results["device_pass"] = False
        return fail("[device] SW/HW diff 的 SW 基准侧未取证")

    # ── RED 两轴 ──
    for flag, key, token in (
        ("--g75-red-vsm", "tamper_vsm_depth", "G75: RED-OK tamper-vsm-depth"),
        ("--g75-red-tsr", "tamper_tsr_jitter", "G75: RED-OK tamper-tsr-jitter"),
    ):
        rc, o2, e2 = run(
            ["cargo", "run", "-q", "-p", "uc06-renderer", "--features", "vulkan",
             "--bin", "uc06-renderer", "--", flag],
            env=env,
        )
        blob2 = o2 + e2
        red[key] = token in blob2
        if rc != 0 or not red[key]:
            print(blob2[-2400:], file=sys.stderr)
            results["device_pass"] = False
            return fail(f"[device] RED 轴 {flag} 失效(篡改后对拍仍通过)")

    results["device_pass"] = True
    print(
        f"[{TAG}] 步骤 8+9 PASS: RD-038 余项 device 真跑对拍 "
        f"(vsm_depth={doc['measured_vsm_depth_max_abs']:.3e}/{doc['tol_vsm_depth']:.3e} "
        f"vsm_sample={doc['measured_vsm_sample_max_abs']:.3e}(零容差,{doc['vsm_samples']} 采样) "
        f"tsr={doc['measured_tsr_max_abs']:.3e}/{doc['tol_tsr']:.3e})"
        f"+ SW 基准侧逐位 + RED 两轴全过;validation 零错误"
    )
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "renderer_raster_diff_smoke",
        "milestone": "G7.5 HW raster diff 与 RD-038 余项 / G-G7-7",
        "step": 95,
        "spec_clauses": ["RXS-0171", "RXS-0203", "RXS-0205", "RXS-0223"],
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {
            k: results[k]
            for k in (
                "literal_matrix_pass",
                "oracle_tests_pass",
                "visbuffer_abi_freeze_pass",
                "kernel_emit_pass",
                "hw_raster_blocked_honest_pass",
                "w1w2_zero_drift_pass",
                "red_tamper_spv_pass",
            )
            if results.get(k) is not None
        },
        "device_pass": results.get("device_pass"),
        "device_skip_reason": results.get("device_skip_reason"),
        "device_name": results.get("device_name"),
        "validation_enabled": results.get("validation_enabled", False),
        "device_red": results.get("device_red", {}),
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": results.get("toolchain_skip") is not None
        or results.get("device_pass") is None,
        "require_real": os.environ.get("RURIX_REQUIRE_REAL") == "1",
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    for key in (
        "literal_rows",
        "visbuffer_abi",
        "kernels",
        "hw_raster_diff",
        "sw_baseline",
        "device_capability_snapshot",
        "residual_parity",
        "residual_stats",
        "input_provenance",
    ):
        if results.get(key) is not None:
            doc[key] = results[key]
    ev = EVIDENCE_DIR / f"renderer_raster_diff_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"[{TAG}] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    work = ROOT / "target" / "g7_raster_diff_smoke"
    work.mkdir(parents=True, exist_ok=True)

    host_ok = literal_matrix_section(results)
    if host_ok:
        host_ok = oracle_section(results)
    if host_ok:
        host_ok = visbuffer_abi_section(results)
    if host_ok:
        host_ok = kernel_emit_section(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = hw_raster_capability_section(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = w1w2_zero_drift_section(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = red_tamper_spv_section(results, work)

    if results.get("toolchain_skip") is not None:
        write_evidence(results, host_ok, 0)
        return skip(f"[host] {results['toolchain_skip']}(spirv-val/spirv-dis 缺;编译段判据未取证)")

    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return fail("host 段未过(冻结面/oracle/ABI/编译/blocked-honest/零漂移/RED 反证)")
    if device_rc != 0:
        return device_rc
    print(
        f"[{TAG}] PASS(host 恒跑全绿;device 段 RD-038 余项真跑全绿)。"
        f"**G-G7-7 未全绿**:HW 光栅 SW/HW 整数域 diff=0 仍 blocked-honest"
        f"(见 evidence hw_raster_diff),RD-038 维持 open。"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
