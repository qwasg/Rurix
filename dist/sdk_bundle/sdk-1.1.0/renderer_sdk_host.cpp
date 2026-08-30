// G31+ 波 C Task C1:渲染器 SDK 外部宿主 demo(G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #48
// 交付判据「≥1 个外部 C++ 宿主集成 demo 真跑」兑现面)。
//
// **非内置最小控制台宿主**:只见 export_c codegen 生成头 `rurix_renderer.h`
// (自始生成、不手写,RXS-0253)+ 链 `rurix_renderer.lib`(import lib)——不见
// 任何 Rurix 类型/内部头,纯 C ABI 标量与裸指针集成。流程 = API 面全要素:
// 初始化(设备/能力协商)→ 场景提交(bistro 生产契约路径)→ 帧循环
// (render_frame ×(warmup+frames),末帧 readback 取 digest/帧时)→ 参数更新
// 见证(digest 落袋后 set_camera/set_exposure 合法面 rc==0 + 非法面确定性拒
// + 更新后续渲两帧见证通路)→ present 句柄 → 关闭。
//
// 机器可核 token(stdout;ci/g31_renderer_sdk_smoke.py 消费):
//   RXSDK_HOST_ABI=0x00010000
//   RXSDK_HOST_CAPS=<u64>
//   RXSDK_HOST_LOAD_OK tier=<t> frames=<n> warmup=<w>
//   RXSDK_HOST_FRAME mean=<ms> p50=<ms> n=<post-warmup 样本数>
//   RXSDK_HOST_DIGEST sha256:<64hex>(末帧 readback,canonical 序列对拍面)
//   RXSDK_HOST_PARAMS_OK(参数更新见证:合法 rc==0 + 非法确定性拒 + 续渲绿)
//   RXSDK_HOST_PRESENT_OK
//   RXSDK_HOST_OK(全链成功终 token)
//
// 退出码(三态纪律;v3 embed 宿主 rc=2/3 dev-env 先例):0 = 全链绿;
// 2 = create 失败(Vulkan loader 不可用,dev-env);3 = load_scene 资产/能力缺失
// (dev-env);1 = API 状态码错误(真 FAIL)。
#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

#include "rurix_renderer.h"

namespace {

// canonical 口径(G14.3 bench 腿同字面):frames+warmup 次迭代,末帧 readback。
struct Args {
    std::string contract;
    std::string gltf;
    std::string scene = "bistro-interior";
    std::string spv_dir;
    uint32_t tier = 100;
    uint32_t frames = 160;
    uint32_t warmup = 10;
};

bool parse_args(int argc, char** argv, Args& a) {
    for (int i = 1; i + 1 < argc; i += 2) {
        const std::string k = argv[i];
        const std::string v = argv[i + 1];
        if (k == "--contract") a.contract = v;
        else if (k == "--gltf") a.gltf = v;
        else if (k == "--scene") a.scene = v;
        else if (k == "--spv-dir") a.spv_dir = v;
        else if (k == "--tier") a.tier = static_cast<uint32_t>(std::stoul(v));
        else if (k == "--frames") a.frames = static_cast<uint32_t>(std::stoul(v));
        else if (k == "--warmup") a.warmup = static_cast<uint32_t>(std::stoul(v));
        else return false;
    }
    return !a.contract.empty() && !a.gltf.empty() && !a.spv_dir.empty()
        && a.frames >= 1;
}

int api_fail(const char* op, int32_t rc) {
    std::fprintf(stderr, "RXSDK_HOST: FAIL op=%s rc=%d\n", op, static_cast<int>(rc));
    return 1;
}

}  // namespace

