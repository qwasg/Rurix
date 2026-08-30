# Rurix 渲染器 SDK — 语义化版本与破坏性变更政策（v1）

> G31+ 波 C Task C1（G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #48「渲染器 SDK 稳定
> API 面：embedding API 语义化版本 + stable 快照守卫扩展到渲染器面」）交付件。
> 适用面 = `rurix_renderer.dll` 的 C ABI 导出集（`apps/g31-renderer-sdk/src/sdk.rx`
> 经 `rurixc --emit=dll` 产 DLL + import lib + 生成头 `rurix_renderer.h`；
> 生成头自始生成、不手写，RXS-0253/0254）。实现层 `rurix-renderer-sdk` cdylib
> 的 `rxsdk_*` 符号面是**内部实现面**（镜像 RXS-0194 `rxrt_*` 口径：含义冻结、
> 非用户 stable ABI），用户面仅以本文件 + 生成头为准。

## 1. 版本号

- 形态：`MAJOR.MINOR.PATCH` 语义化版本；`rurix_renderer_abi_version()` 返回
  打包值 `major << 16 | minor << 8 | patch`（v1 = **1.0.0** = `0x00010000`）。
- 单一事实源 = `src/rurix-renderer-sdk/src/lib.rs` `ABI_VERSION_PACKED` 常量；
  .rx 面逐字转发、stable 快照 `renderer_sdk` 段程序读 sdk.rx 转发关系锚定、
  本文件字面 = 其四面的同一字面。
- 宿主兼容裁决：`(abi_version() >> 16)` 与自身构建期 MAJOR 不等 → 不得继续
  调用（MAJOR 语义见 §2）。

## 2. 变更政策

| 档 | 含义 | 例子 |
|---|---|---|
| MAJOR | **破坏性变更**（删除/改名导出、签名收窄或语义不兼容、状态码含义变更） | 须先 RFC（10 §3 判档；FFI ABI 面触 AGENTS 硬规则 5 取严）；新旧 MAJOR DLL **并存分发**（`rurix_renderer.dll` 名称随 MAJOR 加后缀，如 `rurix_renderer2.dll`），同进程可共存；旧 MAJOR 维持安全修复至次 MAJOR 发布 |
| MINOR | **同 MAJOR 内加性扩展**（新增导出函数、新增能力位、新增只读查询） | 只增不破坏（镜像 RXS-0180 L2「同一 edition 内 stable 面只增不破坏」纪律）；既有导出签名/语义/状态码 0-byte；生成头随之加性变长，stable 快照经 `RURIX_BLESS=1` 重 bless + bless_log 追加（RD-008 机制渲染器面延伸，见 §4） |
| PATCH | **语义不变修复**（实现层缺陷修复、性能改进、诊断文案改进） | 导出集/生成头 0-byte；快照不变 |

- **预 1.0 纪律**：v1 即 1.0.0——本面以「bistro 生产管线 canonical 序列末帧
  digest 与 Stage A 锚 `bistro-interior_t100_tsr_device` 位级对拍」为语义锚
  （机器证明 SDK 面 ≡ 生产管线），首版即按 stable 纪律治理。
- **快照守卫**：导出集（名字 + 签名）与 ABI 版本纳入 `ci/stable_snapshot.py`
  `renderer_sdk` 段快照比对；任何变更（含 MAJOR 演进）都表现为快照漂移，须
  经 bless 程序留痕——破坏性变更因此**不可能静默发生**。

## 3. v1 语义面（导出集闭集，10 函数）

| 函数 | 语义 |
|---|---|
| `rurix_renderer_abi_version() -> u32` | 打包 ABI 版本（§1） |
| `rurix_renderer_caps_probe() -> u64` | 轻量能力探测：bit0 = Vulkan loader 可用；完整能力链（ray query 四扩展 + sync2）协商归 create/load_scene fail-closed |
| `rurix_renderer_create(backend: u32) -> u64` | 创建会话（backend 闭集 0=auto/Vulkan）；0 = 失败 |
| `rurix_renderer_load_scene(...) -> i32` | 场景提交：bistro 生产契约（digest 门 == FROZEN）+ gltf 装配 + canonical SPV 四件套 + 统一四 pass TSR 车道创建（能力链 fail-closed） |
| `rurix_renderer_set_camera(...) -> i32` | 相机全量替换（eye/fwd/up + fov_y_rad + near/far；192B 帧参数 uniform 通路逐帧生效） |
| `rurix_renderer_set_exposure_ev100(r, ev100) -> i32` | 曝光更新（128B TSR 参数通路逐帧生效） |
| `rurix_renderer_render_frame(...) -> i32` | 渲染一帧（canonical 确定性序列）；`readback != 0` 帧回读 TSR 输出算 digest 写宿主缓冲 |
| `rurix_renderer_present(r) -> i32` | present 句柄：**v1 = 离屏会话呈现完成计数**（真窗口 swapchain present 归后续 MINOR 加性，不冒充） |
| `rurix_renderer_destroy(r) -> i32` | 关闭会话（资源逆序回收；无效/双 destroy → 状态码 2） |

