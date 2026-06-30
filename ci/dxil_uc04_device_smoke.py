#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-04 deferred 渲染器 device smoke(G-G2-4;RFC-0006 + RFC-0007 严格面:lighting pass
**真采样 G-buffer**,真延迟着色)。

G-G2-2(ci/dxil_device_smoke.py)证单 pass B 链 DXIL 在硬件出图;G-G2-3
(ci/dxil_binding_device_smoke.py)证 RFC-0005 RTS0 在硬件被 CreateRootSignature 接受。本
smoke 证 **G-G2-4**:UC-04 deferred 渲染器端到端——

  1. 把 4 个 UC-04 着色器(几何 pass VS/FS + lighting/合成 pass VS/FS)**Rurix 源**经 rurixc
     图形=B DXIL 链(cargo example emit_uc04_dxil:RXS-0171 body 降级 + RXS-0172/0173 签名
     保真 + 强制 signature_gate)落盘 DXIL 容器字节(**非手写 HLSL/DXIL**)。
  2. signed dxc pin 的 dxv.exe 逐个 validator 接受 4 个 DXIL。
  3. cargo build -p uc04-demo --features real-shim(cc 编 D3D12 离屏 shim,消费 Rurix DXIL +
     RFC-0005 RTS0,P-11)。
  4. 真硬件:几何 pass(Rurix VS/FS)写 G-buffer MRT → albedo RT→SRV barrier(RXS-0176 IR1)→
     lighting pass(Rurix VS/FS)**经 SRV t0 + Sampler s0 真采样 G-buffer albedo**(RXS-0175/0176,
     OpImageSampleExplicitLod LOD0)→ final = f(采样值) → offscreen readback 取 albedo 与 final
     中心像素对照(DXIL_UC04: ok 见证行);final.R 追踪采样到的 gbuffer.R。
  5. **数据流严格红绿(RXS-0176 IR2,核心)**:变体几何 FS 源(albedo 0.75→0.5)经同一图形=B 链产
     DXIL → device 复跑 → final 像素随之改变(证 final=f(采样值),非 lighting 自身输入);复原绿。
  6. DXIL 容器篡改红绿:篡改一个 Rurix DXIL 容器字节 → dxv 拒(validator 红)+ device PSO 创建拒
     (DXIL hash 不符 → device 红)→ 复原绿。

防降级硬门(G-G2-4):VS/FS 全部来自 Rurix 源经图形=B DXIL;RTS0 经 CreateRootSignature 真机
解析进 PSO;真 hardware 多 pass deferred draw + offscreen readback。禁手写 HLSL/DXIL、CPU 预
填、单 pass、fullscreen copy、固定像素、host-only、窗口截图、SKIP 充绿、复用 G-G2-2/3 smoke。

signed pin 纪律:signed DXC dir 须含 dxc.exe + dxv.exe + dxil.dll(PATH Vulkan dxc 不算)。
RURIX_REQUIRE_REAL=1 时缺工具/D3D12/MSVC 即硬失败;否则 SKIP 退出 0。run URL 不伪造:本机记
"local interactive runner",真实 GitHub Actions URL 为 owner-provided provenance。
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "target" / "dxil_uc04_device_smoke"
KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")
KNOWN_SPIRV_CROSS = Path(r"C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe")

SHADERS = ["uc04_gbuffer_vs", "uc04_gbuffer_fs", "uc04_lighting_vs", "uc04_lighting_fs"]
SRC_DIR = ROOT / "conformance" / "dxil" / "graphics" / "accept"


