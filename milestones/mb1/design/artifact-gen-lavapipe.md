I have everything needed. This is a read-only design deliverable — returning it directly.

---

# mb1 实现设计:RXS-0208 marshalling + RXS-0209 artifact 泛化 + 第二 ICD

## 0. Base 现实核对(决定诚实措辞的地基)

在 mb1 base(`H:\rurix_mb1`,分支 `mb1/governance-package`,off origin/main)实测确认:

| 对象 | RFC-0011 §4.7/§4.8 假设的锚点 | mb1 base 实况 |
|---|---|---|
| `rxrt_launch` 符号 | `lib.rs:701` slots/kinds ABI | **不存在**(`grep rxrt_launch src/` 空) |
| `rurix-rt-cabi` crate | C ABI 层 | **不存在**(无该 crate 目录) |
| `artifacts.rs` @__rx_gpu_artifacts blob(v1/48B) | `src/rurix-rt/src/artifacts.rs` | **不存在**(`find src -name artifacts.rs` 空) |
| `emit_gpu_artifact_globals` | `codegen.rs:1028` | **不存在**(mb1 `codegen.rs` 只 1126 行、是 host LLVM-IR emitter,非 device blob) |
| `ArtifactKind`/`SmTarget`/`DeviceArtifactSet` | `fatbin.rs` | **存在**(RXS-0150/0151,G1.5) |
| `LockArtifact{kind,sm_target: String}` | `lock.rs:31` | **存在且已 format-generic** |

结论:RFC-0011 §4.7/§4.8 是站在「MS1.2 已合入」的 trunk 视角写的;mb1 base **没有** rxrt_launch / cabi / artifacts blob / emit_gpu_artifact_globals。因此 **RXS-0208 的「保 MS1.2 ABI」在本 base 无对象**,**RXS-0209 的「描述表 v2 blob bump」在本 base 也无对象**。这两块必须诚实降为「前瞻性兼容承诺 + honest-defer RD」,不能在 mb1 假装有 rxrt_launch 去「保兼容」,否则是伪造条款对象。

可在 mb1 base 真正落地的只有:`fatbin.rs` 的 `ArtifactKind`/`ArchKey` 泛化 + `lock.rs` 的诚实登记(零码改)+ `vk.rs` marshalling 语义的条款形式化。

编号确认:**RXS 0208/0209 已在 `spec/vulkan_backend.md` §1 预留区间登记但未落条款体**(现最高条款体 = RXS-0207/v1.5)。RD 最新已注册 = RD-026;RD-027/028=MS1 规划占用、RD-029=mesh/task/RT(mb1),**下一可用 = RD-030、RD-031**。

---

## 1. RXS-0209 artifact 泛化 — 具体 enum + 全 ripple + 最小 diff

### 1.1 `ArtifactKind` 加 `Spirv`(纯加性)

`src/rurix-rt/src/fatbin.rs:17-35`:

```rust
pub enum ArtifactKind {
    Ptx,
    Cubin,
    Fatbin,
    Spirv,   // ← 新增:Vulkan 可移植 device 产物(驱动 JIT 装载,占「可移植槽」)
}
// as_str():
    ArtifactKind::Spirv => "spirv",   // ← 新增 arm
```
无破坏面:所有既有 match 都是穷举 `Ptx/Cubin/Fatbin`,加 `Spirv` 会让编译器在 fatbin.rs 测试的 `assert_eq!` 处提示补一行;lock.rs 侧 `kind` 是 String,不受影响。

### 1.2 `SmTarget` → `ArchKey`(真工作,prefix-dispatch enum)

`fatbin.rs:37-60` 现状:`SmTarget(String)` 硬编 `strip_prefix("sm_")` + `is_ascii_digit` 守卫 —— **它会拒绝 `gfx1100`**,这就是必须泛化的点。替换为:

```rust
/// device 产物架构键(RXS-0209)。NVIDIA `sm_89`(cubin AOT)/ AMD `gfx1100`(hsaco AOT)/
/// 可移植槽(驱动 JIT:Vulkan SPIR-V 或 NVPTX PTX)。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArchKey {
    Sm(String),      // "sm_89"  —— NVIDIA compute capability,per-arch AOT cubin
    Gfx(String),     // "gfx1100" —— AMD GCN/RDNA ISA,per-arch AOT hsaco
    SpirvPortable,   // 可移植槽(无 per-arch 键;lock sm_target = "")
}

impl ArchKey {
    /// NVIDIA compute capability → Sm(sm_xx)(承 RXS-0151 既有语义,零漂移)。
    pub fn from_capability(major: u32, minor: u32) -> Self {
        ArchKey::Sm(format!("sm_{major}{minor}"))
    }
    /// prefix-dispatch 解析:`sm_<digits>` → Sm / `gfx<alnum>` → Gfx / `""` → SpirvPortable。
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() { return Some(ArchKey::SpirvPortable); }
        if let Some(d) = s.strip_prefix("sm_") {
            return (!d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
                .then(|| ArchKey::Sm(s.to_owned()));
        }
        if let Some(d) = s.strip_prefix("gfx") {
            return (!d.is_empty() && d.bytes().all(|b| b.is_ascii_alphanumeric()))
                .then(|| ArchKey::Gfx(s.to_owned()));
        }
        None
    }
    /// lock `sm_target` 字面量(Sm/Gfx 回其键;SpirvPortable → "")。
    pub fn as_str(&self) -> &str {
        match self { ArchKey::Sm(s) | ArchKey::Gfx(s) => s, ArchKey::SpirvPortable => "" }
    }
}
```

设计要点(条款语义警示,直接来自 RFC-0011 §4.8):NVIDIA 模型 PTX=可移植 JIT fallback / cubin=per-arch AOT;Vulkan 世界 **SPIR-V 占可移植槽**(驱动 JIT),`gfxNNNN` AOT(AMD hsaco)占 per-arch 槽。`SpirvPortable` 与 `Ptx` 是同一「可移植槽」的两个厂商实现。

### 1.3 全 ripple 点(逐一,file:line)

| # | 位置 | 现状 | 改动(最小) |
|---|---|---|---|
| R1 | `fatbin.rs:39` | `pub struct SmTarget(String)` | 删,替为 §1.2 `ArchKey` |
| R2 | `fatbin.rs:64-67` `CubinVariant{ sm: SmTarget, bytes }` | `sm: SmTarget` | `sm: ArchKey`(字段语义扩为「per-arch AOT 变体键」;为省 churn **保留 `CubinVariant` 名**,doc 注明现承 Sm、后承 Gfx) |
| R3 | `fatbin.rs:71-72` `CubinVariant::sm() -> &SmTarget` | 返回类型 | `-> &ArchKey` |
| R4 | `fatbin.rs:99` `with_cubin(sm: SmTarget, ...)` | 形参 | `sm: ArchKey` |
| R5 | `fatbin.rs:119` `cubin_for(&self, sm: &SmTarget)` | 形参 | `&ArchKey` |
| R6 | `fatbin.rs:124` `cubin_targets() -> Vec<&SmTarget>` | 返回 | `Vec<&ArchKey>` |
| R7 | `fatbin.rs:134-139` `LoadChoice::Cubin(SmTarget)` | payload | `Cubin(ArchKey)` + **新增 `SpirvPortable` arm**(见 §1.4) |
| R8 | `fatbin.rs:147` `select_load_variant(device_sm: &SmTarget, ...)` | 形参 | `device_key: &ArchKey`(逻辑不变) |
| R9 | `lib.rs:300-302` `SmTarget::from_capability` / `LoadChoice::Cubin(sm)` | 调用 | `ArchKey::from_capability` / `LoadChoice::Cubin(sm)` 名替 |
| R10 | `bin/fatbin_saxpy.rs:19,82` `use fatbin::{...SmTarget}` / `SmTarget::parse(CUBIN_ARCH)` | import + 调用 | `ArchKey` 名替(`CUBIN_ARCH="sm_89"` → `ArchKey::Sm`,行为不变) |
| R11 | `fatbin.rs:158-218` 两个 `#[test]` | `SmTarget::` × N + `ArtifactKind` assert | 名替 + 加 `assert_eq!(ArtifactKind::Spirv.as_str(),"spirv")` + `assert!(ArchKey::parse("gfx1100").is_some())` + `parse("")==SpirvPortable` |
| R12 | `sys.rs:46,269,360,826` 注释引用 `SmTarget`/`select_load_variant` | 仅注释 | 注释名替(sys.rs 无 `SmTarget` **类型**使用,只注释提及;确认:`grep` 命中全在注释/doc) |

