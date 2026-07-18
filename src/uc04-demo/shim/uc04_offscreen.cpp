// UC-04 deferred 渲染器 offscreen D3D12 shim(G2.4 / RFC-0006 + RFC-0007;严格面:lighting
// pass **真采样 G-buffer**)。D3D12/DXGI 的 COM 复杂度全部留在 C++ shim(不进语言,D-130 先例,
// 对齐 src/rurix-d3d12/shim/rx_d3d12_shim.cpp);Rust 侧仅见版本化扁平 `extern "C"` 面。
//
// 消费 **Rurix 源经 rurixc 图形=B DXIL 链**产出的 4 个 DXIL 着色器对象(几何 pass VS/FS +
// lighting pass VS/FS)+ RFC-0005 推导的**每 pass** RTS0 root signature 容器字节(P-11 单一事实
// 源):几何 pass RTS0 = IA-only 空资源;lighting pass RTS0 = SRV t0 + Sampler s0 descriptor
// table(由 infer_root_signature([Texture2D, Sampler]) 推导)。在真 hardware 上:
//   pass 1(几何):VS 透传顶点缓冲 pos→SV_Position + uv/normal varying → FS 写 G-buffer
//                 MRT(albedo R8G8B8A8 / normal R16F / depth R32F,3 渲染目标);
//   barrier(RXS-0176 IR1):albedo RT **RENDER_TARGET → PIXEL_SHADER_RESOURCE**;
//   pass 2(lighting/合成):VS 透传 + FS **真采样 G-buffer albedo SRV(t0)经 sampler(s0)**
//                 (RXS-0175/0176,`dx.op.sampleLevel` LOD0)→ final = f(采样值) → 写 final R8;
//   手动 barrier(RXS-0169 编排锚点)→ offscreen readback 取 albedo 与 final 中心像素。
//
// **G-G2-4 防降级 + RFC-0007 严格面**:VS/FS 全部来自 Rurix 源经图形=B DXIL(非手写 HLSL/DXIL);
// 每 pass RTS0 经 CreateRootSignature 真机解析进 PSO;lighting pass 经 SRV/Sampler descriptor table
// 真采样几何 pass 写入的 G-buffer(final 真依赖采样值,数据流红绿见 ci/dxil_uc04_device_smoke.py)。

#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <wrl/client.h>
#include <d3d12.h>
#include <dxgi1_6.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <cwchar>
#include <vector>

using Microsoft::WRL::ComPtr;

