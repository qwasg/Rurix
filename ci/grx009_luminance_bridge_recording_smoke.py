#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-009 segment 4d: real Windows D3D12 dispatch recording smoke for the bridge.

Unlike the segment 4c *standalone* dispatch smoke (which builds its own D3D12
device and never touches the bridge), this harness proves the **Rurix Godot
bridge** (``rurix_godot.dll``, built with the ``d3d12-recording-shim`` feature)
can record **one minimal luminance compute dispatch on a real D3D12 device /
command queue** through its C ABI. It produces measured *bridge* smoke evidence
only. It does NOT:

  * enable the Godot runtime luminance Rurix path (that stays default-disabled),
  * mark the Godot runtime luminance pass as complete,
  * make the default (feature-off) bridge return RXGD_STATUS_OK,
  * claim any FPS / visual diff / measured fallback telemetry / GPU timestamp.

Discipline (mirrors ci/grx009_luminance_d3d12_dispatch_smoke.py):

  * The device/command queue/resources are always real: fake/null handles are
    never accepted. No hardware D3D12 adapter, no D3D12 runtime, or a device
    without the 64-bit integer shader capability records ``status=skip`` with a
    concrete reason. SKIP never advances the ready gate.
  * The tracked DXIL / root signature / descriptor layout digests must match the
    segment 3a offline compile evidence, and the descriptor layout must match the
    current resource mapping. Any mismatch is ``status=fail``.
  * ``rxgd_record_pass`` may return RXGD_STATUS_OK for RXGD_PASS_LUMINANCE_REDUCTION
    ONLY when the bridge recording shim is linked, the harness passes real D3D12
    device/queue and src/dst handles with the dispatch_bringup opt-in + record-arm
    flags + the 64-bit integer capability, and the embedded artifact bytes hash to
    the offline digests. Otherwise the bridge falls back and the smoke fails.
  * A ``status=success`` run records adapter/device info, artifact hashes, the
    bridge dispatch dimensions, fence completion, dst readback checksum, and the
    bridge frame stats (recorded_passes / fallback_passes / last_error / cpu_ns).
    GPU timestamps are not implemented: ``gpu_timestamp_status=not_yet`` and
    ``gpu_time_ns`` is never fabricated.

If RURIX_REQUIRE_REAL=1, an environment that would otherwise SKIP becomes a hard
failure (exit 1); otherwise SKIP exits 0, matching the repo GPU-smoke policy.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "luminance_reduction"
ARTIFACTS = PASS_DIR / "artifacts"
DXIL = ARTIFACTS / "luminance_reduction.dxil"
RTS0 = ARTIFACTS / "luminance_reduction.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "luminance_reduction_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
EVIDENCE_OUT = PASS_DIR / "bridge_dispatch_recording_evidence.json"
RURIX_GODOT_DIR = ROOT / "src" / "rurix-godot"
RURIX_GODOT_HEADER_DIR = RURIX_GODOT_DIR / "include"
RURIX_GODOT_DLL = ROOT / "target" / "debug" / "rurix_godot.dll"
WORK = ROOT / "target" / "grx009_bridge_recording_smoke"

SUBJECT = "grx009_luminance_bridge_d3d12_dispatch_recording_smoke"


def run(cmd: list[str], *, cwd: Path | None = None, env: dict | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd or ROOT, capture_output=True, text=True, env=env)


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


# Snapshot of the exact feature-built DLL this run exercised. target/debug is a
# mutable build tree; keeping an immutable copy under target/ (gitignored, never
# committed) lets later readers reproduce the exact artifact behind the evidence.
SNAPSHOT_DLL = WORK / "rurix_godot_d3d12_recording_shim.dll"


