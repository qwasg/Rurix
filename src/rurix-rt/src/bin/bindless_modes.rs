//! G3.4 bindless **device 索引判据 harness**(RXS-0231~0235;RFC-0013 §4.C4;验收门
//! G-G3-4;counter `g3.counter.bindless_descriptor_smoke` ≥1)。镜像 `bin/sampling_modes`
//! 的 device 真跑 / SKIP 三态 + 「篡改→像素变=RED,复原=GREEN」数据流纪律(RXS-0176 IR2)。
//!
//! ## 判据结构(RFC-0013 §4.C4;数值阈值 = **owner 本机迭代填**,TODO)
//!
//! 无界表 `[Texture2D<f32>]` 注册 ≥4 纹理,片元着色器按屏幕象限动态非均匀索引
//! `table[nonuniform(idx)].sample(samp, uv)` 采样(codegen:OpTypeRuntimeArray + NonUniform
//! + clamp,已 spirv-val vulkan1.2 accept)。三判据:
//!
//! - `quadrant_index_four_color`:idx = 象限号(0/1/2/3)→ 四象限各采不同注册纹理 → 四色;
//!   每象限采样点像素 == 对应注册纹理主色(expect_* 谓词 TODO owner 校准)。
//! - `tamper_register_order_swap`:交换注册序(register 顺序)→ 象限颜色换位 = RED
//!   (证动态索引真按注册序命中,非静默取元素 0);复原注册序 → GREEN。
//! - `feature_chain_missing_err`:探测 `VkPhysicalDeviceDescriptorIndexingFeatures` 四 bit
//!   (shaderSampledImageArrayNonUniformIndexing / descriptorBindingSampledImageUpdateAfterBind
//!   / descriptorBindingPartiallyBound / runtimeDescriptorArray)任一缺失 → 确定性 `Err`
//!   (RXS-0193 封口,不占 RX 码,无静默降级)。
//!
//! ## device 真跑 / SKIP 三态(RXS-0235 / §4.C4)
//! device 执行路(无界表 descriptor pool UPDATE_AFTER_BIND + PARTIALLY_BOUND + feature chain
//! 探测 + set4 独占 + push-constant 表长下发 + 四象限渲染回读)归 **owner 本机主循环真跑**
//! (活 Vulkan 驱动 RTX 4070 Ti);本 harness 判据结构就位、数值阈值 TODO(agent 无 GPU,只到
//! 编译 + bindless SPIR-V spirv-val vulkan1.2 accept〔`bindless_vulkan_spirv_val`〕)。故本
//! harness 首期打印 `BINDLESS_MODES: PARTIAL`(判据结构定义、device 执行阈值待 owner 校准),
//! `ci/bindless_smoke.py` 据此 SKIP 三态(**非 fake pass**;`RURIX_REQUIRE_REAL=1` 翻硬红,促
//! owner 收敛)。无 Vulkan loader/设备 → `BINDLESS_MODES: SKIP`。**AMD 真卡见证 = G-MB1-6
//! 独立尾门**(本机 RTX 4070 Ti measured 不充作 AMD)。device 真跑绝不伪造。

/// bindless device 判据模式(RFC-0013 §4.C4;evidence modes_ok enum 同源)。
const MODES: &[&str] = &[
    "quadrant_index_four_color",
    "tamper_register_order_swap",
    "feature_chain_missing_err",
];

fn main() {
    // 判据结构就位声明(owner 主循环消费:四象限动态索引四色 + 篡改注册序换位 RED +
    // feature chain 四 bit 缺失 Err;数值阈值/采样点/容差 = owner 本机迭代校准 TODO)。
    println!("[bindless_modes] G3.4 bindless device 判据 harness(RFC-0013 §4.C4,G-G3-4)");
    for (i, m) in MODES.iter().enumerate() {
        println!(
            "[bindless_modes]   判据 {}: {m}(阈值 TODO owner 本机)",
            i + 1
        );
    }
    // device 执行路(无界表 descriptor pool + feature chain + set4 + push-constant 表长 +
    // 四象限渲染回读)归 owner 主循环真跑;本 harness 编译 + bindless SPIR-V spirv-val
    // vulkan1.2 accept 已兑现,device 数值判据阈值待 owner 校准 → 诚实 PARTIAL(非 fake 绿)。
    println!(
        "BINDLESS_MODES: PARTIAL modes_planned={} (device 执行判据阈值待 owner 本机主循环校准;\
         bindless SPIR-V 已 spirv-val vulkan1.2 accept)",
        MODES.len()
    );
    // 退 0:判据结构就位、编译绿;device 数值真跑 = owner 主循环(判据阈值 TODO,不伪造绿)。
    std::process::exit(0);
}