`SmTarget` 无 `impl Deref`/无外部 re-export 之外用法 —— ripple 收敛于上 12 点、跨 3 个源文件(`fatbin.rs` 主体 + `lib.rs` 2 处 + `fatbin_saxpy.rs` 2 处)。**NVIDIA 运行时路径(sys.rs cubin 装载)零逻辑改动**:`ArchKey::from_capability` 仍产 `Sm`,`select_load_variant` 命中逻辑同前。

### 1.4 `DeviceArtifactSet` 可移植槽 —— 加性,不动 NV `ptx_fallback`

NV 路径 `new(ptx: impl Into<String>)` + `ptx_fallback() -> &str` 依赖 `String`;SPIR-V 是字节。**不改 NV 构造签名**(否则 ripple `fatbin_saxpy.rs:80`),改为**平行加槽**:

```rust
pub struct DeviceArtifactSet {
    ptx_fallback: String,               // NV 可移植 JIT 槽(不动,字节等价)
    spirv_fallback: Option<Vec<u8>>,    // ← 新增:Vulkan 可移植槽(SPIR-V 字节)
    cubin_variants: Vec<CubinVariant>,  // per-arch AOT(Sm 现;Gfx hsaco 后续)
}
// 加性 builder / accessor:
pub fn with_spirv_fallback(mut self, spv: Vec<u8>) -> Self { self.spirv_fallback = Some(spv); self }
pub fn spirv_fallback(&self) -> Option<&[u8]> { self.spirv_fallback.as_deref() }
```

`LoadChoice` 加 `SpirvPortable` arm;`select_load_variant`:当 `device_key` 未命中 per-arch AOT 且 `set.spirv_fallback.is_some()` → `LoadChoice::SpirvPortable`,否则 `LoadChoice::PtxFallback`(NV 兜底不变)。`Default`/`new` 里 `spirv_fallback: None` → **NV-only 集行为逐字节不变**。

注:mb1 `vk.rs::run_compute` 当前直吃 `spv: &[u32]`,**不经 `DeviceArtifactSet`**。故本槽在 mb1 base 是**模型层准备**(让 lock/artifact 模型能表达 spirv 变体),把 Vulkan 运行时接到 `load_module_artifacts` 走产物集是后续分片,不在 RXS-0209。条款须诚实这么说。

### 1.5 rurix.lock `kind="spirv"`/`sm_target="gfx1100"` 已 format-generic(证)

`lock.rs:31-39` `LockArtifact{ kind: String, sm_target: String, ... }` —— 两字段皆自由 `String`;序列化 `lock.rs:113-114` `format!("kind = {}", quote(&a.kind))` 对任意串工作;解析 `lock.rs:181-184` `get("kind")?`/`get("sm_target")?` 不校验枚举值;排序键 `(package,kind,sm_target)` 字典序对 `"spirv"`/`"gfx1100"` 天然成立。**结论:`kind="spirv"` + `sm_target="gfx1100"` 零 schema/零码改动即可锁定**。唯一改动 = **doc-comment**:`lock.rs:33` 注释 `"ptx" | "cubin" | "fatbin"` → 加 `| "spirv"`;`lock.rs:35` `sm_target` 注释「cubin 预编架构键」→ 泛化为「per-arch AOT 键(`sm_89`/`gfx1100`);可移植槽为空」。lock roundtrip 测试(`lock.rs:292`)加一条 `kind="spirv", sm_target="gfx1100"` 变体断言即锚定 RXS-0209。

