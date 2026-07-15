# unsafe-audit: rurix-rt-cabi(宿主编排 rxrt C ABI 运行时边界)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6(无注册条目的 unsafe 是 CI 错误);
> 14 §2 常驻集 unsafe-audit 完整性。MS1.2 激活(RFC-0009 §4.3:host `.rx` codegen ↔
> rurix-rt 的 staticlib C ABI 绑定,G-MS1-1)。
> 决策依据:D-113(FFI 战略:C ABI 唯一,Windows x64 唯一 ABI)、D-231(销毁纪律:
> free 前 sync)、RXS-0193(运行期确定性失败 / poisoned,对齐 RXS-0077)、RXS-0194
> (rxrt C ABI 边界;**含义冻结、非语言 ABI**,RXS-0180 L3 口径)。档位 = Full RFC
> **RFC-0009**(Agent Approved 2026-07-14,🔒 §4.3/§4.4 FFI ABI 面)。

## 范围与豁免

- crate:`src/rurix-rt-cabi`(`[lints.rust] unsafe_code = "allow"`;
  `undocumented_unsafe_blocks = "deny"` 维持——每个 unsafe 块 / `unsafe impl` 强制
  `// SAFETY:`)。
- 全仓其余 crate 维持 `unsafe_code = "deny"`(根 workspace 默认),不受影响。
- unsafe 全部集中于**调用方指针契约边界**(C ABI 入参裸指针 → 拷贝/切片/CStr 视图)
  与 **!Send 句柄跨线程存表豁免**(`unsafe impl Send`);实际 CUDA FFI 原语在 rurix-rt
  (U1~U22),本 crate 经其 safe API(pipeline shared 族 + fatbin 装载协商)消费,不
  重复其义务。C ABI 入口保持 **safe `extern "C"` 签名**(导出符号面;裸指针解引用契约
  见下表,函数级 `#[allow(clippy::not_unsafe_ptr_arg_deref)]` 携注释豁免)。

## 原语清单与验证义务(RustBelt 式)

| # | 原语 | 位置 | 验证义务(SAFETY 不变量) |
|---|---|---|---|
| U25 | C ABI 导出属性 `#[unsafe(no_mangle)] pub extern "C" fn rxrt_*`(staticlib 符号面) | `src/rurix-rt-cabi/src/lib.rs` 全部 `rxrt_*` 入口 | 符号以 `rxrt_` 前缀唯一,与既有 C ABI 导出(`rurix_uc01_*` RXS-0125 / `rurix_engine_*` RXS-0149)不冲突(no_mangle 符号唯一性);签名 = RFC-0009 §4.3 冻结含义(u64 句柄表 + 标量按值 + 裸指针),句柄 `0` = 无效/失败;运行期失败 → stderr 确定性诊断 `RXRT: error op=<op> detail=<...>` + 失败值(0 / 负 i32 / null),**不 panic 越过 C ABI**;任何 CUDA 失败置位所属 ctx poisoned,后续该 ctx 系操作确定性失败(RXS-0193,无 UB 无静默降级) |
| U25 | 嵌入产物描述表解析(`copy_nonoverlapping` 头 48 字节 + `from_raw_parts` PTX/cubin 载荷) | `artifacts.rs` `parse` ← `lib.rs` `rxrt_ctx_create` | `desc` 为 codegen 发射的 `@__rx_gpu_artifacts` 常量地址(RFC-0009 §4.4),指向 ≥48 字节 v1 布局(little-endian);`ptx_ptr`/`cubin_ptr` 指向同产物常量段、长度 = 对应 `*_len`、进程生命期有效;null / 版本不符 / 缺 PTX / 坏 sm 键 / 非 UTF-8 在解引用载荷指针**之前**确定性拒绝;载荷即拷贝为 owned(`String`/`Vec<u8>`),不持外部指针越出调用 |
| U25 | 调用方缓冲视图(`from_raw_parts(_mut)` src/dst/slots/kinds + `CStr::from_ptr` entry 名) | `lib.rs` `rxrt_buf_upload` / `rxrt_buf_download` / `rxrt_launch` | 指针一律先判非 null;`src`/`dst` 指向 `bytes` 字节有效可读/可写主机内存且调用期存活(`bytes` 与缓冲分配字节数不匹配在触 CUDA 前确定性拒绝);`entry` 为 NUL 终止字符串常量(codegen 以 device MIR 同源 mangle 名发射);`slots`/`kinds` 为长度 `n_args` 平行数组,读入即拷贝为 owned Vec;launch 实参物化纪律镜像 interop.rs `AcquiredFrame::launch`(U7 调用方义务):slot 存储 `storage` 先固定(地址稳定),`params` 指向各 slot,存活至 `cuLaunchKernel` 返回;buffer 位换设备指针前校验句柄存在且所属 ctx 与 stream 一致 |
| U25 | !Send 句柄跨线程存表豁免(`unsafe impl Send for SendStream/SendModule/SendPinned`) | `lib.rs` 句柄表包装 | CUstream/CUmodule 为绑 context 的进程级驱动对象、锁页指针为进程级主机内存(镜像 U13/U14 论证);每个 cabi 操作先经 `SharedContext::bind` 重绑 current context 再触 CUDA;内层类型各持 `Arc<SharedInner>` 保证 context 存活、Drop 自行重绑 current 后释放(单一所有权,Drop 仅一次);句柄表 `Mutex` 全程互斥,存表仅 move 语义、无跨线程共享 `&`(仅 `Send` 豁免,不豁免 `Sync`) |

