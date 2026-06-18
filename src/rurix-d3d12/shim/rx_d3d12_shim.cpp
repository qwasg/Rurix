// rx_d3d12_shim.cpp — D3D12/DXGI present 薄 C/C++ shim（RFC-0001 §4.2）。
//
// ⚠ 仅 feature `real-shim` 经 build.rs + cc 编译;需 MSVC + Windows SDK（D3D12 头/库 +
//   d3dcompiler）+ 交互桌面会话。**本会话（无 MSVC on PATH）未编译验证**——设备真跑
//   （步骤 40/41）在交互桌面 runner 上经 `--features d3d12-interop-real` 验证,run URL 回填。
//
// COM 复杂度全部留此（不进语言,D-130）。Rust 侧仅见版本化扁平 C ABI（见 RFC-0001 §4.2.1 /
//   src/lib.rs extern "C"）。窗口/消息泵/factory/device/queue/swapchain/present shader/共享
//   resource·fence 全部由 shim 拥有,固定创建线程。
//
// present shader:为减少 build.rs 复杂度,present.hlsl 经 d3dcompiler `D3DCompile` 在 create
//   时编译一次（非每帧;非 Rurix shader codegen）。RFC-0001 §4.2.2 更偏好「构建期 DXBC 嵌入」,
//   两者功能等价,build-time 嵌入为后续硬化项（runner 上 reconcile）。

#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <d3dcompiler.h>
#include <cstdint>
#include <cstring>
#include <string>

using Microsoft::WRL::ComPtr;

#define RX_D3D12_ABI_VERSION 1u
#define RX_D3D12_PRESENT_VSYNC 0x1u
static const uint32_t kFrameCount = 2;

namespace {

const char* kPresentHlsl = R"HLSL(
cbuffer Dims : register(b0) { uint render_w; uint render_h; uint window_w; uint window_h; };
ByteAddressBuffer rgb : register(t0);
struct VSOut { float4 pos : SV_Position; };
VSOut VSMain(uint vid : SV_VertexID) {
    VSOut o; float2 uv = float2((vid << 1) & 2, vid & 2);
    o.pos = float4(uv * float2(2.0,-2.0) + float2(-1.0,1.0), 0.0, 1.0); return o;
}
float4 PSMain(VSOut i) : SV_Target {
    uint px=(uint)i.pos.x, py=(uint)i.pos.y;
    uint rx=(window_w==render_w)?px:(px*render_w)/max(window_w,1u);
    uint ry=(window_h==render_h)?py:(py*render_h)/max(window_h,1u);
    rx=min(rx,render_w-1u); ry=min(ry,render_h-1u);
    uint base=(ry*render_w+rx)*3u;
    float r=asfloat(rgb.Load(base*4u+0u)), g=asfloat(rgb.Load(base*4u+4u)), b=asfloat(rgb.Load(base*4u+8u));
    return float4(r/255.0,g/255.0,b/255.0,1.0);
}
)HLSL";

LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    if (msg == WM_CLOSE || msg == WM_DESTROY) { return 0; }
    return DefWindowProcW(hwnd, msg, wp, lp);
}

} // namespace

// 不透明会话(对齐 Rust RxD3D12Present)。
struct RxD3D12Present {
    DWORD owner_thread = 0;
    HWND hwnd = nullptr;
    HINSTANCE hinst = nullptr;
    uint32_t render_w = 0, render_h = 0, window_w = 0, window_h = 0;
    ComPtr<IDXGIFactory4> factory;
    ComPtr<ID3D12Device> device;
    ComPtr<ID3D12CommandQueue> queue;
    ComPtr<IDXGISwapChain3> swapchain;
    ComPtr<ID3D12DescriptorHeap> rtv_heap;
    ComPtr<ID3D12Resource> rts[kFrameCount];
    UINT rtv_size = 0;
    ComPtr<ID3D12Resource> shared_buffer; // committed, D3D12_HEAP_FLAG_SHARED
    ComPtr<ID3D12Fence> shared_fence;      // D3D12_FENCE_FLAG_SHARED
    ComPtr<ID3D12RootSignature> root_sig;
    ComPtr<ID3D12PipelineState> pso;
    ComPtr<ID3D12CommandAllocator> alloc;
    ComPtr<ID3D12GraphicsCommandList> cmd;
    ComPtr<ID3D12Fence> local_fence; // wait_idle/host 同步
    UINT64 local_fence_val = 0;
    HANDLE local_event = nullptr;
};

