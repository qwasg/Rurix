#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""GRX-016: standalone real Windows D3D12 dispatch smoke for instance_compaction.

instance_compaction is a THREE-kernel scan/compaction chain: scan_local (local
prefix + per-group totals) -> UAV barrier -> scan_groups (group offsets +
survivor count) -> UAV barrier -> scatter (stable compaction of the surviving
transforms). This harness records all three dispatches with the correct
inter-dispatch UAV/state barriers in ONE command list and ONE submit on a real
D3D12 device, then verifies every measured GPU output word against the tracked
``generate_math_parity_evidence.py`` ``reference_chain`` **exactly** (the whole
chain is u32 adds + bit-preserving uint4 moves, so the tolerance is ZERO). It
compares the final compacted ``dst_transforms`` buffer AND the intermediates
(``local_prefix``, ``group_totals``, ``group_offsets``, ``survivor_count``).

It produces measured smoke evidence only: it does NOT mark the Godot runtime
pass complete, make the bridge default to RXGD_STATUS_OK, or claim any FPS /
visual / measured telemetry. Real device/queue only (SKIP otherwise; SKIP never
advances the ready gate). Seven tracked digests (three DXIL kernels, two root
signatures, and the descriptor layout) are verified against the offline compile
evidence. Capacity is fail-closed at N <= 65536 (num_groups <= 256). If
RURIX_REQUIRE_REAL=1 a SKIP becomes a hard failure.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import importlib.util
import json
import os
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PASS_DIR = ROOT / "spike" / "godot-rurix" / "passes" / "instance_compaction"
ARTIFACTS = PASS_DIR / "artifacts"
SCAN_LOCAL_DXIL = ARTIFACTS / "instance_compaction_scan_local.dxil"
SCAN_LOCAL_RTS0 = ARTIFACTS / "instance_compaction_scan_local.rts0.bin"
SCAN_GROUPS_DXIL = ARTIFACTS / "instance_compaction_scan_groups.dxil"
SCAN_GROUPS_RTS0 = ARTIFACTS / "instance_compaction_scan_groups.rts0.bin"
SCATTER_DXIL = ARTIFACTS / "instance_compaction_scatter.dxil"
SCATTER_RTS0 = ARTIFACTS / "instance_compaction_scatter.rts0.bin"
DESCRIPTOR_LAYOUT = ARTIFACTS / "instance_compaction_descriptor_layout.json"
OFFLINE_EVIDENCE = PASS_DIR / "offline_compile_evidence.json"
MATH_PARITY_SCRIPT = PASS_DIR / "generate_math_parity_evidence.py"
EVIDENCE_OUT = PASS_DIR / "real_d3d12_dispatch_smoke.json"
WORK = ROOT / "target" / "grx016_d3d12_dispatch_smoke"

SUBJECT = "grx016_instance_compaction_real_d3d12_dispatch_smoke"

GROUP_SIZE = 256
MAX_GROUPS = 256
MAX_INSTANCES = 65536
TRANSFORM_STRIDE_FLOATS = 12
TRANSFORM_STRIDE_BYTES = TRANSFORM_STRIDE_FLOATS * 4  # 48 bytes / instance
VALUE_TOLERANCE = 0

# The five tracked fixtures (mirrors the inline case list in the generator's
# main(); the sparse-survival rule is the deterministic LCG the generator uses).
def _lcg_survive(p: int) -> bool:
    return ((p * 1103515245 + 12345) >> 16) % 4 == 0


CASES = [
    ("sparse_survival_multi_group", 600, _lcg_survive, False),
    ("all_survive", 513, lambda p: True, False),
    ("zero_survive", 384, lambda p: False, False),
    ("mask_tail_garbage_bits_ignored", 70, lambda p: p % 5 == 0, True),
    ("single_survivor_last_instance_empty_leading_group", 300, lambda p: p == 299, False),
]


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


def load_math_parity_reference():
    spec = importlib.util.spec_from_file_location(
        "grx016_instance_compaction_math_parity", MATH_PARITY_SCRIPT
    )
    if spec is None or spec.loader is None:
        return None
    module = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(module)
    except Exception:  # noqa: BLE001
        return None
    return module


def offline_artifact_digests(evidence: dict) -> dict[str, str | None]:
    artifacts = evidence.get("artifacts")
    keys = (
        "dxil_scan_local",
        "root_signature_scan_local",
        "dxil_scan_groups",
        "root_signature_scan_groups",
        "dxil_scatter",
        "root_signature_scatter",
        "descriptor_layout",
    )
    out: dict[str, str | None] = {k: None for k in keys}
    if isinstance(artifacts, dict):
        for key in keys:
            entry = artifacts.get(key)
            if isinstance(entry, dict):
                sha = entry.get("sha256")
                if isinstance(sha, str):
                    out[key] = sha
    return out


