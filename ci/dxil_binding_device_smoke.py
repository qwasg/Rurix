#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""DXIL binding-layout device smoke for G-G2-3 (E2b-4).

The G-G2-2 device smoke (ci/dxil_device_smoke.py) proves a B-chain DXIL VS/PS pair
draws on real hardware with an *empty* root signature. This smoke proves the half it
cannot: the G2.3 **binding-layout product** itself on a real D3D12 device.

  1. Emit the Rurix-derived RTS0 root-signature container bytes via the public
     binding_layout API (cargo example emit_binding_rts0) for the production-reachable
     resource set {Texture2D<f32> tex (SRV), Sampler samp}.
  2. Cross-check those bytes' SHA-256 against the *blessed* golden baseline
     (tests/dxil/binding/fs_tex_samp.binding-golden) — proving the device consumes the
     exact blessed RTS0 product (ties device run to the host golden).
  3. Compile a textured VS/PS pair with the signed dxc pin; validate with dxv.exe.
  4. On real hardware: create the root signature *directly from the Rurix RTS0 bytes*
     (device-parse proof), bind the texture+sampler through the derived SRV/Sampler
     descriptor tables, draw a textured triangle offscreen, read back the center pixel,
     and verify the sampled texel color.
  5. Device-level red path: a tampered RTS0 container must be rejected by
     CreateRootSignature (proves the accept is a real parse, not a no-op).

Signed pin discipline (owner requirement, E2b-4): the signed DXC dir MUST contain
dxc.exe + dxv.exe + dxil.dll. A PATH Vulkan-SDK dxc (no dxv/dxil.dll) is NOT accepted
as a signed basis. RURIX_DXC_DIR=H:\\dxc-round7\\extracted\\bin\\x64 takes priority.

If RURIX_REQUIRE_REAL=1, missing tools are hard failures; otherwise missing tools SKIP
with exit 0, matching the existing GPU/D3D12 smoke discipline. Run URLs are never
fabricated: local runs record "local interactive runner"; the real GitHub Actions URL
is owner-provided provenance.
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
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORK = ROOT / "target" / "dxil_binding_device_smoke"
KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")
GOLDEN = ROOT / "tests" / "dxil" / "binding" / "fs_tex_samp.binding-golden"


def run(cmd: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True)