int main(int argc, char** argv) {
    Args a;
    if (!parse_args(argc, argv, a)) {
        std::fprintf(stderr,
            "usage: renderer_sdk_host --contract <c.json> --gltf <s.gltf> "
            "--spv-dir <dir> [--scene bistro-interior] [--tier 100] "
            "[--frames 160] [--warmup 10]\n");
        return 1;
    }

    // ① 初始化(版本/能力协商面)。
    const uint32_t abi = rurix_renderer_abi_version();
    std::printf("RXSDK_HOST_ABI=0x%08x\n", abi);
    if ((abi >> 16) != 1u) {
        std::fprintf(stderr, "RXSDK_HOST: FAIL ABI MAJOR %u ≠ 1(政策见 API_VERSIONING.md)\n",
            abi >> 16);
        return 1;
    }
    const uint64_t caps = rurix_renderer_caps_probe();
    std::printf("RXSDK_HOST_CAPS=%llu\n", static_cast<unsigned long long>(caps));

    const uint64_t r = rurix_renderer_create(0);
    if (r == 0) {
        std::fprintf(stderr, "RXSDK_HOST: create 失败(Vulkan loader 不可用,dev-env)\n");
        return 2;
    }

    // ② 场景提交(bistro 生产契约 + gltf + canonical SPV 四件套目录)。
    const int32_t rc_load = rurix_renderer_load_scene(
        r,
        reinterpret_cast<const uint8_t*>(a.contract.data()),
        static_cast<uint32_t>(a.contract.size()),
        reinterpret_cast<const uint8_t*>(a.gltf.data()),
        static_cast<uint32_t>(a.gltf.size()),
        reinterpret_cast<const uint8_t*>(a.scene.data()),
        static_cast<uint32_t>(a.scene.size()),
        a.tier,
        reinterpret_cast<const uint8_t*>(a.spv_dir.data()),
        static_cast<uint32_t>(a.spv_dir.size()));
    if (rc_load != 0) {
        std::fprintf(stderr, "RXSDK_HOST: load_scene rc=%d(资产/能力缺失,dev-env)\n",
            static_cast<int>(rc_load));
        rurix_renderer_destroy(r);
        return 3;
    }
    std::printf("RXSDK_HOST_LOAD_OK tier=%u frames=%u warmup=%u\n", a.tier, a.frames, a.warmup);

    // ③ 帧循环(warmup+frames 次迭代;post-warmup 帧时统计;末帧 readback 取
    // digest——与生产 bench 腿逐字同式的 canonical 序列,Stage A 锚对拍面;
    // 循环内不动相机/曝光)。
    std::vector<double> frame_ms;
    frame_ms.reserve(a.frames);
    char digest[72];
    uint32_t digest_len = 0;
    const uint32_t total = a.warmup + a.frames;
    for (uint32_t i = 0; i < total; ++i) {
        double ms = 0.0;
        const uint32_t readback = (i + 1 == total) ? 1u : 0u;
        const int32_t rc = rurix_renderer_render_frame(
            r, readback, &ms, reinterpret_cast<uint8_t*>(digest),
            static_cast<uint32_t>(sizeof(digest)), &digest_len);
        if (rc != 0) {
            rurix_renderer_destroy(r);
            return api_fail("render_frame", rc);
        }
        if (i >= a.warmup) frame_ms.push_back(ms);
    }
    std::sort(frame_ms.begin(), frame_ms.end());
    double mean = 0.0;
    for (double v : frame_ms) mean += v;
    mean /= static_cast<double>(frame_ms.size());
    const double p50 = frame_ms[frame_ms.size() / 2];
    std::printf("RXSDK_HOST_FRAME mean=%.4f p50=%.4f n=%zu\n", mean, p50, frame_ms.size());
    std::printf("RXSDK_HOST_DIGEST %.*s\n", static_cast<int>(digest_len), digest);

    // ④ 参数更新见证(digest 落袋后,不污染 canonical 对拍面):合法调用 rc==0
    // + 非法输入确定性拒(rc==3)+ 更新后续渲两帧见证参数通路生效。
    {
        const float eye[3] = {0.0f, 1.0f, -2.0f};
        const float fwd[3] = {0.0f, 0.0f, 1.0f};
        const float up[3] = {0.0f, 1.0f, 0.0f};
        if (rurix_renderer_set_camera(r, eye, fwd, up, 1.0f, 0.1f, 100.0f) != 0)
            return api_fail("set_camera", 0);
        if (rurix_renderer_set_exposure_ev100(r, 11.3f) != 0)
            return api_fail("set_exposure_ev100", 0);
        // 非法面确定性拒(fov 越域 → 状态码 3;拒后会话仍可用)。
        const int32_t rc_bad = rurix_renderer_set_camera(r, eye, fwd, up, -1.0f, 0.1f, 100.0f);
        if (rc_bad != 3)
            return api_fail("set_camera_reject", rc_bad);
        double ms = 0.0;
        uint32_t dlen = 0;
        for (uint32_t k = 0; k < 2; ++k) {
            const int32_t rc = rurix_renderer_render_frame(
                r, 0, &ms, reinterpret_cast<uint8_t*>(digest),
                static_cast<uint32_t>(sizeof(digest)), &dlen);
            if (rc != 0) {
                rurix_renderer_destroy(r);
                return api_fail("render_frame_post_params", rc);
            }
        }
    }
    std::printf("RXSDK_HOST_PARAMS_OK\n");

    // ⑤ present 句柄 + 关闭。
    const int32_t rc_present = rurix_renderer_present(r);
    if (rc_present != 0) {
        rurix_renderer_destroy(r);
        return api_fail("present", rc_present);
    }
    std::printf("RXSDK_HOST_PRESENT_OK\n");
    const int32_t rc_destroy = rurix_renderer_destroy(r);
    if (rc_destroy != 0) return api_fail("destroy", rc_destroy);
    std::printf("RXSDK_HOST_OK\n");
    return 0;
}