static int32_t fail(HRESULT hr) { return (int32_t)hr; }

extern "C" int32_t rx_d3d12_present_create(
    uint32_t abi_version, const uint8_t cuda_luid[8], uint32_t cuda_node_mask,
    uint32_t render_width, uint32_t render_height, uint32_t window_width, uint32_t window_height,
    uint32_t flags, RxD3D12Present** out_present, RxD3D12InteropExport* out_export)
{
    if (abi_version != RX_D3D12_ABI_VERSION) return fail(E_INVALIDARG);
    if (!out_present || !out_export || !cuda_luid) return fail(E_POINTER);
    if ((flags & ~RX_D3D12_PRESENT_VSYNC) != 0) return fail(E_INVALIDARG);
    if (render_width == 0 || render_height == 0 || window_width == 0 || window_height == 0)
        return fail(E_INVALIDARG);

    auto* p = new (std::nothrow) RxD3D12Present();
    if (!p) return fail(E_OUTOFMEMORY);
    p->owner_thread = GetCurrentThreadId();
    p->render_w = render_width; p->render_h = render_height;
    p->window_w = window_width; p->window_h = window_height;
    HRESULT hr;

    // 窗口。
    WNDCLASSEXW wc = { sizeof(wc) };
    wc.lpfnWndProc = WndProc; wc.hInstance = GetModuleHandleW(nullptr);
    wc.lpszClassName = L"RxD3D12PresentWnd";
    RegisterClassExW(&wc);
    p->hinst = wc.hInstance;
    RECT rc = { 0, 0, (LONG)window_width, (LONG)window_height };
    AdjustWindowRect(&rc, WS_OVERLAPPEDWINDOW, FALSE);
    p->hwnd = CreateWindowExW(0, wc.lpszClassName, L"Rurix G1.1 — soft raster (CUDA→D3D12)",
        WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, rc.right - rc.left, rc.bottom - rc.top,
        nullptr, nullptr, wc.hInstance, nullptr);
    if (!p->hwnd) { delete p; return fail(HRESULT_FROM_WIN32(GetLastError())); }
    ShowWindow(p->hwnd, SW_SHOW);

    hr = CreateDXGIFactory2(0, IID_PPV_ARGS(&p->factory));
    if (FAILED(hr)) { delete p; return fail(hr); }

    // 按 LUID 找 adapter（RFC-0001 §4.4）。
    ComPtr<IDXGIAdapter1> adapter; ComPtr<IDXGIAdapter1> chosen;
    for (UINT i = 0; p->factory->EnumAdapters1(i, &adapter) != DXGI_ERROR_NOT_FOUND; ++i) {
        DXGI_ADAPTER_DESC1 d; adapter->GetDesc1(&d);
        if (memcmp(&d.AdapterLuid, cuda_luid, 8) == 0) { chosen = adapter; break; }
    }
    if (!chosen) { delete p; return fail(E_FAIL); } // 无同 LUID adapter
    hr = D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&p->device));
    if (FAILED(hr)) { delete p; return fail(hr); }

    D3D12_COMMAND_QUEUE_DESC qd = {};
    hr = p->device->CreateCommandQueue(&qd, IID_PPV_ARGS(&p->queue));
    if (FAILED(hr)) { delete p; return fail(hr); }

    DXGI_SWAP_CHAIN_DESC1 scd = {};
    scd.BufferCount = kFrameCount; scd.Width = window_width; scd.Height = window_height;
    scd.Format = DXGI_FORMAT_R8G8B8A8_UNORM; scd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    scd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD; scd.SampleDesc.Count = 1;
    ComPtr<IDXGISwapChain1> sc1;
    hr = p->factory->CreateSwapChainForHwnd(p->queue.Get(), p->hwnd, &scd, nullptr, nullptr, &sc1);
    if (FAILED(hr)) { delete p; return fail(hr); }
    sc1.As(&p->swapchain);

    D3D12_DESCRIPTOR_HEAP_DESC rhd = {}; rhd.NumDescriptors = kFrameCount;
    rhd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    p->device->CreateDescriptorHeap(&rhd, IID_PPV_ARGS(&p->rtv_heap));
    p->rtv_size = p->device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
    D3D12_CPU_DESCRIPTOR_HANDLE rtv = p->rtv_heap->GetCPUDescriptorHandleForHeapStart();
    for (UINT i = 0; i < kFrameCount; ++i) {
        p->swapchain->GetBuffer(i, IID_PPV_ARGS(&p->rts[i]));
        p->device->CreateRenderTargetView(p->rts[i].Get(), nullptr, rtv);
        rtv.ptr += p->rtv_size;
    }

    // 共享 committed buffer（RGB f32,行主序）。
    uint64_t mapping_size = (uint64_t)render_width * render_height * 3ull * sizeof(float);
    D3D12_HEAP_PROPERTIES hp = {}; hp.Type = D3D12_HEAP_TYPE_DEFAULT;
    D3D12_RESOURCE_DESC bd = {};
    bd.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER; bd.Width = mapping_size; bd.Height = 1;
    bd.DepthOrArraySize = 1; bd.MipLevels = 1; bd.SampleDesc.Count = 1;
    bd.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR; bd.Flags = D3D12_RESOURCE_FLAG_NONE;
    hr = p->device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_SHARED, &bd,
        D3D12_RESOURCE_STATE_COMMON, nullptr, IID_PPV_ARGS(&p->shared_buffer));
    if (FAILED(hr)) { delete p; return fail(hr); }
    D3D12_RESOURCE_ALLOCATION_INFO ai = p->device->GetResourceAllocationInfo(0, 1, &bd);

    HANDLE mem_handle = nullptr;
    hr = p->device->CreateSharedHandle(p->shared_buffer.Get(), nullptr, GENERIC_ALL, nullptr, &mem_handle);
    if (FAILED(hr)) { delete p; return fail(hr); }

    hr = p->device->CreateFence(0, D3D12_FENCE_FLAG_SHARED, IID_PPV_ARGS(&p->shared_fence));
    if (FAILED(hr)) { CloseHandle(mem_handle); delete p; return fail(hr); }
    HANDLE fence_handle = nullptr;
    hr = p->device->CreateSharedHandle(p->shared_fence.Get(), nullptr, GENERIC_ALL, nullptr, &fence_handle);
    if (FAILED(hr)) { CloseHandle(mem_handle); delete p; return fail(hr); }

    // present shader + root sig + PSO（root: 32-bit constants b0 ×4 + SRV table t0）。
    ComPtr<ID3DBlob> vs, ps, err;
    hr = D3DCompile(kPresentHlsl, strlen(kPresentHlsl), "present.hlsl", nullptr, nullptr,
        "VSMain", "vs_5_1", 0, 0, &vs, &err);
    if (FAILED(hr)) { CloseHandle(mem_handle); CloseHandle(fence_handle); delete p; return fail(hr); }
    hr = D3DCompile(kPresentHlsl, strlen(kPresentHlsl), "present.hlsl", nullptr, nullptr,
        "PSMain", "ps_5_1", 0, 0, &ps, &err);
    if (FAILED(hr)) { CloseHandle(mem_handle); CloseHandle(fence_handle); delete p; return fail(hr); }

    D3D12_DESCRIPTOR_RANGE srv_range = {};
    srv_range.RangeType = D3D12_DESCRIPTOR_RANGE_TYPE_SRV; srv_range.NumDescriptors = 1;
    D3D12_ROOT_PARAMETER rp[2] = {};
    rp[0].ParameterType = D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS;
    rp[0].Constants.Num32BitValues = 4; rp[0].ShaderVisibility = D3D12_SHADER_VISIBILITY_ALL;
    rp[1].ParameterType = D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE;
    rp[1].DescriptorTable.NumDescriptorRanges = 1; rp[1].DescriptorTable.pDescriptorRanges = &srv_range;
    rp[1].ShaderVisibility = D3D12_SHADER_VISIBILITY_PIXEL;
    D3D12_ROOT_SIGNATURE_DESC rsd = {}; rsd.NumParameters = 2; rsd.pParameters = rp;
    rsd.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
    ComPtr<ID3DBlob> rs_blob;
    hr = D3D12SerializeRootSignature(&rsd, D3D_ROOT_SIGNATURE_VERSION_1, &rs_blob, &err);
    if (FAILED(hr)) { CloseHandle(mem_handle); CloseHandle(fence_handle); delete p; return fail(hr); }
    p->device->CreateRootSignature(0, rs_blob->GetBufferPointer(), rs_blob->GetBufferSize(),
        IID_PPV_ARGS(&p->root_sig));

    D3D12_GRAPHICS_PIPELINE_STATE_DESC pd = {};
    pd.pRootSignature = p->root_sig.Get();
    pd.VS = { vs->GetBufferPointer(), vs->GetBufferSize() };
    pd.PS = { ps->GetBufferPointer(), ps->GetBufferSize() };
    pd.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID; pd.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
    pd.BlendState.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
    pd.SampleMask = UINT_MAX; pd.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
    pd.NumRenderTargets = 1; pd.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM; pd.SampleDesc.Count = 1;
    hr = p->device->CreateGraphicsPipelineState(&pd, IID_PPV_ARGS(&p->pso));
    if (FAILED(hr)) { CloseHandle(mem_handle); CloseHandle(fence_handle); delete p; return fail(hr); }

    p->device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&p->alloc));
    p->device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, p->alloc.Get(), nullptr,
        IID_PPV_ARGS(&p->cmd));
    p->cmd->Close();
    p->device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&p->local_fence));
    p->local_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);

    // 回填 export。
    memset(out_export, 0, sizeof(*out_export));
    out_export->abi_version = RX_D3D12_ABI_VERSION;
    out_export->struct_size = (uint32_t)sizeof(RxD3D12InteropExport); // = 96
    out_export->memory_handle = mem_handle;
    out_export->allocation_size = ai.SizeInBytes;
    out_export->mapping_size = mapping_size;
    out_export->fence_handle = fence_handle;
    {
        DXGI_ADAPTER_DESC1 d; chosen->GetDesc1(&d);
        memcpy(out_export->adapter_luid, &d.AdapterLuid, 8);
    }
    out_export->node_mask = cuda_node_mask;
    out_export->render_width = render_width; out_export->render_height = render_height;
    out_export->window_width = window_width; out_export->window_height = window_height;
    out_export->channels = 3;
    *out_present = p;
    return 0;
}

