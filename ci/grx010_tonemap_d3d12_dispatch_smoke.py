#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-010: standalone real Windows D3D12 dispatch smoke for the tonemap pass.

Template copy of ci/grx009_luminance_d3d12_dispatch_smoke.py pointed at the
GRX-010 tonemap package. It proves the *offline* tonemap artifacts (the
DXC-compiled DXIL container, the Rurix-owned RTS0 root signature, and the
descriptor layout) can complete **one minimal compute dispatch on a real
D3D12 device and command queue**, and additionally verifies the measured GPU
output texel against the CPU reference
(``linear_to_srgb(src * luminance_multiplier * exposure)``) within a small
tolerance. It produces measured smoke evidence only. It does NOT:

  * mark the Godot runtime tonemap pass as complete,
  * make the bridge default to RXGD_STATUS_OK,
  * claim any FPS / visual diff / measured fallback telemetry.

Discipline (mirrors the GRX-009 segment 4c smoke):

  * The device/command queue are always real: fake/null handles are never
    accepted. If there is no hardware D3D12 adapter or no D3D12 runtime, the
    harness records ``status=skip`` with a concrete reason. SKIP never
    advances the ready gate.
  * The tracked DXIL / RTS0 / descriptor layout artifacts are used as-is.
    Their SHA-256 digests must match the tracked offline compile evidence,
    and the descriptor layout must match the tonemap resource mapping
    (src_color=t0 texture2d SRV + dst_color=u0 rwtexture2d UAV, a 28-byte b0
    root-constant block). Any mismatch is ``status=fail``.
  * The SRV/UAV/root-constant bindings are created strictly from the
    descriptor layout; the harness never guesses resource shapes.
  * A ``status=success`` run records adapter/device info, artifact hashes,
    dispatch dimensions, fence completion, a readback checksum of the
    dst_color UAV, and the measured-vs-CPU-reference comparison.

If RURIX_REQUIRE_REAL=1, an environment that would otherwise SKIP becomes a
hard failure (exit 1); otherwise SKIP exits 0, matching the repo GPU-smoke
policy.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "tonemap"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "tonemap.dxil"
RTS0 = ARTIFACTS / "tonemap.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "tonemap_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
EVIDENCE_OUT = PASS_DIR / "real_d3d12_dispatch_smoke.json"
WORK = ROOT / "target" / "grx010_d3d12_dispatch_smoke"

SUBJECT = "grx010_tonemap_real_d3d12_dispatch_smoke"


def run(cmd: list[str], *, cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True)


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def now_iso() -> str:
    return _dt.datetime.now().astimezone().replace(microsecond=0).isoformat()


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")


def locate_signed_dxc_dir() -> Path | None:
    dirs: list[Path] = []
    for key in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        v = os.environ.get(key)
        if v:
            dirs.append(Path(v))
    dirs.append(KNOWN_DXC_DIR)
    for d in dirs:
        if (d / "dxil.dll").is_file():
            return d
    return None


def locate_dxcapi_include(dxc_dir: Path | None) -> Path | None:
    if dxc_dir is None:
        return None
    for base in (dxc_dir, *dxc_dir.parents):
        for name in ("inc", "include"):
            candidate = base / name / "dxcapi.h"
            if candidate.is_file():
                return candidate.parent
    return None


def locate_vcvars64() -> Path | None:
    override = os.environ.get("RURIX_VCVARS64")
    if override:
        p = Path(override)
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


def load_json(path: Path) -> dict | None:
    if not path.is_file():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def offline_artifact_digests(evidence: dict) -> dict[str, str | None]:
    artifacts = evidence.get("artifacts")
    out: dict[str, str | None] = {"dxil": None, "root_signature": None, "descriptor_layout": None}
    if isinstance(artifacts, dict):
        for key in out:
            entry = artifacts.get(key)
            if isinstance(entry, dict):
                sha = entry.get("sha256")
                if isinstance(sha, str):
                    out[key] = sha
    return out


