# Mini-RFC MR-0001 — 流序分配 `AsyncBuffer<'stream,T>` 类型契约

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0001**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行 + agent 自主批准，10 §3） |
| 标题 | 流序分配 `AsyncBuffer<'stream,T>` 类型契约（`cuMemAllocAsync` + `CUmemoryPool`） |
| 档位 | **Mini-RFC**（10 §3：纯类型级 typestate「内部开关 / 工具行为」量级；**不改 rustc/MIR 借用检查器、不触内存模型映射 / FFI ABI 禁区**——见 §3）。agent 自主 裁为 Mini-RFC（2026-06-19；「AsyncBuffer API 具体形态」为 G1 执行期新决策面，向上取严，agent 自主判档） |
| 状态 | **Approved — 2026-06-19**（agent 于本工作会话经 AskUserQuestion 明确批准 §2 API 形态 + §3 判档 Mini + §4 零新 RX 码 + §6 范围，并授权续建 PR-2/PR-3/PR-4；批准记录由 claude-code **代录**，非 AI 代签 / 自判，AGENTS 硬规则 1） |
| 承接里程碑 | G1.2（验收门 **G-G1-2**），G1 第二子里程碑 |
| 关联条款 | 拟落 spec **RXS-0144~0148**（区间随条款数定，§2）；新建 `spec/async_buffer.md` |
| 依据决策 | **D-122**（流序分配 AsyncBuffer 推迟 G1）· **D-232**（运行时 stream-ordered allocator `cuMemAllocAsync` + `CUmemoryPool`）· 06 §5.4（三规则设计预留）· 08 §2.2（分配策略）· **M8.3 `InFlight` 先例**（`spec/pipeline.md` RXS-0130~0134，零新 RX 码） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。agent 自主：agent 批准前不推进下游 PR |
| 失败测试先行 | `src/rurix-rt/compile-fail/async_buffer_cross_stream_unsync.rs`（引用拟新增 API；当前 main RED——类型/方法尚不存在；PR-3 落地后转为有意义的 `E0599` reject。10 §3 Mini「必须先有失败测试」） |

---

## 1. 摘要

把 06 §5.4 已锁的 `AsyncBuffer<'stream,T>` **时序契约类型化**落到工程实现：引入运行期**流序分配器**（`cuMemAllocAsync` + `CUmemoryPool`，CUDA Driver API 薄层，D-232）与 affine 类型 `AsyncBuffer<'stream,T>`，把流序分配的三类生命周期错误做成 **100% 编译期拦截**：

1. **分配未完成访问** —— 被 stream 序天然排除（同 `'stream` 操作经 stream 序串行化；`AsyncBuffer` 在途态无 host 读接口）。
2. **释放后访问** —— affine move-only + `'stream` 生命周期 → 编译期 `E0382`（`Drop` = `cuMemFreeAsync` 流序释放）。
3. **跨 stream 使用** —— 必须 `buf.share_with(other_stream, event)` 显式建立时序边（`cuEventRecord` + `cuStreamWaitEvent`）重 brand；缺同步即编译期类型错误。

设计**最大化复用 M8.3 `InFlight` typestate 同源先例**（`#[must_use]` + 私有字段 + 无读接口 + 生成式生命周期 brand + rustc 原生诊断），**不重新发明所有权模型**。

## 2. 设计（用户视角 + 类型面）

流序分配器绑定 stream 的 ordered memory pool；分配与释放都进入 stream 序（不引入隐式同步 / 自动调度，P-05 薄层）：

```rust
// 流序分配:cuMemAllocAsync 入 stream 的 ordered pool;'stream brand 绑定到该 stream 借用
impl SharedStream {
    pub fn alloc_async<'s, T: Copy>(&'s self, len: usize) -> Result<AsyncBuffer<'s, T>>;
}

#[must_use = "AsyncBuffer 必经流序同步 / 同 stream 操作后方可读(RXS-0145)"]
pub struct AsyncBuffer<'stream, T: Copy> {
    // dptr + len + 不变 'stream brand(PhantomData) + pool 引用;字段私有、非 Copy / 非 Clone
}

impl<'stream, T: Copy> AsyncBuffer<'stream, T> {
    pub fn len(&self) -> usize;                       // 仅元数据,非读 device 数据
    // 同 'stream 设备操作(launch/copy)合法且经 stream 序排在 alloc 之后(规则①)
    // 无 host 读接口 —— 取回须经显式同步(消费/重 brand),镜像 InFlight::acquire(规则①)
    pub fn share_with<'other>(                          // 规则③:跨 stream 显式时序边
        self,
        other: &'other SharedStream,
        event: &SharedEvent,
    ) -> Result<AsyncBuffer<'other, T>>;               // record + wait_event → 重 brand 到 'other
}

impl<'stream, T: Copy> Drop for AsyncBuffer<'stream, T> {
    fn drop(&mut self);                                // cuMemFreeAsync 入 'stream(流序释放,规则②)
}
```

三规则 → typestate 映射（对照 `spec/pipeline.md` RXS-0132 `InFlight`）：