def dll_fingerprint(path: Path) -> dict:
    """Pin the feature-built DLL that this run exercised.

    ``target/debug/rurix_godot.dll`` is a *mutable* build artifact: a later
    feature-off ``cargo build -p rurix-godot`` overwrites it in place, so the
    bare path alone cannot prove which binary produced this evidence. Record the
    hash/size/mtime so a probe can tell whether the current on-disk DLL still
    matches the historical measured run, and note how to refresh it.
    """
    fp: dict = {
        "dll_path_at_run": str(path.relative_to(ROOT)).replace("\\", "/"),
        "dll_sha256": None,
        "dll_size_bytes": None,
        "dll_mtime_utc": None,
        "build_profile": "debug",
        "features": ["d3d12-recording-shim"],
        "mutable_artifact_note": (
            "target/debug/rurix_godot.dll is a mutable build artifact; a later "
            "feature-off `cargo build -p rurix-godot` can overwrite it in place. "
            "Rerun ci/grx009_luminance_bridge_recording_smoke.py to refresh this "
            "fingerprint and reproduce the exact feature-built DLL."
        ),
    }
    if not path.is_file():
        return fp
    stat = path.stat()
    fp["dll_sha256"] = sha256_file(path)
    fp["dll_size_bytes"] = stat.st_size
    fp["dll_mtime_utc"] = (
        _dt.datetime.fromtimestamp(stat.st_mtime, tz=_dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
    )
    return fp


def snapshot_feature_dll(path: Path) -> dict:
    """Copy the feature-built DLL to an immutable snapshot under target/ so the
    exact artifact stays reproducible even after target/debug is overwritten. The
    snapshot lives under target/ (gitignored) and is never committed to Git."""
    if not path.is_file():
        return {"snapshot_dll_path": None, "snapshot_dll_sha256": None,
                "snapshot_error": "feature-built DLL missing at snapshot time"}
    WORK.mkdir(parents=True, exist_ok=True)
    try:
        shutil.copy2(path, SNAPSHOT_DLL)
    except OSError as exc:
        return {"snapshot_dll_path": None, "snapshot_dll_sha256": None,
                "snapshot_error": f"{type(exc).__name__}: {exc}"}
    return {
        "snapshot_dll_path": str(SNAPSHOT_DLL.relative_to(ROOT)).replace("\\", "/"),
        "snapshot_dll_sha256": sha256_file(SNAPSHOT_DLL),
    }


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


KNOWN_DXC_DIR = Path(r"H:\dxc-round7\extracted\bin\x64")


def locate_signed_dxc_dir() -> Path | None:
    """Signed pin dir carrying dxil.dll (the DXIL validator used to sign the
    container so it loads without Developer Mode). RURIX_DXC_DIR takes priority."""
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
    """Return None when the descriptor layout matches the tracked resource
    mapping, otherwise a human-readable mismatch reason."""
    resources = layout.get("resources")
    if not isinstance(resources, list) or len(resources) != 2:
        return "descriptor layout does not declare exactly 2 resources"
    src, dst = resources[0], resources[1]
    if not (isinstance(src, dict) and src.get("name") == "src_luminance"
            and src.get("class") == "t" and src.get("register") == 0):
        return "resource[0] is not src_luminance SRV t0"
    if not (isinstance(dst, dict) and dst.get("name") == "dst_luminance"
            and dst.get("class") == "u" and dst.get("register") == 0):
        return "resource[1] is not dst_luminance UAV u0"
    if layout.get("root_signature_parameters") != 2:
        return "root_signature_parameters != 2"
    if layout.get("root_constants") != 5:
        return "root_constants != 5"
    mapping = layout.get("segment3b_mapping")
    if not isinstance(mapping, dict):
        return "missing segment3b_mapping"
    if mapping.get("root_constant_bytes") != 28 or mapping.get("root_constant_dwords") != 7:
        return "root constant block is not 28 bytes / 7 dwords"
    if mapping.get("requires_64bit_integer_shader_capability") is not True:
        return "layout does not require the 64-bit integer shader capability"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx009-bridge-recording-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx009-bridge-recording-smoke] SKIP {msg}(降级 SKIP,退出 0)")
    write_evidence("skip", reason=msg, extra=extra or {})
    return 0


# Assembled at runtime so the evidence always records the exact digests/paths.
_EVIDENCE_BASE: dict = {}


def write_evidence(status: str, *, reason: str | None = None, extra: dict | None = None) -> None:
    doc = dict(_EVIDENCE_BASE)
    doc["status"] = status
    # success is the only status allowed to assert a real bridge-recorded dispatch.
    doc["bridge_recorded_d3d12_dispatch"] = status == "success"
    doc["timestamp"] = now_iso()
    doc["run_url"] = github_run_url()
    if reason is not None:
        doc["reason"] = reason
    if extra:
        doc.update(extra)
    EVIDENCE_OUT.parent.mkdir(parents=True, exist_ok=True)
    # Byte-level LF only (repo .gitattributes pins `* -text`); never emit CRLF.
    EVIDENCE_OUT.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"[grx009-bridge-recording-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