namespace {

// shim C ABI 版本(与 Rust 侧 RX_UC04_ABI_VERSION 一致)。v2 = RFC-0007 严格面:每 pass 双 RTS0
// (geom_rts0 + light_rts0)+ lighting 真采样(SRV/Sampler descriptor table)。
//
// **每入口独立版本常量**(E-1/§4.A4):`kAbiVersion`==2 恒守 offscreen 入口(步骤 48 0-byte);
// present 入口(rx_uc04_present_run)携其自有版本常量 kPresentAbiVersion==3(>=3 语义),
// 与 offscreen 正交——新能力一律走新增独立入口,不 bump/不扩 offscreen 参数面(SC-6)。
constexpr uint32_t kAbiVersion = 2;

// G3.2 present 入口(rx_uc04_present_run)自有 ABI 版本(与 Rust RX_UC04_PRESENT_ABI_VERSION 一致)。
// v3 = 可见窗口 flip-model swapchain present + resize 重建 + 三点 backbuffer readback(RFC-0013 §4.A)。
constexpr uint32_t kPresentAbiVersion = 3;

D3D12_HEAP_PROPERTIES heap_props(D3D12_HEAP_TYPE type) {
    D3D12_HEAP_PROPERTIES hp = {};
    hp.Type = type;
    hp.CreationNodeMask = 1;
    hp.VisibleNodeMask = 1;
    return hp;
}

D3D12_RESOURCE_DESC buffer_desc(UINT64 bytes) {
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

D3D12_RESOURCE_DESC rt_desc(UINT w, UINT h, DXGI_FORMAT fmt) {
    D3D12_RESOURCE_DESC d = {};
    d.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    d.Width = w;
    d.Height = h;
    d.DepthOrArraySize = 1;
    d.MipLevels = 1;
    d.Format = fmt;
    d.SampleDesc.Count = 1;
    d.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
    return d;
}

D3D12_RESOURCE_BARRIER transition(ID3D12Resource* res, D3D12_RESOURCE_STATES before,
                                  D3D12_RESOURCE_STATES after) {
    D3D12_RESOURCE_BARRIER b = {};
    b.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    b.Transition.pResource = res;
    b.Transition.StateBefore = before;
    b.Transition.StateAfter = after;
    b.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    return b;
}

void narrow_into(const wchar_t* s, char* out, size_t cap) {
    if (!out || cap == 0) return;
    int n = WideCharToMultiByte(CP_UTF8, 0, s, -1, nullptr, 0, nullptr, nullptr);
    if (n <= 0) { out[0] = '\0'; return; }
    std::vector<char> buf((size_t)n);
    WideCharToMultiByte(CP_UTF8, 0, s, -1, buf.data(), n, nullptr, nullptr);
    std::strncpy(out, buf.data(), cap - 1);
    out[cap - 1] = '\0';
}

// ── G3.2 present:可见窗口消息面(D-130:只搬运 WM_SIZE/关闭两类事实,不暴露输入事件面)──

// 窗口过程搬运的窗口事实(WM_SIZE 新客户区 / 关闭请求);无输入面(D-130 红线)。
struct PresentWndState {
    UINT client_w = 0;
    UINT client_h = 0;
    bool size_changed = false;
    bool close_requested = false;
};

// 可见窗口的窗口过程:仅处理 WM_SIZE(记录新客户区)与关闭(WM_CLOSE/WM_DESTROY),
// 其余一律 DefWindowProcW(**不暴露键鼠/输入事件面**,D-130)。
LRESULT CALLBACK present_wnd_proc(HWND hwnd, UINT msg, WPARAM wparam, LPARAM lparam) {
    auto* st = reinterpret_cast<PresentWndState*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    switch (msg) {
        case WM_SIZE:
            if (st && wparam != SIZE_MINIMIZED) {
                st->client_w = LOWORD(lparam);
                st->client_h = HIWORD(lparam);
                st->size_changed = true;
            }
            return 0;
        case WM_CLOSE:
        case WM_DESTROY:
            if (st) st->close_requested = true;
            return 0;
        default:
            return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
}

// 非阻塞消息泵(PM_REMOVE 排空;只搬运 WM_SIZE/关闭,其余 present_wnd_proc 走 DefWindowProc)。
void pump_present_messages(HWND hwnd) {
    MSG msg = {};
    while (PeekMessageW(&msg, hwnd, 0, 0, PM_REMOVE)) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

}  // namespace

extern "C" {

// 返回 shim ABI 版本(Rust 侧编译期/运行期核对)。
__declspec(dllexport) uint32_t rx_uc04_abi_version(void) { return kAbiVersion; }

// UC-04 offscreen 两 pass deferred draw + readback。
//
// 入参:width/height = offscreen 尺寸;geom_rts0 = 几何 pass RFC-0005 RTS0(IA-only 空资源,可为空
//   root sig);light_rts0 = lighting pass RFC-0005 RTS0(SRV t0 + Sampler s0 descriptor table);
//   geom_vs/geom_fs/light_vs/light_fs = Rurix 图形=B DXIL 容器字节(VS/FS 各 pass);
//   顶点缓冲(全屏三角形 pos + uv=(0.5,0.5) + normal=0.5)由 shim 内置(host 几何数据,非着色器)。
// 出参:out_gbuffer_pixel[4] = G-buffer albedo 中心像素 RGBA8(证几何 pass FS 写 MRT);
//   out_final_pixel[4] = lighting/合成 final 中心像素 RGBA8(证 lighting pass FS 真采样 G-buffer 出图);
//   out_adapter = 选中的硬件 adapter 名(UTF-8)。
// 返回 0 成功;非 0 = HRESULT 位码或哨兵失败码(Rust 侧不伪造 device 绿)。
__declspec(dllexport) int rx_uc04_offscreen_run(
    uint32_t abi_version, uint32_t width, uint32_t height,
    const uint8_t* geom_rts0, size_t geom_rts0_len,
    const uint8_t* light_rts0, size_t light_rts0_len,
    const uint8_t* geom_vs, size_t geom_vs_len,
    const uint8_t* geom_fs, size_t geom_fs_len,
    const uint8_t* light_vs, size_t light_vs_len,
    const uint8_t* light_fs, size_t light_fs_len,
    uint8_t* out_gbuffer_pixel, uint8_t* out_final_pixel,
    char* out_adapter, size_t out_adapter_cap) {
    if (abi_version != kAbiVersion) return -1;
    if (!geom_vs || !geom_fs || !light_vs || !light_fs) return -2;
    if (!light_rts0 || light_rts0_len == 0) return -4;  // lighting 采样须 SRV/Sampler RTS0
    if (width == 0 || height == 0) return -3;

    // 1) 选硬件 adapter + 建 device。
    ComPtr<IDXGIFactory6> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) return -10;
    ComPtr<IDXGIAdapter1> chosen;
    DXGI_ADAPTER_DESC1 chosen_desc = {};
    SIZE_T best_mem = 0;
    for (UINT i = 0;; ++i) {
        ComPtr<IDXGIAdapter1> adapter;
        HRESULT hr = factory->EnumAdapters1(i, &adapter);
        if (hr == DXGI_ERROR_NOT_FOUND) break;
        if (FAILED(hr)) return -11;
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
    if (!chosen) return -12;
    if (out_adapter) narrow_into(chosen_desc.Description, out_adapter, out_adapter_cap);
    ComPtr<ID3D12Device> device;
    if (FAILED(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device))))
        return -13;

    // 2) root signature(每 pass):直接由 Rurix RFC-0005 RTS0 容器字节 CreateRootSignature(P-11
    //    单一事实源,device-parse)。几何 pass = IA-only 空资源;lighting pass = SRV t0 + Sampler s0
    //    descriptor table —— device 真机解析 light_rts0 即证 RFC-0005 推导的采样绑定布局合法。
    ComPtr<ID3D12RootSignature> root;
    if (geom_rts0 && geom_rts0_len > 0) {
        if (FAILED(device->CreateRootSignature(0, geom_rts0, geom_rts0_len, IID_PPV_ARGS(&root))))
            return -14;
    } else {
        // 空资源集:序列化一个空 root signature(等价 infer_root_signature(&[]) 的空布局)。
        D3D12_ROOT_SIGNATURE_DESC rsd = {};
        rsd.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
        ComPtr<ID3DBlob> blob, err;
        if (FAILED(D3D12SerializeRootSignature(&rsd, D3D_ROOT_SIGNATURE_VERSION_1, &blob, &err)))
            return -15;
        if (FAILED(device->CreateRootSignature(0, blob->GetBufferPointer(),
                                               blob->GetBufferSize(), IID_PPV_ARGS(&root))))
            return -16;
    }
    // lighting root signature(SRV t0 表 = root param 0,Sampler s0 表 = root param 1;
    // infer_root_signature 确定性序:SRV/UAV 表 → Sampler 表)。device 真机解析非 no-op。
    ComPtr<ID3D12RootSignature> light_root;
    if (FAILED(device->CreateRootSignature(0, light_rts0, light_rts0_len,
                                           IID_PPV_ARGS(&light_root))))
        return -19;

    ComPtr<ID3D12CommandQueue> queue;
    D3D12_COMMAND_QUEUE_DESC qd = {};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)))) return -17;
    ComPtr<ID3D12CommandAllocator> alloc;
    if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT,
                                              IID_PPV_ARGS(&alloc))))
        return -18;

    // 3) G-buffer MRT(几何 pass 写):albedo R8 / normal R16F / depth R32F + final R8(lighting)。
    const DXGI_FORMAT gbuf_fmt[3] = {DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R16G16B16A16_FLOAT,
                                     DXGI_FORMAT_R32_FLOAT};
    ComPtr<ID3D12Resource> gbuf[3];
    for (int i = 0; i < 3; ++i) {
        D3D12_CLEAR_VALUE cv = {};
        cv.Format = gbuf_fmt[i];
        auto hp = heap_props(D3D12_HEAP_TYPE_DEFAULT);
        auto d = rt_desc(width, height, gbuf_fmt[i]);
        if (FAILED(device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &d,
                                                   D3D12_RESOURCE_STATE_RENDER_TARGET, &cv,
                                                   IID_PPV_ARGS(&gbuf[i]))))
            return -20 - i;
    }
    ComPtr<ID3D12Resource> final_rt;
    {
        D3D12_CLEAR_VALUE cv = {};
        cv.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        cv.Color[3] = 1.0f;
        auto hp = heap_props(D3D12_HEAP_TYPE_DEFAULT);
        auto d = rt_desc(width, height, DXGI_FORMAT_R8G8B8A8_UNORM);
        if (FAILED(device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &d,
                                                   D3D12_RESOURCE_STATE_RENDER_TARGET, &cv,
                                                   IID_PPV_ARGS(&final_rt))))
            return -24;
    }

    // RTV heap:3 gbuffer + 1 final = 4 descriptor。
    D3D12_DESCRIPTOR_HEAP_DESC rtv_hd = {};
    rtv_hd.NumDescriptors = 4;
    rtv_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    ComPtr<ID3D12DescriptorHeap> rtv_heap;
    if (FAILED(device->CreateDescriptorHeap(&rtv_hd, IID_PPV_ARGS(&rtv_heap)))) return -25;
    const UINT rtv_stride = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
    D3D12_CPU_DESCRIPTOR_HANDLE rtv_base = rtv_heap->GetCPUDescriptorHandleForHeapStart();
    D3D12_CPU_DESCRIPTOR_HANDLE gbuf_rtv[3];
    for (int i = 0; i < 3; ++i) {
        gbuf_rtv[i] = rtv_base;
        gbuf_rtv[i].ptr += (SIZE_T)rtv_stride * i;
        device->CreateRenderTargetView(gbuf[i].Get(), nullptr, gbuf_rtv[i]);
    }
    D3D12_CPU_DESCRIPTOR_HANDLE final_rtv = rtv_base;
    final_rtv.ptr += (SIZE_T)rtv_stride * 3;
    device->CreateRenderTargetView(final_rt.Get(), nullptr, final_rtv);

    // SRV heap(shader-visible CBV_SRV_UAV,1× SRV for albedo G-buffer)+ Sampler heap
    // (shader-visible,1× 默认 sampler)。lighting pass 经 descriptor table 真采样(RXS-0176)。
    D3D12_DESCRIPTOR_HEAP_DESC srv_hd = {};
    srv_hd.NumDescriptors = 1;
    srv_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
    srv_hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> srv_heap;
    if (FAILED(device->CreateDescriptorHeap(&srv_hd, IID_PPV_ARGS(&srv_heap)))) return -41;
    // albedo SRV(匹配 gbuf[0] RGBA8 格式;TEXTURE2D,1 mip)。
    D3D12_SHADER_RESOURCE_VIEW_DESC srvd = {};
    srvd.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    srvd.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
    srvd.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
    srvd.Texture2D.MipLevels = 1;
    device->CreateShaderResourceView(gbuf[0].Get(), &srvd,
                                     srv_heap->GetCPUDescriptorHandleForHeapStart());

    D3D12_DESCRIPTOR_HEAP_DESC samp_hd = {};
    samp_hd.NumDescriptors = 1;
    samp_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER;
    samp_hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
    ComPtr<ID3D12DescriptorHeap> samp_heap;
    if (FAILED(device->CreateDescriptorHeap(&samp_hd, IID_PPV_ARGS(&samp_heap)))) return -42;
    // 默认 sampler(RXS-0176 DS4 首期:min/mag/mip 线性 + UVW clamp-to-edge)。
    D3D12_SAMPLER_DESC sd = {};
    sd.Filter = D3D12_FILTER_MIN_MAG_MIP_LINEAR;
    sd.AddressU = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
    sd.AddressV = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
    sd.AddressW = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
    sd.MaxLOD = D3D12_FLOAT32_MAX;
    device->CreateSampler(&sd, samp_heap->GetCPUDescriptorHandleForHeapStart());

    // 4) 顶点缓冲(全屏三角形,host 几何数据):每顶点 {pos vec4, uv vec2, normal f32} = 28B。
    //    uv=(0.5,0.5)、normal=0.5 → 插值常量;覆盖中心像素。layout 语义名 pos/uv/normal 匹配 VS
    //    输入签名(uv 为 vec2<f32>,承 RFC-0007 采样坐标;albedo 均匀 → 采样值与精确 uv 无关)。
    struct Vtx { float pos[4]; float uv[2]; float normal; };
    const Vtx verts[3] = {
        {{-1.0f, -1.0f, 0.0f, 1.0f}, {0.5f, 0.5f}, 0.5f},
        {{-1.0f, 3.0f, 0.0f, 1.0f}, {0.5f, 0.5f}, 0.5f},
        {{3.0f, -1.0f, 0.0f, 1.0f}, {0.5f, 0.5f}, 0.5f},
    };
    ComPtr<ID3D12Resource> vb;
    {
        auto hp = heap_props(D3D12_HEAP_TYPE_UPLOAD);
        auto d = buffer_desc(sizeof(verts));
        if (FAILED(device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &d,
                                                   D3D12_RESOURCE_STATE_GENERIC_READ, nullptr,
                                                   IID_PPV_ARGS(&vb))))
            return -26;
        uint8_t* p = nullptr;
        D3D12_RANGE none = {0, 0};
        if (FAILED(vb->Map(0, &none, reinterpret_cast<void**>(&p)))) return -27;
        std::memcpy(p, verts, sizeof(verts));
        vb->Unmap(0, nullptr);
    }
    D3D12_VERTEX_BUFFER_VIEW vbv = {};
    vbv.BufferLocation = vb->GetGPUVirtualAddress();
    vbv.StrideInBytes = sizeof(Vtx);
    vbv.SizeInBytes = sizeof(verts);

    // input layout(IA):语义名匹配 VS 输入签名(pos/uv/normal,RXS-0159 IR1(a) 保名)。
    // uv = R32G32_FLOAT(vec2<f32>,offset 16);normal = R32_FLOAT(offset 24)。
    D3D12_INPUT_ELEMENT_DESC il[3] = {
        {"pos", 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 0, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
        {"uv", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 16, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
        {"normal", 0, DXGI_FORMAT_R32_FLOAT, 0, 24, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
    };

    auto make_pso = [&](ID3D12RootSignature* rs, const uint8_t* vs, size_t vs_len,
                        const uint8_t* fs, size_t fs_len, UINT num_rt, const DXGI_FORMAT* fmts,
                        ComPtr<ID3D12PipelineState>& out) -> HRESULT {
        D3D12_GRAPHICS_PIPELINE_STATE_DESC pd = {};
        pd.pRootSignature = rs;
        pd.VS = {vs, vs_len};
        pd.PS = {fs, fs_len};
        pd.InputLayout = {il, 3};
        pd.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
        pd.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
        pd.RasterizerState.DepthClipEnable = TRUE;
        for (UINT i = 0; i < num_rt; ++i) {
            pd.BlendState.RenderTarget[i].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
        }
        pd.SampleMask = UINT_MAX;
        pd.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
        pd.NumRenderTargets = num_rt;
        for (UINT i = 0; i < num_rt; ++i) pd.RTVFormats[i] = fmts[i];
        pd.SampleDesc.Count = 1;
        return device->CreateGraphicsPipelineState(&pd, IID_PPV_ARGS(&out));
    };

    ComPtr<ID3D12PipelineState> pso_geom, pso_light;
    if (FAILED(make_pso(root.Get(), geom_vs, geom_vs_len, geom_fs, geom_fs_len, 3, gbuf_fmt,
                        pso_geom)))
        return -30;
    const DXGI_FORMAT final_fmt[1] = {DXGI_FORMAT_R8G8B8A8_UNORM};
    if (FAILED(make_pso(light_root.Get(), light_vs, light_vs_len, light_fs, light_fs_len, 1,
                        final_fmt, pso_light)))
        return -31;

    // readback 缓冲(albedo + final 各一)。
    auto make_readback = [&](ID3D12Resource* res, ComPtr<ID3D12Resource>& rb,
                             D3D12_PLACED_SUBRESOURCE_FOOTPRINT& fp, UINT64& total) -> HRESULT {
        auto d = res->GetDesc();
        UINT rows = 0;
        UINT64 row_size = 0;
        device->GetCopyableFootprints(&d, 0, 1, 0, &fp, &rows, &row_size, &total);
        auto hp = heap_props(D3D12_HEAP_TYPE_READBACK);
        auto bd = buffer_desc(total);
        return device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &bd,
                                               D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                               IID_PPV_ARGS(&rb));
    };
    ComPtr<ID3D12Resource> rb_gbuf, rb_final;
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT fp_gbuf = {}, fp_final = {};
    UINT64 total_gbuf = 0, total_final = 0;
    if (FAILED(make_readback(gbuf[0].Get(), rb_gbuf, fp_gbuf, total_gbuf))) return -32;
    if (FAILED(make_readback(final_rt.Get(), rb_final, fp_final, total_final))) return -33;

    // 5) 命令录制:pass1 几何写 MRT → pass2 lighting 写 final → barrier → copy readback。
    ComPtr<ID3D12GraphicsCommandList> cmd;
    if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(),
                                         pso_geom.Get(), IID_PPV_ARGS(&cmd))))
        return -34;
    D3D12_VIEWPORT vp = {0, 0, (float)width, (float)height, 0, 1};
    D3D12_RECT scr = {0, 0, (LONG)width, (LONG)height};
    const float clear0[4] = {0, 0, 0, 0};
    const float clear_final[4] = {0, 0, 0, 1};

    // pass 1:几何 pass 写 G-buffer MRT(3 渲染目标)。
    cmd->SetGraphicsRootSignature(root.Get());
    cmd->RSSetViewports(1, &vp);
    cmd->RSSetScissorRects(1, &scr);
    cmd->OMSetRenderTargets(3, gbuf_rtv, FALSE, nullptr);
    for (int i = 0; i < 3; ++i) cmd->ClearRenderTargetView(gbuf_rtv[i], clear0, 0, nullptr);
    cmd->SetPipelineState(pso_geom.Get());
    cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    cmd->IASetVertexBuffers(0, 1, &vbv);
    cmd->DrawInstanced(3, 1, 0, 0);

    // RXS-0176 IR1 编排锚点(手动 barrier):albedo G-buffer **RENDER_TARGET → PIXEL_SHADER_RESOURCE**
    // —— lighting pass 真采样前的写后读可见性(RXS-0176 DS6)。其余 gbuffer 目标维持 RENDER_TARGET。
    D3D12_RESOURCE_BARRIER after_geom =
        transition(gbuf[0].Get(), D3D12_RESOURCE_STATE_RENDER_TARGET,
                   D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE);
    cmd->ResourceBarrier(1, &after_geom);

    // pass 2:lighting/合成 pass **真采样 G-buffer albedo SRV(t0)经 sampler(s0)** 写 final
    // (RXS-0175/0176;light_root 的 SRV 表 = root param 0、Sampler 表 = root param 1)。
    cmd->OMSetRenderTargets(1, &final_rtv, FALSE, nullptr);
    cmd->ClearRenderTargetView(final_rtv, clear_final, 0, nullptr);
    cmd->SetGraphicsRootSignature(light_root.Get());
    ID3D12DescriptorHeap* heaps[] = {srv_heap.Get(), samp_heap.Get()};
    cmd->SetDescriptorHeaps(2, heaps);
    cmd->SetGraphicsRootDescriptorTable(0, srv_heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetGraphicsRootDescriptorTable(1, samp_heap->GetGPUDescriptorHandleForHeapStart());
    cmd->SetPipelineState(pso_light.Get());
    cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    cmd->IASetVertexBuffers(0, 1, &vbv);
    cmd->DrawInstanced(3, 1, 0, 0);

    // lighting 后:final → COPY_SOURCE;albedo PIXEL_SHADER_RESOURCE → COPY_SOURCE(gbuffer 见证回读)。
    D3D12_RESOURCE_BARRIER after_light[2] = {
        transition(final_rt.Get(), D3D12_RESOURCE_STATE_RENDER_TARGET,
                   D3D12_RESOURCE_STATE_COPY_SOURCE),
        transition(gbuf[0].Get(), D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                   D3D12_RESOURCE_STATE_COPY_SOURCE),
    };
    cmd->ResourceBarrier(2, after_light);

    // copy albedo + final → readback。
    auto copy_to_readback = [&](ID3D12Resource* src, ID3D12Resource* rb,
                                const D3D12_PLACED_SUBRESOURCE_FOOTPRINT& fp) {
        D3D12_TEXTURE_COPY_LOCATION s = {};
        s.pResource = src;
        s.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        s.SubresourceIndex = 0;
        D3D12_TEXTURE_COPY_LOCATION d = {};
        d.pResource = rb;
        d.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
        d.PlacedFootprint = fp;
        cmd->CopyTextureRegion(&d, 0, 0, 0, &s, nullptr);
    };
    copy_to_readback(gbuf[0].Get(), rb_gbuf.Get(), fp_gbuf);
    copy_to_readback(final_rt.Get(), rb_final.Get(), fp_final);
    if (FAILED(cmd->Close())) return -35;

    // 6) 提交 + fence 同步。
    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);
    ComPtr<ID3D12Fence> fence;
    if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) return -36;
    HANDLE ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!ev) return -37;
    if (FAILED(queue->Signal(fence.Get(), 1))) { CloseHandle(ev); return -38; }
    if (fence->GetCompletedValue() < 1) {
        fence->SetEventOnCompletion(1, ev);
        WaitForSingleObject(ev, INFINITE);
    }
    CloseHandle(ev);

    // 7) 读回中心像素。albedo R8G8B8A8(几何 FS 写常量 0.75 → R≈191);final R8G8B8A8(lighting FS
    //    **真采样** albedo SRV → R≈采样到的 albedo ≈191,即 final.R 追踪 gbuffer.R)。中心 (w/2, h/2)。
    auto read_center = [&](ID3D12Resource* rb, const D3D12_PLACED_SUBRESOURCE_FOOTPRINT& fp,
                           UINT64 total, uint8_t* out4) -> bool {
        uint8_t* mapped = nullptr;
        D3D12_RANGE range = {0, (SIZE_T)total};
        if (FAILED(rb->Map(0, &range, reinterpret_cast<void**>(&mapped)))) return false;
        const UINT x = width / 2, y = height / 2;
        const uint8_t* px = mapped + fp.Offset + (SIZE_T)y * fp.Footprint.RowPitch + (SIZE_T)x * 4;
        if (out4) { out4[0] = px[0]; out4[1] = px[1]; out4[2] = px[2]; out4[3] = px[3]; }
        rb->Unmap(0, nullptr);
        return true;
    };
    if (!read_center(rb_gbuf.Get(), fp_gbuf, total_gbuf, out_gbuffer_pixel)) return -39;
    if (!read_center(rb_final.Get(), fp_final, total_final, out_final_pixel)) return -40;
    return 0;
}