def descriptor_layout_matches_resource_mapping(layout: dict) -> str | None:
    """Return None when the descriptor layout matches the tracked GRX-016
    instance_compaction resource mapping, otherwise a mismatch reason. The layout
    declares per-kernel bindings under ``variants`` (3 kernels), not a top-level
    ``resources`` array."""
    if layout.get("root_constants") != 8:
        return "root_constants != 8"
    variants = layout.get("variants")
    if not isinstance(variants, list) or len(variants) != 3:
        return "descriptor layout does not declare exactly 3 variants"
    expected_counts = {"scan_local": 3, "scan_groups": 3, "scatter": 5}
    seen = {}
    for v in variants:
        if not isinstance(v, dict):
            return "variant is not an object"
        name = v.get("variant")
        resources = v.get("resources")
        if not isinstance(resources, list):
            return f"variant {name} has no resources list"
        seen[name] = len(resources)
        if v.get("root_signature_parameters") != 2:
            return f"variant {name} root_signature_parameters != 2"
    if seen != expected_counts:
        return f"variant resource counts {seen} != {expected_counts}"
    mapping = layout.get("grx016_mapping")
    if not isinstance(mapping, dict):
        return "missing grx016_mapping"
    if mapping.get("root_constant_bytes") != 32 or mapping.get("root_constant_dwords") != 8:
        return "root constant block is not 32 bytes / 8 dwords"
    if mapping.get("requires_64bit_integer_shader_capability") is not False:
        return "grx016_mapping must record requires_64bit_integer_shader_capability=false"
    if mapping.get("transform_stride_floats") != TRANSFORM_STRIDE_FLOATS:
        return "grx016_mapping transform_stride_floats is not 12"
    if mapping.get("transform_stride_vec4") != 3:
        return "grx016_mapping transform_stride_vec4 is not 3"
    return None


def fail(msg: str, extra: dict | None = None) -> int:
    print(f"[grx016-d3d12-dispatch-smoke] FAIL {msg}", file=sys.stderr)
    write_evidence("fail", reason=msg, extra=extra or {})
    return 1


def skip(msg: str, extra: dict | None = None) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(f"(RURIX_REQUIRE_REAL) {msg}", extra=extra)
    print(f"[grx016-d3d12-dispatch-smoke] SKIP {msg}(降级 SKIP,退出 0)")
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
    print(f"[grx016-d3d12-dispatch-smoke] wrote {EVIDENCE_OUT.relative_to(ROOT)} status={status}")


# ---------------------------------------------------------------------------
# Params-file builder. The harness zero-uploads the intermediate + output UAVs;
# only the two inputs (visibility_mask, src_transforms) are uploaded verbatim.
#
# Params binary (little-endian):
#   uint32 total_instances
#   uint32 num_groups
#   uint32 mask_bytes
#   uint32 src_bytes
#   uint8  b0[32]
#   uint8  mask[mask_bytes]
#   uint8  src[src_bytes]
# ---------------------------------------------------------------------------
def build_case_payload(parity, case) -> dict:
    case_id, n, survive, garbage_tail = case
    if n < 1 or n > MAX_INSTANCES:
        raise ValueError(f"case {case_id} n={n} out of capacity")
    mask_words = parity.pack_mask(n, survive, garbage_tail)
    ref = parity.reference_chain(n, mask_words)
    num_groups = (n + GROUP_SIZE - 1) // GROUP_SIZE
    if num_groups != ref["num_groups"] or num_groups > MAX_GROUPS:
        raise ValueError(f"case {case_id} num_groups mismatch")
    mask_bytes = struct.pack(f"<{len(mask_words)}I", *mask_words)
    src_bytes = struct.pack(f"<{n * TRANSFORM_STRIDE_FLOATS}f", *ref["src"])
    b0 = struct.pack("<8I", n, len(mask_words), num_groups, 3, 0, 0, 0, 0)
    params = struct.pack("<IIII", n, num_groups, len(mask_bytes), len(src_bytes))
    params += b0 + mask_bytes + src_bytes
    expected = {
        "dst": struct.pack(f"<{n * TRANSFORM_STRIDE_FLOATS}f", *ref["dst"]),
        "local_prefix": struct.pack(f"<{n}I", *ref["local_prefix"]),
        "group_totals": struct.pack(f"<{num_groups}I", *ref["group_totals"]),
        "group_offsets": struct.pack(f"<{num_groups}I", *ref["group_offsets"]),
        "survivor_count": struct.pack("<I", ref["survivor_count"]),
    }
    return {
        "case_id": case_id,
        "n": n,
        "num_groups": num_groups,
        "survivor_count": ref["survivor_count"],
        "params": params,
        "expected": expected,
        "sizes": {
            "dst": len(expected["dst"]),
            "local_prefix": len(expected["local_prefix"]),
            "group_totals": len(expected["group_totals"]),
            "group_offsets": len(expected["group_offsets"]),
            "survivor_count": 4,
        },
    }


def compare_outputs(payload: dict, out_bin: Path) -> dict:
    raw = out_bin.read_bytes()
    order = ["dst", "local_prefix", "group_totals", "group_offsets", "survivor_count"]
    sizes = payload["sizes"]
    total = sum(sizes[k] for k in order)
    if len(raw) != total:
        return {"match": False, "reason": f"output size {len(raw)} != {total}"}
    offset = 0
    mismatches: dict[str, int] = {}
    worst = None
    for k in order:
        seg = raw[offset:offset + sizes[k]]
        offset += sizes[k]
        exp = payload["expected"][k]
        if seg != exp:
            # count differing 4-byte words
            n_words = sizes[k] // 4
            diff = sum(
                1
                for i in range(n_words)
                if seg[i * 4:i * 4 + 4] != exp[i * 4:i * 4 + 4]
            )
            mismatches[k] = diff
            if worst is None:
                for i in range(n_words):
                    o = seg[i * 4:i * 4 + 4]
                    e = exp[i * 4:i * 4 + 4]
                    if o != e:
                        worst = {"buffer": k, "word_index": i,
                                 "observed_hex": o.hex(), "reference_hex": e.hex()}
                        break
    return {
        "match": len(mismatches) == 0,
        "mismatched_buffers": mismatches,
        "value_tolerance": VALUE_TOLERANCE,
        "worst": worst,
    }


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

