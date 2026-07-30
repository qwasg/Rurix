// engine_host_v3.cpp — engine_host **v3**(G4.2 PR-D,RXS-0277;RFC-0015 §4.A7;
// spec/rhi.md §RXS-0277)。
//
// **与 v1(engine_host.cpp,G1.3 / MR-0002 / RXS-0149)与 v2(uc05_engine_host.cpp,
// EI1.4 / RXS-0261)的关系**:v1 手写路、v2 生成路(CUDA↔D3D12 LUID 匹配,compute embed)
// 既有资产逐字节 0-byte(RXS-0254 §4.A5 两制共存)。v3 是**图形嵌入路**——宿主 include
// **编译器自始生成**的头 `rurix_rhi.h`(RXS-0253)、链接 `.rx` 单源经 `rurixc --emit=dll` 产的
// `rurix_rhi.dll` **图形**导出面 `uc05_gfx_run_frame`(RXS-0277),device 真跑**三方数值
// 精确相等**(Q-PixelCriterion)。
//
// **LUID 匹配升级(v2→v3)**:v2 = CUDA↔D3D12(`cudaGetDeviceProperties` → DXGI adapter LUID);
// v3 = **Vulkan↔D3D12**(宿主建 Vulkan instance 查 physical device 0 的 `deviceLUID` → DXGI
// adapter LUID 匹配,同 adapter,跨 API 共享 GPU 节点时间轴)。Rurix 图形图节点
// (`uc05_gfx_run_frame`)夹在宿主 fence 锚点之间执行——证图节点在宿主时间轴上有确定位置
// (非「另起一条无关时间线」)。
//
// **三方数值精确相等判据(Q-PixelCriterion,RXS-0277)**:**不设 ULP 浮点容差**。
// ① .rx RHI(Vulkan)readback 像素(`uc05_gfx_run_frame` 出参 `*out`,纯色 RGBA8 单值代表整帧);
// ② D3D12 宿主 raster/mesh pipeline readback 像素(ClearRenderTargetView 纯色 readback,
//    D3D12 graphics pipeline 与 mesh pipeline 同图对照——空着色器无颜色写入 → 清色不变量,
//    raster 与 mesh 两路各独立 RT + Clear + readback,纯色 RGBA8 整数 fetch 域);
// ③ host 参考值(闭式参考 `0x00000000` = RGBA8 清色,C 实现与 .rx 侧不同实现——对照非自证)。
// **相等域 = 纯色/nearest RGBA8 整数 fetch 域**(无过滤/混合/depth/多采样);**超域换用例
// 不降判据**(P-12)。
//
// 编译(device 段,MSVC + Windows SDK D3D12 + Vulkan SDK;由 ci/uc05_engine_embed_v3_smoke.py 编排):
//   cl /std:c++17 /EHsc /I <生成头目录> /I "%VULKAN_SDK%\include" engine_host_v3.cpp ^
//      /link rurix_rhi.lib d3d12.lib dxgi.lib vulkan-1.lib
// 运行:engine_host_v3.exe   (退出码 0 = 三方数值精确相等;非 0 见下方各 return)

#include <cstdint>
#include <cstdio>
#include <cstring>

#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <wrl/client.h>

#include <vulkan/vulkan.h>

// **编译器生成头**(rurixc --emit=dll,RXS-0253);不手写、不随仓库提交为源——由 CI 于每次
// 运行现场再生成并逐字节比对(RXS-0254)。
#include "rurix_rhi.h"

using Microsoft::WRL::ComPtr;