---

## 2. RXS-0208 marshalling clause — 诚实条款体

### 2.1 诚实性问题与解法

RFC-0011 §4.7 标题写「保 MS1.2 rxrt_launch ABI 兼容」,但 mb1 base 无 `rxrt_launch`。若条款正文声称「保 rxrt_launch ABI 字节不变」,则是对**不存在对象**的空/伪断言。诚实解法 = **两分**:

- **(A) 现在可形式化、有对象的**:vk.rs 已实现的 descriptor-binding marshalling **语义** + 它与 codegen RXS-0203 描述符布局的**单一事实源一致性**。这是本条正文(normative body)。
- **(B) 无对象、前瞻的**:MS1.2 `rxrt_launch` ABI 字节兼容 —— 写成**条件性 Implementation Requirement + honest-defer RD-030**,措辞明确「当 rxrt_launch/rurix-rt-cabi 合入本分支时」才有回归对象与测试。不假装现在在保兼容。

### 2.2 条款体要点(FLS 体例,严禁 UB 节)

**RXS-0208 launch marshalling(descriptor-binding;与 RXS-0203 单一事实源;MS1.2 ABI 前瞻兼容)**

- **Syntax**:无语言文法面(运行时/FFI 面)。

- **Legality**
  - L1(marshalling 面):运行期把 `buffers[i]` 序位 marshalling 为 `(set=0, binding=i)` StorageBuffer;标量按序位 marshalling 为单一 push-constant 块的顺排偏移。序位(ordinal)是唯一分派依据,无按名绑定、无按类型推断。
  - L2(单一事实源):运行期 (set,binding) 与 push-constant offset **必须**与 codegen RXS-0203 IR1(`binding` = buffer 形参出现序)/ IR2(push-constant member `Offset` = 标量形参出现序,4 字节顺排)产出的 SPIR-V 描述符装饰**一致**;两侧同源于形参序,非各自约定。不一致 → 见 L3。
  - L3(fail-closed):marshalling 与 SPIR-V 装饰不符时,由 Vulkan validation(如 pipeline `pName` 不符 → VUID-VkPipelineShaderStageCreateInfo-pName-00707;binding 数不符 → descriptor VUID)在运行期确定性拒绝并返回 `Err`(非 panic、非静默,承 RXS-0207 L2,P-01 strict-only)。**不占 RX 码**(运行期工具层口径,对齐 spec/release.md §3),非编译期 6xxx。

- **Dynamic Semantics**:`run_compute(spv, entry, buffers, push_constants, groups)`(`vk.rs:551`)—— `buffers[i]` → `(set 0, binding i)` StorageBuffer(in/out 原位回写,`vk.rs:954-962` 描述符布局、`vk.rs:1066-1080` write set),`push_constants` 字节整块经 `vkCmdPushConstants`(`vk.rs:1118-1127`,offset 0)喂入 shader push-constant 块(标量顺排 4 字节对齐,同 RXS-0203 IR2)。序位化 marshalling 对相同 `(buffers, push_constants)` 布局确定。