def descriptor_layout_matches_resource_mapping(layout: dict) -> str | None:
    """Return None when the descriptor layout matches the tracked GRX-010
    tonemap resource mapping, otherwise a human-readable mismatch reason."""
    resources = layout.get("resources")
    if not isinstance(resources, list) or len(resources) != 2:
        return "descriptor layout does not declare exactly 2 resources"
    src, dst = resources[0], resources[1]
    if not (isinstance(src, dict) and src.get("name") == "src_color"
            and src.get("class") == "t" and src.get("register") == 0
            and src.get("binding_kind") == "texture2d"):
        return "resource[0] is not src_color SRV t0 (binding_kind texture2d)"
    if not (isinstance(dst, dict) and dst.get("name") == "dst_color"
            and dst.get("class") == "u" and dst.get("register") == 0
            and dst.get("binding_kind") == "rwtexture2d"):
        return "resource[1] is not dst_color UAV u0 (binding_kind rwtexture2d)"
    if layout.get("root_signature_parameters") != 2:
        return "root_signature_parameters != 2"
    if layout.get("root_constants") != 5:
        return "root_constants != 5"
    mapping = layout.get("grx010_mapping")
    if not isinstance(mapping, dict):
        return "missing grx010_mapping"
    if mapping.get("root_constant_bytes") != 28 or mapping.get("root_constant_dwords") != 7:
        return "root constant block is not 28 bytes / 7 dwords"
    names = [entry.get("name") for entry in layout.get("root_constant_layout", []) if isinstance(entry, dict)]
    if names != ["source_width", "source_height", "exposure", "white", "luminance_multiplier"]:
        return "root_constant_layout names do not match the tonemap contract"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx010-d3d12-dispatch-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx010-d3d12-dispatch-smoke] SKIP {msg}(降级 SKIP,退出 0)")
    write_evidence("skip", reason=msg, extra=extra or {})
    return 0


_EVIDENCE_BASE: dict = {}


def write_evidence(status: str, *, reason: str | None = None, extra: dict | None = None) -> None:
    doc = dict(_EVIDENCE_BASE)
    doc["status"] = status
    doc["timestamp"] = now_iso()
    doc["run_url"] = github_run_url()
    if reason is not None:
        doc["reason"] = reason
    if extra:
        doc.update(extra)
    EVIDENCE_OUT.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE_OUT.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx010-d3d12-dispatch-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