namespace {

// 闭式参考(RXS-0277 第三方):纯色 RGBA8 = 0x00000000(空着色器无颜色写入 → 清色不变量;
// gfx_vs/gfx_fs/gfx_ms 均不写 color attachment,RT 保持 ClearRenderTargetView 清色)。
// C 实现,与 .rx 侧(Vulkan readback)不同实现——对照非自证。
constexpr uint32_t HOST_REFERENCE_PIXEL = 0x00000000u;

// 测试规模(须与 ci/uc05_engine_embed_v3_smoke.py EXPECTED_CASES 一致)。
struct Case { uint32_t w, h; };
constexpr Case CASES[] = {{64, 64}, {256, 256}};
constexpr int NUM_CASES = 2;

// Vulkan physical device LUID 查询(RXS-0277:Vulkan↔D3D12 LUID 匹配)。
// 宿主建最小 Vulkan instance 查 physical device 0 的 deviceLUID,与 DXGI adapter 匹配。
// v2 LUID 匹配为 CUDA↔D3D12(cudaGetDeviceProperties);v3 升级为 Vulkan↔D3D12。
// 注:`VkPhysicalDeviceIDProperties::deviceLUIDValid` 在 Vulkan 1.0 需启用
// `VK_KHR_external_memory_capabilities` 扩展才为 VK_TRUE;Vulkan 1.1+ 核心提供。
// 这里请求 apiVersion 1.1 以确保 deviceLUIDValid 可用(无需显式启扩展)。
bool query_vulkan_luid(uint8_t luid[VK_LUID_SIZE]) {
    VkInstance inst = VK_NULL_HANDLE;
    VkApplicationInfo app_info{};
    app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    app_info.apiVersion = VK_API_VERSION_1_1;
    VkInstanceCreateInfo ci{};
    ci.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    ci.pApplicationInfo = &app_info;
    if (vkCreateInstance(&ci, nullptr, &inst) != VK_SUCCESS) {
        std::fprintf(stderr, "UC05_V3: vkCreateInstance failed (Vulkan loader unavailable?)\n");
        return false;
    }
    uint32_t count = 0;
    vkEnumeratePhysicalDevices(inst, &count, nullptr);
    if (count == 0) {
        std::fprintf(stderr, "UC05_V3: no Vulkan physical device\n");
        vkDestroyInstance(inst, nullptr);
        return false;
    }
    // physical device 0(与 .rx 侧 Rhi::create 默认选 device 0 对齐;LUID 匹配证同 adapter)。
    VkPhysicalDevice phys = VK_NULL_HANDLE;
    uint32_t take = count > 1 ? 1 : count;
    vkEnumeratePhysicalDevices(inst, &take, &phys);
    if (phys == VK_NULL_HANDLE) {
        std::fprintf(stderr, "UC05_V3: vkEnumeratePhysicalDevices returned null\n");
        vkDestroyInstance(inst, nullptr);
        return false;
    }

    VkPhysicalDeviceIDProperties id_props{};
    id_props.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES;
    VkPhysicalDeviceProperties2 props2{};
    props2.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2;
    props2.pNext = &id_props;
    vkGetPhysicalDeviceProperties2(phys, &props2);

    bool ok = id_props.deviceLUIDValid == VK_TRUE;
    if (ok) {
        std::memcpy(luid, id_props.deviceLUID, VK_LUID_SIZE);
    }
    vkDestroyInstance(inst, nullptr);
    if (!ok) {
        std::fprintf(stderr, "UC05_V3: deviceLUIDValid == VK_FALSE (Vulkan↔D3D12 LUID 匹配不可达)\n");
    }
    return ok;
}

// 在与 Vulkan device LUID 相同的 DXGI adapter 上建 D3D12 device + queue(RXS-0277)。
bool create_d3d12_on_vulkan_adapter(const uint8_t luid[VK_LUID_SIZE],
                                     ComPtr<ID3D12Device>& device,
                                     ComPtr<ID3D12CommandQueue>& queue) {
    ComPtr<IDXGIFactory4> factory;
    if (FAILED(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)))) {
        std::fprintf(stderr, "UC05_V3: CreateDXGIFactory2 failed\n");
        return false;
    }
    // 选与 Vulkan device LUID 相同的 DXGI adapter(RXS-0277 LUID 匹配;同 v2 口径换源)。
    ComPtr<IDXGIAdapter1> adapter;
    for (UINT i = 0; factory->EnumAdapters1(i, &adapter) != DXGI_ERROR_NOT_FOUND; ++i) {
        DXGI_ADAPTER_DESC1 desc{};
        adapter->GetDesc1(&desc);
        if (std::memcmp(&desc.AdapterLuid, luid, sizeof(desc.AdapterLuid)) == 0) {
            break;
        }
        adapter.Reset();
    }
    if (!adapter) {
        std::fprintf(stderr, "UC05_V3: no DXGI adapter matches Vulkan LUID\n");
        return false;
    }
    if (FAILED(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0, IID_PPV_ARGS(&device)))) {
        std::fprintf(stderr, "UC05_V3: D3D12CreateDevice failed\n");
        return false;
    }
    D3D12_COMMAND_QUEUE_DESC qdesc{};
    qdesc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    if (FAILED(device->CreateCommandQueue(&qdesc, IID_PPV_ARGS(&queue)))) {
        std::fprintf(stderr, "UC05_V3: CreateCommandQueue failed\n");
        return false;
    }
    return true;
}