- **Implementation Requirements**
  - IR1(ordinal 映射):运行时 descriptor set layout binding `i` = buffer 序 `i`(`vk.rs:954-962`),与 codegen `classify_param` 的 `next_binding` 递增(`vulkan_codegen.rs:638-639`)**同序**;push-constant 块单块、成员按标量序 `Offset i*4`(codegen `vulkan_codegen.rs:737-743`)。两侧唯一事实源 = 形参出现序。
  - IR2(🔒 边界声明,不落本体):MS1.2 `rxrt_launch` 的 `slots[u64]+kinds[u8]` 扁平 kernelParams ABI **二进制布局**属 RFC-0011 §4.7 🔒 升档禁区;本条只声明「Vulkan 侧按 ordinal 从 slots 推导 (set,binding)/push-constant」的**映射义务**,不定义 rxrt_launch 字节布局本体。
  - IR3(前瞻兼容,条件):当 `rxrt_launch`/`rurix-rt-cabi` 合入本分支后,Vulkan backend 消费其序位化 slots 时**必须**保持 CUDA 路 `rxrt_launch` 符号面字节不变(RXS-0194「符号面只追加」口径;主选零 ABI 新增,备选 `rxrt_dispatch_*` 新符号)。**本 base 无该符号 → ABI-字节回归测试对象缺席 → honest-defer RD-030**(backfill:MS1.2 合入本分支)。
  - IR4(锚定):≥1 `//@ spec: RXS-0208`,host 可测,覆盖「ordinal→(set,binding) 映射规则 + 与 RXS-0203 codegen binding 序一致」—— 见 §4 anchor。

---

## 3. lavapipe / SwiftShader 第二 ICD — 可行性调研(不下载)

### 3.1 ICD 选择机制(确定,可直接用于 CI)

Vulkan loader 经 **ICD manifest JSON** 定位驱动。CI 里强制只用软件 ICD 的开关:

- `VK_DRIVER_FILES`(loader ≥ 1.3.207,现行首选)= 指向 ICD JSON 的绝对路径,**覆盖**系统 ICD 发现,loader 只加载所列 driver。
- `VK_ICD_FILENAMES`(旧名,仍兼容)= 同义,老 loader 用。
- 辅助:`VK_LOADER_DRIVERS_SELECT` / `VK_LOADER_DEBUG=all`(排障)。

CI 用法:`set VK_DRIVER_FILES=C:\vk-icd\lvp_icd.x86_64.json` 后跑 `bin/vk_saxpy` → 强制 lavapipe 执行同一 `.spv`,拿跨厂商数值回归(NV 真卡 + 软件 ICD 双证)。这条机制**无需任何二进制即可先在 CI 脚本里接线**(`ci/vulkan_device_smoke.py` 已是 G-MB1-3 gate)。

### 3.2 二进制获取途径(调研结论,获取本身留 follow-up)

| ICD | Windows 二进制可得性 | 获取途径(不在本轮下载) | 工程量 |
|---|---|---|---|
| **Mesa lavapipe**(`vulkan_lvp.dll` + `lvp_icd.x86_64.json`) | **可得(推荐)** | Mesa 官方不发 lavapipe Windows 正式包,但社区 `pal1000/mesa-dist-win`(GitHub releases)长期发 Windows Mesa 打包**含 lavapipe**;或 CI 内 MSYS2 `pacman -S mingw-w64-x86_64-mesa`;或 Mesa 官方 CI artifact。纯 CPU、无 GPU 依赖,适合 CI runner。 | 低:下载解包 + 设 `VK_DRIVER_FILES` |
| **Google SwiftShader**(`vk_swiftshader.dll` + `vk_swiftshader_icd.json`) | **需源码自建** | `github.com/google/swiftshader` CMake 自编(需 MSVC + CMake);无官方 prebuilt release(仅内嵌于 Chromium/Dawn/Skia CI)。 | 中高:一次性 build,产物可缓存 |

判断:**lavapipe 为首选第二 ICD**(可得性 + CPU-only + Mesa 成熟 SPIR-V 消费);SwiftShader 作为 lavapipe 不可得时的自建备选。二者皆为纯软件光栅,能验证「SPIR-V 跨非-NVIDIA 驱动可消费 + 数值一致」,但**不代替 AMD 真卡 / Android 真机**(G-MB1-6/7 open 尾门不受影响,RFC-0011 边界:软件 ICD 跑通 ≠ AMD/Android 已验证)。