# ---------------------------------------------------------------------------
# Real D3D12 compute-dispatch harness (C++/MSVC), compiled on demand.
#
# argv: <dxil> <rts0> [dxil.dll]
# Exit codes: 0 = success, 1 = fail, 2 = skip (no adapter / runtime).
#
# Root signature is created DIRECTLY from the Rurix RTS0 bytes, the compute
# PSO from the Rurix DXIL container, and the SRV(t0)/UAV(u0)/b0 root
# constants are bound per the descriptor layout:
#   root param 0 = 7-dword (28-byte) b0 root constants
#   root param 1 = descriptor table [ SRV t0 (src_color), UAV u0 (dst_color) ]
#
# The 8x8 R32G32B32A32_FLOAT source is filled with rgb=1.0, a=0.25;
# constants are exposure=0.5, white=1.0, luminance_multiplier=1.0, so the
# CPU reference for every output texel is rgb=linear_to_srgb(0.5),
# a=0.25. The harness verifies the first readback texel against that
# reference within a small tolerance (measured GPU math check).
# ---------------------------------------------------------------------------
HARNESS_CPP = r"""#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <string>
#include <vector>

#include <dxcapi.h>

using Microsoft::WRL::ComPtr;

static int fail_hr(const char* what, HRESULT hr) {
    std::fprintf(stderr, "RXGD_DISPATCH: fail %s hr=0x%08lx\n", what, (unsigned long)hr);
    return 1;
}
static int fail_msg(const char* what) {
    std::fprintf(stderr, "RXGD_DISPATCH: fail %s\n", what);
    return 1;
}
static int skip_msg(const char* what) {
    std::fprintf(stderr, "RXGD_DISPATCH: skip %s\n", what);
    return 2;
}

static std::vector<uint8_t> read_file(const wchar_t* path, bool* ok) {
    *ok = false;
    std::ifstream f(path, std::ios::binary);
    if (!f) return {};
    f.seekg(0, std::ios::end);
    const auto n = f.tellg();
    if (n <= 0) return {};
    f.seekg(0, std::ios::beg);
    std::vector<uint8_t> data((size_t)n);
    f.read(reinterpret_cast<char*>(data.data()), n);
    if (!f) return {};
    *ok = true;
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
static D3D12_RESOURCE_DESC tex2d_desc(UINT w, UINT h, DXGI_FORMAT fmt, D3D12_RESOURCE_FLAGS flags) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    d.Width = w;
    d.Height = h;
    d.DepthOrArraySize = 1;
    d.MipLevels = 1;
    d.Format = fmt;
    d.SampleDesc.Count = 1;
    d.Flags = flags;
    return d;
}
static std::string narrow(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string out((size_t)std::max(n - 1, 0), '\0');
    if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr);
    return out;
}

// CPU reference: tonemap.glsl linear_to_srgb (a = 0.055).
static float linear_to_srgb_ref(float c) {
    if (c < 0.0f) c = 0.0f;
    if (c < 0.0031308f) return 12.92f * c;
    return 1.055f * std::pow(c, 1.0f / 2.4f) - 0.055f;
}

// Minimal in-memory IDxcBlob so the DXIL validator can sign our container
// bytes in place (the tracked artifact file is never modified).
struct MemBlob : public IDxcBlob {
    LONG m_ref;
    void* m_ptr;
    SIZE_T m_size;
    MemBlob(void* p, SIZE_T s) : m_ref(1), m_ptr(p), m_size(s) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == __uuidof(IUnknown) || riid == __uuidof(IDxcBlob)) {
            *ppv = static_cast<IDxcBlob*>(this);
            AddRef();
            return S_OK;
        }
        *ppv = nullptr;
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return (ULONG)InterlockedIncrement(&m_ref); }
    ULONG STDMETHODCALLTYPE Release() override { return (ULONG)InterlockedDecrement(&m_ref); }
    LPVOID STDMETHODCALLTYPE GetBufferPointer() override { return m_ptr; }
    SIZE_T STDMETHODCALLTYPE GetBufferSize() override { return m_size; }
};

static bool sign_dxil_in_place(std::vector<uint8_t>& dxil, const wchar_t* dxil_dll,
                               std::string* err) {
    HMODULE lib = dxil_dll ? LoadLibraryW(dxil_dll) : LoadLibraryW(L"dxil.dll");
    if (!lib) { *err = "LoadLibrary(dxil.dll) failed"; return false; }
    auto create = reinterpret_cast<DxcCreateInstanceProc>(GetProcAddress(lib, "DxcCreateInstance"));
    if (!create) { *err = "GetProcAddress(DxcCreateInstance) failed"; return false; }
    IDxcValidator* validator = nullptr;
    if (FAILED(create(CLSID_DxcValidator, __uuidof(IDxcValidator),
                      reinterpret_cast<void**>(&validator))) || !validator) {
        *err = "DxcCreateInstance(CLSID_DxcValidator) failed";
        return false;
    }
    MemBlob blob(dxil.data(), dxil.size());
    IDxcOperationResult* result = nullptr;
    HRESULT hr = validator->Validate(&blob, DxcValidatorFlags_InPlaceEdit, &result);
    bool ok = false;
    if (SUCCEEDED(hr) && result) {
        HRESULT status = E_FAIL;
        result->GetStatus(&status);
        ok = SUCCEEDED(status);
        if (!ok) *err = "validator rejected the DXIL container";
    } else {
        *err = "IDxcValidator::Validate failed";
    }
    if (result) result->Release();
    validator->Release();
    return ok;
}
"""