#include <dxcapi.h>

using Microsoft::WRL::ComPtr;

static int fail_hr(const char* what, HRESULT hr) {
    std::fprintf(stderr, "RXGD_DISPATCH: fail %s hr=0x%08lx\n", what, (unsigned long)hr);
    return 1;
}
static int fail_msg(const char* what) { std::fprintf(stderr, "RXGD_DISPATCH: fail %s\n", what); return 1; }
static int skip_msg(const char* what) { std::fprintf(stderr, "RXGD_DISPATCH: skip %s\n", what); return 2; }

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
    D3D12_HEAP_PROPERTIES hp = {}; hp.Type = type; hp.CreationNodeMask = 1; hp.VisibleNodeMask = 1; return hp;
}
static D3D12_RESOURCE_DESC buffer_desc(UINT64 bytes, D3D12_RESOURCE_FLAGS flags) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; d.Width = bytes; d.Height = 1; d.DepthOrArraySize = 1;
    d.MipLevels = 1; d.Format = DXGI_FORMAT_UNKNOWN; d.SampleDesc.Count = 1;
    d.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; d.Flags = flags; return d;
}
static std::string narrow(const wchar_t* s) {
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    std::string out((size_t)std::max(n - 1, 0), '\0');
    if (n > 1) WideCharToMultiByte(CP_UTF8, 0, s, -1, out.data(), n, nullptr, nullptr);
    return out;
}

struct MemBlob : public IDxcBlob {
    LONG m_ref; void* m_ptr; SIZE_T m_size;
    MemBlob(void* p, SIZE_T s) : m_ref(1), m_ptr(p), m_size(s) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** ppv) override {
        if (!ppv) return E_POINTER;
        if (riid == __uuidof(IUnknown) || riid == __uuidof(IDxcBlob)) { *ppv = static_cast<IDxcBlob*>(this); AddRef(); return S_OK; }
        *ppv = nullptr; return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return (ULONG)InterlockedIncrement(&m_ref); }
    ULONG STDMETHODCALLTYPE Release() override { return (ULONG)InterlockedDecrement(&m_ref); }
    LPVOID STDMETHODCALLTYPE GetBufferPointer() override { return m_ptr; }
    SIZE_T STDMETHODCALLTYPE GetBufferSize() override { return m_size; }
};

static bool sign_dxil_in_place(std::vector<uint8_t>& dxil, const wchar_t* dxil_dll, std::string* err) {
    HMODULE lib = dxil_dll ? LoadLibraryW(dxil_dll) : LoadLibraryW(L"dxil.dll");
    if (!lib) { *err = "LoadLibrary(dxil.dll) failed"; return false; }
    auto create = reinterpret_cast<DxcCreateInstanceProc>(GetProcAddress(lib, "DxcCreateInstance"));
    if (!create) { *err = "GetProcAddress(DxcCreateInstance) failed"; return false; }
    IDxcValidator* validator = nullptr;
    if (FAILED(create(CLSID_DxcValidator, __uuidof(IDxcValidator), reinterpret_cast<void**>(&validator))) || !validator) {
        *err = "DxcCreateInstance(CLSID_DxcValidator) failed"; return false;
    }
    MemBlob blob(dxil.data(), dxil.size());
    IDxcOperationResult* result = nullptr;
    HRESULT hr = validator->Validate(&blob, DxcValidatorFlags_InPlaceEdit, &result);
    bool ok = false;
    if (SUCCEEDED(hr) && result) { HRESULT status = E_FAIL; result->GetStatus(&status); ok = SUCCEEDED(status); if (!ok) *err = "validator rejected the DXIL container"; }
    else { *err = "IDxcValidator::Validate failed"; }
    if (result) result->Release();
    validator->Release();
    return ok;
}