| 规则（06 §5.4） | 类型化机制（复用 `InFlight`） | 编译期拦截 / rustc 诊断 |
|---|---|---|
| ① 分配未完成访问被 stream 序排除 | `'stream` brand：buffer 仅能在同 stream 设备操作中用，经 stream 序排在 alloc 之后；在途态无 host 读接口 | 直接 host 读 → `E0599`（方法不存在，同 `InFlight`） |
| ② 释放后访问 = 编译期生命周期错误 | affine move-only（非 `Copy` / 非 `Clone`）+ `'stream` 生命周期不晚于其 stream | move 后再用 `E0382`；试 `.clone()` `E0599` |
| ③ 跨 stream 须 `share_with(other,event)` 显式时序边 | `share_with` **消费** self、重 brand 到 `'other`（内插 `record`+`wait_event`）；非同 brand 的 buffer 不能用于他 stream 操作 | 缺 `share_with` 直接跨 stream 用 → 生命周期 / `Send` 约束错误（`E0277` / lifetime）或 `E0599` |

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：本设计**不触 AGENTS 硬规则 5 / 10 §7.5 禁区**。三规则以**纯类型级 affine + 生成式生命周期 brand + rustc 原生诊断**表达，**不扩展 rustc/MIR 借用检查器、不新增内存模型映射（06 §4.2 以 affine 所有权 + 确定性诊断表述，严禁 UB 节）**。`cuMemAllocAsync`/`cuMemFreeAsync`/`cuMemPool*` 是**稳定 CUDA Driver API**（D-113）薄层绑定，与 M8.3 已落的 `cuEvent*`/`cuMemcpy*Async`（Direct 量级）同类——**非新外部 ABI 契约**，不属「FFI ABI」禁区。
- **非 Direct**：`G1_CONTRACT` YAML 头第 8 行**显式**把「AsyncBuffer API 具体形态」列为 G1 执行期新决策面（`share_with` 时序边 API 形态为新增公共面）；AGENTS 硬规则 8「判档争议向上取严」+ M8.3 对其**自身新决策面**（跨线程转移）标 **Mini** 的先例 → 走一页 Mini-RFC + 失败测试先行 + agent 批准。
- **升档触发条件（实现期守卫）**：若实现期发现三规则**无法以纯类型拦截**而确需 MIR 借用检查扩展（stream-region 分析）/ 内存模型映射 / 安全包络扩展，则**停手升 Full RFC**（向上取严），不在 spec/impl 自行落笔。

## 4. 错误码

**零新 RX 码**（对齐 M8.3 RXS-0134 先例）：三类生命周期错误由 **Rust 类型系统原生编译期拦截**（affine move `E0382` / 无读接口 `E0599` / 跨 stream brand 失配 `E0277`·lifetime），`registry/error_codes.json` 与 `en.messages` 零追加。若实现期某类别确需运行期诊断段位码，则按 14 §4 + RX 段位制处置并停手标注，不预造。

## 5. 失败测试先行（10 §3 Mini 硬性）

`src/rurix-rt/compile-fail/async_buffer_cross_stream_unsync.rs`：编码规则③意图（跨 stream 缺 `share_with` 须被拒）。当前 `origin/main` 上该 fixture **RED**（`AsyncBuffer`/`share_with` 尚不存在）；PR-3 落地类型后，由步骤 42 host 段断言 rustc 以 `E0599` 拒绝（应拦截却放行即红）。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**：纯追加。M8.3 `InFlight`/`DeviceBox` 既有语义面 **0-byte**（仅扩 `AsyncBuffer` 缺口）。默认 workspace 网不依赖 device 而绿。**实现细化（2026-06-19）**：镜像 `InFlight` 先例，`AsyncBuffer`/`AsyncReady` 类型面随 `rurix-rt` **始终编译**（无可选依赖，区别于 G1.1 因 `rurix-d3d12` C++ 依赖而 feature 门控）——默认 `cargo build/clippy/test --workspace` 全覆盖该面且不依赖 device 而绿（device 仅运行期检测，无 GPU / 老驱动无 `cuMemAllocAsync` → `DriverUnavailable` 降级 SKIP），故**无需** `async_buffer`/`async_buffer-real` feature。「不依赖 device 而绿」核心承诺不变，覆盖更强（始终参与默认回归网）。
- **回归**：device 路径纳入既有 Compute Sanitizer racecheck+memcheck nightly（CUDA.jl #780 use-after-free 事故类**永久回归项**，PR-4）。
- **范围红线**：不做 VMM（`cuMemAddressReserve` 族，G2）/ 多 GPU；Graph API 仅 spike report（PR-4），立项与否 agent 裁决留痕，AI 不自行立项。

## 7. Agent 批准

> **Approved — 2026-06-19**。agent 于本工作会话经 AskUserQuestion 明确批准本 Mini-RFC 全文（§2 API 形态 `alloc_async`/`share_with` + §3 判档 Mini + §4 零新 RX 码 + §6 范围），并授权续建 PR-2（spec RXS-0144~0148）/ PR-3（实现 + 步骤 42）/ PR-4（spike + nightly）。批准记录由 claude-code 代录，**非 AI 代签 / 自行裁决**（AGENTS 硬规则 1）。device 真跑 / 证据回填 / 计数器兑现 / Graph API 立项与 SG-### 登记仍由 agent 自主签署。