HARNESS_CPP += r"""
int wmain(int argc, wchar_t** argv) {
    if (argc < 3 || argc > 4) return fail_msg("usage: harness dxil rts0 [dxil.dll]");
    bool ok_dxil = false, ok_rts0 = false;
    auto dxil = read_file(argv[1], &ok_dxil);
    const auto rts0 = read_file(argv[2], &ok_rts0);
    if (!ok_dxil || dxil.empty()) return fail_msg("read dxil");
    if (!ok_rts0 || rts0.empty()) return fail_msg("read rts0");
    const wchar_t* dxil_dll = (argc >= 4) ? argv[3] : nullptr;

    bool experimental = false;
    {
        static const GUID kExp = D3D12ExperimentalShaderModels;
        experimental = SUCCEEDED(D3D12EnableExperimentalFeatures(1, &kExp, nullptr, nullptr));
    }
    std::printf("RXGD_DISPATCH: experimental_shader_models=%s\n", experimental ? "on" : "off");

    std::string sign_err;
    const bool dxil_signed = sign_dxil_in_place(dxil, dxil_dll, &sign_err);
    std::printf("RXGD_DISPATCH: dxil_signed_for_load=%s\n", dxil_signed ? "yes" : "no");
    if (!dxil_signed)
        std::fprintf(stderr, "RXGD_DISPATCH: sign note: %s\n", sign_err.c_str());

    ComPtr<IDXGIFactory6> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory))))
        return skip_msg("no DXGI factory (no D3D12 runtime)");

    ComPtr<IDXGIAdapter1> chosen;
    DXGI_ADAPTER_DESC1 chosen_desc = {};
    SIZE_T best_mem = 0;
    for (UINT i = 0;; ++i) {
        ComPtr<IDXGIAdapter1> adapter;
        HRESULT e = factory->EnumAdapters1(i, &adapter);
        if (e == DXGI_ERROR_NOT_FOUND) break;
        if (FAILED(e)) break;
        DXGI_ADAPTER_DESC1 d = {};
        adapter->GetDesc1(&d);
        if (d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) continue;
        if (SUCCEEDED(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0,
                                        __uuidof(ID3D12Device), nullptr)) &&
            d.DedicatedVideoMemory >= best_mem) {
            best_mem = d.DedicatedVideoMemory;
            chosen = adapter;
            chosen_desc = d;
        }
    }
    if (!chosen) return skip_msg("no hardware D3D12 adapter");

    ComPtr<ID3D12Device> device;
    if (FAILED(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device))))
        return skip_msg("D3D12CreateDevice failed on hardware adapter");
    std::printf("RXGD_DISPATCH: adapter=\"%s\"\n", narrow(chosen_desc.Description).c_str());

    // (A) Root signature DIRECTLY from the Rurix RTS0 bytes (device-parse proof).
    ComPtr<ID3D12RootSignature> root;
    HRESULT hr_root = device->CreateRootSignature(0, rts0.data(), rts0.size(), IID_PPV_ARGS(&root));
    if (FAILED(hr_root)) return fail_hr("CreateRootSignature(rurix rts0)", hr_root);

    // (B) Compute PSO from the Rurix DXIL container.
    D3D12_COMPUTE_PIPELINE_STATE_DESC pd = {};
    pd.pRootSignature = root.Get();
    pd.CS = {dxil.data(), dxil.size()};
    ComPtr<ID3D12PipelineState> pso;
    HRESULT hr_pso = device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&pso));
    if (FAILED(hr_pso)) return fail_hr("CreateComputePipelineState(rurix dxil)", hr_pso);

    D3D12_COMMAND_QUEUE_DESC qd = {};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue;
    if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue))))
        return fail_msg("CreateCommandQueue");
    ComPtr<ID3D12CommandAllocator> alloc;
    if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc))))
        return fail_msg("CreateCommandAllocator");

    // Minimal source: 8x8 R32G32B32A32_FLOAT src_color (t0), rgb=1.0 a=0.25.
    const UINT w = 8, h = 8;
    const float src_rgb = 1.0f, src_a = 0.25f;
    const float exposure = 0.5f, white = 1.0f, lum_mult = 1.0f;
    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    auto upload_heap = heap_props(D3D12_HEAP_TYPE_UPLOAD);
    auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);

    auto src_desc = tex2d_desc(w, h, DXGI_FORMAT_R32G32B32A32_FLOAT, D3D12_RESOURCE_FLAG_NONE);
    ComPtr<ID3D12Resource> src;
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &src_desc,
                                               D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                               IID_PPV_ARGS(&src))))
        return fail_msg("CreateCommittedResource(src_color)");

    // dst_color (u0): full-res R32G32B32A32_FLOAT UAV (dst extent == src extent).
    auto dst_desc = tex2d_desc(w, h, DXGI_FORMAT_R32G32B32A32_FLOAT,
                               D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    ComPtr<ID3D12Resource> dst;
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &dst_desc,
                                               D3D12_RESOURCE_STATE_UNORDERED_ACCESS, nullptr,
                                               IID_PPV_ARGS(&dst))))
        return fail_msg("CreateCommittedResource(dst_color)");

    // Upload src texels.
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT sfp = {};
    UINT srows = 0; UINT64 srow_size = 0, stotal = 0;
    device->GetCopyableFootprints(&src_desc, 0, 1, 0, &sfp, &srows, &srow_size, &stotal);
    auto sup_desc = buffer_desc(stotal);
    ComPtr<ID3D12Resource> src_upload;
    if (FAILED(device->CreateCommittedResource(&upload_heap, D3D12_HEAP_FLAG_NONE, &sup_desc,
                                               D3D12_RESOURCE_STATE_GENERIC_READ, nullptr,
                                               IID_PPV_ARGS(&src_upload))))
        return fail_msg("CreateCommittedResource(src_upload)");
    uint8_t* sup = nullptr;
    D3D12_RANGE empty = {0, 0};
    if (FAILED(src_upload->Map(0, &empty, reinterpret_cast<void**>(&sup))))
        return fail_msg("Map src_upload");
    for (UINT y = 0; y < h; ++y) {
        float* rowp = reinterpret_cast<float*>(sup + sfp.Offset + (SIZE_T)y * sfp.Footprint.RowPitch);
        for (UINT x = 0; x < w; ++x) {
            rowp[x * 4 + 0] = src_rgb;
            rowp[x * 4 + 1] = src_rgb;
            rowp[x * 4 + 2] = src_rgb;
            rowp[x * 4 + 3] = src_a;
        }
    }
    src_upload->Unmap(0, nullptr);

    // Descriptor table heap: index 0 = SRV(t0, src), index 1 = UAV(u0, dst).
    D3D12_DESCRIPTOR_HEAP_DESC hd = {};
    hd.NumDescriptors = 2;
    hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap;
    if (FAILED(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap))))
        return fail_msg("CreateDescriptorHeap(cbv_srv_uav)");
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    D3D12_CPU_DESCRIPTOR_HANDLE cpu0 = heap->GetCPUDescriptorHandleForHeapStart();
    D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
    srv.Format = DXGI_FORMAT_R32G32B32A32_FLOAT;
    srv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
    srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    srv.Texture2D.MipLevels = 1;
    device->CreateShaderResourceView(src.Get(), &srv, cpu0);
    D3D12_CPU_DESCRIPTOR_HANDLE cpu1 = cpu0;
    cpu1.ptr += inc;
    D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
    uav.Format = DXGI_FORMAT_R32G32B32A32_FLOAT;
    uav.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
    device->CreateUnorderedAccessView(dst.Get(), nullptr, &uav, cpu1);

    // Readback buffer for the dst UAV.
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT dfp = {};
    UINT drows = 0; UINT64 drow_size = 0, dtotal = 0;
    device->GetCopyableFootprints(&dst_desc, 0, 1, 0, &dfp, &drows, &drow_size, &dtotal);
    auto rb_desc = buffer_desc(dtotal);
    ComPtr<ID3D12Resource> readback;
    if (FAILED(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                               D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                               IID_PPV_ARGS(&readback))))
        return fail_msg("CreateCommittedResource(readback)");

    ComPtr<ID3D12GraphicsCommandList> cmd;
    if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(),
                                        pso.Get(), IID_PPV_ARGS(&cmd))))
        return fail_msg("CreateCommandList");

    D3D12_TEXTURE_COPY_LOCATION tdst = {};
    tdst.pResource = src.Get();
    tdst.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    tdst.SubresourceIndex = 0;
    D3D12_TEXTURE_COPY_LOCATION tsrc = {};
    tsrc.pResource = src_upload.Get();
    tsrc.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    tsrc.PlacedFootprint = sfp;
    cmd->CopyTextureRegion(&tdst, 0, 0, 0, &tsrc, nullptr);
    D3D12_RESOURCE_BARRIER tb = {};
    tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    tb.Transition.pResource = src.Get();
    tb.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST;
    tb.Transition.StateAfter = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
    tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &tb);

    // Bind the Rurix root signature and issue one minimal dispatch.
    cmd->SetComputeRootSignature(root.Get());
    ID3D12DescriptorHeap* heaps[] = {heap.Get()};
    cmd->SetDescriptorHeaps(1, heaps);
    uint32_t rc[7];
    uint64_t sw = w, sh = h;
    std::memcpy(&rc[0], &sw, sizeof(uint64_t));      // source_width  (i64, dwords 0..1)
    std::memcpy(&rc[2], &sh, sizeof(uint64_t));      // source_height (i64, dwords 2..3)
    std::memcpy(&rc[4], &exposure, sizeof(float));   // exposure
    std::memcpy(&rc[5], &white, sizeof(float));      // white (unused for LINEAR)
    std::memcpy(&rc[6], &lum_mult, sizeof(float));   // luminance_multiplier
    cmd->SetComputeRoot32BitConstants(0, 7, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetPipelineState(pso.Get());
    const UINT gx = (w + 7) / 8, gy = (h + 7) / 8, gz = 1;
    cmd->Dispatch(gx, gy, gz);

    // Read back the dst UAV.
    D3D12_RESOURCE_BARRIER db = {};
    db.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    db.Transition.pResource = dst.Get();
    db.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    db.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
    db.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &db);
    D3D12_TEXTURE_COPY_LOCATION cdst = {};
    cdst.pResource = readback.Get();
    cdst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    cdst.PlacedFootprint = dfp;
    D3D12_TEXTURE_COPY_LOCATION csrc = {};
    csrc.pResource = dst.Get();
    csrc.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    csrc.SubresourceIndex = 0;
    cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
    if (FAILED(cmd->Close())) return fail_msg("Close command list");

    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence;
    if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence))))
        return fail_msg("CreateFence");
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!ev) return fail_msg("CreateEvent");
    if (FAILED(queue->Signal(fence.Get(), 1))) return fail_msg("Signal fence");
    if (fence->GetCompletedValue() < 1) {
        if (FAILED(fence->SetEventOnCompletion(1, ev))) return fail_msg("SetEventOnCompletion");
        WaitForSingleObject(ev, INFINITE);
    }
    CloseHandle(ev);
    const UINT64 fence_done = fence->GetCompletedValue();
    if (fence_done < 1) return fail_msg("fence did not reach completion");

    // Checksum + CPU-reference check of the dst readback.
    uint8_t* mapped = nullptr;
    D3D12_RANGE range = {0, (SIZE_T)dtotal};
    if (FAILED(readback->Map(0, &range, reinterpret_cast<void**>(&mapped))))
        return fail_msg("Map readback");
    uint32_t checksum = 2166136261u;  // FNV-1a over the dst rows
    float first[4] = {0, 0, 0, 0};
    bool got_first = false;
    for (UINT y = 0; y < h; ++y) {
        const uint8_t* rowp = mapped + dfp.Offset + (SIZE_T)y * dfp.Footprint.RowPitch;
        for (UINT x = 0; x < w; ++x) {
            const uint8_t* px = rowp + (SIZE_T)x * 16;
            if (!got_first) { std::memcpy(first, px, 16); got_first = true; }
            for (int b = 0; b < 16; ++b) { checksum ^= px[b]; checksum *= 16777619u; }
        }
    }
    readback->Unmap(0, nullptr);

    const float expected_rgb = linear_to_srgb_ref(src_rgb * lum_mult * exposure);
    const float tol_rgb = 2e-3f;   // absorbs GPU pow() approximation differences
    const float tol_a = 1e-6f;
    const bool rgb_ok = std::fabs(first[0] - expected_rgb) <= tol_rgb &&
                        std::fabs(first[1] - expected_rgb) <= tol_rgb &&
                        std::fabs(first[2] - expected_rgb) <= tol_rgb;
    const bool a_ok = std::fabs(first[3] - src_a) <= tol_a;
    std::printf("RXGD_DISPATCH: cpu_reference expected_rgb=%g expected_a=%g observed=%g,%g,%g,%g match=%d\n",
                expected_rgb, src_a, first[0], first[1], first[2], first[3],
                (rgb_ok && a_ok) ? 1 : 0);
    if (!(rgb_ok && a_ok)) return fail_msg("dst texel does not match the CPU reference");

    std::printf("RXGD_DISPATCH: ok adapter=\"%s\" dispatch=%u,%u,%u fence=%llu "
                "dst=%ux%u dst_first=%g checksum=0x%08x\n",
                narrow(chosen_desc.Description).c_str(), gx, gy, gz,
                (unsigned long long)fence_done, w, h, first[0], checksum);
    return 0;
}
"""