extern "C" int32_t rx_d3d12_present_pump(RxD3D12Present* p, uint32_t* out_should_close) {
    if (!p || !out_should_close) return fail(E_POINTER);
    if (p->owner_thread != GetCurrentThreadId()) return fail(RPC_E_WRONG_THREAD);
    *out_should_close = 0;
    MSG msg;
    while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
        if (msg.message == WM_QUIT) { *out_should_close = 1; }
        TranslateMessage(&msg); DispatchMessageW(&msg);
    }
    if (!IsWindow(p->hwnd)) *out_should_close = 1;
    return 0;
}

extern "C" int32_t rx_d3d12_present_submit(RxD3D12Present* p, uint64_t cuda_done_value, uint64_t d3d_done_value) {
    if (!p) return fail(E_POINTER);
    if (p->owner_thread != GetCurrentThreadId()) return fail(RPC_E_WRONG_THREAD);
    HRESULT hr;
    // queue 等待 CUDA 完成本帧写（RFC-0001 §4.3）。
    hr = p->queue->Wait(p->shared_fence.Get(), cuda_done_value);
    if (FAILED(hr)) return fail(hr);

    UINT idx = p->swapchain->GetCurrentBackBufferIndex();
    p->alloc->Reset();
    p->cmd->Reset(p->alloc.Get(), p->pso.Get());

    // SRV(ByteAddressBuffer/RAW)需 shader-visible heap;简化:每帧用一个 CBV/SRV/UAV heap。
    ComPtr<ID3D12DescriptorHeap> srv_heap;
    D3D12_DESCRIPTOR_HEAP_DESC shd = {}; shd.NumDescriptors = 1;
    shd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    shd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    p->device->CreateDescriptorHeap(&shd, IID_PPV_ARGS(&srv_heap));
    D3D12_SHADER_RESOURCE_VIEW_DESC sd = {};
    sd.Format = DXGI_FORMAT_R32_TYPELESS; sd.ViewDimension = D3D12_SRV_DIMENSION_BUFFER;
    sd.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    sd.Buffer.NumElements = (UINT)(p->render_w * p->render_h * 3u);
    sd.Buffer.Flags = D3D12_BUFFER_SRV_FLAG_RAW;
    p->device->CreateShaderResourceView(p->shared_buffer.Get(), &sd,
        srv_heap->GetCPUDescriptorHandleForHeapStart());

    D3D12_RESOURCE_BARRIER b = {};
    b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    b.Transition.pResource = p->rts[idx].Get();
    b.Transition.StateBefore = D3D12_RESOURCE_STATE_PRESENT;
    b.Transition.StateAfter = D3D12_RESOURCE_STATE_RENDER_TARGET;
    b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    p->cmd->ResourceBarrier(1, &b);

    D3D12_CPU_DESCRIPTOR_HANDLE rtv = p->rtv_heap->GetCPUDescriptorHandleForHeapStart();
    rtv.ptr += (SIZE_T)idx * p->rtv_size;
    p->cmd->OMSetRenderTargets(1, &rtv, FALSE, nullptr);
    D3D12_VIEWPORT vp = { 0, 0, (float)p->window_w, (float)p->window_h, 0, 1 };
    D3D12_RECT sr = { 0, 0, (LONG)p->window_w, (LONG)p->window_h };
    p->cmd->RSSetViewports(1, &vp); p->cmd->RSSetScissorRects(1, &sr);
    ID3D12DescriptorHeap* heaps[] = { srv_heap.Get() };
    p->cmd->SetDescriptorHeaps(1, heaps);
    p->cmd->SetGraphicsRootSignature(p->root_sig.Get());
    uint32_t dims[4] = { p->render_w, p->render_h, p->window_w, p->window_h };
    p->cmd->SetGraphicsRoot32BitConstants(0, 4, dims, 0);
    p->cmd->SetGraphicsRootDescriptorTable(1, srv_heap->GetGPUDescriptorHandleForHeapStart());
    p->cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    p->cmd->DrawInstanced(3, 1, 0, 0);

    b.Transition.StateBefore = D3D12_RESOURCE_STATE_RENDER_TARGET;
    b.Transition.StateAfter = D3D12_RESOURCE_STATE_PRESENT;
    p->cmd->ResourceBarrier(1, &b);
    p->cmd->Close();
    ID3D12CommandList* lists[] = { p->cmd.Get() };
    p->queue->ExecuteCommandLists(1, lists);
    hr = p->swapchain->Present(1, 0);
    if (FAILED(hr)) return fail(hr);
    // signal 下一写权（d3d_done = 2n+2，RFC-0001 §4.3）。
    hr = p->queue->Signal(p->shared_fence.Get(), d3d_done_value);
    // 等待本帧 GPU 完成后再复用 allocator/srv_heap（简化:每帧 host 等待 present 完成）。
    p->local_fence_val++;
    p->queue->Signal(p->local_fence.Get(), p->local_fence_val);
    if (p->local_fence->GetCompletedValue() < p->local_fence_val) {
        p->local_fence->SetEventOnCompletion(p->local_fence_val, p->local_event);
        WaitForSingleObject(p->local_event, INFINITE);
    }
    return FAILED(hr) ? fail(hr) : 0;
}