状态码闭集：0 成功 / 2 句柄无效·状态错序 / 3 输入越界 / 4 资产缺失 /
5 device·能力链缺失 / 6 渲染执行失败 / 7 validation ERROR 计数非零。
句柄 `0` = 无效/失败。资源句柄对宿主恒不透明（u64；跨堆所有权不越界——
宿主缓冲一律调用方分配，DLL 不分配-并-返回，RXS-0255 口径）。

## 4. 治理锚定

- **stable 快照**：`tests/stable/stable_api.snapshot` `renderer_sdk` 段
  （G31+ 波 C Task C1 扩展；RD-008 closed 机制的渲染器面延伸——处置登记见
  registry/deferred.json RD-008 history 2026-08-25 行，字面不冒充）。
- **RD-036 判档**：v1 导出签名全落 export_c subset v1（标量 + 裸指针 + unit）；
  超界四项（repr(C) struct 按值 / 回调指针 / 数组按值 / 跨堆所有权）逐项不
  触——判档不成立、RD-036 维持 open 登记（registry/deferred.json RD-036
  history 2026-08-25 行）。
- **unsafe 注册**：实现层 FFI 边界 = unsafe-audit/rurix-renderer-sdk.md（U-59）。

## 5. v1 已知限制（诚实登记，不冒充）

1. **共享车道体的 CLI `fail`（process::exit）路径**：`g14_3_lane_body.rs` 为
   bin 历史面，`UnifiedLaneBits::load` 等缺件路径走进程退出。SDK 面以前置
   校验收口（SPV/资产/契约缺件 → 状态码 4，先验后调），使该路径**经 SDK
   面不可达**；残余理论窗口（装载后文件被并发删除的竞态）如实登记——共享体
   0-byte 纪律（digest 锚定逻辑禁动）下不在本任务改造，归后续 MINOR 评估。
2. **会话间串行化**：句柄表 `Mutex` 全程互斥含 GPU 工作段（镜像
   rurix-rt-cabi 先例）；多会话并发渲染归后续 MINOR。
3. **present 离屏语义**：v1 无真窗口 swapchain（G31 波 A `g31_window_present`
   的 win32 present 面为 bin 形态）；窗口 present 句柄归后续 MINOR 加性。
4. **契约 digest 冻结面**：v1 场景提交接受与 FROZEN 锚 digest 相等的生产契约
   （bistro-interior/cornell-box 双场景行均在契约内，`scene` 参数选择）；
   自定义契约/场景（非冻结 digest）归后续 MAJOR/MINOR 评估（digest 门语义
   本身属 stable 面）。

## 6. G35-8 粒子作者面登记（2026-08-27；RFC-0049 §4.11 冻结四签名——内部实现面加性）

- **新增内部符号（4，`rxsdk_*` 实现面）**：`rxsdk_particles_emitter_create(sys,
  desc_json, len, out)` / `rxsdk_particles_emitter_set_param(h, key, klen, value)` /
  `rxsdk_particles_emitter_destroy(h)` / `rxsdk_particles_stats(sys, out)`——u64
  句柄 + i32 状态码复用 §3 闭集语义（0/2/3），单线程 apartment 同既有；悬空
  句柄/空指针 fail-closed。按本文件前言口径，`rxsdk_*` 为**内部实现面、非用户
  stable ABI**——本批对 stable 快照 **0 漂移**（快照只读 sdk.rx 导出集 +
  `ABI_VERSION_PACKED`，两者本批 0-byte，`ci/stable_snapshot.py --check` 维持绿）。
- **v1 粒子臂 = host 臂（诚实登记）**：emitter 资产（JSON 十字段闭集，
  `src/rurix-render/src/particles/emitter_asset.rs` fail-closed 解析）→ host
  金标准粒子系统（`particles/core.rs`，每 emitter 独立池 cap=4096/dt=1/60/
  seed 冻结常量）随 `rxsdk_renderer_render_frame` 每成功帧 tick 一帧；
  `rxsdk_particles_stats` 写 alive_total(u64)。**device 车道接线归 G35 收口批**，
  不冒充；无 emitter 的会话 render_frame 行为与前版逐字一致（既有面 0-byte）。
- **用户面 MINOR 流程登记为待 G35 收口批（如实登记，禁伪造）**：
  `rurix_renderer_particles_*` 薄转发（sdk.rx）+ 生成头再生 + **MINOR
  1.0.0 → 1.1.0** + stable 快照 `RURIX_BLESS=1` 重 bless + bless_log 追加
  （§2 MINOR 行流程）**本批不执行**——理由：五面 `1.0.0`/9 导出字面当前冻结于
  g31.waveC 门族（`ci/g31_renderer_sdk_smoke.py` EXPECTED_EXPORTS=9/0x00010000、
  `ci/g31_support_policy_smoke.py` EXPECTED_ABI_VERSION=1.0.0 + export_count=9、
  `docs/renderer/support_policy.md` §4.1），该三文件不在 G35-8 名下文件集；
  先行 bump 而不同批更新上述门字面 = 破坏既有门可复跑绿（违 §2「只增不破坏」
  精神）。收口批执行序：sdk.rx 四薄转发 + `ABI_VERSION_PACKED` → 1.1.0 +
  重 bless + bless_log 行 + 同批更新 waveC 门族版本字面。v1.0.0 用户面导出集
  本批未变，版本号维持 1.0.0 即为其正确语义（MINOR 与用户面导出集变更同批，
  §2 定义字面）。