def compile_harness(vcvars: Path, cpp: Path, exe: Path, include_dir: Path | None) -> tuple[bool, str]:
    obj = WORK / "harness.obj"
    bat = WORK / "build_dispatch_smoke.bat"
    include_flag = f'/I "{include_dir}" ' if include_dir is not None else ""
    bat.write_text(
        "@echo off\n"
        f'call "{vcvars}" >nul\n'
        "if errorlevel 1 exit /b %errorlevel%\n"
        f'cl /nologo /std:c++17 /EHsc /W4 /O2 /DUNICODE /D_UNICODE {include_flag}"{cpp}" '
        f'/Fe:"{exe}" /Fo:"{obj}" /link d3d12.lib dxgi.lib\n',
        encoding="utf-8",
    )
    p = subprocess.run(["cmd.exe", "/d", "/c", str(bat)], cwd=WORK, capture_output=True, text=True)
    log = (p.stdout + p.stderr).strip()
    if p.returncode != 0 or not exe.is_file():
        return False, log[-3000:]
    return True, log[-2000:]


def parse_harness_output(output: str) -> dict:
    parsed: dict = {}
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("RXGD_DISPATCH: experimental_shader_models="):
            parsed["experimental_shader_models"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_DISPATCH: dxil_signed_for_load="):
            parsed["dxil_signed_for_load"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_DISPATCH: cpu_reference "):
            for token in ("expected_rgb=", "expected_a=", "observed=", "match="):
                idx = line.find(token)
                if idx >= 0:
                    value = line[idx + len(token):].split(" ", 1)[0]
                    parsed["cpu_" + token.rstrip("=")] = value
        elif line.startswith("RXGD_DISPATCH: ok "):
            for token in ("dispatch=", "fence=", "dst=", "dst_first=", "checksum="):
                idx = line.find(token)
                if idx >= 0:
                    value = line[idx + len(token):].split(" ", 1)[0]
                    parsed[token.rstrip("=")] = value
            a0 = line.find('adapter="')
            if a0 >= 0:
                a0 += len('adapter="')
                a1 = line.find('"', a0)
                if a1 > a0:
                    parsed["adapter"] = line[a0:a1]
    return parsed


def main() -> int:
    global _EVIDENCE_BASE

    for path in (DXIL, RTS0, DESCRIPTOR_LAYOUT, OFFLINE_EVIDENCE):
        if not path.is_file():
            _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
            return fail(f"required artifact missing: {path.relative_to(ROOT)}")

    dxil_sha = sha256_file(DXIL)
    rts0_sha = sha256_file(RTS0)
    layout_sha = sha256_file(DESCRIPTOR_LAYOUT)
    offline = load_json(OFFLINE_EVIDENCE)
    layout = load_json(DESCRIPTOR_LAYOUT)
    if offline is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read offline_compile_evidence.json")
    if layout is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read tonemap_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "tonemap",
        "segment": "standalone_dispatch_smoke",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "artifacts": {
            "dxil": {"path": str(DXIL.relative_to(ROOT)).replace("\\", "/"), "sha256": dxil_sha},
            "root_signature": {"path": str(RTS0.relative_to(ROOT)).replace("\\", "/"), "sha256": rts0_sha},
            "descriptor_layout": {
                "path": str(DESCRIPTOR_LAYOUT.relative_to(ROOT)).replace("\\", "/"),
                "sha256": layout_sha,
            },
        },
        "offline_evidence": {
            "path": str(OFFLINE_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
            "dxil_sha256": offline_digests["dxil"],
            "root_signature_sha256": offline_digests["root_signature"],
            "descriptor_layout_sha256": offline_digests["descriptor_layout"],
        },
        "artifact_hashes_match_offline_evidence": (
            dxil_sha == offline_digests["dxil"]
            and rts0_sha == offline_digests["root_signature"]
            and layout_sha == offline_digests["descriptor_layout"]
        ),
        "note": (
            "GRX-010 standalone real D3D12 dispatch smoke evidence only. Even a "
            "success here keeps runtime_state=fallback_only and real_gpu_pass=false; "
            "it is not a Godot runtime pass, visual, perf, or measured-telemetry claim."
        ),
    }

    if not _EVIDENCE_BASE["artifact_hashes_match_offline_evidence"]:
        return fail(
            "artifact SHA-256 does not match tracked offline compile evidence "
            f"(dxil={dxil_sha} vs {offline_digests['dxil']}, "
            f"rts0={rts0_sha} vs {offline_digests['root_signature']}, "
            f"layout={layout_sha} vs {offline_digests['descriptor_layout']})"
        )

    layout_issue = descriptor_layout_matches_resource_mapping(layout)
    if layout_issue is not None:
        return fail(f"descriptor layout / resource mapping mismatch: {layout_issue}")

    vcvars = locate_vcvars64()
    if vcvars is None:
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64);无法编译真实 D3D12 dispatch harness")

    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip(
            "未找到含 dxil.dll 的签名 DXC pin(set RURIX_DXC_DIR=H:\\dxc-round7\\extracted\\bin\\x64);"
            "无法为 DXC 产出的 DXIL container 在内存中签名以在非 Developer-Mode device 上加载"
        )
    include_dir = locate_dxcapi_include(dxc_dir)
    if include_dir is None:
        return skip(f"未在 {dxc_dir} 附近找到 dxcapi.h(签名路径无法编译)")
    dxil_dll = dxc_dir / "dxil.dll"

    WORK.mkdir(parents=True, exist_ok=True)
    cpp = WORK / "tonemap_dispatch_harness.cpp"
    exe = WORK / "tonemap_dispatch_harness.exe"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")

    built, build_log = compile_harness(vcvars, cpp, exe, include_dir)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 dispatch harness 失败(可能缺 Windows SDK D3D12 头/库)",
                    extra={"build_log_tail": build_log})

    p = run([str(exe), str(DXIL), str(RTS0), str(dxil_dll)], cwd=WORK)
    output = (p.stdout + p.stderr).strip()
    print(output)
    parsed = parse_harness_output(output)
    device_info = {
        "adapter": parsed.get("adapter"),
        "experimental_shader_models": parsed.get("experimental_shader_models"),
        "dxil_signed_for_load": parsed.get("dxil_signed_for_load"),
        "dxil_validator": str(dxil_dll).replace("\\", "/"),
    }

    if p.returncode == 2:
        return skip(
            "no real D3D12 device harness available (see harness output)",
            extra={"device": device_info, "stdout": output},
        )
    if p.returncode != 0 or "RXGD_DISPATCH: ok" not in output:
        return fail(
            "real D3D12 tonemap dispatch smoke failed",
            extra={"device": device_info, "exit_code": p.returncode, "stdout": output},
        )

    dispatch = {
        "dimensions": parsed.get("dispatch"),
        "fence_completed_value": parsed.get("fence"),
        "dst_shape": parsed.get("dst"),
        "dst_first_value": parsed.get("dst_first"),
        "readback_checksum": parsed.get("checksum"),
    }
    cpu_reference = {
        "formula": "linear_to_srgb(src.rgb * luminance_multiplier * exposure); alpha passthrough",
        "constants": {"exposure": 0.5, "white": 1.0, "luminance_multiplier": 1.0, "src_rgb": 1.0, "src_a": 0.25},
        "expected_rgb": parsed.get("cpu_expected_rgb"),
        "expected_a": parsed.get("cpu_expected_a"),
        "observed": parsed.get("cpu_observed"),
        "match": parsed.get("cpu_match"),
        "rgb_tolerance": 2e-3,
    }
    write_evidence(
        "success",
        extra={
            "device": device_info,
            "dispatch": dispatch,
            "cpu_reference": cpu_reference,
            "checks": {
                "artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "root_signature_create_from_rurix_rts0": True,
                "compute_pso_from_rurix_dxil": True,
                "srv_uav_root_constants_bound_from_layout": True,
                "dispatch_executed": True,
                "fence_completed": True,
                "dst_uav_readback": True,
                "dst_matches_cpu_reference": parsed.get("cpu_match") == "1",
            },
            "stdout": output,
        },
    )
    print(f"[grx010-d3d12-dispatch-smoke] PASS measured real D3D12 dispatch; adapter={device_info['adapter']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