// ── G3.2 present 入口(加性独立入口,ABI v3;RFC-0013 §4.A;RXS-0220~0222)──

// 返回 present 入口 ABI 版本(与 Rust RX_UC04_PRESENT_ABI_VERSION 一致;与 offscreen 正交)。
__declspec(dllexport) uint32_t rx_uc04_present_abi_version(void) { return kPresentAbiVersion; }

// UC-04 可见窗口 flip-model swapchain present:每帧复用 deferred 编排(几何 pass 写 G-buffer
// MRT → lighting pass 采样 albedo SRV 写 **swapchain backbuffer**)→ backbuffer
// RENDER_TARGET→COPY_SOURCE(readback)→PRESENT → Present(sync_interval)。可见窗口
// WS_OVERLAPPEDWINDOW + ShowWindow;WM_SIZE(经 SetWindowPos 合成)→ ResizeBuffers 重建
// backbuffer RTV + readback + lighting viewport;三点 backbuffer 中心像素回读(首帧 / 重建后
// 首帧 / 末帧,RXS-0222)。消息泵只搬运 WM_SIZE/关闭(D-130,不暴露输入面)。
//
// 入参:width/height=初始客户区;buffer_count∈{2,3};frames=呈现帧数;sync_interval∈{0,1};
//   allow_tearing(bool,须 sync_interval=0 + CheckFeatureSupport 通过方生效);resize_frame=
//   注入 resize 的帧序(1-based,0=不 resize);resize_width/height=重建后客户区;geom/light
//   RTS0 + 4 DXIL(与 offscreen 同,Rurix 图形=B)。出参:out_first/rebuilt/last_pixel[4]=三点
//   backbuffer 中心像素 RGBA8;out_frames_presented=Present 逐帧 S_OK 计数;out_adapter=adapter 名。
// 返回 0 成功;非 0 = HRESULT 位码或哨兵失败码(Rust 侧不伪造 device 绿)。
__declspec(dllexport) int rx_uc04_present_run(
    uint32_t abi_version, uint32_t width, uint32_t height,
    uint32_t buffer_count, uint32_t frames, uint32_t sync_interval, uint32_t allow_tearing,
    uint32_t resize_frame, uint32_t resize_width, uint32_t resize_height,
    const uint8_t* geom_rts0, size_t geom_rts0_len,
    const uint8_t* light_rts0, size_t light_rts0_len,
    const uint8_t* geom_vs, size_t geom_vs_len,
    const uint8_t* geom_fs, size_t geom_fs_len,
    const uint8_t* light_vs, size_t light_vs_len,
    const uint8_t* light_fs, size_t light_fs_len,
    uint8_t* out_first_pixel, uint8_t* out_rebuilt_pixel, uint8_t* out_last_pixel,
    uint32_t* out_frames_presented, char* out_adapter, size_t out_adapter_cap) {
    if (abi_version != kPresentAbiVersion) return -1;
    if (!geom_vs || !geom_fs || !light_vs || !light_fs) return -2;
    if (!light_rts0 || light_rts0_len == 0) return -4;
    if (width == 0 || height == 0) return -3;
    if (buffer_count < 2 || buffer_count > 3) return -5;   // flip-model BufferCount ∈ {2,3}
    if (sync_interval > 1) return -6;                       // sync_interval ∈ {0,1}
    if (frames == 0) frames = 1;
    if (allow_tearing && sync_interval != 0) return -7;     // tearing 须与 sync_interval=0 成对
    if (out_frames_presented) *out_frames_presented = 0;

    const DXGI_FORMAT bb_fmt = DXGI_FORMAT_R8G8B8A8_UNORM;

    // 1) 硬件 adapter + device(与 offscreen 同选取序:非软件、最大显存)。
    ComPtr<IDXGIFactory6> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) return -10;
    ComPtr<IDXGIAdapter1> chosen;
    DXGI_ADAPTER_DESC1 chosen_desc = {};
    SIZE_T best_mem = 0;
    for (UINT i = 0;; ++i) {
        ComPtr<IDXGIAdapter1> adapter;
        HRESULT hr = factory->EnumAdapters1(i, &adapter);
        if (hr == DXGI_ERROR_NOT_FOUND) break;
        if (FAILED(hr)) return -11;
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
    if (!chosen) return -12;
    if (out_adapter) narrow_into(chosen_desc.Description, out_adapter, out_adapter_cap);
    ComPtr<ID3D12Device> device;
    if (FAILED(D3D12CreateDevice(chosen.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device))))
        return -13;

    // tearing 能力探测(Q-P-TearingFail:能力缺失 = 确定性运行期拒,不静默降级为 vsync)。
    UINT tearing_flag = 0;
    if (allow_tearing) {
        BOOL supported = FALSE;
        if (SUCCEEDED(factory->CheckFeatureSupport(DXGI_FEATURE_PRESENT_ALLOW_TEARING,
                                                   &supported, sizeof(supported))) &&
            supported) {
            tearing_flag = DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING;
        } else {
            return -8;  // 请求 tearing 但能力缺失 → 确定性拒(不占 RX 码,环境失败)
        }
    }

    // 2) 每 pass root signature(P-11:直接 CreateRootSignature RFC-0005 RTS0 字节)。
    ComPtr<ID3D12RootSignature> root;
    if (geom_rts0 && geom_rts0_len > 0) {
        if (FAILED(device->CreateRootSignature(0, geom_rts0, geom_rts0_len, IID_PPV_ARGS(&root))))
            return -14;
    } else {
        D3D12_ROOT_SIGNATURE_DESC rsd = {};
        rsd.Flags = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT;
        ComPtr<ID3DBlob> blob, err;
        if (FAILED(D3D12SerializeRootSignature(&rsd, D3D_ROOT_SIGNATURE_VERSION_1, &blob, &err)))
            return -15;
        if (FAILED(device->CreateRootSignature(0, blob->GetBufferPointer(),
                                               blob->GetBufferSize(), IID_PPV_ARGS(&root))))
            return -16;
    }
    ComPtr<ID3D12RootSignature> light_root;
    if (FAILED(device->CreateRootSignature(0, light_rts0, light_rts0_len,
                                           IID_PPV_ARGS(&light_root))))
        return -19;

    // 3) command queue(flip-model swapchain 经 queue 呈现)。
    ComPtr<ID3D12CommandQueue> queue;
    D3D12_COMMAND_QUEUE_DESC qd = {};
    qd.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    if (FAILED(device->CreateCommandQueue(&qd, IID_PPV_ARGS(&queue)))) return -17;
    ComPtr<ID3D12CommandAllocator> alloc;
    if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT,
                                              IID_PPV_ARGS(&alloc))))
        return -18;

    // 4) 可见 win32 窗口(WS_OVERLAPPEDWINDOW + ShowWindow;与 vk.rs 隐藏窗形态区分)。
    HINSTANCE hinst = GetModuleHandleW(nullptr);
    wchar_t cls_name[64];
    swprintf(cls_name, 64, L"RurixUc04Present_%lu", (unsigned long)GetCurrentProcessId());
    WNDCLASSEXW wc = {};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = present_wnd_proc;
    wc.hInstance = hinst;
    wc.lpszClassName = cls_name;
    if (RegisterClassExW(&wc) == 0) return -50;
    PresentWndState wnd_state = {};
    RECT wr = {0, 0, (LONG)width, (LONG)height};
    AdjustWindowRect(&wr, WS_OVERLAPPEDWINDOW, FALSE);
    HWND hwnd = CreateWindowExW(0, cls_name, L"rurix-uc04-present", WS_OVERLAPPEDWINDOW,
                                CW_USEDEFAULT, CW_USEDEFAULT, wr.right - wr.left, wr.bottom - wr.top,
                                nullptr, nullptr, hinst, nullptr);
    if (!hwnd) { UnregisterClassW(cls_name, hinst); return -51; }
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(&wnd_state));
    ShowWindow(hwnd, SW_SHOW);
    pump_present_messages(hwnd);

    // present 主体(lambda 隔离 ComPtr 生命周期:返回前全部 ComPtr 释放、后拆窗;避免 goto 跨初始化)。
    auto run_present = [&]() -> int {
        // 5) flip-model swapchain(FLIP_DISCARD;imageUsage RENDER_TARGET_OUTPUT;可回读 backbuffer)。
        ComPtr<IDXGISwapChain1> swapchain1;
        DXGI_SWAP_CHAIN_DESC1 scd = {};
        scd.Width = width;
        scd.Height = height;
        scd.Format = bb_fmt;
        scd.Stereo = FALSE;
        scd.SampleDesc.Count = 1;
        scd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
        scd.BufferCount = buffer_count;
        scd.Scaling = DXGI_SCALING_STRETCH;
        scd.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;  // flip-model 恒定(blt-model 不进本面)
        scd.AlphaMode = DXGI_ALPHA_MODE_UNSPECIFIED;
        scd.Flags = tearing_flag;
        if (FAILED(factory->CreateSwapChainForHwnd(queue.Get(), hwnd, &scd, nullptr, nullptr,
                                                   &swapchain1)))
            return -52;
        // 禁用 DXGI Alt+Enter 全屏切换(present 面不进全屏,D-130 只泵 WM_SIZE)。
        factory->MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER);
        ComPtr<IDXGISwapChain3> swapchain;
        if (FAILED(swapchain1.As(&swapchain))) return -53;

        // ── 静态资源(尺寸无关 / G-buffer 固定初始尺寸)──
        // G-buffer MRT(几何 pass 写;固定 width×height)+ lighting 采样 albedo SRV。
        const DXGI_FORMAT gbuf_fmt[3] = {DXGI_FORMAT_R8G8B8A8_UNORM,
                                         DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_R32_FLOAT};
        ComPtr<ID3D12Resource> gbuf[3];
        for (int i = 0; i < 3; ++i) {
            D3D12_CLEAR_VALUE cv = {};
            cv.Format = gbuf_fmt[i];
            auto hp = heap_props(D3D12_HEAP_TYPE_DEFAULT);
            auto d = rt_desc(width, height, gbuf_fmt[i]);
            if (FAILED(device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &d,
                                                       D3D12_RESOURCE_STATE_RENDER_TARGET, &cv,
                                                       IID_PPV_ARGS(&gbuf[i])))) {
                return -54;
            }
        }
        // RTV heap:3 gbuffer + buffer_count backbuffer。
        D3D12_DESCRIPTOR_HEAP_DESC rtv_hd = {};
        rtv_hd.NumDescriptors = 3 + buffer_count;
        rtv_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
        ComPtr<ID3D12DescriptorHeap> rtv_heap;
        if (FAILED(device->CreateDescriptorHeap(&rtv_hd, IID_PPV_ARGS(&rtv_heap)))) {
            return -55;
        }
        const UINT rtv_stride =
            device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
        D3D12_CPU_DESCRIPTOR_HANDLE rtv_base = rtv_heap->GetCPUDescriptorHandleForHeapStart();
        D3D12_CPU_DESCRIPTOR_HANDLE gbuf_rtv[3];
        for (int i = 0; i < 3; ++i) {
            gbuf_rtv[i] = rtv_base;
            gbuf_rtv[i].ptr += (SIZE_T)rtv_stride * i;
            device->CreateRenderTargetView(gbuf[i].Get(), nullptr, gbuf_rtv[i]);
        }
        // backbuffer RTV(在 3 gbuffer 之后;resize 后重建)。
        auto backbuffer_rtv = [&](UINT idx) -> D3D12_CPU_DESCRIPTOR_HANDLE {
            D3D12_CPU_DESCRIPTOR_HANDLE h = rtv_base;
            h.ptr += (SIZE_T)rtv_stride * (3 + idx);
            return h;
        };

        // SRV heap(albedo)+ Sampler heap(lighting descriptor table,RXS-0176 真采样)。
        D3D12_DESCRIPTOR_HEAP_DESC srv_hd = {};
        srv_hd.NumDescriptors = 1;
        srv_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
        srv_hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
        ComPtr<ID3D12DescriptorHeap> srv_heap;
        if (FAILED(device->CreateDescriptorHeap(&srv_hd, IID_PPV_ARGS(&srv_heap)))) {
            return -56;
        }
        D3D12_SHADER_RESOURCE_VIEW_DESC srvd = {};
        srvd.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
        srvd.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
        srvd.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
        srvd.Texture2D.MipLevels = 1;
        device->CreateShaderResourceView(gbuf[0].Get(), &srvd,
                                         srv_heap->GetCPUDescriptorHandleForHeapStart());
        D3D12_DESCRIPTOR_HEAP_DESC samp_hd = {};
        samp_hd.NumDescriptors = 1;
        samp_hd.Type = D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER;
        samp_hd.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;
        ComPtr<ID3D12DescriptorHeap> samp_heap;
        if (FAILED(device->CreateDescriptorHeap(&samp_hd, IID_PPV_ARGS(&samp_heap)))) {
            return -57;
        }
        D3D12_SAMPLER_DESC sd = {};
        sd.Filter = D3D12_FILTER_MIN_MAG_MIP_LINEAR;
        sd.AddressU = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
        sd.AddressV = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
        sd.AddressW = D3D12_TEXTURE_ADDRESS_MODE_CLAMP;
        sd.MaxLOD = D3D12_FLOAT32_MAX;
        device->CreateSampler(&sd, samp_heap->GetCPUDescriptorHandleForHeapStart());

        // 顶点缓冲(全屏三角形,host 几何,与 offscreen 同布局)。
        struct Vtx { float pos[4]; float uv[2]; float normal; };
        const Vtx verts[3] = {
            {{-1.0f, -1.0f, 0.0f, 1.0f}, {0.5f, 0.5f}, 0.5f},
            {{-1.0f, 3.0f, 0.0f, 1.0f}, {0.5f, 0.5f}, 0.5f},
            {{3.0f, -1.0f, 0.0f, 1.0f}, {0.5f, 0.5f}, 0.5f},
        };
        ComPtr<ID3D12Resource> vb;
        {
            auto hp = heap_props(D3D12_HEAP_TYPE_UPLOAD);
            auto d = buffer_desc(sizeof(verts));
            if (FAILED(device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &d,
                                                       D3D12_RESOURCE_STATE_GENERIC_READ, nullptr,
                                                       IID_PPV_ARGS(&vb)))) {
                return -58;
            }
            uint8_t* p = nullptr;
            D3D12_RANGE none = {0, 0};
            if (FAILED(vb->Map(0, &none, reinterpret_cast<void**>(&p)))) { return -59; }
            std::memcpy(p, verts, sizeof(verts));
            vb->Unmap(0, nullptr);
        }
        D3D12_VERTEX_BUFFER_VIEW vbv = {};
        vbv.BufferLocation = vb->GetGPUVirtualAddress();
        vbv.StrideInBytes = sizeof(Vtx);
        vbv.SizeInBytes = sizeof(verts);
        D3D12_INPUT_ELEMENT_DESC il[3] = {
            {"pos", 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 0, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
            {"uv", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 16, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
            {"normal", 0, DXGI_FORMAT_R32_FLOAT, 0, 24, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
        };

        auto make_pso = [&](ID3D12RootSignature* rs, const uint8_t* vs, size_t vs_len,
                            const uint8_t* fs, size_t fs_len, UINT num_rt, const DXGI_FORMAT* fmts,
                            ComPtr<ID3D12PipelineState>& out) -> HRESULT {
            D3D12_GRAPHICS_PIPELINE_STATE_DESC pd = {};
            pd.pRootSignature = rs;
            pd.VS = {vs, vs_len};
            pd.PS = {fs, fs_len};
            pd.InputLayout = {il, 3};
            pd.RasterizerState.FillMode = D3D12_FILL_MODE_SOLID;
            pd.RasterizerState.CullMode = D3D12_CULL_MODE_NONE;
            pd.RasterizerState.DepthClipEnable = TRUE;
            for (UINT i = 0; i < num_rt; ++i)
                pd.BlendState.RenderTarget[i].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
            pd.SampleMask = UINT_MAX;
            pd.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
            pd.NumRenderTargets = num_rt;
            for (UINT i = 0; i < num_rt; ++i) pd.RTVFormats[i] = fmts[i];
            pd.SampleDesc.Count = 1;
            return device->CreateGraphicsPipelineState(&pd, IID_PPV_ARGS(&out));
        };
        ComPtr<ID3D12PipelineState> pso_geom, pso_light;
        if (FAILED(make_pso(root.Get(), geom_vs, geom_vs_len, geom_fs, geom_fs_len, 3, gbuf_fmt,
                            pso_geom))) {
            return -60;
        }
        const DXGI_FORMAT bb_fmt_arr[1] = {bb_fmt};
        if (FAILED(make_pso(light_root.Get(), light_vs, light_vs_len, light_fs, light_fs_len, 1,
                            bb_fmt_arr, pso_light))) {
            return -61;
        }

        // fence。
        ComPtr<ID3D12Fence> fence;
        if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) {
            return -62;
        }
        HANDLE fence_ev = CreateEventW(nullptr, FALSE, FALSE, nullptr);
        if (!fence_ev) { return -63; }
        UINT64 fence_val = 0;
        auto wait_gpu = [&]() {
            ++fence_val;
            queue->Signal(fence.Get(), fence_val);
            if (fence->GetCompletedValue() < fence_val) {
                fence->SetEventOnCompletion(fence_val, fence_ev);
                WaitForSingleObject(fence_ev, INFINITE);
            }
        };

        // ── 尺寸相关资源(backbuffer RTV / readback / viewport);resize 后重建 ──
        UINT cur_w = width, cur_h = height;
        std::vector<ComPtr<ID3D12Resource>> backbuffers(buffer_count);
        ComPtr<ID3D12Resource> rb_back;   // backbuffer readback
        D3D12_PLACED_SUBRESOURCE_FOOTPRINT fp_back = {};
        UINT64 total_back = 0;
        HRESULT hr_rebuild = S_OK;
        auto rebuild_size_deps = [&]() -> HRESULT {
            // backbuffer RTV。
            for (UINT i = 0; i < buffer_count; ++i) {
                if (FAILED(swapchain->GetBuffer(i, IID_PPV_ARGS(&backbuffers[i]))))
                    return E_FAIL;
                device->CreateRenderTargetView(backbuffers[i].Get(), nullptr, backbuffer_rtv(i));
            }
            // backbuffer readback footprint(取 backbuffer[0] desc)。
            auto bd = backbuffers[0]->GetDesc();
            UINT rows = 0;
            UINT64 row_size = 0;
            device->GetCopyableFootprints(&bd, 0, 1, 0, &fp_back, &rows, &row_size, &total_back);
            auto hp = heap_props(D3D12_HEAP_TYPE_READBACK);
            auto rbd = buffer_desc(total_back);
            rb_back.Reset();
            return device->CreateCommittedResource(&hp, D3D12_HEAP_FLAG_NONE, &rbd,
                                                   D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                                   IID_PPV_ARGS(&rb_back));
        };
        hr_rebuild = rebuild_size_deps();
        if (FAILED(hr_rebuild)) { CloseHandle(fence_ev); return -64; }

        // command list(初始 geom PSO)。
        ComPtr<ID3D12GraphicsCommandList> cmd;
        if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, alloc.Get(),
                                             pso_geom.Get(), IID_PPV_ARGS(&cmd)))) {
            CloseHandle(fence_ev); return -65;
        }
        cmd->Close();  // 首帧循环开头 Reset。

        const float clear0[4] = {0, 0, 0, 0};
        const float clear_bb[4] = {0, 0, 0, 1};
        uint32_t presented = 0;

        auto read_center = [&](uint8_t* out4) -> bool {
            uint8_t* mapped = nullptr;
            D3D12_RANGE range = {0, (SIZE_T)total_back};
            if (FAILED(rb_back->Map(0, &range, reinterpret_cast<void**>(&mapped)))) return false;
            const UINT x = cur_w / 2, y = cur_h / 2;
            const uint8_t* px =
                mapped + fp_back.Offset + (SIZE_T)y * fp_back.Footprint.RowPitch + (SIZE_T)x * 4;
            if (out4) { out4[0] = px[0]; out4[1] = px[1]; out4[2] = px[2]; out4[3] = px[3]; }
            rb_back->Unmap(0, nullptr);
            return true;
        };

        // ── 呈现循环 ──
        for (uint32_t frame = 1; frame <= frames; ++frame) {
            // resize 注入(RXS-0221:WM_SIZE 经 SetWindowPos 合成 → ResizeBuffers 重建)。
            bool rebuilt_this_frame = false;
            if (resize_frame != 0 && frame == resize_frame && resize_width > 0 && resize_height > 0) {
                wait_gpu();  // idle → 释放尺寸依赖引用 → 重建。
                for (UINT i = 0; i < buffer_count; ++i) backbuffers[i].Reset();
                RECT rr = {0, 0, (LONG)resize_width, (LONG)resize_height};
                AdjustWindowRect(&rr, WS_OVERLAPPEDWINDOW, FALSE);
                SetWindowPos(hwnd, nullptr, 0, 0, rr.right - rr.left, rr.bottom - rr.top,
                             SWP_NOMOVE | SWP_NOZORDER);
                pump_present_messages(hwnd);
                if (FAILED(swapchain->ResizeBuffers(buffer_count, resize_width, resize_height,
                                                    DXGI_FORMAT_UNKNOWN, tearing_flag))) {
                    CloseHandle(fence_ev); return -66;
                }
                cur_w = resize_width;
                cur_h = resize_height;
                if (FAILED(rebuild_size_deps())) { CloseHandle(fence_ev); return -67; }
                rebuilt_this_frame = true;
            }

            UINT idx = swapchain->GetCurrentBackBufferIndex();
            alloc->Reset();
            cmd->Reset(alloc.Get(), pso_geom.Get());

            D3D12_VIEWPORT vp_g = {0, 0, (float)width, (float)height, 0, 1};
            D3D12_RECT sc_g = {0, 0, (LONG)width, (LONG)height};
            D3D12_VIEWPORT vp_bb = {0, 0, (float)cur_w, (float)cur_h, 0, 1};
            D3D12_RECT sc_bb = {0, 0, (LONG)cur_w, (LONG)cur_h};

            // pass 1:几何 pass 写 G-buffer MRT(3 渲染目标,固定 width×height)。
            cmd->SetGraphicsRootSignature(root.Get());
            cmd->RSSetViewports(1, &vp_g);
            cmd->RSSetScissorRects(1, &sc_g);
            cmd->OMSetRenderTargets(3, gbuf_rtv, FALSE, nullptr);
            for (int i = 0; i < 3; ++i) cmd->ClearRenderTargetView(gbuf_rtv[i], clear0, 0, nullptr);
            cmd->SetPipelineState(pso_geom.Get());
            cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            cmd->IASetVertexBuffers(0, 1, &vbv);
            cmd->DrawInstanced(3, 1, 0, 0);

            // barrier:albedo RENDER_TARGET → PIXEL_SHADER_RESOURCE(lighting 采样前);
            //          backbuffer PRESENT → RENDER_TARGET(呈现终态 → draw 目标)。
            D3D12_RESOURCE_BARRIER pre_light[2] = {
                transition(gbuf[0].Get(), D3D12_RESOURCE_STATE_RENDER_TARGET,
                           D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE),
                transition(backbuffers[idx].Get(), D3D12_RESOURCE_STATE_PRESENT,
                           D3D12_RESOURCE_STATE_RENDER_TARGET),
            };
            cmd->ResourceBarrier(2, pre_light);

            // pass 2:lighting/合成 pass **真采样 albedo SRV** 写 swapchain backbuffer RTV。
            D3D12_CPU_DESCRIPTOR_HANDLE bb_rtv = backbuffer_rtv(idx);
            cmd->OMSetRenderTargets(1, &bb_rtv, FALSE, nullptr);
            cmd->RSSetViewports(1, &vp_bb);
            cmd->RSSetScissorRects(1, &sc_bb);
            cmd->ClearRenderTargetView(bb_rtv, clear_bb, 0, nullptr);
            cmd->SetGraphicsRootSignature(light_root.Get());
            ID3D12DescriptorHeap* heaps[] = {srv_heap.Get(), samp_heap.Get()};
            cmd->SetDescriptorHeaps(2, heaps);
            cmd->SetGraphicsRootDescriptorTable(0, srv_heap->GetGPUDescriptorHandleForHeapStart());
            cmd->SetGraphicsRootDescriptorTable(1, samp_heap->GetGPUDescriptorHandleForHeapStart());
            cmd->SetPipelineState(pso_light.Get());
            cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            cmd->IASetVertexBuffers(0, 1, &vbv);
            cmd->DrawInstanced(3, 1, 0, 0);

            // barrier(RXS-0220 逐帧迁移锚点):albedo PSR → RENDER_TARGET(下帧复用);
            //          backbuffer RENDER_TARGET → COPY_SOURCE(readback)。
            D3D12_RESOURCE_BARRIER post_light[2] = {
                transition(gbuf[0].Get(), D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
                           D3D12_RESOURCE_STATE_RENDER_TARGET),
                transition(backbuffers[idx].Get(), D3D12_RESOURCE_STATE_RENDER_TARGET,
                           D3D12_RESOURCE_STATE_COPY_SOURCE),
            };
            cmd->ResourceBarrier(2, post_light);

            // copy backbuffer → readback(present 面必要 device 证据,RXS-0222)。
            D3D12_TEXTURE_COPY_LOCATION src = {};
            src.pResource = backbuffers[idx].Get();
            src.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
            src.SubresourceIndex = 0;
            D3D12_TEXTURE_COPY_LOCATION dst = {};
            dst.pResource = rb_back.Get();
            dst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
            dst.PlacedFootprint = fp_back;
            cmd->CopyTextureRegion(&dst, 0, 0, 0, &src, nullptr);

            // barrier(RXS-0220 终态):backbuffer COPY_SOURCE → PRESENT(呈现前终态)。
            D3D12_RESOURCE_BARRIER to_present =
                transition(backbuffers[idx].Get(), D3D12_RESOURCE_STATE_COPY_SOURCE,
                           D3D12_RESOURCE_STATE_PRESENT);
            cmd->ResourceBarrier(1, &to_present);
            if (FAILED(cmd->Close())) { CloseHandle(fence_ev); return -68; }

            ID3D12CommandList* lists[] = {cmd.Get()};
            queue->ExecuteCommandLists(1, lists);
            wait_gpu();  // 令 readback 与逐帧资源复用安全。

            // 三点回读(首帧 / 重建后首帧 / 末帧,RXS-0222)。
            if (frame == 1) { if (!read_center(out_first_pixel)) { CloseHandle(fence_ev); return -69; } }
            if (rebuilt_this_frame) { if (!read_center(out_rebuilt_pixel)) { CloseHandle(fence_ev); return -70; } }
            if (frame == frames) { if (!read_center(out_last_pixel)) { CloseHandle(fence_ev); return -71; } }

            // Present(sync_interval, flags):flip-model 逐帧 S_OK;tearing 仅 sync_interval=0 生效。
            UINT present_flags = (sync_interval == 0 && tearing_flag) ? DXGI_PRESENT_ALLOW_TEARING : 0;
            HRESULT pr = swapchain->Present(sync_interval, present_flags);
            if (FAILED(pr)) { CloseHandle(fence_ev); return -72; }
            ++presented;
            pump_present_messages(hwnd);  // 只搬运 WM_SIZE/关闭(D-130)。
        }
        wait_gpu();
        CloseHandle(fence_ev);
        if (out_frames_presented) *out_frames_presented = presented;
        return 0;
    };  // run_present

    int rc = run_present();
    DestroyWindow(hwnd);
    UnregisterClassW(cls_name, hinst);
    return rc;
}

}  // extern "C"