static bool make_buffer(ID3D12Device* device, ID3D12GraphicsCommandList* cmd, UINT64 bytes,
                        D3D12_RESOURCE_FLAGS flags, D3D12_RESOURCE_STATES after, const uint8_t* data,
                        ComPtr<ID3D12Resource>& buf, ComPtr<ID3D12Resource>& upload, const char* label) {
    auto default_heap = heap_props(D3D12_HEAP_TYPE_DEFAULT);
    auto desc = buffer_desc(bytes, flags);
    D3D12_RESOURCE_STATES initial = data ? D3D12_RESOURCE_STATE_COPY_DEST : after;
    if (FAILED(device->CreateCommittedResource(&default_heap, D3D12_HEAP_FLAG_NONE, &desc, initial, nullptr, IID_PPV_ARGS(&buf)))) {
        std::fprintf(stderr, "RXGD_DISPATCH: fail CreateCommittedResource(%s)\n", label); return false;
    }
    if (data) {
        auto upload_heap = heap_props(D3D12_HEAP_TYPE_UPLOAD);
        auto up_desc = buffer_desc(bytes, D3D12_RESOURCE_FLAG_NONE);
        if (FAILED(device->CreateCommittedResource(&upload_heap, D3D12_HEAP_FLAG_NONE, &up_desc, D3D12_RESOURCE_STATE_GENERIC_READ, nullptr, IID_PPV_ARGS(&upload)))) {
            std::fprintf(stderr, "RXGD_DISPATCH: fail CreateCommittedResource(%s upload)\n", label); return false;
        }
        uint8_t* p = nullptr; D3D12_RANGE empty = {0, 0};
        if (FAILED(upload->Map(0, &empty, reinterpret_cast<void**>(&p)))) { std::fprintf(stderr, "RXGD_DISPATCH: fail Map(%s)\n", label); return false; }
        std::memcpy(p, data, (size_t)bytes); upload->Unmap(0, nullptr);
        cmd->CopyBufferRegion(buf.Get(), 0, upload.Get(), 0, bytes);
        D3D12_RESOURCE_BARRIER b = {};
        b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION; b.Transition.pResource = buf.Get();
        b.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST; b.Transition.StateAfter = after;
        b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
        cmd->ResourceBarrier(1, &b);
    }
    return true;
}

static void srv(ID3D12Device* d, ID3D12Resource* r, UINT bytes, UINT stride, D3D12_CPU_DESCRIPTOR_HANDLE h) {
    D3D12_SHADER_RESOURCE_VIEW_DESC s = {};
    s.Format = DXGI_FORMAT_UNKNOWN; s.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
    s.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    s.Buffer.FirstElement = 0; s.Buffer.NumElements = std::max<UINT>(bytes / stride, 1u);
    s.Buffer.StructureByteStride = stride; s.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_NONE;
    d->CreateShaderResourceView(r, &s, h);
}
static void uav(ID3D12Device* d, ID3D12Resource* r, UINT bytes, UINT stride, D3D12_CPU_DESCRIPTOR_HANDLE h) {
    D3D12_UNORDERED_ACCESS_VIEW_DESC u = {};
    u.Format = DXGI_FORMAT_UNKNOWN; u.ViewDimension = D3D12_UAV_DIMENSION_BUFFER;
    u.Buffer.FirstElement = 0; u.Buffer.NumElements = std::max<UINT>(bytes / stride, 1u);
    u.Buffer.StructureByteStride = stride; u.Buffer.CounterOffsetInBytes = 0; u.Buffer.Flags = D3D12_BUFFER_UAV_FLAG_NONE;
    d->CreateUnorderedAccessView(r, nullptr, &u, h);
}

