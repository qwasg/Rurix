// minimal_host.cpp — Rurix 渲染器 C ABI 最小宿主示例（G31+ 波 C Task C2；
// docs/renderer/integration_guide.md §4「最小集成五步」配套可执行见证）。
//
// 对齐 EI1 UC-05 既有 C ABI 面（spec/export_c.md RXS-0250~0255 + spec/rhi.md
// RXS-0261/0277；C1 SDK API 面在飞未定型，本例以既有面为准）：
//   · 头文件 rurix_rhi.h = 编译器自始生成（rurixc --emit=dll 副产，RXS-0253），不手写；
//   · 链接 rurix_rhi.lib（link.exe /DLL 副产 import lib，RXS-0252）；
//   · 导出函数封闭 GPU 上下文/图/资源（单次调用内创建与销毁，EI1.4 同构）；
//   · 宿主只见 C ABI 标量与裸指针（subset v1，RXS-0251），错误面 = i32 状态码闭集。
//
// 编译运行见同目录 README.md / build.ps1。退出码：0 = 全绿（打印
// RURIX_MINIMAL_HOST_OK）；非 0 = 各步骤自检失败（见 stderr 行号）。

#include <cstdint>
#include <cstdio>

#include "rurix_rhi.h"  // 编译器生成头（构建期现场生成于 build/；仓库内零手写副本）

namespace {

// 帧参数（本例用 UC-05 图形导出面最小合法规模；w/h ∈ [1, 4096] 为导出契约域）。
constexpr int32_t FRAME_W = 64;
constexpr int32_t FRAME_H = 64;
constexpr int FRAME_COUNT = 4;  // 演示帧循环：每帧一次导出调用

}  // namespace

int main() {
    // ── 步骤 1 · 初始化（宿主装载 + 不触 GPU 的自检调用）────────────────────
    // 纯常量导出：图形状自述（raster + mesh 两 pass = 2）。头↔DLL 调用面通达
    // 先行核对——失败说明 DLL/import lib/头三者版本错位（单一事实源漂移）。
    const int32_t passes = uc05_gfx_pass_count();
    if (passes != 2) {
        std::fprintf(stderr, "MINIMAL_HOST step1 init: unexpected pass count %d (expect 2)\n",
                     passes);
        return 1;
    }

    // ── 步骤 2 · 场景（图声明封闭在导出体内；宿主以标量入参参数化）─────────
    // 无宿主侧场景对象：uc05_gfx_run_frame 导出体内部建 Context/Rhi/两 color
    // target/raster+mesh 两 pass。本步在宿主侧只体现为帧参数就绪。

    // ── 步骤 4 · 错误面负例先行（推荐集成顺序：先证错误可判定）─────────────
    // w=0 越界 → 状态码 2（不进 GPU 路、不 panic、不跨 ABI 展开；RD-026 无
    // Result 面纪律）。subset v1 无指针→整数 cast，out 非空 = 调用方前置条件。
    uint32_t pixel = 0xDEADBEEFu;
    const int32_t bad_rc = uc05_gfx_run_frame(&pixel, 0, FRAME_H);
    if (bad_rc != 2) {
        std::fprintf(stderr, "MINIMAL_HOST step4 error-face: expect status 2 for w=0, got %d\n",
                     bad_rc);
        return 2;
    }
    if (pixel != 0xDEADBEEFu) {
        std::fprintf(stderr, "MINIMAL_HOST step4 error-face: out param written on reject path\n");
        return 3;
    }

    // ── 步骤 3 · 帧循环（每帧一次导出调用；真 GPU 真跑）────────────────────
    // 每次调用：图装配核验（I3/I4/I5）+ hazard 推导 + 真派发 + 真 D2H 读回；
    // GPU 资源单次调用内创建与销毁（无跨调用泄漏面）。
    for (int frame = 0; frame < FRAME_COUNT; ++frame) {
        const int32_t rc = uc05_gfx_run_frame(&pixel, FRAME_W, FRAME_H);
        if (rc != 0) {
            std::fprintf(stderr, "MINIMAL_HOST step3 frame %d: rc=%d\n", frame, rc);
            return 4;
        }
    }

    // ── 步骤 5 · 关闭（导出体内资源已随末次调用拆除；宿主直接卸载）─────────
    // 无 shutdown 导出可调（契约面：资源生命周期 = 单次调用域）。
    // 预期像素 = 0x00000000（空着色器无颜色写入 → 清色不变量，RXS-0277
    // Q-PixelCriterion 纯色 RGBA8 整数 fetch 域）。
    std::printf("RURIX_MINIMAL_HOST_OK passes=%d frames=%d pixel=0x%08X\n", passes, FRAME_COUNT,
                pixel);
    return 0;
}
