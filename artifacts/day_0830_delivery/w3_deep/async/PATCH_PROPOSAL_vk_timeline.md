# PATCH 提案 — vk.rs timeline semaphore 探测/创建/提交 最小 diff(试做文本,**不合入**)

> W3 深水区可行性试做交付物。vk.rs 本体本窗禁改,以下为实施窗(M59 判档 go + RFC 修订行登记后)的
> 加性 diff 提案;全部落 `src/rurix-rt/src/vk.rs`(或镜像 `vk_g31_*.rs` body-include 先例独立文件),
> 既有入口 0-byte。数值纪律:sType/枚举值按仓惯例经 SDK 1.3.296 `vulkan_core.h` 逐值核对 + `assert_eq!`
> 锚(vk.rs:27820-27837 先例);下列值为起草值,**落地前须逐值复核**。

## A. 新增常量(常量区,邻 `ST_PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES` 27305 段)

```rust
// sType(VK_KHR_timeline_semaphore 扩展号 207 → 1.2 core 收编编号不变;待 SDK 逐值核对)。
const ST_SEMAPHORE_TYPE_CREATE_INFO: u32 = 1_000_207_002;
const ST_TIMELINE_SEMAPHORE_SUBMIT_INFO: u32 = 1_000_207_003;
const ST_SEMAPHORE_WAIT_INFO: u32 = 1_000_207_004;
/// `VkSemaphoreType`:VK_SEMAPHORE_TYPE_TIMELINE = 1(BINARY = 0)。
const SEMAPHORE_TYPE_TIMELINE: u32 = 1;
// GPU 时间戳(evidence 面,§H):
const ST_QUERY_POOL_CREATE_INFO: u32 = 11;
const QUERY_TYPE_TIMESTAMP: u32 = 2;
const QUERY_RESULT_64_BIT: u32 = 0x1;
const QUERY_RESULT_WAIT_BIT: u32 = 0x2;
```

## B. 新增结构(repr(C) + size/align 锚)

```rust
/// `VkSemaphoreTypeCreateInfo`(1.2 core;挂 `SemaphoreCreateInfo.p_next`)。
/// 布局:sType@0 +pad → pNext@8 → semaphoreType@16 +pad → initialValue@24 ⇒ 32B align 8。
#[repr(C)]
struct SemaphoreTypeCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    semaphore_type: u32,
    initial_value: u64,
}

/// `VkTimelineSemaphoreSubmitInfo`(挂 `SubmitInfo.p_next`;wait/signal 值数组与
/// SubmitInfo 的 semaphore 数组一一对应,binary 语义槽位填 0 忽略)。48B align 8。
#[repr(C)]
struct TimelineSemaphoreSubmitInfo {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_value_count: u32,
    p_wait_semaphore_values: *const u64,
    signal_semaphore_value_count: u32,
    p_signal_semaphore_values: *const u64,
}

/// `VkSemaphoreWaitInfo`(host 侧 `vkWaitSemaphores`;flags=0 = wait all)。40B align 8。
#[repr(C)]
struct SemaphoreWaitInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    semaphore_count: u32,
    p_semaphores: *const VkSemaphore,
    p_values: *const u64,
}
```

锚单测(既有 `size_of` 锚段追加):

```rust
assert_eq!(size_of::<SemaphoreTypeCreateInfo>(), 32);
assert_eq!(size_of::<TimelineSemaphoreSubmitInfo>(), 48);
assert_eq!(size_of::<SemaphoreWaitInfo>(), 40);
assert_eq!(ST_SEMAPHORE_TYPE_CREATE_INFO, 1_000_207_002);
```

## C. 新增 fn 指针类型(1.2 core,device 级取址;缺失即能力探测已挡)

```rust
/// 同 `vkWaitSemaphores`(device, &SemaphoreWaitInfo, timeout_ns)→ VkResult。
type FnWaitSemaphores =
    unsafe extern "system" fn(VkDevice, *const SemaphoreWaitInfo, u64) -> i32;
/// 同 `vkGetSemaphoreCounterValue`(device, semaphore, &mut value)→ VkResult。
type FnGetSemaphoreCounterValue =
    unsafe extern "system" fn(VkDevice, VkSemaphore, *mut u64) -> i32;
```

## D. 探测:`probe_async_queue_caps`(#62 硬前置;复用 27656 段 feature 链逻辑)

```rust
/// 异步车道能力探测(instance/physical device 级,不建 device;真值不进 golden)。
pub struct AsyncQueueCaps {
    /// `timelineSemaphore` feature(1.2 core;链式查询,扩展缺席恒 false)。
    pub timeline_semaphore: bool,
    /// 首个 compute-only family(COMPUTE 且非 GRAPHICS;真异步族)。None = 仅共享族。
    pub compute_only_family: Option<u32>,
    /// 该族 `timestamp_valid_bits`(重叠量 evidence 有效性;0 = 该队列禁时间戳)。
    pub compute_only_timestamp_bits: u32,
    /// graphics 族 index(现选族逻辑同律:首个含 GRAPHICS 位)。
    pub graphics_family: u32,
}

pub fn probe_async_queue_caps(/* gipa 或复用 DeviceCapabilityReport 采集点 */)
    -> Result<AsyncQueueCaps, String>
{
    // 1) vkGetPhysicalDeviceQueueFamilyProperties 全枚举;
    // 2) compute_only = flags & COMPUTE != 0 && flags & GRAPHICS == 0(取首个);
    // 3) timeline feature:复用 PhysicalDeviceTimelineSemaphoreFeatures 单链查询
    //    (vk.rs 27656-27785 现成逻辑,探测归探测——本函数把结果反哺 device 创建判据)。
    todo!("实施窗落地")
}
```