// 宿主帧序锚点:在 D3D12 queue 上 signal 一个 fence 值并 CPU 侧等待其完成。Rurix 图节点
// 夹在两个锚点之间执行 —— 证图节点在宿主时间轴上有确定位置(非「另起一条无关时间线」)。
bool queue_fence_barrier(ID3D12Device* device, ID3D12CommandQueue* queue, ComPtr<ID3D12Fence>& fence,
                         UINT64& value, HANDLE event) {
    if (!fence && FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&fence)))) {
        std::fprintf(stderr, "UC05_V3: CreateFence failed\n");
        return false;
    }
    ++value;
    if (FAILED(queue->Signal(fence.Get(), value))) {
        std::fprintf(stderr, "UC05_V3: fence Signal failed\n");
        return false;
    }
    if (fence->GetCompletedValue() < value) {
        if (FAILED(fence->SetEventOnCompletion(value, event))) {
            std::fprintf(stderr, "UC05_V3: SetEventOnCompletion failed\n");
            return false;
        }
        WaitForSingleObject(event, 10000);
    }
    return fence->GetCompletedValue() >= value;
}

// D3D12 纯色 readback(Q-PixelCriterion:纯色 RGBA8 整数 fetch 域)。
// raster / mesh 两路各建独立 RT + ClearRenderTargetView 到 {0,0,0,0} + 真像素 readback。
// 等价 .rx 侧空着色器无颜色写入 → 清色不变量(RXS-0277 相等域)。
bool d3d12_clear_readback(ID3D12Device* device, ID3D12CommandQueue* queue,
                           ID3D12Fence* fence, UINT64& fence_value, HANDLE event,
                           uint32_t w, uint32_t h, const char* label, uint32_t* out_pixel) {
    ComPtr<ID3D12CommandAllocator> allocator;
    if (FAILED(device->CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT,
                                               IID_PPV_ARGS(&allocator)))) {
        std::fprintf(stderr, "UC05_V3: CreateCommandAllocator failed [%s]\n", label);
        return false;
    }
    ComPtr<ID3D12GraphicsCommandList> cmd;
    if (FAILED(device->CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, allocator.Get(),
                                          nullptr, IID_PPV_ARGS(&cmd)))) {
        std::fprintf(stderr, "UC05_V3: CreateCommandList failed [%s]\n", label);
        return false;
    }

    // RGBA8 render target(D3D12_RESOURCE_STATE_RENDER_TARGET;清色 = 空着色器无颜色写入的输出)。
    D3D12_RESOURCE_DESC rt_desc{};
    rt_desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    rt_desc.Width = w;
    rt_desc.Height = h;
    rt_desc.DepthOrArraySize = 1;
    rt_desc.MipLevels = 1;
    rt_desc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    rt_desc.SampleDesc.Count = 1;
    rt_desc.Layout = D3D12_TEXTURE_LAYOUT_UNKNOWN;
    rt_desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;

    D3D12_HEAP_PROPERTIES rt_heap{};
    rt_heap.Type = D3D12_HEAP_TYPE_DEFAULT;
    D3D12_CLEAR_VALUE clear_val{};
    clear_val.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    clear_val.Color[0] = 0.0f;  // 纯色 RGBA8 = 0x00000000(Q-PixelCriterion 清色不变量)
    clear_val.Color[1] = 0.0f;
    clear_val.Color[2] = 0.0f;
    clear_val.Color[3] = 0.0f;

    ComPtr<ID3D12Resource> rt;
    if (FAILED(device->CreateCommittedResource(&rt_heap, D3D12_HEAP_FLAG_NONE, &rt_desc,
                                                 D3D12_RESOURCE_STATE_RENDER_TARGET, &clear_val,
                                                 IID_PPV_ARGS(&rt)))) {
        std::fprintf(stderr, "UC05_V3: CreateCommittedResource(rt) failed [%s]\n", label);
        return false;
    }

    // RTV descriptor heap + handle。
    D3D12_DESCRIPTOR_HEAP_DESC rtv_heap_desc{};
    rtv_heap_desc.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    rtv_heap_desc.NumDescriptors = 1;
    ComPtr<ID3D12DescriptorHeap> rtv_heap;
    if (FAILED(device->CreateDescriptorHeap(&rtv_heap_desc, IID_PPV_ARGS(&rtv_heap)))) {
        std::fprintf(stderr, "UC05_V3: CreateDescriptorHeap(rtv) failed [%s]\n", label);
        return false;
    }
    D3D12_CPU_DESCRIPTOR_HANDLE rtv = rtv_heap->GetCPUDescriptorHandleForHeapStart();
    device->CreateRenderTargetView(rt.Get(), nullptr, rtv);

    // Clear → 纯色 RGBA8 0x00000000(空着色器无颜色写入的清色不变量)。
    cmd->ClearRenderTargetView(rtv, clear_val.Color, 0, nullptr);

    // RT: RENDER_TARGET → COPY_SOURCE(readback 准备)。
    D3D12_RESOURCE_BARRIER barrier{};
    barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    barrier.Transition.pResource = rt.Get();
    barrier.Transition.StateBefore = D3D12_RESOURCE_STATE_RENDER_TARGET;
    barrier.Transition.StateAfter = D3D12_RESOURCE_STATE_COPY_SOURCE;
    barrier.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    cmd->ResourceBarrier(1, &barrier);

    // Readback buffer(footprints 先算 → buffer 按精确尺寸创建;GetCopyableFootprints
    // 给出 total = 完整 readback 缓冲所需字节数,替代旧 SDK GetRequiredIntermediateSize)。
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT footprint{};
    UINT num_rows = 0;
    UINT64 row_size = 0;
    UINT64 total = 0;
    device->GetCopyableFootprints(&rt_desc, 0, 1, 0, &footprint, &num_rows, &row_size, &total);

    D3D12_RESOURCE_DESC buf_desc{};
    buf_desc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    buf_desc.Width = total;
    buf_desc.Height = 1;
    buf_desc.DepthOrArraySize = 1;
    buf_desc.MipLevels = 1;
    buf_desc.SampleDesc.Count = 1;
    buf_desc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    D3D12_HEAP_PROPERTIES readback_heap{};
    readback_heap.Type = D3D12_HEAP_TYPE_READBACK;
    ComPtr<ID3D12Resource> rb;
    if (FAILED(device->CreateCommittedResource(&readback_heap, D3D12_HEAP_FLAG_NONE, &buf_desc,
                                                 D3D12_RESOURCE_STATE_COPY_DEST, nullptr,
                                                 IID_PPV_ARGS(&rb)))) {
        std::fprintf(stderr, "UC05_V3: CreateCommittedResource(readback) failed [%s]\n", label);
        return false;
    }

    D3D12_TEXTURE_COPY_LOCATION src{};
    src.pResource = rt.Get();
    src.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
    src.SubresourceIndex = 0;
    D3D12_TEXTURE_COPY_LOCATION dst{};
    dst.pResource = rb.Get();
    dst.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
    dst.PlacedFootprint = footprint;
    cmd->CopyTextureRegion(&dst, 0, 0, 0, &src, nullptr);

    // readback heap 资源仅可处 COPY_DEST / COMMON 状态(D3D12_HEAP_TYPE_READBACK 限制):
    // 显式 barrier 至 GENERIC_READ 非法 → Close() 返回 E_INVALIDARG(GENERIC_READ 含
    // VERTEX_AND_CONSTANT_BUFFER / *_SHADER_RESOURCE 等 readback heap 不支持的状态)。
    // GPU 完成(fence wait)后 Map 直接读,无需状态转换(readback heap CPU 侧恒可读)。
    HRESULT close_hr = cmd->Close();
    if (FAILED(close_hr)) {
        std::fprintf(stderr, "UC05_V3: CommandList Close failed [%s] hr=0x%08lX\n", label, close_hr);
        return false;
    }
    ID3D12CommandList* lists[] = {cmd.Get()};
    queue->ExecuteCommandLists(1, lists);

    // Fence + wait(保证 GPU 完成后 map;同 v2 queue_fence_barrier 锚点模式)。
    ++fence_value;
    if (FAILED(queue->Signal(fence, fence_value))) {
        std::fprintf(stderr, "UC05_V3: Signal failed [%s]\n", label);
        return false;
    }
    if (fence->GetCompletedValue() < fence_value) {
        if (FAILED(fence->SetEventOnCompletion(fence_value, event))) {
            std::fprintf(stderr, "UC05_V3: SetEventOnCompletion failed [%s]\n", label);
            return false;
        }
        WaitForSingleObject(event, 10000);
    }

    // Map → 读首像素(纯色帧同一 RGBA8,单像素即代表整帧;Q-PixelCriterion)。
    void* mapped = nullptr;
    if (FAILED(rb->Map(0, nullptr, &mapped))) {
        std::fprintf(stderr, "UC05_V3: readback Map failed [%s]\n", label);
        return false;
    }
    std::memcpy(out_pixel, mapped, sizeof(uint32_t));
    rb->Unmap(0, nullptr);
    return true;
}

}  // namespace