extern "C" int32_t rx_d3d12_present_wait_idle(RxD3D12Present* p) {
    if (!p) return fail(E_POINTER);
    if (p->owner_thread != GetCurrentThreadId()) return fail(RPC_E_WRONG_THREAD);
    p->local_fence_val++;
    HRESULT hr = p->queue->Signal(p->local_fence.Get(), p->local_fence_val);
    if (FAILED(hr)) return fail(hr);
    if (p->local_fence->GetCompletedValue() < p->local_fence_val) {
        p->local_fence->SetEventOnCompletion(p->local_fence_val, p->local_event);
        WaitForSingleObject(p->local_event, INFINITE);
    }
    return 0;
}

extern "C" int32_t rx_d3d12_close_shared_handle(void* handle) {
    if (!handle) return fail(E_POINTER);
    return CloseHandle((HANDLE)handle) ? 0 : fail(HRESULT_FROM_WIN32(GetLastError()));
}

extern "C" void rx_d3d12_present_destroy(RxD3D12Present* p) {
    if (!p) return;
    // 进入此函数前,Rust 侧已按 RFC-0001 §4.4 销毁 CUDA mapped pointer/semaphore/memory。
    rx_d3d12_present_wait_idle(p);
    if (p->local_event) CloseHandle(p->local_event);
    if (p->hwnd) DestroyWindow(p->hwnd);
    // ComPtr 析构释放 fence/resource/queue/swapchain/device/factory。
    delete p;
}
