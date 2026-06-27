#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""DXIL device smoke for G-G2-2.

This is the device half that the host-only DXIL smoke cannot prove:

  1. Compile a minimal VS/PS pair with dxc to signed DXIL.
  2. Validate both containers with dxv.exe.
  3. Build a tiny C++ D3D12 harness with MSVC.
  4. Create a real hardware D3D12 graphics PSO, draw offscreen, copy to readback,
     and verify the center pixel.

No window or external graphics test framework is required. If RURIX_REQUIRE_REAL=1,
missing tools are hard failures; otherwise missing tools SKIP with exit 0, matching
the existing GPU/D3D12 smoke discipline.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
import textwrap
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "target" / "dxil_device_smoke"
KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")


def run(cmd: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True)


def fail(msg: str) -> int:
    print(f"[dxil_device_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[dxil_device_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    return 0


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def candidate_dxc_dirs() -> list[Path]:
    dirs: list[Path] = []
    for key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        value = os.environ.get(key)
        if value:
            dirs.append(Path(value))
    if os.environ.get("RURIX_DXC"):
        dirs.append(Path(os.environ["RURIX_DXC"]).parent)
    dirs.append(KNOWN_DXC_DIR)
    found = shutil.which("dxc")
    if found:
        dirs.append(Path(found).parent)
    # Windows SDK dxc generally has dxil.dll but not dxv.exe; keep it last so
    # the diagnostic can say exactly what is missing.
    kits = Path(r"C:\Program Files (x86)\Windows Kits\10\bin")
    if kits.exists():
        for d in sorted(kits.glob(r"*\x64"), reverse=True):
            dirs.append(d)
    out: list[Path] = []
    seen: set[str] = set()
    for d in dirs:
        key = str(d).lower()
        if key not in seen:
            seen.add(key)
            out.append(d)
    return out


def locate_signed_dxc_dir() -> Path | None:
    for d in candidate_dxc_dirs():
        if (d / "dxc.exe").is_file() and (d / "dxv.exe").is_file() and (d / "dxil.dll").is_file():
            return d
    return None


def locate_vcvars64() -> Path | None:
    if os.environ.get("RURIX_VCVARS64"):
        p = Path(os.environ["RURIX_VCVARS64"])
        if p.is_file():
            return p
    candidates = [
        Path(r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"),
        Path(r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"),
    ]
    candidates.extend(Path(r"C:\Program Files").glob(r"Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat"))
    candidates.extend(Path(r"C:\Program Files (x86)").glob(r"Microsoft Visual Studio\*\*\VC\Auxiliary\Build\vcvars64.bat"))
    for p in candidates:
        if p.is_file():
            return p
    return None


VS_HLSL = """\
struct VsOut {
    float4 pos : SV_Position;
    float4 color : COLOR0;
};

VsOut VSMain(uint vid : SV_VertexID) {
    float2 p;
    if (vid == 0) {
        p = float2(-1.0, -1.0);
    } else if (vid == 1) {
        p = float2(-1.0, 3.0);
    } else {
        p = float2(3.0, -1.0);
    }
    VsOut o;
    o.pos = float4(p, 0.0, 1.0);
    o.color = float4(0.25, 0.50, 1.00, 1.0);
    return o;
}
"""

PS_HLSL = """\
struct PsIn {
    float4 pos : SV_Position;
    float4 color : COLOR0;
};

float4 PSMain(PsIn i) : SV_Target {
    return i.color;
}
"""


HARNESS_CPP = r"""#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <d3dcompiler.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

static void die_hr(const char* what, HRESULT hr) {
    std::fprintf(stderr, "DXIL_DEVICE: fail %s hr=0x%08lx\n", what, (unsigned long)hr);
    std::exit(1);
}

static void die_msg(const char* what) {
    std::fprintf(stderr, "DXIL_DEVICE: fail %s\n", what);
    std::exit(1);
}

static void check(HRESULT hr, const char* what) {
    if (FAILED(hr)) die_hr(what, hr);
}

static std::vector<uint8_t> read_file(const wchar_t* path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) die_msg("open shader");
    f.seekg(0, std::ios::end);
    const auto n = f.tellg();
    if (n <= 0) die_msg("empty shader");
    f.seekg(0, std::ios::beg);
    std::vector<uint8_t> data((size_t)n);
    f.read(reinterpret_cast<char*>(data.data()), n);
    if (!f) die_msg("read shader");
    return data;
}

static D3D12_HEAP_PROPERTIES heap_props(D3D12_HEAP_TYPE type) {
    D3D12_HEAP_PROPERTIES hp = {};
    hp.Type = type;
    hp.CPUPageProperty = D3D12_CPU_PAGE_PROPERTY_UNKNOWN;
    hp.MemoryPoolPreference = D3D12_MEMORY_POOL_UNKNOWN;
    hp.CreationNodeMask = 1;
    hp.VisibleNodeMask = 1;
    return hp;
}

static D3D12_RESOURCE_DESC buffer_desc(UINT64 bytes) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    d.Alignment = 0;
    d.Width = bytes;
    d.Height = 1;
    d.DepthOrArraySize = 1;
    d.MipLevels = 1;
    d.Format = DXGI_FORMAT_UNKNOWN;
    d.SampleDesc.Count = 1;
    d.SampleDesc.Quality = 0;
    d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    d.Flags = D3D12_RESOURCE_FLAG_NONE;
    return d;
}

static std::string narrow(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string out((size_t)std::max(n - 1, 0), '\0');
    if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr);
    return out;
}

int wmain(int argc, wchar_t** argv) {
    if (argc != 3) die_msg("usage: dxil_device_harness.exe vs.dxil ps.dxil");
    const auto vs = read_file(argv[1]);
    const auto ps = read_file(argv[2]);

    ComPtr<IDXGIFactory6> factory;
    check(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)), "CreateDXGIFactory2");

    ComPtr<IDXGIAdapter1> chosen;
    DXGI_ADAPTER_DESC1 chosen_desc = {};
    SIZE_T best_mem = 0;
    for (UINT i = 0;; ++i) {
        ComPtr<IDXGIAdapter1> adapter;
        HRESULT hr = factory->EnumAdapters1(i, &adapter);
        if (hr == DXGI_ERROR_NOT_FOUND) break;
        check(hr, "EnumAdapters1");
        DXGI_ADAPTER_DESC1 desc = {};
        adapter->GetDesc1(&desc);
        if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) continue;
        if (SUCCEEDED(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0,
                                        __uuidof(ID3D12Device), nullptr)) &&
            desc.DedicatedVideoMemory >= best_mem) {
            best_mem = desc.DedicatedVideoMemory;
            chosen = adapter;
            chosen_desc = desc;
        }
    }
    if (!chosen) die_msg("no hardware D3D12 adapter");

    ComPtr<ID3D12Device> device;
    check(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)),
          "D3D12CreateDevice");

    D3D12_COMMAND_QUEUE_DESC qd = {};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue;
    check(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)), "CreateCommandQueue");

    D3D12_ROOT_SIGNATURE_DESC rsd = {};
    rsd.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
    ComPtr<ID3DBlob> rs_blob;
    ComPtr<ID3DBlob> rs_err;
    check(D3D12SerializeRootSignature(&rsd, D3D_ROOT_SIGNATURE_VERSION_1, &rs_blob, &rs_err),
          "D3D12SerializeRootSignature");
    ComPtr<ID3D12RootSignature> root;
    check(device->CreateRootSignature(0, rs_blob->GetBufferPointer(), rs_blob->GetBufferSize(),
                                      IID_PPV_ARGS(&root)),
          "CreateRootSignature");

    const UINT width = 64;
    const UINT height = 64;
    D3D12_RESOURCE_DESC rt_desc = {};
    rt_desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    rt_desc.Width = width;
    rt_desc.Height = height;
    rt_desc.DepthOrArraySize = 1;
    rt_desc.MipLevels = 1;
    rt_desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    rt_desc.SampleDesc.Count = 1;
    rt_desc.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
    rt_desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
    D3D12_CLEAR_VALUE clear = {};
    clear.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    clear.Color[3] = 1.0f;
    ComPtr<ID3D12Resource> rt;
    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    check(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &rt_desc,
                                          D3D12_RESOURCE_STATE_RENDER_TARGET, &clear,
                                          IID_PPV_ARGS(&rt)),
          "CreateCommittedResource(rt)");

    D3D12_DESCRIPTOR_HEAP_DESC rtv_hd = {};
    rtv_hd.NumDescriptors = 1;
    rtv_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    ComPtr<ID3D12DescriptorHeap> rtv_heap;
    check(device->CreateDescriptorHeap(&rtv_hd, IID_PPV_ARGS(&rtv_heap)), "CreateDescriptorHeap(rtv)");
    D3D12_CPU_DESCRIPTOR_HANDLE rtv = rtv_heap->GetCPUDescriptorHandleForHeapStart();
    device->CreateRenderTargetView(rt.Get(), nullptr, rtv);

    D3D12_GRAPHICS_PIPELINE_STATE_DESC pd = {};
    pd.pRootSignature = root.Get();
    pd.VS = {vs.data(), vs.size()};
    pd.PS = {ps.data(), ps.size()};
    pd.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
    pd.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
    pd.RasterizerState.DepthClipEnable = TRUE;
    pd.BlendState.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
    pd.DepthStencilState.DepthEnable = FALSE;
    pd.DepthStencilState.StencilEnable = FALSE;
    pd.SampleMask = UINT_MAX;
    pd.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
    pd.NumRenderTargets = 1;
    pd.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
    pd.SampleDesc.Count = 1;
    ComPtr<ID3D12PipelineState> pso;
    check(device->CreateGraphicsPipelineState(&pd, IID_PPV_ARGS(&pso)), "CreateGraphicsPipelineState");

    D3D12_PLACED_SUBRESOURCE_FOOTPRINT footprint = {};
    UINT rows = 0;
    UINT64 row_size = 0;
    UINT64 total_bytes = 0;
    device->GetCopyableFootprints(&rt_desc, 0, 1, 0, &footprint, &rows, &row_size, &total_bytes);
    auto rb_desc = buffer_desc(total_bytes);
    auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);
    ComPtr<ID3D12Resource> readback;
    check(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                          D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                          IID_PPV_ARGS(&readback)),
          "CreateCommittedResource(readback)");

    ComPtr<ID3D12CommandAllocator> alloc;
    check(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc)),
          "CreateCommandAllocator");
    ComPtr<ID3D12GraphicsCommandList> cmd;
    check(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(), pso.Get(),
                                   IID_PPV_ARGS(&cmd)),
          "CreateCommandList");

    const float clear_color[4] = {0, 0, 0, 1};
    cmd->OMSetRenderTargets(1, &rtv, FALSE, nullptr);
    cmd->ClearRenderTargetView(rtv, clear_color, 0, nullptr);
    D3D12_VIEWPORT vp = {0, 0, (float)width, (float)height, 0, 1};
    D3D12_RECT sc = {0, 0, (LONG)width, (LONG)height};
    cmd->RSSetViewports(1, &vp);
    cmd->RSSetScissorRects(1, &sc);
    cmd->SetGraphicsRootSignature(root.Get());
    cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    cmd->DrawInstanced(3, 1, 0, 0);

    D3D12_RESOURCE_BARRIER b = {};
    b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    b.Transition.pResource = rt.Get();
    b.Transition.StateBefore = D3D12_RESOURCE_STATE_RENDER_TARGET;
    b.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
    b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &b);

    D3D12_TEXTURE_COPY_LOCATION src = {};
    src.pResource = rt.Get();
    src.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    src.SubresourceIndex = 0;
    D3D12_TEXTURE_COPY_LOCATION dst = {};
    dst.pResource = readback.Get();
    dst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    dst.PlacedFootprint = footprint;
    cmd->CopyTextureRegion(&dst, 0, 0, 0, &src, nullptr);
    check(cmd->Close(), "Close command list");

    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence;
    check(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)), "CreateFence");
    HANDLE event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!event) die_msg("CreateEvent");
    check(queue->Signal(fence.Get(), 1), "Signal");
    if (fence->GetCompletedValue() < 1) {
        check(fence->SetEventOnCompletion(1, event), "SetEventOnCompletion");
        WaitForSingleObject(event, INFINITE);
    }
    CloseHandle(event);

    uint8_t* mapped = nullptr;
    D3D12_RANGE range = {0, (SIZE_T)total_bytes};
    check(readback->Map(0, &range, reinterpret_cast<void**>(&mapped)), "Map readback");
    const UINT x = width / 2;
    const UINT y = height / 2;
    const uint8_t* px = mapped + footprint.Offset + y * footprint.Footprint.RowPitch + x * 4;
    const uint8_t r = px[0], g = px[1], bl = px[2], a = px[3];
    readback->Unmap(0, nullptr);

    const bool ok = (r >= 55 && r <= 75 && g >= 118 && g <= 138 && bl >= 240 && a >= 250);
    if (!ok) {
        std::fprintf(stderr, "DXIL_DEVICE: fail pixel=%u,%u,%u,%u\n", r, g, bl, a);
        return 1;
    }
    std::printf("DXIL_DEVICE: ok adapter=\"%s\" pixel=%u,%u,%u,%u draw=ok\n",
                narrow(chosen_desc.Description).c_str(), r, g, bl, a);
    return 0;
}
"""


def write_inputs(work: Path) -> tuple[Path, Path, Path]:
    vs_hlsl = work / "device_vs.hlsl"
    ps_hlsl = work / "device_ps.hlsl"
    cpp = work / "dxil_device_harness.cpp"
    vs_hlsl.write_text(VS_HLSL, encoding="utf-8")
    ps_hlsl.write_text(PS_HLSL, encoding="utf-8")
    cpp.write_text(HARNESS_CPP, encoding="utf-8")
    return vs_hlsl, ps_hlsl, cpp


def compile_shader(dxc: Path, hlsl: Path, profile: str, entry: str, out: Path) -> bool:
    p = run([str(dxc), "-T", profile, "-E", entry, "-Fo", str(out), str(hlsl)], cwd=hlsl.parent)
    if p.returncode != 0 or not out.is_file():
        print((p.stdout + p.stderr)[-1400:], file=sys.stderr)
        return False
    return True


def dxv_validate(dxv: Path, path: Path) -> bool:
    p = run([str(dxv), str(path)], cwd=path.parent)
    if p.returncode != 0:
        print((p.stdout + p.stderr)[-1000:], file=sys.stderr)
        return False
    return "Validation succeeded" in (p.stdout + p.stderr)


def dxv_rejects_tamper(dxv: Path, src: Path, dst: Path) -> bool:
    data = bytearray(src.read_bytes())
    data[0] ^= 0xFF
    dst.write_bytes(data)
    p = run([str(dxv), str(dst)], cwd=dst.parent)
    return p.returncode != 0


def compile_harness(vcvars: Path, cpp: Path, exe: Path) -> bool:
    obj = WORK / "dxil_device_harness.obj"
    bat = WORK / "build_dxil_device.bat"
    bat.write_text(
        "@echo off\n"
        f'call "{vcvars}" >nul\n'
        "if errorlevel 1 exit /b %errorlevel%\n"
        f'cl /nologo /std:c++17 /EHsc /W4 /DUNICODE /D_UNICODE "{cpp}" '
        f'/Fe:"{exe}" /Fo:"{obj}" '
        "/link d3d12.lib dxgi.lib d3dcompiler.lib\n",
        encoding="utf-8",
    )
    p = subprocess.run(["cmd.exe", "/d", "/c", str(bat)], cwd=WORK, capture_output=True, text=True)
    if p.returncode != 0 or not exe.is_file():
        print((p.stdout + p.stderr)[-3000:], file=sys.stderr)
        return False
    return True


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def main() -> int:
    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip("未找到含 dxc.exe + dxil.dll + dxv.exe 的签名 DXC 目录(set RURIX_DXC_DIR)")
    vcvars = locate_vcvars64()
    if vcvars is None:
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64)")

    WORK.mkdir(parents=True, exist_ok=True)
    vs_hlsl, ps_hlsl, cpp = write_inputs(WORK)
    dxc = dxc_dir / "dxc.exe"
    dxv = dxc_dir / "dxv.exe"
    vs_dxil = WORK / "device_vs.dxil"
    ps_dxil = WORK / "device_ps.dxil"
    exe = WORK / "dxil_device_harness.exe"

    if not compile_shader(dxc, vs_hlsl, "vs_6_0", "VSMain", vs_dxil):
        return fail("dxc VSMain -> DXIL 失败")
    if not compile_shader(dxc, ps_hlsl, "ps_6_0", "PSMain", ps_dxil):
        return fail("dxc PSMain -> DXIL 失败")
    if not dxv_validate(dxv, vs_dxil):
        return fail("VS DXIL 未过 dxv validator")
    if not dxv_validate(dxv, ps_dxil):
        return fail("PS DXIL 未过 dxv validator")
    if not dxv_rejects_tamper(dxv, ps_dxil, WORK / "device_ps.tampered.dxil"):
        return fail("dxv 篡改红路径失效(损坏 DXIL 未被拒绝)")

    if not compile_harness(vcvars, cpp, exe):
        return fail("MSVC 编译 D3D12 device harness 失败")

    p = run([str(exe), str(vs_dxil), str(ps_dxil)], cwd=WORK)
    output = (p.stdout + p.stderr).strip()
    print(output)
    if p.returncode != 0 or "DXIL_DEVICE: ok" not in output:
        return fail("D3D12 device draw/readback 失败")

    doc = {
        "schema_version": 1,
        "subject": "dxil_device_smoke",
        "status": "measured_local",
        "timestamp": _dt.datetime.now().astimezone().replace(microsecond=0).isoformat(),
        "tools": {
            "dxc_dir": str(dxc_dir),
            "dxc_sha256": sha256_file(dxc),
            "dxv_sha256": sha256_file(dxv),
            "vcvars64": str(vcvars),
        },
        "checks": {
            "vs_dxv": True,
            "ps_dxv": True,
            "tamper_dxv_reject": True,
            "d3d12_pso": True,
            "offscreen_draw_readback": True,
        },
        "run_url": github_run_url(),
        "stdout": output,
    }
    result = WORK / "result.json"
    result.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[dxil_device_smoke] PASS 写 {result.relative_to(ROOT)}; run_url={doc['run_url']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
