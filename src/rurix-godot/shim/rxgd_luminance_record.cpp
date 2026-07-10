// rxgd_luminance_record.cpp — GRX-009 segment 4d bridge D3D12 dispatch recording shim.
//
// Compiled only under the rurix-godot `d3d12-recording-shim` feature (see
// build.rs). It records ONE minimal luminance compute dispatch on a REAL
// caller-provided D3D12 device / command queue and REAL src/dst
// ID3D12Resource* handles, using the tracked offline luminance DXIL container +
// RTS0 root signature (bytes are passed in by the Rust bridge, which first
// verified they hash to the segment 3a offline compile evidence digests).
//
// This shim NEVER creates a device, NEVER accepts fake/null handles, and NEVER
// fakes a dispatch: any failure returns a negative status and the bridge falls
// back. It does not enable the Godot runtime luminance path.
//
// Layout (from the tracked descriptor layout, mirrors the segment 4c smoke):
//   root param 0 = 7-dword (28-byte) b0 root constants
//                  (source_width/source_height as i64 + 3 f32 scalars)
//   root param 1 = descriptor table [ SRV t0 (src_luminance), UAV u0 (dst) ]
//
// Contract with the harness (ci/grx009_luminance_bridge_recording_smoke.py):
//   * src is an R32_FLOAT Texture2D already populated and left in
//     D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE.
//   * dst is an R32_FLOAT Texture2D (ALLOW_UNORDERED_ACCESS) in
//     D3D12_RESOURCE_STATE_UNORDERED_ACCESS.
// The shim only dispatches, transitions dst -> COPY_SOURCE, copies to its own
// readback buffer, waits on a fence, and checksums the readback.

#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#ifdef RXGD_HAVE_DXCAPI
#include <dxcapi.h>
#endif

using Microsoft::WRL::ComPtr;

// Shim <-> Rust ABI version (kept in sync with the Rust d3d12_shim module).
static const uint32_t kShimAbiVersion = 1u;

extern "C" struct RxgdRecordResult {
    uint64_t fence_completed_value;
    uint32_t dispatch_x;
    uint32_t dispatch_y;
    uint32_t dispatch_z;
    uint32_t dst_width;
    uint32_t dst_height;
    uint32_t readback_checksum;
    float dst_first_value;
    int32_t dxil_signed;      // 1 = in-memory DXIL was signed, 0 = not signed
    char error_detail[256];
};

static void set_detail(RxgdRecordResult* out, const char* what, HRESULT hr) {
    if (!out) return;
    std::snprintf(out->error_detail, sizeof(out->error_detail),
                  "%s hr=0x%08lx", what ? what : "", (unsigned long)hr);
}
static void set_detail_msg(RxgdRecordResult* out, const char* what) {
    if (!out) return;
    std::snprintf(out->error_detail, sizeof(out->error_detail), "%s",
                  what ? what : "");
}

