I have everything I need. Here is the implementation design for **RXS-0206 Backend/GpuDevice trait 抽象**, grounded in the four files read.

---

## 0. 设计判断:抽象粒度

两个后端的运行时形态**不对称**,这决定了最小可行面:

- **CUDA**(`lib.rs`/`pipeline.rs`):细粒度、持久句柄——`Context`(affine 根)→ `load_module` → `function` → `alloc`/`copy_from_host` → `Stream::launch`(`grid/block` + `&mut [*mut c_void]` params)→ `synchronize` → `copy_to_host`。
- **Vulkan**(`vk.rs:551` `run_compute`):**单发一次性**——一个调用内建 instance/device/queue → shader module → pipeline → descriptor+push constant → dispatch → `vkQueueWaitIdle` → 回读,全部句柄线性创建即销毁。vk.rs **没有任何持久句柄类型**。

因此「最小可行」的公共面必须落在 **一次 compute dispatch(one-shot)** 的粒度上——这是 vk 已经支持、且 CUDA 能用**纯组合既有 public API**表达的唯一交集。更细的 `Module`/`Buffer` 关联类型面(会强制把 vk.rs 拆成持久 Instance/Device/Buffer 类型)属 churn 过大,划归 **RXS-0208 marshalling ABI** 后续分片。句柄以关联类型 `Session` 暴露(CUDA=拥有 `CUcontext` 的 `Context`;Vulkan=ZST,一次性自建)——满足「句柄用关联类型」且零成本区分。

---

## 1. `trait ComputeBackend` 定义(新文件 `src/rurix-rt/src/backend.rs`)

```rust
//! Compute 后端抽象(RXS-0206;RFC-0011 §4.7)。把「跑一个 compute:artifacts→module→
//! buffers→launch→readback」收敛为单一 trait,CUDA 收敛为一实现、Vulkan 为并列实现。
//! **纯 host 薄层,零 unsafe**(组合各后端安全 public API + 安全字节转换)——不入 unsafe-audit。

#[cfg(feature = "vulkan")]
use crate::vk;

/// 一次 compute dispatch 的后端无关描述(in/out 原位回写)。
pub struct ComputeJob<'a> {
    /// 编译产物字节:CUDA = PTX 文本(UTF-8);Vulkan = SPIR-V 字流小端字节(len%4==0)。
    pub artifact: &'a [u8],
    /// 入口符号名(kernel 名 / OpEntryPoint 名;codegen mangled 符号)。
    pub entry: &'a str,
    /// StorageBuffer / kernel 指针实参对应 host 数据(in/out,原位回写);
    /// 顺序 = (set 0, binding i) / kernel 指针形参序。
    pub buffers: &'a mut [Vec<u8>],
    /// 标量实参块:Vulkan = push constant 块字节;CUDA = 尾随标量(marshalling 归 RXS-0208)。
    pub scalars: &'a [u8],
    /// 工作组数([x,y,z];= vkCmdDispatch / cuLaunchKernel grid)。
    pub groups: [u32; 3],
    /// 每工作组线程数([x,y,z]);Vulkan 侧 LocalSize 编码在 SPIR-V 内运行期忽略,CUDA 为 block。
    pub block: [u32; 3],
}

/// 统一后端错误(host 侧收敛;P-01 fail-closed,无静默 fallback)。
#[derive(Debug)]
pub enum BackendError {
    /// 请求后端未编译进本二进制(feature 缺失)。
    NotCompiled(&'static str),
    /// RURIX_BACKEND 未知取值。
    Unknown(String),
    /// 后端执行失败(CUDA driver / Vulkan runtime 诊断)。
    Run(String),
}

/// Compute 后端(RXS-0206):open 建会话 → dispatch 跑一个 compute。
pub trait ComputeBackend {
    /// 后端持久句柄(**关联类型**):CUDA = 拥有 `CUcontext` 的 [`crate::Context`];
    /// Vulkan = ZST(instance/device 每次 dispatch 由 `vk::run_compute` 一次性自建)。
    type Session;

    /// 后端稳定标识("cuda" / "vulkan";诊断/选择器)。
    fn name(&self) -> &'static str;

    /// 建立会话(打开 device/context)。fail-closed:不可用 → `Err`。
    fn open(&self) -> Result<Self::Session, BackendError>;

    /// 一次 compute:artifact→module→buffers 上传→launch→同步→回读(`job.buffers` 原位回写)。
    fn dispatch(
        &self,
        session: &Self::Session,
        job: &mut ComputeJob<'_>,
    ) -> Result<(), BackendError>;
}
```

