# 03 · 资源生命周期

> API 收敛期(RD-008):语法可能随版本变化。本章为**走读章**:运行时资源类型需要包上下文与真实设备,故代码以参考片段呈现,链接到既有 conformance 语料(已受解析门约束)与 `src/uc02-demo` 真实演示。

Rurix 把 GPU 资源的生命周期错误从**运行时崩溃**变成**编译错误**。这是它相对 CUDA C++ 最核心的安全增量。

## affine 资源类型

`Context`、`Stream`、`Event`、`Buffer` 都是 **affine 类型**(move-only、RAII):一旦被移动或销毁就不能再用。配合 **context brand 生命周期**,编译器能静态拒绝一整类生命周期 bug。

参考片段(完整可解析样例见 [`conformance/syntax/buffers_context.rx`](../conformance/syntax/buffers_context.rx)):

```rurix
fn setup() -> Result<(), Error> {
    let dev: Device = Device::enumerate()?.first()?;
    let ctx: Context = dev.create_context()?;
    let stream: Stream<'_> = ctx.create_stream()?;
    let buf: DeviceBuffer<f32> = ctx.alloc::<f32>(1 << 20)?;
    let pinned: PinnedBuffer<f32> = ctx.alloc_pinned::<f32>(1 << 20)?;
    stream.copy_to_device(&pinned, &buf)?;
    stream.synchronize_and_destroy()?;
    Ok(())
}
```

- `Device::enumerate()` 由真实设备探测驱动(strict-only:能力位来自实测,不静默降级)。
- `ctx.create_stream()` 产出的 `Stream<'_>` 带一条 brand 生命周期,把流绑定到该 context;`alloc` 出的 buffer 同样带 brand。
- `?` 传播 `Result`,资源在作用域结束或显式 `synchronize_and_destroy()` 时按 affine 规则回收。

## 四类被编译期拦截的错误

UC-02(三 stream 重叠流水线)把以下四类资源生命周期错误**全部变成编译错误**:

| 错误类别 | 现状(CUDA C++)| Rurix |
|---|---|---|
| use-after-free(流序分配后过早释放再用) | 运行时炸 | 编译错误(affine 已移动) |
| double-free | 运行时未定义 | 编译错误 |
| 跨线程误用(如跨线程 `cuCtxDestroy`) | 运行时炸 | 编译错误(brand + Send 约束) |
| 跨流未同步(读未就绪结果) | 数据竞争 | 编译错误(事件/typestate) |

## 真实演示:UC-02 三流水线

旗舰用例 UC-02 的端到端实现见 [`src/uc02-demo`](../src/uc02-demo)。它演示:

- **单线程三流重叠**:H2D 上传 / compute / D2H 下载三条 `Stream` 经事件(`record_event`/`wait_event`)重叠。
- **typestate 流水**:`InFlight` → `acquire`(等事件、rebrand)→ 使用 → `commit`(记录事件),把"未同步就读"挡在编译期。
- **跨线程所有权转移**:设备缓冲随 `Send` 上下文移动到 worker 线程,跨线程生命周期由类型系统保证。

> 想看"违反即编译错误"的反例,参考 `conformance/launch/reject/` 等 reject 语料:故意写错的样例附带期望诊断码。

---

进阶章节(views/地址空间细则、shared memory/stdlib、UC-02 流水线逐行走读、UC-03 SPH+软光栅)随后续 PR 补齐。

深入参考:`spec/pipeline.md`(stream/event/ownership 语义)、`05_LANGUAGE_ARCHITECTURE.md`(affine 资源与 brand 生命周期)、`08_RUNTIME_AND_TOOLING.md`(execution resources)。