### 3.3 落地方案 + DoD

- **本轮(设计/条款先行)**:`ci/vulkan_device_smoke.py` 已接线 lavapipe 分支(`CI_GATES §2.55`)。补:脚本里加「若 `VK_DRIVER_FILES` 指向的 lavapipe ICD 存在 → 跑第二遍数值对照 + validation;不存在 → 打印 `SKIP: second ICD unavailable (dev-env degrade)` 并**不**判红」。这是既有「缺工具 SKIP」纪律(对齐 RXS-0073 ptxas 干验证 SKIP),**非伪造绿**。
- **follow-up(获取二进制后转真绿)**:runner 上装 lavapipe → `VK_DRIVER_FILES=<lvp_icd.json>` → `bin/vk_saxpy` 输出 `out=a*x+out` 与 NV 结果 max_err=0 + `VK_LAYER_KHRONOS_validation` 零报错。
- **DoD(G-MB1-3 内)**:NV + lavapipe 双 ICD compute 真绿、validation 零报错、NVIDIA 零回归、run URL 归档 `MB1_CONTRACT §8`。**无新 RXS/RD**(第二 ICD 是 G-gate/CI-gate 层的取证义务,非 deferred 语义面;缺二进制 = 既有 open 尾门 + dev-env SKIP,不新造条款)。

---

## 4. RXS-0208/0209 条款体要点 + anchor 方案

| 条款 | anchor(`//@ spec:`) | 载体 | 真实红绿 |
|---|---|---|---|
| RXS-0208 | ≥1,host 可测 | `vk.rs` 新 `#[test] marshalling_ordinal_matches_codegen_binding`:构造 N buffers → 断言 binding 序 = 0..N;push-constant offset 序 = 标量序 ×4;与 `vulkan_codegen` 侧 `classify_param` 序位一致(可直接断规则,或跑 saxpy `--target vulkan` 产物核 `Binding`/`Offset` 装饰值) | device 真绿沿用 RXS-0207 `bin/vk_saxpy`(错入口名 → VUID 红 / 正确 → 绿),证 marshalling 一致 |
| RXS-0209 | ≥1,host 可测 | `fatbin.rs` 扩现有 `#[test]`:`ArtifactKind::Spirv.as_str()=="spirv"` + `ArchKey::parse("gfx1100")==Some(Gfx)` + `parse("")==SpirvPortable` + `with_spirv_fallback` roundtrip;`lock.rs` roundtrip 测试加 `kind="spirv",sm_target="gfx1100"` 变体 | 纯 host 类型,回归网不依赖 GPU 而绿(承 fatbin.rs 纪律) |

条款体两条都按 FLS 分 **Syntax/Legality/Dynamic Semantics/Implementation Requirements**,**严禁 UB 节**(P-01 strict-only)。RXS-0208 正文见 §2.2;RXS-0209 正文要点:L1 变体类别加性(Spirv)/ L2 ArchKey 三槽语义(Sm/Gfx AOT + SpirvPortable/Ptx 可移植)/ L3 lock format-generic 零码改 / IR1 fatbin.rs 加性 + ripple / IR2 描述表 v2 blob **honest-defer RD-031**(mb1 base 无 @__rx_gpu_artifacts blob / emit_gpu_artifact_globals,待 MS1.2 合入)。

**错误码**:两条**均不新增 RX 码**。RXS-0208 marshalling 不符 = 运行期 Vulkan validation 拒(工具层,不占 RX,同 RXS-0207 L2);RXS-0209 是纯 host 产物模型类型,`ArchKey::parse` 未知前缀 → `None` → 装载协商降级(非致命,同 RXS-0151),无编译期诊断。