trait 因关联类型 `Session` 不同而**非 object-safe**——这是刻意的最小取舍:静态用途直接用 trait(零成本单态化),运行期选择用下方 `run_job`/枚举分派。

---

## 2. 两个实现

### CUDA(最小 churn,纯组合 `lib.rs` public API,**零改 Context/Stream**)

```rust
/// CUDA 后端(RXS-0206;组合既有 Context/Module/Stream/DeviceBuffer public API,零改其类型)。
pub struct CudaBackend;

/// CUDA 会话 = 拥有 `CUcontext` 的 affine 根(可跨多次 dispatch 复用)。
pub struct CudaSession {
    ctx: crate::Context,
}

fn rr(e: crate::CudaError) -> BackendError {
    BackendError::Run(format!("{e:?}"))
}

impl ComputeBackend for CudaBackend {
    type Session = CudaSession;

    fn name(&self) -> &'static str { "cuda" }

    fn open(&self) -> Result<CudaSession, BackendError> {
        Ok(CudaSession { ctx: crate::Context::new().map_err(rr)? })
    }

    fn dispatch(&self, s: &CudaSession, job: &mut ComputeJob<'_>) -> Result<(), BackendError> {
        let ptx = core::str::from_utf8(job.artifact)
            .map_err(|_| BackendError::Run("CUDA artifact 非 UTF-8 PTX".into()))?;
        let module = s.ctx.load_module(ptx).map_err(rr)?;      // lib.rs:202
        let kernel = module.function(job.entry).map_err(rr)?;  // lib.rs:514
        // buffers→设备内存 + H2D(binding 序)。
        let mut bufs = Vec::with_capacity(job.buffers.len());
        for host in job.buffers.iter() {
            let mut d = s.ctx.alloc::<u8>(host.len().max(1)).map_err(rr)?; // lib.rs:136
            d.copy_from_host(host).map_err(rr)?;                            // lib.rs:381
            bufs.push(d);
        }
        // 实参 marshalling:buffer 设备指针(binding 序)已装配;标量块细节 = RXS-0208
        // (本条只承诺 buffer orchestration + geometry;scalars 由 RXS-0208 形式化 ABI 装配)。
        let mut ptr_store: Vec<crate::sys::CuDevicePtr> =
            bufs.iter().map(|b| b.device_ptr()).collect();          // lib.rs:376
        let mut params: Vec<*mut core::ffi::c_void> = ptr_store
            .iter_mut()
            .map(|p| (p as *mut _ as *mut core::ffi::c_void))
            .collect();
        let stream = s.ctx.create_stream().map_err(rr)?;           // lib.rs:191
        stream.launch(&kernel, job.groups, job.block, &mut params).map_err(rr)?; // lib.rs:469
        stream.synchronize().map_err(rr)?;                        // lib.rs:486
        // 回读(D2H)。
        for (host, d) in job.buffers.iter_mut().zip(bufs.iter()) {
            d.copy_to_host(host).map_err(rr)?;                    // lib.rs:391
        }
        Ok(())
    }
}
```

**关键**:`CudaBackend` 只调用 `Context`/`Module`/`Stream`/`DeviceBuffer` 的**既有 pub 方法**,不新增 CUDA 类型、不触 `sys.rs`/`pipeline.rs`。标量 marshalling 诚实地划归 RXS-0208(与 spec 自身「launch marshalling = 锁区/独立条」边界一致)。

### Vulkan(1:1 委托 `vk::run_compute`,**零改 vk.rs**)