# ---------------------------------------------------------------------------
# Real D3D12 bridge recording harness (C++/MSVC), compiled on demand.
#
# argv: <rurix_godot.dll>
# Exit codes: 0 = success, 1 = fail, 2 = skip (no adapter / no runtime / device
# lacks 64-bit int caps / dll load or symbol resolution failed at environment
# level).
#
# The harness creates a REAL D3D12 device/queue and real src(8x8 R32F, filled)
# and dst(1x1 R32F UAV) resources, then dynamically loads rurix_godot.dll and
# drives the bridge C ABI: rxgd_create_d3d12_session -> rxgd_register_texture ->
# rxgd_record_pass(RXGD_PASS_LUMINANCE_REDUCTION) -> rxgd_collect_timestamps.
# The bridge's linked recording shim performs the real dispatch.
# ---------------------------------------------------------------------------
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
#include <string>

#include "rurix_godot.h"

using Microsoft::WRL::ComPtr;

typedef uint32_t (*PFN_rxgd_abi_version)(void);
typedef int32_t (*PFN_rxgd_shim_available)(void);
typedef int32_t (*PFN_rxgd_create_session)(void*, void*, RxGdCaps, RxGdSession**);
typedef int32_t (*PFN_rxgd_register_texture)(RxGdSession*, RxGdResource);
typedef int32_t (*PFN_rxgd_record_pass)(RxGdSession*, uint32_t, const RxGdResource*, uint64_t, const uint8_t*, uint64_t);
typedef int32_t (*PFN_rxgd_collect)(RxGdSession*, uint64_t, RxGdFrameStats*);
typedef void (*PFN_rxgd_destroy)(RxGdSession*);

