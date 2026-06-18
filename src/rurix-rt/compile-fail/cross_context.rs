//@ compile-fail — RXS-0141 跨 context:`scope` 以 `for<'ctx>` 生成不可逃逸的不变 brand。
//   把携 `'ctx` brand 的 `ReadyFrame<'ctx>` 作为 scope 返回值 R 逃出闭包 → `'ctx` 无法
//   统一到外部具体生命周期 → rustc 生命周期错误（"lifetime may not live long enough"）。
//   资源不可逃逸 scope / 不可跨 context 混用,编译期拦截;若编译通过即红。
//   需 rurix-rt 以 --features d3d12-interop 构建。
use rurix_rt::interop::scope;

fn main() {
    // R 不可依赖 for<'ctx> 生成的 'ctx——返回 ReadyFrame<'ctx> 即编译错误。
    let _ = scope(0, [2, 2], [2, 2], |_cx, ready| Ok(ready));
}