判据:`timeline_semaphore && compute_only_family.is_some()` 才走双队列;否则显式回落
(harness 重编译 `enable_async=false`,receipt 记 `fallback_reason`)。

## E. harness 专用 device 创建(镜像 TIRT 并行上下文先例 vk.rs:1033-1241,既有入口 0-byte)

与 `create_tirt_vulkan_device` 的三处差异:

1. `DeviceCreateInfo.p_next` 挂 feature 链(TIRT 为 null):

```rust
let mut feat_tl = PhysicalDeviceTimelineSemaphoreFeatures {
    s_type: ST_PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES, // 既有常量 27309
    p_next: std::ptr::null_mut(),
    timeline_semaphore: 1, // 探测为 true 才启;探测 false 不建此 device(回落)
};
// dci.p_next = &raw mut feat_tl(api 1.2+;1.1 设备走 VK_KHR_timeline_semaphore 扩展名启用)
```

2. 队列对 = `graphics_family` + `compute_only_family`(TIRT 为「各取首个含位」,常同族);
   两条 `DeviceQueueCreateInfo` 恒两条(异族已由探测保证)。
3. api_version 至少 1.2(timeline core);探测 `blob.api_version < 1.2` 时如实回落(不做扩展腿,
   诚实边界登记——RTX 4070 Ti 环境恒 1.3+)。

## F. `create_timeline_semaphore`

```rust
/// 创建 timeline semaphore(initial_value 起点;单条,值域 = FencePair.value 映射 2v-1/2v)。
unsafe fn create_timeline_semaphore(
    device: VkDevice, create: FnCreateSemaphore, initial_value: u64,
) -> Result<VkSemaphore, String> {
    let tci = SemaphoreTypeCreateInfo {
        s_type: ST_SEMAPHORE_TYPE_CREATE_INFO,
        p_next: std::ptr::null(),
        semaphore_type: SEMAPHORE_TYPE_TIMELINE,
        initial_value,
    };
    let sci = SemaphoreCreateInfo { /* 既有结构 */ s_type: ST_SEMAPHORE_CREATE_INFO,
        p_next: (&raw const tci).cast(), flags: 0 };
    // vkCreateSemaphore(device, &sci, null, &mut sem) → VK_SUCCESS 判
    todo!("实施窗落地")
}
```

## G. 提交面:`submit_async_lanes`(消费 harness 段切分产物,禁二次推导)

```rust
/// 段结构与 g31_async_lanes_probe::SubmissionSegment 字面同构(执行器逐字消费,
/// 镜像 RXS-0240「逐字重放」纪律)。
/// 每段一次 vkQueueSubmit:
///   SubmitInfo.p_next = &TimelineSemaphoreSubmitInfo {
///       wait values  = seg.wait_points(dst stage mask = COMPUTE/ALL_COMMANDS 按队列),
///       signal values = seg.signal_points,
///   }
///   semaphore 数组 = 同一条 timeline 句柄重复 len(points) 次。
/// 跨车道屏障折分(PLAN §2.1-4):sync_before ≠ 本段车道的屏障,before 侧折至生产段末
/// (release),after 侧留本段首(acquire);判档窗 CONCURRENT sharing 简化臂 acquire 侧
/// 仅 layout transition(src stage = NONE,内存依赖由 semaphore 全量给出)。
/// 帧末:vkWaitSemaphores(timeline, 终值 = 2 * fences.len()) 替代 QueueWaitIdle。
```

## H. GPU 时间戳(evidence-only;#54 剖析面可复用)

```rust
// vkCreateQueryPool(QUERY_TYPE_TIMESTAMP, count = 2 * 段数);
// 每段:段首/段末 vkCmdWriteTimestamp(TOP/BOTTOM_OF_PIPE);
// 帧末:vkGetQueryPoolResults(64_BIT | WAIT_BIT)→ ticks;
// ns = ticks * limits.timestampPeriod(f32;props blob 偏移落地时以 vulkan_core.h 实测,
//      CAP_LIMITS_BASE=296 段先例 vk.rs:27336-27345);
// compute-only 族 timestamp_valid_bits == 0 时该臂 SKIP=not-triggered(不充 measured)。
// receipt:overlap_ms / overlap_ratio / frame_ms_median_{on,off} / noise_floor。
```

## I. 不做清单(本 diff 明确不含)

- 不动既有任何入口/既有 device 创建(TIRT/render_exec/present 全 0-byte);
- 不加 `PassSpec.queue` 字段(RHI/语言面维持 RXS-0239 缺省形态;多队列只在 G5 图执行器面);
- 不做 transfer 队列(#91)/多 timeline(#64)/VkEvent(#92)/跨帧 async post(#61);
- 不进 CI gate,不产 canonical 产物;新 unsafe 折叠 U26/U27 既有审计边界(graphics FFI 边界内,
  预期 0 新号,确有新边界才领)。

## J. 试编说明

本窗 vk.rs 禁改 ⇒ 未在树内试编;结构布局(32/48/40B)按 `vulkan_core.h` 定义手推并与既有
`PhysicalDeviceTimelineSemaphoreFeatures`(24B 锚,vk.rs:27836)同法校验;`repr(C)` 对齐律与
vk.rs 全部既有 FFI 结构一致(sType@0 + pad + pNext@8 先例)。落地 PR 首件事 = sType/size 锚单测
(§B)+ validation layer 冒烟。