static int fail_hr(const char* what, HRESULT hr) {
    std::fprintf(stderr, "RXGD_BRIDGE_HARNESS: fail %s hr=0x%08lx\n", what, (unsigned long)hr);
    return 1;
}
static int fail_msg(const char* what) {
    std::fprintf(stderr, "RXGD_BRIDGE_HARNESS: fail %s\n", what);
    return 1;
}
static int skip_msg(const char* what) {
    std::fprintf(stderr, "RXGD_BRIDGE_HARNESS: skip %s\n", what);
    return 2;
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
"""

HARNESS_CPP += r"""
int wmain(int argc, wchar_t** argv) {
    if (argc != 2) return fail_msg("usage: harness <rurix_godot.dll>");
    const wchar_t* dll_path = argv[1];

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
    std::printf("RXGD_BRIDGE_HARNESS: adapter=\"%s\"\n", narrow(chosen_desc.Description).c_str());

    D3D12_FEATURE_DATA_D3D12_OPTIONS1 opt1 = {};
    device->CheckFeatureSupport(D3D12_FEATURE_D3D12_OPTIONS1, &opt1, sizeof(opt1));
    std::printf("RXGD_BRIDGE_HARNESS: int64_shader_ops=%d\n", opt1.Int64ShaderOps ? 1 : 0);
    if (!opt1.Int64ShaderOps)
        return skip_msg("device lacks Int64ShaderOps (pass requires 64-bit integer shader capability)");

    D3D12_COMMAND_QUEUE_DESC qd = {};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue;
    if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue))))
        return fail_msg("CreateCommandQueue");
    ComPtr<ID3D12CommandAllocator> alloc;
    if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc))))
        return fail_msg("CreateCommandAllocator");

    // Real src (8x8 R32_FLOAT, filled 1.0) + dst (1x1 R32_FLOAT UAV).
    const UINT src_w = 8, src_h = 8;
    const UINT dst_w = std::max<UINT>((src_w + 7) / 8, 1u);
    const UINT dst_h = std::max<UINT>((src_h + 7) / 8, 1u);
    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    auto upload_heap = heap_props(D3D12_HEAP_TYPE_UPLOAD);

    auto src_desc = tex2d_desc(src_w, src_h, DXGI_FORMAT_R32_FLOAT, D3D12_RESOURCE_FLAG_NONE);
    ComPtr<ID3D12Resource> src;
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &src_desc,
                                               D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                               IID_PPV_ARGS(&src))))
        return fail_msg("CreateCommittedResource(src_luminance)");
    auto dst_desc = tex2d_desc(dst_w, dst_h, DXGI_FORMAT_R32_FLOAT,
                               D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS);
    ComPtr<ID3D12Resource> dst;
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &dst_desc,
                                               D3D12_RESOURCE_STATE_UNORDERED_ACCESS, nullptr,
                                               IID_PPV_ARGS(&dst))))
        return fail_msg("CreateCommittedResource(dst_luminance)");

    // Upload src texels (all 1.0) and leave src in NON_PIXEL_SHADER_RESOURCE so
    // the bridge shim can bind it as the SRV without another transition.
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
    for (UINT y = 0; y < src_h; ++y) {
        float* rowp = reinterpret_cast<float*>(sup + sfp.Offset + (SIZE_T)y * sfp.Footprint.RowPitch);
        for (UINT x = 0; x < src_w; ++x) rowp[x] = 1.0f;
    }
    src_upload->Unmap(0, nullptr);

    ComPtr<ID3D12GraphicsCommandList> up_cmd;
    if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(),
                                        nullptr, IID_PPV_ARGS(&up_cmd))))
        return fail_msg("CreateCommandList(upload)");
    D3D12_TEXTURE_COPY_LOCATION tdst = {};
    tdst.pResource = src.Get();
    tdst.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    tdst.SubresourceIndex = 0;
    D3D12_TEXTURE_COPY_LOCATION tsrc = {};
    tsrc.pResource = src_upload.Get();
    tsrc.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    tsrc.PlacedFootprint = sfp;
    up_cmd->CopyTextureRegion(&tdst, 0, 0, 0, &tsrc, nullptr);
    D3D12_RESOURCE_BARRIER tb = {};
    tb.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    tb.Transition.pResource = src.Get();
    tb.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST;
    tb.Transition.StateAfter = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
    tb.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    up_cmd->ResourceBarrier(1, &tb);
    if (FAILED(up_cmd->Close())) return fail_msg("Close upload command list");
    ID3D12CommandList* up_lists[] = {up_cmd.Get()};
    queue->ExecuteCommandLists(1, up_lists);
    ComPtr<ID3D12Fence> up_fence;
    if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&up_fence))))
        return fail_msg("CreateFence(upload)");
    HANDLE up_ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!up_ev) return fail_msg("CreateEvent(upload)");
    if (FAILED(queue->Signal(up_fence.Get(), 1))) return fail_msg("Signal(upload)");
    if (up_fence->GetCompletedValue() < 1) {
        up_fence->SetEventOnCompletion(1, up_ev);
        WaitForSingleObject(up_ev, INFINITE);
    }
    CloseHandle(up_ev);
