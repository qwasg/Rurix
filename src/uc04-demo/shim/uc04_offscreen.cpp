// UC-04 deferred 渲染器 offscreen D3D12 shim(G2.4 / RFC-0006;选项 B:不采样 G-buffer 的
// 最小多 pass deferred)。D3D12/DXGI 的 COM 复杂度全部留在 C++ shim(不进语言,D-130 先例,
// 对齐 src/rurix-d3d12/shim/rx_d3d12_shim.cpp);Rust 侧仅见版本化扁平 `extern "C"` 面。
//
// 消费 **Rurix 源经 rurixc 图形=B DXIL 链**产出的 4 个 DXIL 着色器对象(几何 pass VS/FS +
// lighting pass VS/FS)+ RFC-0005 推导的 RTS0 root signature 容器字节(P-11 单一事实源),
// 在真 hardware 上:
//   pass 1(几何):VS 透传顶点缓冲 pos→SV_Position + uv/normal varying → FS 写 G-buffer
//                 MRT(albedo R8 / normal R16F / depth R32F,3 渲染目标);
//   pass 2(lighting/合成):VS 透传 + FS 走**自身插值输入**(uv,**不采样 G-buffer**=选项 B
//                 折中边界,采样完备性仍 blocked 于 RD-021 / 06§4.2 禁区)→ 写 final R8;
//   手动 barrier(RXS-0169 编排锚点)→ offscreen readback 取 albedo 与 final 中心像素。
//
// **G-G2-4 防降级**:VS/FS 全部来自 Rurix 源经图形=B DXIL(非手写 HLSL/DXIL);RTS0 经
// CreateRootSignature 真机解析进 PSO;真 hardware 多 pass deferred draw + offscreen readback。

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
#include <vector>

using Microsoft::WRL::ComPtr;

namespace {

// shim C ABI 版本(与 Rust 侧 RX_UC04_ABI_VERSION 一致)。
constexpr uint32_t kAbiVersion = 1;

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

}  // namespace