**trace**:现锚定至 RXS-0207(spec 记 191)。本设计落 RXS-0208+0209 → **+2**(191→193);若同分片带 RXS-0206 Backend trait 另 +1。

---

## 5. 精确改动清单

**代码(加性,NVIDIA 零回归)**
- `src/rurix-rt/src/fatbin.rs`
  - `:17-35` `ArtifactKind` 加 `Spirv` + `as_str` `"spirv"` arm
  - `:37-60` 删 `SmTarget(String)`,替为 `ArchKey{Sm(String),Gfx(String),SpirvPortable}`(§1.2)
  - `:64-129` ripple R2–R6(`CubinVariant.sm`/`sm()`/`with_cubin`/`cubin_for`/`cubin_targets` 全 `SmTarget→ArchKey`)
  - `:82-86` `DeviceArtifactSet` 加 `spirv_fallback: Option<Vec<u8>>` + `with_spirv_fallback`/`spirv_fallback()`(§1.4,`new` 不动)
  - `:132-152` `LoadChoice` 加 `SpirvPortable` arm;`select_load_variant` 形参 `&ArchKey` + spirv 兜底分支
  - `:154-218` 测试名替 + 加 Spirv/Gfx/SpirvPortable/spirv_fallback 断言(**anchor RXS-0209**)+ 新 marshalling 无关
- `src/rurix-rt/src/lib.rs:300-302` `SmTarget→ArchKey` 名替(R9)
- `src/rurix-rt/src/bin/fatbin_saxpy.rs:19,82` `SmTarget→ArchKey` 名替(R10)
- `src/rurix-rt/src/sys.rs:46,269,360,826` 注释名替(R12,仅注释)
- `src/rurix-rt/src/vk.rs` 尾 `#[cfg(test)]` 加 `marshalling_ordinal_matches_codegen_binding`(**anchor RXS-0208**)
- `src/rurix-pkg/src/lock.rs:33,35` **仅 doc-comment**(`kind` 加 `"spirv"`、`sm_target` 泛化)+ `:292` roundtrip 测试加 `spirv/gfx1100` 变体(**anchor RXS-0209**);**schema 零码改**

**spec / 治理**
- `spec/vulkan_backend.md §2` 落 `### RXS-0208`(§2.2)+ `### RXS-0209`(§4 要点)条款体 + 修订表加 v1.6 行;§1 预留区间已含 0208/0209 无需改
- `registry/deferred.json` 加 **RD-030**(RXS-0208 rxrt_launch ABI-字节回归,open,backfill=MS1.2 合入本分支)+ **RD-031**(RXS-0209 描述表 v2 @__rx_gpu_artifacts blob bump + @__rx_gpu_spirv,open,backfill=MS1.2 artifacts blob/codegen 合入);承 10 §9.5 跳号(RD-027/028 MS1 占、029 mesh)
- `conformance/traceability_matrix.json` +2(RXS-0208/0209 锚点),`ci/trace_matrix.py --check` 191→193
- `ci/vulkan_device_smoke.py` 补 lavapipe `VK_DRIVER_FILES` 分支 + 缺 ICD SKIP(§3.3);无新 CI 步骤号
- **不碰**:`registry/error_codes.json`(零新 RX 码)、`vk.rs` runtime 主体逻辑、NVIDIA cubin/ptx 路径逻辑

**LF 纪律**:上述全为既有文件编辑(CRLF 例外保形,用 Edit 工具);deferred.json 尾换行 + report 无(本设计不写 report.md)。

**gating 提醒**:整个 `vulkan_backend.md` 及 mb1 实现 gated on 红线 3 解除 + RFC-0011 批准(已 Owner Approved 2026-07-15),条款落地随 Phase 2 实现 PR。RXS-0208 IR2 触 🔒 §4.7 边界声明(不落 rxrt_launch 字节本体),已在 RFC owner 批准范围内。