static D3D12_HEAP_PROPERTIES heap_props(D3D12_HEAP_TYPE type) {
    D3D12_HEAP_PROPERTIES hp = {};
    hp.Type = type;
    hp.CreationNodeMask = 1;
    hp.VisibleNodeMask = 1;
    return hp;
}
// Resolve a (possibly typeless) resource format to a concrete typed format that
// is view-compatible with the resource's format family. Godot creates its HDR
// color / luminance textures with *typeless* formats (e.g. R16G16B16A16_TYPELESS
// for the scene color, R32_TYPELESS for the reduction target). Creating a view
// whose format is not in the resource's typeless family (the old shim hardcoded
// R32_FLOAT for both) is invalid and on some drivers removes the device
// (DXGI_ERROR_DEVICE_HUNG). This maps the common typeless families to a typed
// member and passes already-typed formats through unchanged.
static DXGI_FORMAT typed_view_format(DXGI_FORMAT resource_format) {
    switch (resource_format) {
        case DXGI_FORMAT_R32_TYPELESS:
            return DXGI_FORMAT_R32_FLOAT;
        case DXGI_FORMAT_R16G16B16A16_TYPELESS:
            return DXGI_FORMAT_R16G16B16A16_FLOAT;
        case DXGI_FORMAT_R32G32B32A32_TYPELESS:
            return DXGI_FORMAT_R32G32B32A32_FLOAT;
        case DXGI_FORMAT_R16_TYPELESS:
            return DXGI_FORMAT_R16_FLOAT;
        case DXGI_FORMAT_R8G8B8A8_TYPELESS:
            return DXGI_FORMAT_R8G8B8A8_UNORM;
        case DXGI_FORMAT_UNKNOWN:
            return DXGI_FORMAT_R32_FLOAT;
        default:
            // Already a typed format: use it as-is so the view matches the
            // resource exactly.
            return resource_format;
    }
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

#ifdef RXGD_HAVE_DXCAPI
// Minimal in-memory IDxcBlob so the DXIL validator can sign our container bytes
// in place (DxcValidatorFlags_InPlaceEdit writes the validation hash directly
// into the buffer we own). Mirrors the segment 4c smoke's MemBlob.
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

// Sign the DXIL container in place with the DXIL validator (dxil.dll). Loads
// dxil.dll from RURIX_DXC_DIR / RURIX_DXC_NEW_DIR (or the system search path).
// Signing does not change shader semantics; it appends the validation hash so
// the same container can create a compute PSO on a normal device. The tracked
// artifact file on disk is never touched — only the caller-owned memory copy.
static bool sign_dxil_in_place(std::vector<uint8_t>& dxil) {
    HMODULE lib = nullptr;
    for (const wchar_t* key : {L"RURIX_DXC_DIR", L"RURIX_DXC_NEW_DIR"}) {
        wchar_t buf[1024];
        DWORD n = GetEnvironmentVariableW(key, buf, 1024);
        if (n == 0 || n >= 1024) continue;
        std::wstring path(buf, n);
        if (!path.empty() && path.back() != L'\\' && path.back() != L'/') path.push_back(L'\\');
        path += L"dxil.dll";
        lib = LoadLibraryW(path.c_str());
        if (lib) break;
    }
    if (!lib) lib = LoadLibraryW(L"dxil.dll");
    if (!lib) return false;
    auto create = reinterpret_cast<DxcCreateInstanceProc>(GetProcAddress(lib, "DxcCreateInstance"));
    if (!create) return false;
    IDxcValidator* validator = nullptr;
    if (FAILED(create(CLSID_DxcValidator, __uuidof(IDxcValidator),
                      reinterpret_cast<void**>(&validator))) || !validator) {
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
    }
    if (result) result->Release();
    validator->Release();
    return ok;
}
#else
static bool sign_dxil_in_place(std::vector<uint8_t>&) {
    // dxcapi.h was not available at build time (no signed DXC pin located).
    // Recording can only succeed on a Developer-Mode device with experimental
    // shader models; otherwise CreateComputePipelineState will fail and the
    // bridge falls back. We never fake success.
    return false;
}
#endif

extern "C" __declspec(dllexport) uint32_t rxgd_luminance_record_shim_abi_version(void) {
    return kShimAbiVersion;
}

// Records one minimal luminance compute dispatch on the caller-provided real
// D3D12 device/queue and src/dst resources. Returns 0 on success, negative on
// failure. Never accepts null handles and never fakes a dispatch.
extern "C" __declspec(dllexport) int32_t rxgd_luminance_record_dispatch(
    uint32_t abi_version,
    void* device_ptr,
    void* queue_ptr,
    const uint8_t* dxil_bytes,
    size_t dxil_len,
    const uint8_t* rts0_bytes,
    size_t rts0_len,
    void* src_resource_ptr,
    void* dst_resource_ptr,
    const uint8_t* push_constants,
    size_t push_constant_len,
    uint32_t src_w,
    uint32_t src_h,
    uint32_t dst_w,
    uint32_t dst_h,
    RxgdRecordResult* out) {
    if (!out) return -1;
    std::memset(out, 0, sizeof(*out));

    if (abi_version != kShimAbiVersion) {
        set_detail_msg(out, "shim abi version mismatch");
        return -2;
    }
    if (!device_ptr || !queue_ptr || !src_resource_ptr || !dst_resource_ptr) {
        set_detail_msg(out, "null device/queue/resource handle");
        return -3;
    }
    if (!dxil_bytes || dxil_len == 0 || !rts0_bytes || rts0_len == 0) {
        set_detail_msg(out, "empty dxil/rts0 bytes");
        return -4;
    }
    if (!push_constants || push_constant_len != 28) {
        set_detail_msg(out, "push constant block is not 28 bytes");
        return -5;
    }

    ID3D12Device* device = reinterpret_cast<ID3D12Device*>(device_ptr);
    ID3D12CommandQueue* queue = reinterpret_cast<ID3D12CommandQueue*>(queue_ptr);
    ID3D12Resource* src = reinterpret_cast<ID3D12Resource*>(src_resource_ptr);
    ID3D12Resource* dst = reinterpret_cast<ID3D12Resource*>(dst_resource_ptr);

    // Sign an in-memory copy of the tracked DXIL container so it can create a
    // compute PSO on a normal (non-Developer-Mode) device. The tracked artifact
    // bytes passed in by the bridge are not modified.
    std::vector<uint8_t> dxil(dxil_bytes, dxil_bytes + dxil_len);
    out->dxil_signed = sign_dxil_in_place(dxil) ? 1 : 0;

    // (A) Root signature DIRECTLY from the Rurix RTS0 bytes (device-parse proof).
    ComPtr<ID3D12RootSignature> root;
    HRESULT hr = device->CreateRootSignature(0, rts0_bytes, rts0_len, IID_PPV_ARGS(&root));
    if (FAILED(hr)) {
        set_detail(out, "CreateRootSignature(rurix rts0)", hr);
        return -10;
    }

    // (B) Compute PSO from the Rurix DXIL container.
    D3D12_COMPUTE_PIPELINE_STATE_DESC pd = {};
    pd.pRootSignature = root.Get();
    pd.CS = {dxil.data(), dxil.size()};
    ComPtr<ID3D12PipelineState> pso;
    hr = device->CreateComputePipelineState(&pd, IID_PPV_ARGS(&pso));
    if (FAILED(hr)) {
        set_detail(out, "CreateComputePipelineState(rurix dxil)", hr);
        return -11;
    }

    ComPtr<ID3D12CommandAllocator> alloc;
    hr = device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&alloc));
    if (FAILED(hr)) {
        set_detail(out, "CreateCommandAllocator", hr);
        return -12;
    }

    // Descriptor heap: index 0 = SRV(t0, src), index 1 = UAV(u0, dst).
    D3D12_DESCRIPTOR_HEAP_DESC hd = {};
    hd.NumDescriptors = 2;
    hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> heap;
    hr = device->CreateDescriptorHeap(&hd, IID_PPV_ARGS(&heap));
    if (FAILED(hr)) {
        set_detail(out, "CreateDescriptorHeap(cbv_srv_uav)", hr);
        return -13;
    }
    const UINT inc = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);
    D3D12_CPU_DESCRIPTOR_HANDLE cpu0 = heap->GetCPUDescriptorHandleForHeapStart();
    // Derive the view formats from the real resource formats so a typeless
    // Godot texture gets a view-compatible typed format instead of a hardcoded
    // R32_FLOAT (which is invalid for e.g. R16G16B16A16_TYPELESS and removes the
    // device on some drivers).
    const DXGI_FORMAT src_view_format = typed_view_format(src->GetDesc().Format);
    const DXGI_FORMAT dst_view_format = typed_view_format(dst->GetDesc().Format);
    D3D12_SHADER_RESOURCE_VIEW_DESC srv = {};
    srv.Format = src_view_format;
    srv.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
    srv.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    srv.Texture2D.MipLevels = 1;
    device->CreateShaderResourceView(src, &srv, cpu0);
    D3D12_CPU_DESCRIPTOR_HANDLE cpu1 = cpu0;
    cpu1.ptr += inc;
    D3D12_UNORDERED_ACCESS_VIEW_DESC uav = {};
    uav.Format = dst_view_format;
    uav.ViewDimension = D3D12_UAV_DIMENSION_TEXTURE2D;
    device->CreateUnorderedAccessView(dst, nullptr, &uav, cpu1);

    // Readback buffer for the dst UAV (dst_w*dst_h R32_FLOAT).
    // Godot's real luminance destination is created with a *typeless* format
    // (e.g. R32_TYPELESS), unlike the segment 4d bare harness which used a typed
    // R32_FLOAT. GetCopyableFootprints cannot size a fully-typeless resource and
    // returns UINT64_MAX; creating a readback buffer that large hangs the
    // device. Resolve the typeless format to the typed single-plane equivalent
    // we bind the UAV as (both are 32-bit, so the placed-footprint copy is
    // bit-compatible) before computing the footprint.
    D3D12_RESOURCE_DESC dst_desc = dst->GetDesc();
    // Build a clean single-mip 2D footprint desc rather than copying the live
    // Godot resource desc verbatim: Godot's texture can carry an Alignment /
    // Flags combination (observed Alignment=512) that GetCopyableFootprints
    // rejects, returning UINT64_MAX. Only the typed format + extent matter for
    // the placed-footprint readback copy.
    D3D12_RESOURCE_DESC footprint_desc = {};
    footprint_desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    footprint_desc.Alignment = 0;
    footprint_desc.Width = dst_desc.Width;
    footprint_desc.Height = dst_desc.Height;
    footprint_desc.DepthOrArraySize = 1;
    footprint_desc.MipLevels = 1;
    footprint_desc.Format = typed_view_format(dst_desc.Format);
    footprint_desc.SampleDesc.Count = 1;
    footprint_desc.SampleDesc.Quality = 0;
    footprint_desc.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
    footprint_desc.Flags = D3D12_RESOURCE_FLAG_NONE;
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT dfp = {};
    UINT drows = 0;
    UINT64 drow_size = 0, dtotal = 0;
    device->GetCopyableFootprints(&footprint_desc, 0, 1, 0, &dfp, &drows, &drow_size, &dtotal);
    if (dtotal == 0 || dtotal == UINT64_MAX) {
        // Invalid/unsized footprint: never create an absurd committed resource
        // (that hangs the device). Fall back cleanly instead.
        set_detail_msg(out, "GetCopyableFootprints returned an invalid total size");
        return -14;
    }
    auto readback_heap = heap_props(D3D12_HEAP_TYPE_READBACK);
    auto rb_desc = buffer_desc(dtotal);
    ComPtr<ID3D12Resource> readback;
    hr = device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &rb_desc,
                                         D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                         IID_PPV_ARGS(&readback));
    if (FAILED(hr)) {
        set_detail(out, "CreateCommittedResource(readback)", hr);
        return -14;
    }

    ComPtr<ID3D12GraphicsCommandList> cmd;
    hr = device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(), pso.Get(),
                                   IID_PPV_ARGS(&cmd));
    if (FAILED(hr)) {
        set_detail(out, "CreateCommandList", hr);
        return -15;
    }

    // Bind the Rurix root signature and issue one minimal dispatch. The src is
    // already in NON_PIXEL_SHADER_RESOURCE and dst in UNORDERED_ACCESS (harness
    // contract), so no pre-dispatch barrier is needed.
    cmd->SetComputeRootSignature(root.Get());
    ID3D12DescriptorHeap* heaps[] = {heap.Get()};
    cmd->SetDescriptorHeaps(1, heaps);
    uint32_t rc[7];
    std::memcpy(rc, push_constants, 28);  // 7 dwords: i64 src_w, i64 src_h, 3 f32
    cmd->SetComputeRoot32BitConstants(0, 7, rc, 0);
    cmd->SetComputeRootDescriptorTable(1, heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetPipelineState(pso.Get());
    const UINT gx = std::max<UINT>((src_w + 7) / 8, 1u);
    const UINT gy = std::max<UINT>((src_h + 7) / 8, 1u);
    const UINT gz = 1;
    cmd->Dispatch(gx, gy, gz);

    // Read back the dst UAV.
    D3D12_RESOURCE_BARRIER db = {};
    db.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    db.Transition.pResource = dst;
    db.Transition.StateBefore = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    db.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
    db.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &db);
    D3D12_TEXTURE_COPY_LOCATION cdst = {};
    cdst.pResource = readback.Get();
    cdst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    cdst.PlacedFootprint = dfp;
    D3D12_TEXTURE_COPY_LOCATION csrc = {};
    csrc.pResource = dst;
    csrc.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    csrc.SubresourceIndex = 0;
    cmd->CopyTextureRegion(&cdst, 0, 0, 0, &csrc, nullptr);
    // Restore dst to UNORDERED_ACCESS so the caller's resource state is unchanged.
    D3D12_RESOURCE_BARRIER rb = db;
    rb.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_SOURCE;
    rb.Transition.StateAfter = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    cmd->ResourceBarrier(1, &rb);
    hr = cmd->Close();
    if (FAILED(hr)) {
        set_detail(out, "Close command list", hr);
        return -16;
    }

    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence;
    hr = device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence));
    if (FAILED(hr)) {
        set_detail(out, "CreateFence", hr);
        return -17;
    }
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!ev) {
        set_detail_msg(out, "CreateEvent");
        return -18;
    }
    if (FAILED(queue->Signal(fence.Get(), 1))) {
        CloseHandle(ev);
        set_detail_msg(out, "Signal fence");
        return -19;
    }
    if (fence->GetCompletedValue() < 1) {
        if (FAILED(fence->SetEventOnCompletion(1, ev))) {
            CloseHandle(ev);
            set_detail_msg(out, "SetEventOnCompletion");
            return -20;
        }
        WaitForSingleObject(ev, INFINITE);
    }
    CloseHandle(ev);
    const UINT64 fence_done = fence->GetCompletedValue();
    if (fence_done < 1) {
        set_detail_msg(out, "fence did not reach completion");
        return -21;
    }

    // Checksum the dst readback bytes (completion + output verification).
    uint8_t* mapped = nullptr;
    D3D12_RANGE range = {0, (SIZE_T)dtotal};
    if (FAILED(readback->Map(0, &range, reinterpret_cast<void**>(&mapped)))) {
        set_detail_msg(out, "Map readback");
        return -22;
    }
    uint32_t checksum = 2166136261u;  // FNV-1a over the dst rows
    float first = 0.0f;
    bool got_first = false;
    const UINT rb_w = std::max<UINT>(dst_w, 1u);
    const UINT rb_h = std::max<UINT>(dst_h, 1u);
    for (UINT y = 0; y < rb_h; ++y) {
        const uint8_t* rowp = mapped + dfp.Offset + (SIZE_T)y * dfp.Footprint.RowPitch;
        for (UINT x = 0; x < rb_w; ++x) {
            const uint8_t* px = rowp + (SIZE_T)x * 4;
            if (!got_first) {
                std::memcpy(&first, px, 4);
                got_first = true;
            }
            for (int b = 0; b < 4; ++b) {
                checksum ^= px[b];
                checksum *= 16777619u;
            }
        }
    }
    readback->Unmap(0, nullptr);

    out->fence_completed_value = fence_done;
    out->dispatch_x = gx;
    out->dispatch_y = gy;
    out->dispatch_z = gz;
    out->dst_width = rb_w;
    out->dst_height = rb_h;
    out->readback_checksum = checksum;
    out->dst_first_value = first;
    std::snprintf(out->error_detail, sizeof(out->error_detail), "ok");
    std::printf("RXGD_BRIDGE_REC: dispatch=%u,%u,%u fence=%llu dst=%ux%u dst_first=%g "
                "checksum=0x%08x dxil_signed=%s\n",
                gx, gy, gz, (unsigned long long)fence_done, rb_w, rb_h, first, checksum,
                out->dxil_signed ? "yes" : "no");
    return 0;
}