extern "C" {

// 返回 shim ABI 版本(Rust 侧编译期/运行期核对)。
__declspec(dllexport) uint32_t rx_uc04_abi_version(void) { return kAbiVersion; }

// UC-04 offscreen 两 pass deferred draw + readback。
//
// 入参:width/height = offscreen 尺寸;rts0 = RFC-0005 RTS0 容器字节(可为空 root sig);
//   geom_vs/geom_fs/light_vs/light_fs = Rurix 图形=B DXIL 容器字节(VS/FS 各 pass);
//   顶点缓冲(全屏三角形 pos + uv=normal=0.5)由 shim 内置(host 几何数据,非着色器)。
// 出参:out_gbuffer_pixel[4] = G-buffer albedo 中心像素 RGBA8(证几何 pass FS 写 MRT);
//   out_final_pixel[4] = lighting/合成 final 中心像素 RGBA8(证 lighting pass FS 出图);
//   out_adapter = 选中的硬件 adapter 名(UTF-8)。
// 返回 0 成功;非 0 = HRESULT 位码或哨兵失败码(Rust 侧不伪造 device 绿)。
__declspec(dllexport) int rx_uc04_offscreen_run(
    uint32_t abi_version, uint32_t width, uint32_t height,
    const uint8_t* rts0, size_t rts0_len,
    const uint8_t* geom_vs, size_t geom_vs_len,
    const uint8_t* geom_fs, size_t geom_fs_len,
    const uint8_t* light_vs, size_t light_vs_len,
    const uint8_t* light_fs, size_t light_fs_len,
    uint8_t* out_gbuffer_pixel, uint8_t* out_final_pixel,
    char* out_adapter, size_t out_adapter_cap) {
    if (abi_version != kAbiVersion) return -1;
    if (!geom_vs || !geom_fs || !light_vs || !light_fs) return -2;
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

    // 2) root signature:直接由 Rurix RFC-0005 RTS0 容器字节 CreateRootSignature(P-11
    //    单一事实源,device-parse;选项 B 无资源 → 空 root sig 仍经真机解析进 PSO)。
    ComPtr<ID3D12RootSignature> root;
    if (rts0 && rts0_len > 0) {
        if (FAILED(device->CreateRootSignature(0, rts0, rts0_len, IID_PPV_ARGS(&root))))
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

    // 4) 顶点缓冲(全屏三角形,host 几何数据):每顶点 {pos vec4, uv f32, normal f32} = 24B。
    //    uv=normal=0.5 → 插值常量;覆盖中心像素。layout 语义名 pos/uv/normal 匹配 VS 输入签名。
    struct Vtx { float pos[4]; float uv; float normal; };
    const Vtx verts[3] = {
        {{-1.0f, -1.0f, 0.0f, 1.0f}, 0.5f, 0.5f},
        {{-1.0f, 3.0f, 0.0f, 1.0f}, 0.5f, 0.5f},
        {{3.0f, -1.0f, 0.0f, 1.0f}, 0.5f, 0.5f},
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
    D3D12_INPUT_ELEMENT_DESC il[3] = {
        {"pos", 0, DXGI_FORMAT_R32G32B32A32_FLOAT, 0, 0, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
        {"uv", 0, DXGI_FORMAT_R32_FLOAT, 0, 16, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
        {"normal", 0, DXGI_FORMAT_R32_FLOAT, 0, 20, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0},
    };

    auto make_pso = [&](const uint8_t* vs, size_t vs_len, const uint8_t* fs, size_t fs_len,
                        UINT num_rt, const DXGI_FORMAT* fmts,
                        ComPtr<ID3D12PipelineState>& out) -> HRESULT {
        D3D12_GRAPHICS_PIPELINE_STATE_DESC pd = {};
        pd.pRootSignature = root.Get();
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
    if (FAILED(make_pso(geom_vs, geom_vs_len, geom_fs, geom_fs_len, 3, gbuf_fmt, pso_geom)))
        return -30;
    const DXGI_FORMAT final_fmt[1] = {DXGI_FORMAT_R8G8B8A8_UNORM};
    if (FAILED(make_pso(light_vs, light_vs_len, light_fs, light_fs_len, 1, final_fmt, pso_light)))
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

    // RXS-0169 编排锚点(手动 barrier;选项 B 不采样 → albedo 转 COPY_SOURCE 供 readback 见证,
    // 非 RT→SRV)。其余 gbuffer 目标维持 RENDER_TARGET(本期不读)。
    D3D12_RESOURCE_BARRIER after_geom =
        transition(gbuf[0].Get(), D3D12_RESOURCE_STATE_RENDER_TARGET,
                   D3D12_RESOURCE_STATE_COPY_SOURCE);
    cmd->ResourceBarrier(1, &after_geom);

    // pass 2:lighting/合成 pass 写 final(走自身插值输入,**不采样 G-buffer**=选项 B)。
    cmd->OMSetRenderTargets(1, &final_rtv, FALSE, nullptr);
    cmd->ClearRenderTargetView(final_rtv, clear_final, 0, nullptr);
    cmd->SetPipelineState(pso_light.Get());
    cmd->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    cmd->IASetVertexBuffers(0, 1, &vbv);
    cmd->DrawInstanced(3, 1, 0, 0);

    D3D12_RESOURCE_BARRIER after_light =
        transition(final_rt.Get(), D3D12_RESOURCE_STATE_RENDER_TARGET,
                   D3D12_RESOURCE_STATE_COPY_SOURCE);
    cmd->ResourceBarrier(1, &after_light);

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

    // 7) 读回中心像素。albedo R8G8B8A8(几何 FS 写,R=uv+0.25);final R8G8B8A8(lighting FS,
    //    R=uv+0.5)。中心 (w/2, h/2)。
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

}  // extern "C"