def fail(msg: str) -> int:
    print(f"[dxil_binding_device_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg)
    print(f"[dxil_binding_device_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    return 0


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def locate_signed_dxc_dir() -> Path | None:
    """Signed pin: dir must carry dxc.exe + dxv.exe + dxil.dll. PATH Vulkan dxc
    (no dxv/dxil.dll) is intentionally NOT a signed basis (E2b-4 owner rule)."""
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


def golden_rts0_sha256() -> str | None:
    """Read the blessed rts0.bytes.sha256 from the golden (ties device input to bless)."""
    if not GOLDEN.is_file():
        return None
    m = re.search(r"rts0\.bytes\.sha256:\s*([0-9a-f]{64})", GOLDEN.read_text(encoding="utf-8"))
    return m.group(1) if m else None


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


VS_HLSL = """\
struct VsOut {
    float4 pos : SV_Position;
    float2 uv : TEXCOORD0;
};

VsOut VSMain(uint vid : SV_VertexID) {
    float2 p;
    float2 uv;
    if (vid == 0) {
        p = float2(-1.0, -1.0); uv = float2(0.0, 1.0);
    } else if (vid == 1) {
        p = float2(-1.0, 3.0); uv = float2(0.0, -1.0);
    } else {
        p = float2(3.0, -1.0); uv = float2(2.0, 1.0);
    }
    VsOut o;
    o.pos = float4(p, 0.0, 1.0);
    o.uv = uv;
    return o;
}
"""

# PS samples a Texture2D through a Sampler bound via the Rurix-derived root signature:
# SRV descriptor table at t0 (param 0), Sampler descriptor table at s0 (param 1).
PS_HLSL = """\
Texture2D g_tex : register(t0);
SamplerState g_samp : register(s0);

struct PsIn {
    float4 pos : SV_Position;
    float2 uv : TEXCOORD0;
};

float4 PSMain(PsIn i) : SV_Target {
    return g_tex.Sample(g_samp, i.uv);
}
"""


HARNESS_CPP = r"""#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>

using Microsoft::WRL::ComPtr;

static void die_hr(const char* what, HRESULT hr) {
    std::fprintf(stderr, "DXIL_BIND: fail %s hr=0x%08lx\n", what, (unsigned long)hr);
    std::exit(1);
}
static void die_msg(const char* what) {
    std::fprintf(stderr, "DXIL_BIND: fail %s\n", what);
    std::exit(1);
}
static void check(HRESULT hr, const char* what) { if (FAILED(hr)) die_hr(what, hr); }

static std::vector<uint8_t> read_file(const wchar_t* path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) die_msg("open input");
    f.seekg(0, std::ios::end);
    const auto n = f.tellg();
    if (n <= 0) die_msg("empty input");
    f.seekg(0, std::ios::beg);
    std::vector<uint8_t> data((size_t)n);
    f.read(reinterpret_cast<char*>(data.data()), n);
    if (!f) die_msg("read input");
    return data;
}

static D3D12_HEAP_PROPERTIES heap_props(D3D12_HEAP_TYPE type) {
    D3D12_HEAP_PROPERTIES hp = {};
    hp.Type = type;
    hp.CreationNodeMask = 1;
    hp.VisibleNodeMask = 1;
    return hp;
}
static D3D12_RESOURCE_DESC buffer_desc(UINT64 bytes) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    d.Width = bytes;
    d.Height = 1;
    d.DepthOrArraySize = 1;
    d.MipLevels = 1;
    d.Format = DXGI_FORMAT_UNKNOWN;
    d.SampleDesc.Count = 1;
    d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    return d;
}
static std::string narrow(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string out((size_t)std::max(n - 1, 0), '\0');
    if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr);
    return out;
}

int wmain(int argc, wchar_t** argv) {
    if (argc != 5) die_msg("usage: harness rts0.bin rts0_tampered.bin vs.dxil ps.dxil");
    const auto rts0 = read_file(argv[1]);
    const auto rts0_bad = read_file(argv[2]);
    const auto vs = read_file(argv[3]);
    const auto ps = read_file(argv[4]);

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

    // (A) Device-parse proof: create the root signature DIRECTLY from the Rurix RTS0
    //     bytes (no D3D12SerializeRootSignature). This is the G-G2-3 binding product.
    ComPtr<ID3D12RootSignature> root;
    HRESULT hr_root = device->CreateRootSignature(0, rts0.data(), rts0.size(), IID_PPV_ARGS(&root));
    if (FAILED(hr_root)) {
        std::fprintf(stderr, "DXIL_BIND: fail rurix RTS0 rejected hr=0x%08lx\n",
                     (unsigned long)hr_root);
        return 1;
    }
    // (B) Device-level red path: the tampered RTS0 container MUST be rejected.
    ComPtr<ID3D12RootSignature> root_bad;
    HRESULT hr_bad = device->CreateRootSignature(0, rts0_bad.data(), rts0_bad.size(),
                                                 IID_PPV_ARGS(&root_bad));
    if (SUCCEEDED(hr_bad)) {
        std::fprintf(stderr, "DXIL_BIND: fail tampered RTS0 accepted (red path dead)\n");
        return 1;
    }
"""
HARNESS_CPP += r"""
    D3D12_COMMAND_QUEUE_DESC qd = {};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue;
    check(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)), "CreateCommandQueue");
    ComPtr<ID3D12CommandAllocator> alloc;
    check(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc)),
          "CreateCommandAllocator");

    // Render target (64x64 RGBA8).
    const UINT width = 64, height = 64;
    D3D12_RESOURCE_DESC rt_desc = {};
    rt_desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    rt_desc.Width = width;
    rt_desc.Height = height;
    rt_desc.DepthOrArraySize = 1;
    rt_desc.MipLevels = 1;
    rt_desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    rt_desc.SampleDesc.Count = 1;
    rt_desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
    D3D12_CLEAR_VALUE clear = {};
    clear.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    clear.Color[3] = 1.0f;
    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    ComPtr<ID3D12Resource> rt;
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

    // Source texture: 1x1 RGBA8 with known texel (64,127,255,255).
    const uint8_t texel[4] = {64, 127, 255, 255};
    D3D12_RESOURCE_DESC tex_desc = {};
    tex_desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    tex_desc.Width = 1;
    tex_desc.Height = 1;
    tex_desc.DepthOrArraySize = 1;
    tex_desc.MipLevels = 1;
    tex_desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    tex_desc.SampleDesc.Count = 1;
    ComPtr<ID3D12Resource> tex;
    check(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &tex_desc,
                                          D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                          IID_PPV_ARGS(&tex)),
          "CreateCommittedResource(tex)");
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT tfp = {};
    UINT trows = 0;
    UINT64 trow_size = 0, ttotal = 0;
    device->GetCopyableFootprints(&tex_desc, 0, 1, 0, &tfp, &trows, &trow_size, &ttotal);
    auto up_desc = buffer_desc(ttotal);
    auto upload_heap = heap_props(D3D12_HEAP_TYPE_UPLOAD);
    ComPtr<ID3D12Resource> tex_upload;
    check(device->CreateCommittedResource(&upload_heap, D3D12_HEAP_FLAG_NONE, &up_desc,
                                          D3D12_RESOURCE_STATE_GENERIC_READ, nullptr,
                                          IID_PPV_ARGS(&tex_upload)),
          "CreateCommittedResource(tex_upload)");
    uint8_t* up_ptr = nullptr;
    D3D12_RANGE empty = {0, 0};
    check(tex_upload->Map(0, &empty, reinterpret_cast<void**>(&up_ptr)), "Map tex_upload");
    std::memcpy(up_ptr + tfp.Offset, texel, 4);
    tex_upload->Unmap(0, nullptr);

    // Shader-visible SRV heap (param 0 = SRV table t0) + Sampler heap (param 1 = s0).
    D3D12_DESCRIPTOR_HEAP_DESC srv_hd = {};
    srv_hd.NumDescriptors = 1;
    srv_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    srv_hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> srv_heap;
    check(device->CreateDescriptorHeap(&srv_hd, IID_PPV_ARGS(&srv_heap)), "CreateDescriptorHeap(srv)");
    D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
    srv.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    srv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
    srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    srv.Texture2D.MipLevels = 1;
    device->CreateShaderResourceView(tex.Get(), &srv,
                                     srv_heap->GetCPUDescriptorHandleForHeapStart());

    D3D12_DESCRIPTOR_HEAP_DESC samp_hd = {};
    samp_hd.NumDescriptors = 1;
    samp_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER;
    samp_hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> samp_heap;
    check(device->CreateDescriptorHeap(&samp_hd, IID_PPV_ARGS(&samp_heap)), "CreateDescriptorHeap(samp)");
    D3D12_SAMPLER_DESC sd = {};
    sd.Filter = D3D12_FILTER_MIN_MAG_MIP_POINT;
    sd.AddressU = sd.AddressV = sd.AddressW = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
    sd.MaxLOD = D3D12_FLOAT32_MAX;
    device->CreateSampler(&sd, samp_heap->GetCPUDescriptorHandleForHeapStart());

    // PSO using the Rurix-derived root signature.
    D3D12_GRAPHICS_PIPELINE_STATE_DESC pd = {};
    pd.pRootSignature = root.Get();
    pd.VS = {vs.data(), vs.size()};
    pd.PS = {ps.data(), ps.size()};
    pd.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
    pd.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
    pd.RasterizerState.DepthClipEnable = TRUE;
    pd.BlendState.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
    pd.SampleMask = UINT_MAX;
    pd.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
    pd.NumRenderTargets = 1;
    pd.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
    pd.SampleDesc.Count = 1;
    ComPtr<ID3D12PipelineState> pso;
    check(device->CreateGraphicsPipelineState(&pd, IID_PPV_ARGS(&pso)),
          "CreateGraphicsPipelineState(rurix root sig)");

    // Readback buffer.
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT fp = {};
    UINT rows = 0;
    UINT64 row_size = 0, total_bytes = 0;
    device->GetCopyableFootprints(&rt_desc, 0, 1, 0, &fp, &rows, &row_size, &total_bytes);
    auto rb_desc = buffer_desc(total_bytes);
    auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);
    ComPtr<ID3D12Resource> readback;
    check(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                          D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                          IID_PPV_ARGS(&readback)),
          "CreateCommittedResource(readback)");

    ComPtr<ID3D12GraphicsCommandList> cmd;
    check(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(), pso.Get(),
                                   IID_PPV_ARGS(&cmd)),
          "CreateCommandList");

    // Upload texel, then transition texture to pixel-shader-resource.
    D3D12_TEXTURE_COPY_LOCATION tdst = {};
    tdst.pResource = tex.Get();
    tdst.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    tdst.SubresourceIndex = 0;
    D3D12_TEXTURE_COPY_LOCATION tsrc = {};
    tsrc.pResource = tex_upload.Get();
    tsrc.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    tsrc.PlacedFootprint = tfp;
    cmd->CopyTextureRegion(&tdst, 0, 0, 0, &tsrc, nullptr);
    D3D12_RESOURCE_BARRIER tb = {};
    tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    tb.Transition.pResource = tex.Get();
    tb.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST;
    tb.Transition.StateAfter = D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
    tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &tb);

    const float clear_color[4] = {0, 0, 0, 1};
    cmd->OMSetRenderTargets(1, &rtv, FALSE, nullptr);
    cmd->ClearRenderTargetView(rtv, clear_color, 0, nullptr);
    D3D12_VIEWPORT vp = {0, 0, (float)width, (float)height, 0, 1};
    D3D12_RECT sc = {0, 0, (LONG)width, (LONG)height};
    cmd->RSSetViewports(1, &vp);
    cmd->RSSetScissorRects(1, &sc);
    cmd->SetGraphicsRootSignature(root.Get());
    ID3D12DescriptorHeap* heaps[] = {srv_heap.Get(), samp_heap.Get()};
    cmd->SetDescriptorHeaps(2, heaps);
    // Derived layout: param 0 = SRV table (t0), param 1 = Sampler table (s0).
    cmd->SetGraphicsRootDescriptorTable(0, srv_heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetGraphicsRootDescriptorTable(1, samp_heap->GetGPUDescriptorHandleForHeapStart());
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
    dst.PlacedFootprint = fp;
    cmd->CopyTextureRegion(&dst, 0, 0, 0, &src, nullptr);
    check(cmd->Close(), "Close command list");

    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence;
    check(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)), "CreateFence");
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!ev) die_msg("CreateEvent");
    check(queue->Signal(fence.Get(), 1), "Signal");
    if (fence->GetCompletedValue() < 1) {
        check(fence->SetEventOnCompletion(1, ev), "SetEventOnCompletion");
        WaitForSingleObject(ev, INFINITE);
    }
    CloseHandle(ev);

    uint8_t* mapped = nullptr;
    D3D12_RANGE range = {0, (SIZE_T)total_bytes};
    check(readback->Map(0, &range, reinterpret_cast<void**>(&mapped)), "Map readback");
    const UINT x = width / 2, y = height / 2;
    const uint8_t* px = mapped + fp.Offset + y * fp.Footprint.RowPitch + x * 4;
    const uint8_t r = px[0], g = px[1], bl = px[2], a = px[3];
    readback->Unmap(0, nullptr);

    const bool ok = (r >= 55 && r <= 75 && g >= 118 && g <= 138 && bl >= 240 && a >= 250);
    if (!ok) {
        std::fprintf(stderr, "DXIL_BIND: fail sampled pixel=%u,%u,%u,%u\n", r, g, bl, a);
        return 1;
    }
    std::printf("DXIL_BIND: ok adapter=\"%s\" rurix_rts0=accept tamper_rts0=reject "
                "sampled=%u,%u,%u,%u draw=ok\n",
                narrow(chosen_desc.Description).c_str(), r, g, bl, a);
    return 0;
}
"""


def emit_rts0(out: Path) -> bool:
    """Emit the Rurix-derived RTS0 bytes via the public binding_layout API."""
    p = run(
        ["cargo", "run", "-q", "-p", "rurixc", "--features", "dxil-backend",
         "--example", "emit_binding_rts0", "--", str(out)],
    )
    if p.returncode != 0 or not out.is_file():
        print((p.stdout + p.stderr)[-1400:], file=sys.stderr)
        return False
    return True


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


def compile_harness(vcvars: Path, cpp: Path, exe: Path) -> bool:
    obj = WORK / "harness.obj"
    bat = WORK / "build_binding_device.bat"
    bat.write_text(
        "@echo off\n"
        f'call "{vcvars}" >nul\n'
        "if errorlevel 1 exit /b %errorlevel%\n"
        f'cl /nologo /std:c++17 /EHsc /W4 /DUNICODE /D_UNICODE "{cpp}" '
        f'/Fe:"{exe}" /Fo:"{obj}" /link d3d12.lib dxgi.lib\n',
        encoding="utf-8",
    )
    p = subprocess.run(["cmd.exe", "/d", "/c", str(bat)], cwd=WORK, capture_output=True, text=True)
    if p.returncode != 0 or not exe.is_file():
        print((p.stdout + p.stderr)[-3000:], file=sys.stderr)
        return False
    return True


def main() -> int:
    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip("未找到含 dxc.exe + dxv.exe + dxil.dll 的签名 DXC pin"
                    "(set RURIX_DXC_DIR=H:\\dxc-round7\\extracted\\bin\\x64;PATH Vulkan dxc 不算签名)")
    vcvars = locate_vcvars64()
    if vcvars is None:
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64)")

    blessed = golden_rts0_sha256()
    if blessed is None:
        return fail("读不到 blessed golden 的 rts0.bytes.sha256(tests/dxil/binding/fs_tex_samp.binding-golden)")

    WORK.mkdir(parents=True, exist_ok=True)
    rts0 = WORK / "rts0.bin"
    if not emit_rts0(rts0):
        return fail("cargo example emit_binding_rts0 落盘 RTS0 失败")

    # Tie device input to the blessed golden: emitted RTS0 SHA-256 must equal baseline.
    emitted_sha = sha256_file(rts0)
    if emitted_sha != blessed:
        return fail(f"RTS0 字节 SHA-256 与 blessed golden 不符: emitted={emitted_sha} blessed={blessed}")

    # Tampered RTS0 (corrupt the DXBC container fourcc) → device must reject (red path).
    rts0_bad = WORK / "rts0_tampered.bin"
    bad = bytearray(rts0.read_bytes())
    bad[0] ^= 0xFF  # 'D' of DXBC → invalid container
    rts0_bad.write_bytes(bad)

    vs_hlsl = WORK / "bind_vs.hlsl"
    ps_hlsl = WORK / "bind_ps.hlsl"
    vs_hlsl.write_text(VS_HLSL, encoding="utf-8")
    ps_hlsl.write_text(PS_HLSL, encoding="utf-8")
    cpp = WORK / "binding_device_harness.cpp"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")

    dxc = dxc_dir / "dxc.exe"
    dxv = dxc_dir / "dxv.exe"
    vs_dxil = WORK / "bind_vs.dxil"
    ps_dxil = WORK / "bind_ps.dxil"
    exe = WORK / "binding_device_harness.exe"

    if not compile_shader(dxc, vs_hlsl, "vs_6_0", "VSMain", vs_dxil):
        return fail("dxc VSMain -> DXIL 失败")
    if not compile_shader(dxc, ps_hlsl, "ps_6_0", "PSMain", ps_dxil):
        return fail("dxc PSMain -> DXIL 失败")
    if not dxv_validate(dxv, vs_dxil):
        return fail("VS DXIL 未过 dxv validator")
    if not dxv_validate(dxv, ps_dxil):
        return fail("PS DXIL 未过 dxv validator")

    if not compile_harness(vcvars, cpp, exe):
        return fail("MSVC 编译 binding device harness 失败")

    p = run([str(exe), str(rts0), str(rts0_bad), str(vs_dxil), str(ps_dxil)], cwd=WORK)
    output = (p.stdout + p.stderr).strip()
    print(output)
    if p.returncode != 0 or "DXIL_BIND: ok" not in output:
        return fail("D3D12 binding-layout device draw/readback 失败")

    doc = {
        "schema_version": 1,
        "subject": "dxil_binding_device_smoke",
        "status": "measured_local",
        "timestamp": _dt.datetime.now().astimezone().replace(microsecond=0).isoformat(),
        "binding_product": {
            "resource_set": "Texture2D<f32> tex (SRV t0) + Sampler samp (s0)",
            "rts0_bytes": rts0.stat().st_size,
            "rts0_sha256": emitted_sha,
            "rts0_sha256_matches_blessed_golden": True,
            "golden": "tests/dxil/binding/fs_tex_samp.binding-golden",
        },
        "tools": {
            "dxc_dir": str(dxc_dir),
            "dxc_sha256": sha256_file(dxc),
            "dxv_sha256": sha256_file(dxv),
            "vcvars64": str(vcvars),
        },
        "checks": {
            "rts0_emitted_matches_golden": True,
            "vs_dxv": True,
            "ps_dxv": True,
            "rurix_rts0_create_root_signature_accept": True,
            "tampered_rts0_create_root_signature_reject": True,
            "textured_pso_with_rurix_root_signature": True,
            "offscreen_textured_draw_readback": True,
        },
        "run_url": github_run_url(),
        "stdout": output,
    }
    result = WORK / "result.json"
    result.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[dxil_binding_device_smoke] PASS 写 {result.relative_to(ROOT)}; run_url={doc['run_url']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