> **rurix-rt 侧配套**:module 惰性缓存消费 `pipeline.rs Bound::load_module_artifacts`
> (U22 原语的 shared 族镜像;`SharedModule` 自持 `Arc<SharedInner>`,`'static` brand 供
> 句柄表跨调用缓存),见 [`rurix-rt.md`](rurix-rt.md) U22 行。

## 销毁纪律(D-231 镜像;RXS-0193)

- `rxrt_ctx_destroy`:先 `bind` + `cuCtxSynchronize` 再落表(poisoned ctx 跳过 sync 直接
  落表;sync 失败诊断后仍销毁,镜像 `Context::drop` best-effort);落表 Drop 序 = module
  卸载 → `SharedContext`(`Arc` 引用计数)。存活的 stream/buffer 仍各持 `Arc<SharedInner>`,
  primary context 不早于其资源释放;其后续操作因 ctx 条目不存在而确定性失败。
- `rxrt_buf_free`:free 前对**所属 ctx** 做 sync(封口 launch 异步窗口,防 in-flight UAF);
  sync 失败 → poison + 诊断,仍落表释放。`DeviceBox` Drop 自行重绑 current 后 `cuMemFree`
  (U13/U3)。
- `rxrt_pinned_free`:直接落表(`rxrt_*` v1 面无异步搬运,无 in-flight pinned 窗口;
  `PinnedBox` Drop 重绑后 `cuMemFreeHost`)。
- poisoned:任何 CUDA 失败置位所属 ctx,后续该 ctx 系操作(sync / create / alloc /
  upload / download / launch / pinned_ptr)全部确定性失败(诊断 + 失败值);destroy/free
  类清理操作仍可落表(不泄漏)。

## 测试

- `cargo test -p rurix-rt-cabi`:host-only(描述表解析接受/拒绝面、未知句柄确定性失败、
  畸形 create 不触 GPU)+ GPU 真跑(saxpy 全链路句柄表往返逐元素精确核对、poisoned
  传播;无 GPU 降级 SKIP,镜像 rurix-rt `gpu_roundtrip` 探测纪律)。
- 端到端(host `.rx` 单源 → EXE → `rxrt_*`)见 `ci/host_orch_smoke.py`(步骤 52,MS1.2
  实现 PR 接线)+ MS1_CONTRACT §8 run URL 归档。