def fail(msg: str) -> int:
    print(f"[dxil_uc04_device_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[dxil_uc04_device_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    return 0


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def locate_signed_dxc_dir() -> Path | None:
    dirs: list[Path] = []
    for key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        v = os.environ.get(key)
        if v:
            dirs.append(Path(v))
    dirs.append(KNOWN_DXC_DIR)
    for d in dirs:
        if (d / "dxc.exe").is_file() and (d / "dxv.exe").is_file() and (d / "dxil.dll").is_file():
            return d
    return None


def locate_spirv_cross() -> Path | None:
    v = os.environ.get("RURIX_SPIRV_CROSS")
    if v and Path(v).is_file():
        return Path(v)
    if KNOWN_SPIRV_CROSS.is_file():
        return KNOWN_SPIRV_CROSS
    from shutil import which
    w = which("spirv-cross")
    return Path(w) if w else None


def tool_env(dxc_dir: Path, spirv_cross: Path) -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_DXC_DIR"] = str(dxc_dir)
    env["RURIX_DXC"] = str(dxc_dir / "dxc.exe")
    env["RURIX_SPIRV_CROSS"] = str(spirv_cross)
    return env


def run(cmd: list[str], *, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env)


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def emit_dxil(src: Path, out: Path, env: dict[str, str]) -> bool:
    """把 UC-04 Rurix 着色源(任意路径)经图形=B DXIL 链落盘 DXIL 容器(cargo example
    emit_uc04_dxil)。数据流变体亦经此同一编译器链产出(非手编 DXIL,防降级)。"""
    p = run(
        ["cargo", "run", "-q", "-p", "rurixc", "--features", "dxil-backend shader-stages",
         "--example", "emit_uc04_dxil", "--", str(src), str(out)],
        env=env,
    )
    if p.returncode != 0 or not out.is_file():
        print((p.stdout + p.stderr)[-1600:], file=sys.stderr)
        return False
    return True


def dxv_validate(dxv: Path, path: Path) -> bool:
    p = run([str(dxv), str(path)])
    return p.returncode == 0 and "Validation succeeded" in (p.stdout + p.stderr)


def build_real_shim() -> bool:
    p = run(["cargo", "build", "-q", "-p", "uc04-demo", "--features", "real-shim"])
    if p.returncode != 0:
        print((p.stdout + p.stderr)[-2400:], file=sys.stderr)
        return False
    return True


PIXEL_RE = re.compile(
    r"DXIL_UC04: ok adapter=\"(?P<adapter>[^\"]*)\" "
    r"gbuffer=(?P<gr>\d+),(?P<gg>\d+),(?P<gb>\d+),(?P<ga>\d+) "
    r"final=(?P<fr>\d+),(?P<fg>\d+),(?P<fb>\d+),(?P<fa>\d+) draw=ok"
)


def device_run(dxil: dict[str, Path], env: dict[str, str]) -> tuple[bool, str, re.Match | None]:
    """运行 uc04-demo --features real-shim,真出图 + readback。返回 (ok, output, match)。"""
    p = run(
        ["cargo", "run", "-q", "-p", "uc04-demo", "--features", "real-shim", "--",
         str(dxil["uc04_gbuffer_vs"]), str(dxil["uc04_gbuffer_fs"]),
         str(dxil["uc04_lighting_vs"]), str(dxil["uc04_lighting_fs"])],
        env=env,
    )
    output = (p.stdout + p.stderr).strip()
    m = PIXEL_RE.search(output)
    ok = p.returncode == 0 and m is not None
    return ok, output, m


def main() -> int:
    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip("未找到含 dxc.exe + dxv.exe + dxil.dll 的签名 DXC pin"
                    "(set RURIX_DXC_DIR=H:\\dxc-round7\\extracted\\bin\\x64;PATH Vulkan dxc 不算签名)")
    spirv_cross = locate_spirv_cross()
    if spirv_cross is None:
        return skip("未找到 spirv-cross(set RURIX_SPIRV_CROSS;图形=B 链必需)")
    dxv = dxc_dir / "dxv.exe"
    env = tool_env(dxc_dir, spirv_cross)

    WORK.mkdir(parents=True, exist_ok=True)

    # 1) Rurix 源 → 图形=B DXIL 容器(4 个着色器;非手写 HLSL/DXIL)。
    dxil: dict[str, Path] = {}
    for stem in SHADERS:
        out = WORK / f"{stem}.dxil"
        if not emit_dxil(SRC_DIR / f"{stem}.rx", out, env):
            return fail(f"cargo example emit_uc04_dxil 产 {stem} DXIL 失败(图形=B 链)")
        dxil[stem] = out

    # 2) dxv validator 逐个接受(签名 pin)。
    for stem in SHADERS:
        if not dxv_validate(dxv, dxil[stem]):
            return fail(f"{stem} DXIL 未过 dxv validator")

    # 3) 编译 real-shim(cc 编 D3D12 离屏 shim)。
    if not build_real_shim():
        return fail("cargo build -p uc04-demo --features real-shim 失败(需 MSVC + Windows SDK D3D12)")

    # 4) device 真出图 + readback(green)。
    ok, output, m = device_run(dxil, env)
    print(output)
    if not ok or m is None:
        return fail("UC-04 device 多 pass deferred draw/readback 失败(green 路径)")
    gr = int(m.group("gr"))
    fr = int(m.group("fr"))
    adapter = m.group("adapter")
    # 严格面(RFC-0007):几何 pass FS 写常量 albedo 0.75 → gbuffer R8 ≈ 191;lighting pass FS
    # **真采样** albedo SRV → final = 采样到的 albedo → final.R 追踪 gbuffer.R(≈191)。
    if not (185 <= gr <= 197):
        return fail(f"G-buffer albedo 中心像素 R={gr} 不在期望 [185,197](几何 pass FS 写常量 0.75)")
    if not (185 <= fr <= 197):
        return fail(f"final 中心像素 R={fr} 不在期望 [185,197](lighting 真采样 albedo 0.75)")
    if abs(fr - gr) > 6:
        return fail(f"final.R={fr} 未追踪 gbuffer.R={gr}(|Δ|>6)——疑似 lighting 未真采样 G-buffer")

    # 5) 数据流严格判据红绿(RXS-0176 IR2,本轮核心):变体几何 FS 源(albedo 常量 0.75→0.5)经
    #    **同一图形=B 编译器链**产 DXIL → device 复跑 → final 像素**必须随之改变**(证 final =
    #    f(采样到的 G-buffer 值),而非 lighting 自身输入);复原绿。**非手编 DXIL**(防降级)。
    orig_geom_fs_src = (SRC_DIR / "uc04_gbuffer_fs.rx").read_text(encoding="utf-8")
    alt_geom_fs_src = orig_geom_fs_src.replace("albedo: 0.75", "albedo: 0.5")
    if alt_geom_fs_src == orig_geom_fs_src:
        return fail("无法构造数据流变体:uc04_gbuffer_fs.rx 未含 'albedo: 0.75' 锚点")
    alt_src = WORK / "uc04_gbuffer_fs_alt.rx"
    alt_src.write_text(alt_geom_fs_src, encoding="utf-8")
    geom_fs_alt = WORK / "uc04_gbuffer_fs_alt.dxil"
    if not emit_dxil(alt_src, geom_fs_alt, env):
        return fail("变体几何 FS(albedo 0.5)图形=B DXIL 产出失败")
    if not dxv_validate(dxv, geom_fs_alt):
        return fail("变体几何 FS DXIL 未过 dxv validator(变体须为合法 Rurix DXIL)")
    dxil_alt = dict(dxil)
    dxil_alt["uc04_gbuffer_fs"] = geom_fs_alt
    ok_alt, out_alt, m_alt = device_run(dxil_alt, env)
    print(out_alt)
    if not ok_alt or m_alt is None:
        return fail("变体几何 FS device 复跑失败(数据流 green 变体路径)")
    gr_alt = int(m_alt.group("gr"))
    fr_alt = int(m_alt.group("fr"))
    # 变体 albedo 0.5 → gbuffer R8 ≈ 127;若 lighting 真采样,final 随之 ≈127。
    if not (120 <= gr_alt <= 134):
        return fail(f"变体 G-buffer albedo R={gr_alt} 不在期望 [120,134](变体几何 FS 写常量 0.5)")
    if abs(fr_alt - fr) < 30:
        return fail(
            f"final 未随采样值改变(原 R={fr} → 变体 R={fr_alt},|Δ|<30)——final 不依赖采样值,未达严格面"
        )
    if abs(fr_alt - gr_alt) > 6:
        return fail(f"变体 final.R={fr_alt} 未追踪变体 gbuffer.R={gr_alt}(|Δ|>6)——疑似未真采样")
    # 复原绿:用原始几何 FS 复跑,final.R 回到 ≈191。
    ok_rg, _org, m_rg = device_run(dxil, env)
    if not ok_rg or m_rg is None or abs(int(m_rg.group("fr")) - fr) > 6:
        return fail("复原原始几何 FS 后 final 未回到原值(数据流红绿不闭合)")

    # 6) DXIL 容器篡改红绿:篡改几何 FS DXIL 容器头(DXBC fourcc 首字节)→ dxv 拒(validator
    #    红)+ device CreateGraphicsPipelineState 拒(device 红,证非 no-op/固定像素);复原绿。
    tampered = WORK / "uc04_gbuffer_fs.tampered.dxil"
    raw = bytearray(dxil["uc04_gbuffer_fs"].read_bytes())
    raw[0] ^= 0xFF  # 翻 DXBC 容器 fourcc 首字节 → 容器非法(对齐 G2.3 RTS0 篡改先例)。
    tampered.write_bytes(raw)
    if dxv_validate(dxv, tampered):
        return fail("篡改 DXIL 仍过 dxv validator(红路径失效)")
    dxil_bad = dict(dxil)
    dxil_bad["uc04_gbuffer_fs"] = tampered
    ok_bad, _out_bad, _m_bad = device_run(dxil_bad, env)
    if ok_bad:
        return fail("篡改 DXIL 仍 device 出图成功(红路径失效;device 未真校验 DXIL)")
    # 复原绿:用原始 DXIL 复跑必须仍 ok。
    ok_restore, _o, _mm = device_run(dxil, env)
    if not ok_restore:
        return fail("复原原始 DXIL 后 device 未恢复绿(红绿不闭合)")

    doc = {
        "schema_version": 1,
        "subject": "dxil_uc04_device_smoke",
        "status": "measured_local",
        "timestamp": _dt.datetime.now().astimezone().replace(microsecond=0).isoformat(),
        "milestone": "G2.4 / G-G2-4 (RFC-0006 + RFC-0007, strict: lighting 真采样 G-buffer)",
        "adapter": adapter,
        "pipeline": {
            "shaders_from_rurix_source": SHADERS,
            "geometry_pass": "Rurix VS/FS → G-buffer MRT (albedo R8G8B8A8 / normal R16F / depth R32F); FS 写常量 albedo 0.75",
            "lighting_pass": "Rurix VS/FS → final = f(sampled G-buffer albedo SRV) (RFC-0007 真采样, OpImageSampleExplicitLod LOD0, dx.op.sampleLevel)",
            "barrier": "RXS-0176 IR1: albedo RENDER_TARGET → PIXEL_SHADER_RESOURCE before lighting samples",
            "rts0": "RFC-0005 serialize_rts0 per-pass (geometry IA-only; lighting SRV t0 + Sampler s0 descriptor table), CreateRootSignature accept",
            "readback": "offscreen center pixel (albedo + final); final.R tracks sampled gbuffer.R",
            "sampling_completeness": "RD-021 closed (RFC-0007 first-converged subset: explicit LOD 0, Texture2D<f32>+Sampler+vec2<f32>→vec4<f32>); RD-022~024 defer remaining",
        },
        "pixels": {
            "gbuffer_albedo": [int(m.group("gr")), int(m.group("gg")), int(m.group("gb")), int(m.group("ga"))],
            "final": [int(m.group("fr")), int(m.group("fg")), int(m.group("fb")), int(m.group("fa"))],
        },
        "dataflow_redgreen": {
            "criterion": "RXS-0176 IR2: tamper geometry FS albedo constant (Rurix source 0.75→0.5, same B-chain) → final pixel changes",
            "original_albedo_const": 0.75,
            "variant_albedo_const": 0.5,
            "gbuffer_R_original": gr,
            "gbuffer_R_variant": gr_alt,
            "final_R_original": fr,
            "final_R_variant": fr_alt,
            "final_changed_with_sampled_value": abs(fr_alt - fr) >= 30,
        },
        "tools": {
            "dxc_dir": str(dxc_dir),
            "dxc_sha256": sha256_file(dxc_dir / "dxc.exe"),
            "dxv_sha256": sha256_file(dxv),
            "spirv_cross": str(spirv_cross),
        },
        "checks": {
            "rurix_source_to_dxil_b_chain": True,
            "dxv_validate_all_shaders": True,
            "real_shim_build": True,
            "hardware_multipass_deferred_draw": True,
            "lighting_samples_gbuffer_srv": True,
            "offscreen_readback_pixel_compare": True,
            "final_tracks_sampled_albedo": True,
            "dataflow_tamper_constant_changes_final": True,
            "tamper_dxil_dxv_reject": True,
            "tamper_dxil_device_reject": True,
            "restore_green": True,
        },
        "run_url": github_run_url(),
        "stdout": output,
    }
    result = WORK / "result.json"
    result.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[dxil_uc04_device_smoke] PASS adapter=\"{adapter}\" gbuffer.R={gr} final.R={fr}; "
          f"写 {result.relative_to(ROOT)}; run_url={doc['run_url']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