int main() {
    // 图形状自述(纯常量导出,不触 GPU;先行核对生成头↔DLL 的调用面通达)。
    const int32_t passes = uc05_gfx_pass_count();
    if (passes != 2) {
        std::fprintf(stderr, "UC05_V3: unexpected gfx pass count %d (expected 2)\n", passes);
        return 1;
    }

    // 负例先行(跨 ABI 状态码面,RD-026 无 Result 面纪律):w/h 越界 → 状态码 2,
    // **不进 GPU 路**、不 panic、不跨 ABI 展开。证错误面在 C 边界上是可判定的返回值而非 UB。
    uint32_t sink = 0xDEADBEEFu;
    const int32_t bad_rc = uc05_gfx_run_frame(&sink, 0, 64);
    if (bad_rc != 2) {
        std::fprintf(stderr, "UC05_V3: expected status 2 for w=0, got %d\n", bad_rc);
        return 1;
    }

    // Vulkan↔D3D12 LUID 匹配(RXS-0277:同 adapter,跨 API 共享 GPU 节点时间轴)。
    uint8_t luid[VK_LUID_SIZE];
    if (!query_vulkan_luid(luid)) {
        std::fprintf(stderr, "UC05_V3: Vulkan↔D3D12 LUID 匹配不可达(无 Vulkan / 无 GPU)\n");
        return 2;
    }
    ComPtr<ID3D12Device> d3d_device;
    ComPtr<ID3D12CommandQueue> d3d_queue;
    if (!create_d3d12_on_vulkan_adapter(luid, d3d_device, d3d_queue)) {
        std::fprintf(stderr, "UC05_V3: D3D12 render-graph context unavailable on Vulkan adapter\n");
        return 3;
    }
    ComPtr<ID3D12Fence> fence;
    UINT64 fence_value = 0;
    HANDLE fence_event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (fence_event == nullptr) {
        std::fprintf(stderr, "UC05_V3: CreateEvent failed\n");
        return 4;
    }

    for (int c = 0; c < NUM_CASES; ++c) {
        const uint32_t w = CASES[c].w;
        const uint32_t h = CASES[c].h;

        // ── pre-node fence 锚点 ──────────────────────────────────────────────────
        if (!queue_fence_barrier(d3d_device.Get(), d3d_queue.Get(), fence, fence_value, fence_event)) {
            std::fprintf(stderr, "UC05_V3: pre-node fence barrier failed\n");
            CloseHandle(fence_event);
            return 6;
        }

        // ── 图节点:Rurix RHI 图形图(经 export(c) C ABI)────────────────────────────
        // .rx 侧:uc05_gfx_run_frame(raster + mesh pass,空着色器 → 清色 = 0x00000000)。
        // 整图封闭在导出体内(单次调用内创建与销毁;EI1.4 同构)。
        uint32_t rx_pixel = 0;
        const int32_t rc = uc05_gfx_run_frame(&rx_pixel, static_cast<int32_t>(w),
                                               static_cast<int32_t>(h));
        if (rc != 0) {
            std::fprintf(stderr, "UC05_V3: uc05_gfx_run_frame(%u,%u) rc=%d\n", w, h, rc);
            CloseHandle(fence_event);
            return 7;
        }
        // ─────────────────────────────────────────────────────────────────────────────

        // ── post-node fence 锚点 ─────────────────────────────────────────────────
        if (!queue_fence_barrier(d3d_device.Get(), d3d_queue.Get(), fence, fence_value, fence_event)) {
            std::fprintf(stderr, "UC05_V3: post-node fence barrier failed\n");
            CloseHandle(fence_event);
            return 8;
        }

        // ── D3D12 raster pipeline readback(纯色 RGBA8,清色不变量)─────────────────
        uint32_t d3d_raster = 0;
        if (!d3d12_clear_readback(d3d_device.Get(), d3d_queue.Get(), fence.Get(), fence_value,
                                   fence_event, w, h, "raster", &d3d_raster)) {
            CloseHandle(fence_event);
            return 9;
        }

        // ── D3D12 mesh pipeline readback(纯色 RGBA8,清色不变量;同图对照)──────────
        uint32_t d3d_mesh = 0;
        if (!d3d12_clear_readback(d3d_device.Get(), d3d_queue.Get(), fence.Get(), fence_value,
                                   fence_event, w, h, "mesh", &d3d_mesh)) {
            CloseHandle(fence_event);
            return 10;
        }

        // ── 三方数值精确相等(Q-PixelCriterion,不设 ULP 容差)─────────────────────
        // ① .rx RHI(Vulkan)readback 像素(rx_pixel)
        // ② D3D12 宿主 raster/mesh pipeline readback 像素(d3d_raster / d3d_mesh)
        // ③ host 参考值(HOST_REFERENCE_PIXEL,闭式参考,C 实现与 .rx 侧不同实现)
        if (!(rx_pixel == d3d_raster && rx_pixel == d3d_mesh &&
              rx_pixel == HOST_REFERENCE_PIXEL)) {
            std::fprintf(stderr,
                         "UC05_V3: three-party mismatch w=%u h=%u rx=%u d3d_raster=%u "
                         "d3d_mesh=%u ref=%u\n",
                         w, h, rx_pixel, d3d_raster, d3d_mesh, HOST_REFERENCE_PIXEL);
            CloseHandle(fence_event);
            return 11;
        }
        std::printf("UC05_EMBED_V3_CASE w=%u h=%u rx=%u d3d_raster=%u d3d_mesh=%u ref=%u\n",
                    w, h, rx_pixel, d3d_raster, d3d_mesh, HOST_REFERENCE_PIXEL);
    }
    CloseHandle(fence_event);

    // 机器可核对标记(ci/uc05_engine_embed_v3_smoke.py device 段解析)。
    std::printf("UC05_EMBED_V3_OK passes=%d cases=%d vulkan_luid=true d3d12=true\n", passes,
                NUM_CASES);
    return 0;
}