"""

HARNESS_CPP += r"""
    // Dynamically load the bridge DLL (built with the d3d12-recording-shim
    // feature) and resolve the C ABI. A missing DLL / symbol is an environment
    // failure (fail, not skip): the smoke was asked to exercise the bridge.
    HMODULE lib = LoadLibraryW(dll_path);
    if (!lib) return fail_msg("LoadLibrary(rurix_godot.dll)");
    auto p_abi = (PFN_rxgd_abi_version)GetProcAddress(lib, "rxgd_abi_version");
    auto p_shim = (PFN_rxgd_shim_available)GetProcAddress(lib, "rxgd_dispatch_recording_shim_available");
    auto p_create = (PFN_rxgd_create_session)GetProcAddress(lib, "rxgd_create_d3d12_session");
    auto p_reg = (PFN_rxgd_register_texture)GetProcAddress(lib, "rxgd_register_texture");
    auto p_record = (PFN_rxgd_record_pass)GetProcAddress(lib, "rxgd_record_pass");
    auto p_collect = (PFN_rxgd_collect)GetProcAddress(lib, "rxgd_collect_timestamps");
    auto p_destroy = (PFN_rxgd_destroy)GetProcAddress(lib, "rxgd_destroy_session");
    if (!p_abi || !p_shim || !p_create || !p_reg || !p_record || !p_collect || !p_destroy)
        return fail_msg("GetProcAddress(rxgd_* symbol missing)");

    std::printf("RXGD_BRIDGE_HARNESS: abi_version=%u\n", p_abi());
    const int shim_available = p_shim();
    std::printf("RXGD_BRIDGE_HARNESS: shim_available=%d\n", shim_available);
    if (shim_available != 1)
        return fail_msg("bridge built without d3d12-recording-shim feature (shim_available!=1)");

    RxGdCaps caps = {};
    caps.abi_version = RXGD_ABI_VERSION;
    caps.struct_size = (uint32_t)sizeof(RxGdCaps);
    caps.backend = RXGD_BACKEND_D3D12;
    caps.render_method = RXGD_RENDER_METHOD_FORWARD_PLUS;
    caps.flags = RXGD_CAP_SHADER_INT64 | RXGD_CAP_LUMINANCE_DISPATCH_BRINGUP |
                 RXGD_CAP_LUMINANCE_DISPATCH_RECORD;
    caps.vendor_id = chosen_desc.VendorId;
    caps.device_id = chosen_desc.DeviceId;

    RxGdSession* session = nullptr;
    int32_t rc = p_create((void*)device.Get(), (void*)queue.Get(), caps, &session);
    if (rc != RXGD_STATUS_OK || !session)
        return fail_msg("rxgd_create_d3d12_session != OK");

    RxGdResource res_src = {};
    res_src.abi_version = RXGD_ABI_VERSION;
    res_src.struct_size = (uint32_t)sizeof(RxGdResource);
    res_src.resource_type = RXGD_RESOURCE_TEXTURE;
    res_src.format = DXGI_FORMAT_R32_FLOAT;
    res_src.width = src_w;
    res_src.height = src_h;
    res_src.depth = 1;
    res_src.mip_levels = 1;
    res_src.native_handle = (uint64_t)(uintptr_t)src.Get();

    RxGdResource res_dst = res_src;
    res_dst.width = dst_w;
    res_dst.height = dst_h;
    res_dst.native_handle = (uint64_t)(uintptr_t)dst.Get();

    p_reg(session, res_src);
    p_reg(session, res_dst);

    RxGdResource resources[2] = {res_src, res_dst};
    uint8_t pc[28];
    uint64_t sw = src_w, sh = src_h;
    std::memcpy(&pc[0], &sw, 8);
    std::memcpy(&pc[8], &sh, 8);
    float maxl = 1.0f, minl = 0.0f, expo = 0.0f;
    std::memcpy(&pc[16], &maxl, 4);
    std::memcpy(&pc[20], &minl, 4);
    std::memcpy(&pc[24], &expo, 4);

    int32_t record_rc = p_record(session, RXGD_PASS_LUMINANCE_REDUCTION, resources, 2, pc, 28);
    std::printf("RXGD_BRIDGE_HARNESS: record_pass_status=%d\n", record_rc);

    RxGdFrameStats stats = {};
    int32_t collect_rc = p_collect(session, 1, &stats);
    std::printf("RXGD_BRIDGE_HARNESS: collect_status=%d\n", collect_rc);
    std::printf("RXGD_BRIDGE_HARNESS: stats recorded=%llu fallback=%llu registered=%llu "
                "gpu_ns=%llu cpu_ns=%llu last_error=%d\n",
                (unsigned long long)stats.recorded_passes,
                (unsigned long long)stats.fallback_passes,
                (unsigned long long)stats.registered_resources,
                (unsigned long long)stats.gpu_time_ns,
                (unsigned long long)stats.cpu_record_ns,
                (int)stats.last_error);

    p_destroy(session);

    if (record_rc != RXGD_STATUS_OK)
        return fail_msg("rxgd_record_pass did not return RXGD_STATUS_OK (no bridge dispatch recorded)");
    if (stats.recorded_passes != 1 || stats.fallback_passes != 0 || stats.last_error != RXGD_STATUS_OK)
        return fail_msg("bridge frame stats inconsistent with one recorded dispatch");
    if (stats.gpu_time_ns != 0)
        return fail_msg("gpu_time_ns must stay 0 (GPU timestamp not implemented, gpu_timestamp_status=not_yet)");

    std::printf("RXGD_BRIDGE_HARNESS: ok adapter=\"%s\"\n", narrow(chosen_desc.Description).c_str());
    return 0;
}
"""


def build_bridge_dll(env: dict) -> tuple[bool, str]:
    """Build rurix_godot.dll with the d3d12-recording-shim feature. RURIX_DXC_DIR
    (in env) lets build.rs find dxcapi.h so the shim can sign the in-memory DXIL."""
    p = subprocess.run(
        ["cargo", "build", "-p", "rurix-godot", "--features", "d3d12-recording-shim"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
    )
    log = (p.stdout + p.stderr).strip()
    ok = p.returncode == 0 and RURIX_GODOT_DLL.is_file()
    return ok, log[-3000:]


def compile_harness(vcvars: Path, cpp: Path, exe: Path) -> tuple[bool, str]:
    obj = WORK / "bridge_harness.obj"
    bat = WORK / "build_bridge_recording_smoke.bat"
    bat.write_text(
        "@echo off\n"
        f'call "{vcvars}" >nul\n'
        "if errorlevel 1 exit /b %errorlevel%\n"
        f'cl /nologo /std:c++17 /EHsc /W4 /DUNICODE /D_UNICODE /I "{RURIX_GODOT_HEADER_DIR}" '
        f'"{cpp}" /Fe:"{exe}" /Fo:"{obj}" /link d3d12.lib dxgi.lib\n',
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
        if line.startswith("RXGD_BRIDGE_HARNESS: adapter="):
            a0 = line.find('adapter="') + len('adapter="')
            a1 = line.find('"', a0)
            if a1 > a0:
                parsed["adapter"] = line[a0:a1]
        elif line.startswith("RXGD_BRIDGE_HARNESS: int64_shader_ops="):
            parsed["int64_shader_ops"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_BRIDGE_HARNESS: abi_version="):
            parsed["abi_version"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_BRIDGE_HARNESS: shim_available="):
            parsed["shim_available"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_BRIDGE_HARNESS: record_pass_status="):
            parsed["record_pass_status"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_BRIDGE_HARNESS: collect_status="):
            parsed["collect_status"] = line.split("=", 1)[1].strip()
        elif line.startswith("RXGD_BRIDGE_HARNESS: stats "):
            body = line[len("RXGD_BRIDGE_HARNESS: stats "):]
            for token in body.split(" "):
                if "=" in token:
                    k, v = token.split("=", 1)
                    parsed[f"stats_{k}"] = v
        elif line.startswith("RXGD_BRIDGE_REC: "):
            body = line[len("RXGD_BRIDGE_REC: "):]
            for token in body.split(" "):
                if "=" in token:
                    k, v = token.split("=", 1)
                    parsed[k] = v
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
        return fail("cannot read luminance_reduction_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "luminance_reduction",
        "segment": "4d",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "godot_runtime_luminance_path_enabled": False,
        "default_enable_state": "disabled",
        "gpu_timestamp_status": "not_yet",
        "gpu_time_ns": None,
        "bridge": {
            "dll": str(RURIX_GODOT_DLL.relative_to(ROOT)).replace("\\", "/"),
            "feature": "d3d12-recording-shim",
        },
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
            "GRX-009 segment 4d bridge real D3D12 dispatch recording smoke evidence "
            "only. Even a success here keeps runtime_state=fallback_only, "
            "real_gpu_pass=false, godot_runtime_luminance_path_enabled=false, and "
            "default_enable_state=disabled: the recording path is compiled only under "
            "the test-only d3d12-recording-shim feature and armed only by this harness. "
            "It is not a Godot runtime pass, and makes no visual, perf, GPU-timestamp, "
            "or measured-fallback-telemetry claim."
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
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64);无法编译真实 D3D12 bridge harness")

    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip(
            "未找到含 dxil.dll 的签名 DXC pin(set RURIX_DXC_DIR=H:\\dxc-round7\\extracted\\bin\\x64);"
            "bridge 录制 shim 无法为编译器产出的 DXIL container 签名以在非 Developer-Mode device 上加载"
        )
    include_dir = locate_dxcapi_include(dxc_dir)
    if include_dir is None:
        return skip(f"未在 {dxc_dir} 附近找到 dxcapi.h(bridge 录制 shim 签名路径无法编译)")

    # Build the bridge DLL with the recording shim. RURIX_DXC_DIR must be visible
    # to build.rs so it can add the dxcapi.h include dir (in-memory DXIL signing).
    build_env = dict(os.environ)
    build_env.setdefault("RURIX_DXC_DIR", str(dxc_dir))
    built_dll, dll_log = build_bridge_dll(build_env)
    if not built_dll:
        print(dll_log, file=sys.stderr)
        return fail("cargo build -p rurix-godot --features d3d12-recording-shim failed",
                    extra={"build_log_tail": dll_log})

    # Pin the exact feature-built DLL (hash/size/mtime) plus an immutable
    # snapshot so a later feature-off build overwriting target/debug cannot make
    # this historical evidence ambiguous. Recorded on every post-build outcome.
    fingerprint = dll_fingerprint(RURIX_GODOT_DLL)
    fingerprint.update(snapshot_feature_dll(RURIX_GODOT_DLL))
    _EVIDENCE_BASE["dll_fingerprint"] = fingerprint

    WORK.mkdir(parents=True, exist_ok=True)
    cpp = WORK / "bridge_recording_harness.cpp"
    exe = WORK / "bridge_recording_harness.exe"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")

    built, build_log = compile_harness(vcvars, cpp, exe)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 bridge harness 失败(可能缺 Windows SDK D3D12 头/库)",
                    extra={"build_log_tail": build_log})

    # The bridge shim signs the in-memory DXIL via dxil.dll from RURIX_DXC_DIR.
    run_env = dict(os.environ)
    run_env.setdefault("RURIX_DXC_DIR", str(dxc_dir))
    p = subprocess.run([str(exe), str(RURIX_GODOT_DLL)], cwd=WORK,
                       capture_output=True, text=True, env=run_env)
    output = (p.stdout + p.stderr).strip()
    print(output)
    parsed = parse_harness_output(output)
    device_info = {
        "adapter": parsed.get("adapter"),
        "int64_shader_ops": parsed.get("int64_shader_ops"),
        "dxil_signed_for_load": parsed.get("dxil_signed"),
        "dxil_validator": str(dxc_dir / "dxil.dll").replace("\\", "/"),
    }
    bridge_info = {
        "abi_version": parsed.get("abi_version"),
        "shim_available": parsed.get("shim_available"),
        "record_pass_status": parsed.get("record_pass_status"),
        "collect_status": parsed.get("collect_status"),
    }
    bridge_stats = {
        "recorded_passes": parsed.get("stats_recorded"),
        "fallback_passes": parsed.get("stats_fallback"),
        "registered_resources": parsed.get("stats_registered"),
        "gpu_time_ns": parsed.get("stats_gpu_ns"),
        "cpu_record_ns": parsed.get("stats_cpu_ns"),
        "last_error": parsed.get("stats_last_error"),
    }

    if p.returncode == 2:
        return skip(
            "no real D3D12 device harness available (see harness output)",
            extra={"device": device_info, "stdout": output},
        )
    if p.returncode != 0 or "RXGD_BRIDGE_HARNESS: ok" not in output:
        return fail(
            "real D3D12 bridge luminance dispatch recording smoke failed",
            extra={
                "device": device_info,
                "bridge": {**_EVIDENCE_BASE["bridge"], **bridge_info},
                "bridge_stats": bridge_stats,
                "exit_code": p.returncode,
                "stdout": output,
            },
        )

    dispatch = {
        "dimensions": parsed.get("dispatch"),
        "fence_completion": parsed.get("fence"),
        "dst_shape": parsed.get("dst"),
        "dst_first_value": parsed.get("dst_first"),
        "readback_checksum": parsed.get("checksum"),
    }
    write_evidence(
        "success",
        extra={
            "device": device_info,
            "bridge": {**_EVIDENCE_BASE["bridge"], **bridge_info},
            "dispatch": dispatch,
            "bridge_stats": bridge_stats,
            "checks": {
                "artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "recording_shim_linked": parsed.get("shim_available") == "1",
                "real_d3d12_device_queue_resource_handles": True,
                "dispatch_bringup_optin_and_record_arm": True,
                "int64_capability": parsed.get("int64_shader_ops") == "1",
                "rxgd_record_pass_returned_ok": parsed.get("record_pass_status") == "0",
                "bridge_recorded_one_pass": parsed.get("stats_recorded") == "1",
                "no_fallback_passes": parsed.get("stats_fallback") == "0",
                "gpu_time_ns_zero": parsed.get("stats_gpu_ns") == "0",
                "fence_completed": bool(parsed.get("fence")),
                "dst_uav_readback": bool(parsed.get("checksum")),
            },
            "stdout": output,
        },
    )
    print(f"[grx009-bridge-recording-smoke] PASS measured bridge D3D12 dispatch recording; "
          f"adapter={device_info['adapter']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