```rust
#[cfg(feature = "vulkan")]
pub struct VulkanBackend;

/// Vulkan 会话 = ZST:instance/device/queue 由 `vk::run_compute` 每次一次性自建(vk.rs:551)。
#[cfg(feature = "vulkan")]
pub struct VulkanSession;

#[cfg(feature = "vulkan")]
fn bytes_to_spirv(b: &[u8]) -> Result<Vec<u32>, BackendError> {
    if b.len() % 4 != 0 {
        return Err(BackendError::Run("SPIR-V 字节数非 4 的倍数".into())); // fail-closed
    }
    Ok(b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

#[cfg(feature = "vulkan")]
impl ComputeBackend for VulkanBackend {
    type Session = VulkanSession;

    fn name(&self) -> &'static str { "vulkan" }

    fn open(&self) -> Result<VulkanSession, BackendError> { Ok(VulkanSession) }

    fn dispatch(&self, _s: &VulkanSession, job: &mut ComputeJob<'_>) -> Result<(), BackendError> {
        let spv = bytes_to_spirv(job.artifact)?;
        vk::run_compute(&spv, job.entry, job.buffers, job.scalars, job.groups) // vk.rs:551
            .map_err(BackendError::Run)
    }
}
```

`ComputeJob.buffers: &mut [Vec<u8>]` 与 `scalars: &[u8]` 已与 `vk::run_compute(spv, entry, buffers, push_constants, groups)` 的签名逐参对齐——委托是字面 1:1,vk.rs 完全不动。

---

## 3. Backend 选择器(P-01 无隐式 fallback)

```rust
/// 运行期后端种类(选择器出参)。
pub enum BackendKind {
    Cuda,
    Vulkan,
}

/// 解析显式后端取值(纯函数,可 host 单测,不读 env)。
pub fn parse_backend(s: &str) -> Result<BackendKind, BackendError> {
    match s {
        "cuda" => Ok(BackendKind::Cuda),
        "vulkan" => {
            #[cfg(feature = "vulkan")]
            { Ok(BackendKind::Vulkan) }
            #[cfg(not(feature = "vulkan"))]
            { Err(BackendError::NotCompiled("vulkan")) } // 未编译 → 确定性 Err,非静默切换
        }
        other => Err(BackendError::Unknown(other.to_string())),
    }
}

/// 显式选定后端:`RURIX_BACKEND`(cuda|vulkan)。未设 = 默认 CUDA(核心后端,NVIDIA 零回归)。
/// **默认选择 ≠ 运行期 fallback**:选定 CUDA 后驱动不可用 → 确定性 Err,绝不自动改跑 Vulkan(P-01)。
pub fn select_backend() -> Result<BackendKind, BackendError> {
    match std::env::var("RURIX_BACKEND") {
        Ok(v) => parse_backend(&v),
        Err(_) => Ok(BackendKind::Cuda),
    }
}

/// 选定后端 → open + dispatch 一次 compute(枚举分派,绕开关联类型 object-safety)。
pub fn run_job(kind: BackendKind, job: &mut ComputeJob<'_>) -> Result<(), BackendError> {
    match kind {
        BackendKind::Cuda => {
            let be = CudaBackend;
            let s = be.open()?;
            be.dispatch(&s, job)
        }
        #[cfg(feature = "vulkan")]
        BackendKind::Vulkan => {
            let be = VulkanBackend;
            let s = be.open()?;
            be.dispatch(&s, job)
        }
        #[cfg(not(feature = "vulkan"))]
        BackendKind::Vulkan => Err(BackendError::NotCompiled("vulkan")),
    }
}
```

无隐式 fallback:唯一「默认」是**未请求时**选 CUDA(既有核心行为),不是失败后切换。未知取值 / 未编译后端一律确定性 `Err`。

---

## 4. NVIDIA 零回归保证

| 文件 | 改动 | 回归面 |
|---|---|---|
| `src/rurix-rt/src/backend.rs` | **新增**(纯 host 薄层,零 unsafe) | 无 |
| `src/rurix-rt/src/lib.rs` | **+1 行** `pub mod backend;` + re-export(见 §6) | 加声明,不改任何既有 item |
| `sys.rs` / `pipeline.rs` / `vk.rs` | **零改动** | 字节不变 |
| `Cargo.toml` | 零改动(无新依赖) | 无 |