// argv: scan_local_dxil scan_groups_dxil scatter_dxil scan_rts0 scatter_rts0 params out [dxil.dll]
int wmain(int argc, wchar_t** argv) {
    if (argc < 8 || argc > 9) return fail_msg("usage: harness sl sg sc scan_rts0 scatter_rts0 params out [dxil.dll]");
    bool ok = false;
    auto sl_dxil = read_file(argv[1], &ok); if (!ok) return fail_msg("read scan_local dxil");
    auto sg_dxil = read_file(argv[2], &ok); if (!ok) return fail_msg("read scan_groups dxil");
    auto sc_dxil = read_file(argv[3], &ok); if (!ok) return fail_msg("read scatter dxil");
    auto scan_rts0 = read_file(argv[4], &ok); if (!ok) return fail_msg("read scan rts0");
    auto scatter_rts0 = read_file(argv[5], &ok); if (!ok) return fail_msg("read scatter rts0");
    auto params = read_file(argv[6], &ok); if (!ok) return fail_msg("read params");
    const wchar_t* out_bin = argv[7];
    const wchar_t* dxil_dll = (argc >= 9) ? argv[8] : nullptr;
    if (params.size() < 16 + 32) return fail_msg("params too small");

    UINT total_instances = 0, num_groups = 0, mask_bytes = 0, src_bytes = 0;
    std::memcpy(&total_instances, params.data() + 0, 4);
    std::memcpy(&num_groups, params.data() + 4, 4);
    std::memcpy(&mask_bytes, params.data() + 8, 4);
    std::memcpy(&src_bytes, params.data() + 12, 4);
    const uint8_t* b0 = params.data() + 16;
    const uint8_t* mask_data = params.data() + 16 + 32;
    const uint8_t* src_data = mask_data + mask_bytes;
    if (params.size() != (size_t)(16 + 32 + mask_bytes + src_bytes)) return fail_msg("params size mismatch");
    if (total_instances == 0 || total_instances > 65536) return fail_msg("total_instances out of capacity");
    if (num_groups == 0 || num_groups > 256) return fail_msg("num_groups out of capacity");

    const UINT local_prefix_bytes = total_instances * 4u;
    const UINT group_bytes = num_groups * 4u;
    const UINT survivor_bytes = 4u;
    const UINT dst_bytes = total_instances * 48u;

    {
        static const GUID kExp = D3D12ExperimentalShaderModels;
        bool experimental = SUCCEEDED(D3D12EnableExperimentalFeatures(1, &kExp, nullptr, nullptr));
        std::printf("RXGD_DISPATCH: experimental_shader_models=%s\n", experimental ? "on" : "off");
    }
    std::string se;
    bool s1 = sign_dxil_in_place(sl_dxil, dxil_dll, &se);
    bool s2 = sign_dxil_in_place(sg_dxil, dxil_dll, &se);
    bool s3 = sign_dxil_in_place(sc_dxil, dxil_dll, &se);
    std::printf("RXGD_DISPATCH: dxil_signed_for_load=%s\n", (s1 && s2 && s3) ? "yes" : "no");

    ComPtr<IDXGIFactory6> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) return skip_msg("no DXGI factory");
    ComPtr<IDXGIAdapter1> chosen; DXGI_ADAPTER_DESC1 chosen_desc = {}; SIZE_T best_mem = 0;
    for (UINT i = 0;; ++i) {
        ComPtr<IDXGIAdapter1> a; HRESULT e = factory->EnumAdapters1(i, &a);
        if (e == DXGI_ERROR_NOT_FOUND) break; if (FAILED(e)) break;
        DXGI_ADAPTER_DESC1 d = {}; a->GetDesc1(&d);
        if (d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE) continue;
        if (SUCCEEDED(D3D12CreateDevice(a.Get(), D3D_FEATURE_LEVEL_11_0, __uuidof(ID3D12Device), nullptr)) && d.DedicatedVideoMemory >= best_mem) {
            best_mem = d.DedicatedVideoMemory; chosen = a; chosen_desc = d;
        }
    }
    if (!chosen) return skip_msg("no hardware D3D12 adapter");
    ComPtr<ID3D12Device> device;
    if (FAILED(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)))) return skip_msg("D3D12CreateDevice failed");
    std::printf("RXGD_DISPATCH: adapter=\"%s\"\n", narrow(chosen_desc.Description).c_str());

    ComPtr<ID3D12RootSignature> scan_root, scatter_root;
    if (FAILED(device->CreateRootSignature(0, scan_rts0.data(), scan_rts0.size(), IID_PPV_ARGS(&scan_root)))) return fail_msg("CreateRootSignature(scan)");
    if (FAILED(device->CreateRootSignature(0, scatter_rts0.data(), scatter_rts0.size(), IID_PPV_ARGS(&scatter_root)))) return fail_msg("CreateRootSignature(scatter)");
    auto make_pso = [&](ID3D12RootSignature* r, std::vector<uint8_t>& dxil, ComPtr<ID3D12PipelineState>& pso, const char* label) -> bool {
        D3D12_COMPUTE_PIPELINE_STATE_DESC pd = {}; pd.pRootSignature = r; pd.CS = {dxil.data(), dxil.size()};
        if (FAILED(device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&pso)))) { std::fprintf(stderr, "RXGD_DISPATCH: fail pso %s\n", label); return false; }
        return true;
    };
    ComPtr<ID3D12PipelineState> pso_sl, pso_sg, pso_sc;
    if (!make_pso(scan_root.Get(), sl_dxil, pso_sl, "scan_local")) return 1;
    if (!make_pso(scan_root.Get(), sg_dxil, pso_sg, "scan_groups")) return 1;
    if (!make_pso(scatter_root.Get(), sc_dxil, pso_sc, "scatter")) return 1;

    D3D12_COMMAND_QUEUE_DESC qd = {}; qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue;
    if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)))) return fail_msg("CreateCommandQueue");
    ComPtr<ID3D12CommandAllocator> alloc;
    if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc)))) return fail_msg("CreateCommandAllocator");
    ComPtr<ID3D12GraphicsCommandList> cmd;
    if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(), nullptr, IID_PPV_ARGS(&cmd)))) return fail_msg("CreateCommandList");

    // Buffers: 0 vis(SRV), 1 src(SRV), 2 local_prefix(UAV), 3 group_totals(UAV),
    // 4 group_offsets(UAV), 5 survivor_count(UAV), 6 dst(UAV).
    std::vector<uint8_t> zpref(local_prefix_bytes, 0), zgt(group_bytes, 0), zgo(group_bytes, 0), zsv(survivor_bytes, 0), zdst(dst_bytes, 0);
    ComPtr<ID3D12Resource> vis_b, vis_u, src_b, src_u, lp_b, lp_u, gt_b, gt_u, go_b, go_u, sv_b, sv_u, dst_b, dst_u;
    if (!make_buffer(device.Get(), cmd.Get(), mask_bytes, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, mask_data, vis_b, vis_u, "visibility_mask")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), src_bytes, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, src_data, src_b, src_u, "src_transforms")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), local_prefix_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, zpref.data(), lp_b, lp_u, "local_prefix")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), group_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, zgt.data(), gt_b, gt_u, "group_totals")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), group_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, zgo.data(), go_b, go_u, "group_offsets")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), survivor_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, zsv.data(), sv_b, sv_u, "survivor_count")) return 1;
    if (!make_buffer(device.Get(), cmd.Get(), dst_bytes, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, zdst.data(), dst_b, dst_u, "dst_transforms")) return 1;

    D3D12_DESCRIPTOR_HEAP_DESC hd = {}; hd.NumDescriptors = 11; hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV; hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap;
    if (FAILED(device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap)))) return fail_msg("CreateDescriptorHeap");
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    auto cpu = [&](UINT i) { D3D12_CPU_DESCRIPTOR_HANDLE h = heap->GetCPUDescriptorHandleForHeapStart(); h.ptr += (SIZE_T)i * inc; return h; };
    auto gpu = [&](UINT i) { D3D12_GPU_DESCRIPTOR_HANDLE h = heap->GetGPUDescriptorHandleForHeapStart(); h.ptr += (UINT64)i * inc; return h; };
    // D1 table (0..2): SRV vis, UAV local_prefix, UAV group_totals.
    srv(device.Get(), vis_b.Get(), mask_bytes, 4, cpu(0));
    uav(device.Get(), lp_b.Get(), local_prefix_bytes, 4, cpu(1));
    uav(device.Get(), gt_b.Get(), group_bytes, 4, cpu(2));
    // D2 table (3..5): SRV group_totals, UAV group_offsets, UAV survivor_count.
    srv(device.Get(), gt_b.Get(), group_bytes, 4, cpu(3));
    uav(device.Get(), go_b.Get(), group_bytes, 4, cpu(4));
    uav(device.Get(), sv_b.Get(), survivor_bytes, 4, cpu(5));
    // D3 table (6..10): SRV vis, SRV src(uint4 stride 16), SRV local_prefix, SRV group_offsets, UAV dst(uint4 stride 16).
    srv(device.Get(), vis_b.Get(), mask_bytes, 4, cpu(6));
    srv(device.Get(), src_b.Get(), src_bytes, 16, cpu(7));
    srv(device.Get(), lp_b.Get(), local_prefix_bytes, 4, cpu(8));
    srv(device.Get(), go_b.Get(), group_bytes, 4, cpu(9));
    uav(device.Get(), dst_b.Get(), dst_bytes, 16, cpu(10));

    ID3D12DescriptorHeap* heaps[] = {heap.Get()};
    cmd->SetDescriptorHeaps(1, heaps);
    uint32_t rc[8]; std::memcpy(rc, b0, 32);
    const UINT groups = std::max<UINT>((total_instances + 255u) / 256u, 1u);

    auto transition = [&](ID3D12Resource* r, D3D12_RESOURCE_STATES before, D3D12_RESOURCE_STATES after) {
        D3D12_RESOURCE_BARRIER b = {}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        b.Transition.pResource = r; b.Transition.StateBefore = before; b.Transition.StateAfter = after;
        b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES; cmd->ResourceBarrier(1, &b);
    };
    auto uav_barrier = [&](ID3D12Resource* r) {
        D3D12_RESOURCE_BARRIER b = {}; b.Type = D3D12_RESOURCE_BARRIER_TYPE_UAV; b.UAV.pResource = r; cmd->ResourceBarrier(1, &b);
    };

    // D1 scan_local.
    cmd->SetComputeRootSignature(scan_root.Get());
    cmd->SetPipelineState(pso_sl.Get());
    cmd->SetComputeRoot32BitConstants(0, 8, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, gpu(0));
    cmd->Dispatch(groups, 1, 1);
    uav_barrier(lp_b.Get()); uav_barrier(gt_b.Get());
    transition(gt_b.Get(), D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    // D2 scan_groups.
    cmd->SetComputeRootSignature(scan_root.Get());
    cmd->SetPipelineState(pso_sg.Get());
    cmd->SetComputeRoot32BitConstants(0, 8, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, gpu(3));
    cmd->Dispatch(1, 1, 1);
    uav_barrier(go_b.Get());
    transition(lp_b.Get(), D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    transition(go_b.Get(), D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
    // D3 scatter.
    cmd->SetComputeRootSignature(scatter_root.Get());
    cmd->SetPipelineState(pso_sc.Get());
    cmd->SetComputeRoot32BitConstants(0, 8, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, gpu(6));
    cmd->Dispatch(groups, 1, 1);

    // Readback: dst, local_prefix, group_totals, group_offsets, survivor_count.
    // local_prefix/group_totals/group_offsets are currently in NON_PIXEL (or
    // UAV for dst/survivor) -> transition each to COPY_SOURCE.
    transition(dst_b.Get(), D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_SOURCE);
    transition(lp_b.Get(), D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_COPY_SOURCE);
    transition(gt_b.Get(), D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_COPY_SOURCE);
    transition(go_b.Get(), D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_COPY_SOURCE);
    transition(sv_b.Get(), D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_SOURCE);

    auto rbheap = heap_props(D3D12_HEAP_TYPE_READBACK);
    auto make_rb = [&](UINT bytes, ComPtr<ID3D12Resource>& rb) -> bool {
        auto d = buffer_desc(bytes, D3D12_RESOURCE_FLAG_NONE);
        return SUCCEEDED(device->CreateCommittedResource(&rbheap, D3D12_HEAP_FLAG_NONE, &d, D3D12_RESOURCE_STATE_COPY_DEST, nullptr, IID_PPV_ARGS(&rb)));
    };
    ComPtr<ID3D12Resource> rb_dst, rb_lp, rb_gt, rb_go, rb_sv;
    if (!make_rb(dst_bytes, rb_dst) || !make_rb(local_prefix_bytes, rb_lp) || !make_rb(group_bytes, rb_gt) || !make_rb(group_bytes, rb_go) || !make_rb(survivor_bytes, rb_sv)) return fail_msg("CreateCommittedResource(readback)");
    cmd->CopyBufferRegion(rb_dst.Get(), 0, dst_b.Get(), 0, dst_bytes);
    cmd->CopyBufferRegion(rb_lp.Get(), 0, lp_b.Get(), 0, local_prefix_bytes);
    cmd->CopyBufferRegion(rb_gt.Get(), 0, gt_b.Get(), 0, group_bytes);
    cmd->CopyBufferRegion(rb_go.Get(), 0, go_b.Get(), 0, group_bytes);
    cmd->CopyBufferRegion(rb_sv.Get(), 0, sv_b.Get(), 0, survivor_bytes);
    if (FAILED(cmd->Close())) return fail_msg("Close command list");

    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence;
    if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) return fail_msg("CreateFence");
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (FAILED(queue->Signal(fence.Get(), 1))) return fail_msg("Signal fence");
    if (fence->GetCompletedValue() < 1) { fence->SetEventOnCompletion(1, ev); WaitForSingleObject(ev, INFINITE); }
    CloseHandle(ev);
    if (fence->GetCompletedValue() < 1) return fail_msg("fence did not complete");

    std::vector<uint8_t> out;
    auto append_rb = [&](ComPtr<ID3D12Resource>& rb, UINT bytes) -> bool {
        uint8_t* m = nullptr; D3D12_RANGE r = {0, (SIZE_T)bytes};
        if (FAILED(rb->Map(0, &r, reinterpret_cast<void**>(&m)))) return false;
        size_t off = out.size(); out.resize(off + bytes); std::memcpy(out.data() + off, m, bytes);
        rb->Unmap(0, nullptr); return true;
    };
    if (!append_rb(rb_dst, dst_bytes) || !append_rb(rb_lp, local_prefix_bytes) || !append_rb(rb_gt, group_bytes) || !append_rb(rb_go, group_bytes) || !append_rb(rb_sv, survivor_bytes)) return fail_msg("Map readback");
    std::ofstream of(out_bin, std::ios::binary);
    if (!of) return fail_msg("open out_bin");
    of.write(reinterpret_cast<const char*>(out.data()), (std::streamsize)out.size());
    of.close(); if (!of) return fail_msg("write out_bin");

    uint32_t checksum = 2166136261u; for (uint8_t b : out) { checksum ^= b; checksum *= 16777619u; }
    std::printf("RXGD_DISPATCH: ok adapter=\"%s\" total=%u groups=%u dst_bytes=%u checksum=0x%08x\n",
                narrow(chosen_desc.Description).c_str(), total_instances, num_groups, dst_bytes, checksum);
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
        f'cl /nologo /std:c++17 /EHsc /W4 /O2 /fp:precise /DUNICODE /D_UNICODE {include_flag}"{cpp}" '
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
        elif line.startswith("RXGD_DISPATCH: ok "):
            for token in ("total=", "groups=", "checksum="):
                idx = line.find(token)
                if idx >= 0:
                    parsed[token.rstrip("=")] = line[idx + len(token):].split(" ", 1)[0]
            a0 = line.find('adapter="')
            if a0 >= 0:
                a0 += len('adapter="')
                a1 = line.find('"', a0)
                if a1 > a0:
                    parsed["adapter"] = line[a0:a1]
    return parsed


def main() -> int:
    global _EVIDENCE_BASE

    required = [
        SCAN_LOCAL_DXIL, SCAN_LOCAL_RTS0, SCAN_GROUPS_DXIL, SCAN_GROUPS_RTS0,
        SCATTER_DXIL, SCATTER_RTS0, DESCRIPTOR_LAYOUT, OFFLINE_EVIDENCE, MATH_PARITY_SCRIPT,
    ]
    for path in required:
        if not path.is_file():
            _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
            return fail(f"required artifact missing: {path.relative_to(ROOT)}")

    digests = {
        "dxil_scan_local": sha256_file(SCAN_LOCAL_DXIL),
        "root_signature_scan_local": sha256_file(SCAN_LOCAL_RTS0),
        "dxil_scan_groups": sha256_file(SCAN_GROUPS_DXIL),
        "root_signature_scan_groups": sha256_file(SCAN_GROUPS_RTS0),
        "dxil_scatter": sha256_file(SCATTER_DXIL),
        "root_signature_scatter": sha256_file(SCATTER_RTS0),
        "descriptor_layout": sha256_file(DESCRIPTOR_LAYOUT),
    }
    offline = load_json(OFFLINE_EVIDENCE)
    layout = load_json(DESCRIPTOR_LAYOUT)
    if offline is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read offline_compile_evidence.json")
    if layout is None:
        _EVIDENCE_BASE = {"schema_version": 1, "subject": SUBJECT}
        return fail("cannot read instance_compaction_descriptor_layout.json")

    offline_digests = offline_artifact_digests(offline)
    hashes_match = all(digests[k] == offline_digests[k] for k in digests)
    _EVIDENCE_BASE = {
        "schema_version": 1,
        "subject": SUBJECT,
        "pass_id": "instance_compaction",
        "segment": "standalone_dispatch_smoke",
        "runtime_state": "fallback_only",
        "real_gpu_pass": False,
        "real_d3d12_dispatch_recorded": False,
        "cpu_reference_match": False,
        "artifacts": {k: {"sha256": v} for k, v in digests.items()},
        "offline_evidence": {"path": str(OFFLINE_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
                             "digests": offline_digests},
        "artifact_hashes_match_offline_evidence": hashes_match,
        "note": (
            "GRX-016 standalone real D3D12 instance_compaction three-kernel-chain "
            "dispatch smoke evidence only (scan_local -> scan_groups -> scatter). A "
            "success flips real_d3d12_dispatch_recorded/cpu_reference_match true but "
            "keeps runtime_state=fallback_only and real_gpu_pass=false."
        ),
    }

    if not hashes_match:
        return fail("artifact SHA-256 does not match tracked offline compile evidence",
                    extra={"observed": digests, "offline": offline_digests})

    layout_issue = descriptor_layout_matches_resource_mapping(layout)
    if layout_issue is not None:
        return fail(f"descriptor layout / resource mapping mismatch: {layout_issue}")

    parity = load_math_parity_reference()
    if parity is None or not hasattr(parity, "reference_chain") or not hasattr(parity, "pack_mask"):
        return fail("cannot import the tracked generate_math_parity_evidence.py reference (reference_chain / pack_mask)")

    vcvars = locate_vcvars64()
    if vcvars is None:
        return skip("未找到 VS vcvars64.bat(set RURIX_VCVARS64);无法编译真实 D3D12 dispatch harness")
    dxc_dir = locate_signed_dxc_dir()
    if dxc_dir is None:
        return skip("未找到含 dxil.dll 的签名 DXC pin(set RURIX_DXC_DIR)")
    include_dir = locate_dxcapi_include(dxc_dir)
    if include_dir is None:
        return skip(f"未在 {dxc_dir} 附近找到 dxcapi.h")
    dxil_dll = dxc_dir / "dxil.dll"

    WORK.mkdir(parents=True, exist_ok=True)
    cpp = WORK / "instance_compaction_dispatch_harness.cpp"
    exe = WORK / "instance_compaction_dispatch_harness.exe"
    cpp.write_text(HARNESS_CPP, encoding="utf-8")
    built, build_log = compile_harness(vcvars, cpp, exe, include_dir)
    if not built:
        print(build_log, file=sys.stderr)
        return skip("MSVC 编译 D3D12 dispatch harness 失败", extra={"build_log_tail": build_log})

    device_info: dict = {}
    case_results: list[dict] = []
    all_match = True
    for case in CASES:
        payload = build_case_payload(parity, case)
        cid = payload["case_id"]
        params_bin = WORK / f"params_{cid}.bin"
        out_bin = WORK / f"out_{cid}.bin"
        params_bin.write_bytes(payload["params"])
        if out_bin.exists():
            out_bin.unlink()
        p = run([str(exe), str(SCAN_LOCAL_DXIL), str(SCAN_GROUPS_DXIL), str(SCATTER_DXIL),
                 str(SCAN_LOCAL_RTS0), str(SCATTER_RTS0), str(params_bin), str(out_bin), str(dxil_dll)], cwd=WORK)
        output = (p.stdout + p.stderr).strip()
        print(f"--- case {cid} ---")
        print(output)
        parsed = parse_harness_output(output)
        if not device_info:
            device_info = {
                "adapter": parsed.get("adapter"),
                "experimental_shader_models": parsed.get("experimental_shader_models"),
                "dxil_signed_for_load": parsed.get("dxil_signed_for_load"),
                "dxil_validator": str(dxil_dll).replace("\\", "/"),
            }
        if p.returncode == 2:
            return skip("no real D3D12 device harness available", extra={"device": device_info, "stdout": output})
        if p.returncode != 0 or "RXGD_DISPATCH: ok" not in output or not out_bin.is_file():
            return fail(f"real D3D12 instance_compaction dispatch smoke failed for case {cid}",
                        extra={"device": device_info, "exit_code": p.returncode, "stdout": output})
        comparison = compare_outputs(payload, out_bin)
        if not comparison.get("match"):
            all_match = False
        case_results.append({
            "case_id": cid,
            "total_instances": payload["n"],
            "num_groups": payload["num_groups"],
            "survivor_count": payload["survivor_count"],
            "readback_checksum": parsed.get("checksum"),
            "comparison": comparison,
        })

    if not all_match:
        return fail("GPU-observed instance_compaction output did not match the tracked reference exactly",
                    extra={"device": device_info, "cases": case_results})

    write_evidence(
        "success",
        extra={
            "real_d3d12_dispatch_recorded": True,
            "cpu_reference_match": True,
            "device": device_info,
            "cpu_reference": {
                "reference_impl": (
                    "spike/godot-rurix/passes/instance_compaction/generate_math_parity_evidence.py "
                    "reference_chain (imported; dst + local_prefix + group_totals + group_offsets + "
                    "survivor_count compared byte-exact)"
                ),
                "value_tolerance": VALUE_TOLERANCE,
                "cases": case_results,
            },
            "checks": {
                "seven_artifact_hashes_match_offline_evidence": True,
                "descriptor_layout_matches_resource_mapping": True,
                "three_kernel_chain_scan_local_scan_groups_scatter": True,
                "inter_dispatch_uav_and_state_barriers": True,
                "capacity_fail_closed_at_65536": True,
                "dispatch_executed": True,
                "fence_completed": True,
                "all_output_words_match_cpu_reference_exactly": True,
            },
        },
    )
    print(f"[grx016-d3d12-dispatch-smoke] PASS measured real D3D12 3-kernel chain over "
          f"{len(case_results)} cases; adapter={device_info.get('adapter')} tolerance=0 (exact)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
