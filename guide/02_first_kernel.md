# 02 · 第一个 kernel

> API 收敛期(RD-008):语法可能随版本变化。本章的 kernel 定义示例经 CI 门 `rx check` 真跑;launch 与运行时部分为参考片段(需包上下文/运行时,链接到既有 conformance 语料)。

经典入门 kernel:**SAXPY**,即 `y = a·x + y`。

## kernel 定义

唯一事实源:[`conformance/tutorial/04_first_kernel.rx`](../conformance/tutorial/04_first_kernel.rx)

```rurix
kernel fn saxpy<const N: usize>(
    grid: Grid<(N,)>,
    a: f32,
    x: View<global, f32, (N,)>,
    y: ViewMut<global, f32, (N,)>,
) {
    let i = grid.thread_index();
    y[i] = a * x[i] + y[i];
}
```

逐项拆解——这三件套是 Rurix kernel 的核心:

- **`kernel fn`**:GPU 入口着色(RXS-0014)。
- **`Grid<(N,)>`**:**类型化执行形**。`(N,)` 是一维网格,`grid.thread_index()` 给出当前线程的索引。执行层级(grid/block/thread)是类型系统一等公民,而非运行时约定。
- **`View<global, f32, (N,)>` / `ViewMut<global, f32, (N,)>`**:设备内存视图。三个类型参数依次是**地址空间**(`global`)、**元素类型**(`f32`)、**形状**(`(N,)`)。地址空间是**类型参数**而非运行时值——跨地址空间误用会变成编译错误。`View` 只读,`ViewMut` 可写。
- `<const N: usize>`:const 泛型,让 kernel 对网格规模通用。

把它跑过静态检查(文件含一个最小 `main`,故也能 `rx check`/`rx run`):

```sh
cargo run -p rx -- check conformance/tutorial/04_first_kernel.rx   # 退出 0
```

## launch(参考片段)

把 kernel 真正发射到流上需要**运行时上下文**(`Stream`/`Buffer` 等),这部分需要包上下文才能解析,故作为参考片段。完整可解析样例见 [`conformance/launch/accept/saxpy_launch.rx`](../conformance/launch/accept/saxpy_launch.rx):

```rurix
kernel fn saxpy(out: ViewMut<global, f32>, x: View<global, f32>, n: usize, t: ThreadCtx<1>) {
    let i = t.global_id();
    if i < n {
        out[i] = x[i];
    }
}

fn run<C>(stream: Stream<C>, out: Buffer<C, f32>, x: Buffer<C, f32>, n: usize) {
    stream.launch(saxpy, GridDim(n), BlockDim(n), (out, x, n));
}
```

> 注意:conformance 语料里 kernel 有两种等价的执行形写法——`Grid<(N,)>` + `grid.thread_index()`(上方教程示例)与 `ThreadCtx<1>` + `t.global_id()`(此 launch 样例)。学习时各取其一即可,**不要混用**。

launch 的类型契约(grid/block 维度一致性、实参元素类型匹配、单一 context brand)由编译器静态校验(RXS-0074 launch 类型契约 / RXS-0075 launch 诊断要求)。`Stream<C>` / `Buffer<C, f32>` 里的 `C` 是 **context brand**,把流与缓冲绑定到同一上下文——这正是下一章资源生命周期安全的基础。

---

下一步 → [03 资源生命周期](03_resources.md)

深入参考:`spec/device.md`(kernel 着色/地址空间/执行形)、`spec/types.md`(View 家族)、launch 契约见 `spec` 中 RXS-0074/0075。