保证链:
1. **CUDA 实现只组合既有 pub API**——不新增 CUDA 类型、不改 `Context`/`Stream`/`DeviceBuffer`/`Module` 语义,NVPTX/cubin 路完全不经过 backend.rs。
2. **默认构建(无 feature)**:backend.rs 只编 trait + `CudaBackend` + 选择器,全 host、无 unsafe;`VulkanBackend`/`bytes_to_spirv` 在 `#[cfg(feature = "vulkan")]` 下,`BackendKind::Vulkan` 臂在 feature-off 时返回 `NotCompiled` 且不引用 `vk`。`cargo build/test -p rurix-rt` 与合入前逐字节等价,既有 CUDA 测试不受影响。
3. **零 unsafe** → `[lints.clippy] undocumented_unsafe_blocks = "deny"`(Cargo.toml:51)不触发,`unsafe-audit/rurix-rt.md` 无需追加(不新增 U 号)。
4. **feature gate 与 vk 一致**:`vulkan` 默认关(Cargo.toml:22),常驻回归网 `cargo build/test/clippy --workspace` 不触 Vulkan 而绿。

---

## 5. RXS-0206 条款体要点 + 单测锚定

**条款置于 `spec/vulkan_backend.md` §2,RXS-0205 与 RXS-0207 之间**(数值序),FLS 结构、严禁 UB 节:

- **标题**:`### RXS-0206 Compute 后端抽象(Backend trait;CUDA 收敛为一实现,Vulkan 并列)`
- **Syntax**:无语言文法面(运行时/库 API 面)。
- **Legality**:
  - L1(抽象面):`trait ComputeBackend` 覆盖 `open`→`dispatch`(load_module → buffers 上传 → launch → synchronize → readback)最小 orchestration;句柄经关联类型 `Session`(CUDA=`Context` / Vulkan=一次性 ZST)零成本区分。
  - L2(P-01 fail-closed,无隐式 fallback):后端经 `RURIX_BACKEND` + feature **显式**选定;未知取值→`Unknown`、未编译后端→`NotCompiled`,失败后**绝不静默切换后端**。
  - L3(零回归硬约束):CUDA 实现只组合既有 public API 不改其类型/语义;Vulkan 实现 gate feature `vulkan`;默认构建 NVPTX 路字节不变。
- **Dynamic Semantics**:`ComputeJob{artifact, entry, buffers(in/out 原位), scalars, groups, block}`;`dispatch` 序 = `load_module`→每 buffer `alloc`+H2D→`launch`(grid=`groups`)→`synchronize`→每 buffer D2H 回写。**实参 marshalling 细节 per-backend,统一 ABI 形式化归 RXS-0208**(本条不承诺统一 marshalling,与 §1 锁区边界一致)。
- **Implementation Requirements**:
  - IR1:新增 `src/rurix-rt/src/backend.rs` 薄层,**零 unsafe**(组合安全 API + 安全字节转换),不入 unsafe-audit、不新增 U 号。
  - IR2:CUDA 实现零改 `sys.rs`/`lib.rs`/`pipeline.rs` 主体(纯组合 pub API);Vulkan 实现 1:1 委托 `vk::run_compute`。
  - IR3(锚定):≥1 `//@ spec: RXS-0206` **纯 host** 单测覆盖 trait/关联类型面存在性 + 选择器 P-01 语义(未知→Err / feature-off vulkan→NotCompiled)+ SPIR-V 字节转换 fail-closed;device 真跑证据**复用** RXS-0207(`bin/vk_saxpy`)与既有 CUDA saxpy,本条不新增 device 冒烟。
- **锚定测试**行:`src/rurix-rt/src/backend.rs` 单测(选择器语义 + trait 面 + 字节转换)。

**单测锚定(host-only,无 GPU,置 backend.rs `#[cfg(test)] mod tests`)**:

```rust
//@ spec: RXS-0206
#[test]
fn backend_trait_and_selector_surface() {
    // trait/关联类型面 + 名称收敛(CudaBackend 恒在,不依赖 GPU)。
    assert_eq!(CudaBackend.name(), "cuda");
    // 选择器 P-01:显式取值确定性映射,无隐式 fallback。
    assert!(matches!(parse_backend("cuda"), Ok(BackendKind::Cuda)));
    assert!(matches!(parse_backend("bogus"), Err(BackendError::Unknown(_))));
    #[cfg(not(feature = "vulkan"))]
    assert!(matches!(parse_backend("vulkan"), Err(BackendError::NotCompiled("vulkan"))));
    #[cfg(feature = "vulkan")]
    assert!(matches!(parse_backend("vulkan"), Ok(BackendKind::Vulkan)));
}

//@ spec: RXS-0206
#[cfg(feature = "vulkan")]
#[test]
fn spirv_byte_conversion_is_fail_closed() {
    // Vulkan artifact 字节→字流:非 4 倍数长度确定性 Err(fail-closed,不 UB)。
    assert!(bytes_to_spirv(&[0u8; 7]).is_err());
    assert!(bytes_to_spirv(&[0u8; 8]).is_ok());
}
```

`parse_backend` 与 env 读取分离,使 P-01 语义可纯 host 断言。第一 test 无 `#[cfg(vulkan)]` 也编译(恒有 CUDA 臂),满足默认构建下 RXS-0206 有 ≥1 `//@ spec` 锚点。

---

## 6. 精确改动清单

**新增文件**
- `src/rurix-rt/src/backend.rs`(LF 换行;§1–§3 全部内容 + §5 单测)——纯 host,零 unsafe。

**既有文件最小 diff**

- `src/rurix-rt/src/lib.rs`
  - 在 `pub mod pipeline;`(:20)后加一行:`pub mod backend;`(恒开,CUDA 后端不 gate)。
  - 在 `pub use pipeline::{...};` 块(:30–33)后加 re-export:
    ```rust
    pub use backend::{
        BackendError, BackendKind, ComputeBackend, ComputeJob, CudaBackend,
        parse_backend, run_job, select_backend,
    };
    #[cfg(feature = "vulkan")]
    pub use backend::VulkanBackend;
    ```
  - 注:backend.rs 内引用 `crate::sys::CuDevicePtr`,`sys` 已 `pub mod sys;`(:21),无需改。

- `spec/vulkan_backend.md`
  - §2 插入 `### RXS-0206` 条款体(§5 要点),位置在 RXS-0205(:178)与 RXS-0207(:204)之间。
  - §3 修订记录追加一行 v1.6(MB1.2 Backend 抽象:落 RXS-0206 条款体 + backend.rs;CUDA 收敛/Vulkan 并列;零 unsafe/零改 sys·lib·pipeline·vk 主体;NVIDIA 零回归;trace `191→192`),档位 Full RFC(RFC-0011)。
  - 表头用「版本」(check_guardrails 凭字面跳表头)。

**无需改动**:`Cargo.toml`(无新依赖、feature `vulkan` 已存在)、`sys.rs`、`pipeline.rs`、`vk.rs`、`unsafe-audit/rurix-rt.md`(零新 unsafe)。

**编号**:RXS-0206(0207 已落,本条补齐 0206 空位;在 §1 预留区间 RXS-0200~0213 内)。零新 RX 错误码(6xxx 段不动)、零新 U 号。

**验证命令**
```
cargo test  -p rurix-rt                      # 默认:trait + CudaBackend + 选择器 host 测绿;NVPTX 零回归
cargo test  -p rurix-rt --features vulkan     # Vulkan 后端 host 测 + bytes_to_spirv + 既有 vk 单测
cargo build -p rurix-rt --features vulkan
cargo clippy -p rurix-rt --all-targets --features vulkan   # undocumented_unsafe_blocks=deny;backend.rs 无 unsafe 通过
cargo test  --workspace                       # 常驻回归网(不触 vulkan/interop)全绿
py -3 ci/trace_matrix.py --check              # 全锚定 191→192(RXS-0206 ≥1 //@ spec;以实际基数为准复核)
```

device 真跑(NVIDIA CUDA saxpy 经 `CudaBackend` / Vulkan 经 `VulkanBackend`→`vk_saxpy`)复用 RXS-0207 既有证据链,本条不新增 GPU 冒烟。

---

**待确认项**(不阻塞设计):trace 目标基数 `191→192` 取自 `spec/vulkan_backend.md` v1.5(mb1 lineage);当前 worktree 若已叠加其他分片,以 `ci/trace_matrix.py --check` 实测基数为准。
